import { completeDependencyGraphLabel, connectionPanel, pendingBar } from "../main";
import type { BoardSnapshot, DependencyGraph, GraphNode, IssueCard, IssueDetail, IssueDocumentState, IssueLink, ShellCopy, Snapshot, TriageRole } from "../protocol";
import { currentProject, mobileClient } from "../view-helpers";
import { editableIssueDraft, editableIssueRelations, formFeedback, issueBlockersFormKey, issueCommentFormKey, issueCreateFormKey, issueEditFormKey, issueOpenFormKey, issueOptionLabel, issueOptionList, issueParentFormKey, issueSearchFormKey } from "../form-keys";
import { escapeHtml, formatCountdown, formatTime, renderMarkdown } from "../client-utils";
import { loopbackNotice } from "./run";
import { panelIsFloating, panelWidth, workbenchIssuePanel } from "../workbench";
import { ui } from "../ui";
import { GRAPH_RELATION_META } from "../graph-meta";

export function projectMain(copy: ShellCopy, snap: Snapshot, reuseGraphCanvas = false): string {
  const project = currentProject(snap);
  if (!project) return loopbackNotice(snap.loopbackPage);
  return `<div class="project-board">
    ${loopbackNotice(snap.loopbackPage)}
    <div class="board-head">
      <div class="board-head-row">
        <div class="project-heading">
          <h1>${escapeHtml(project.name)}</h1>
          <p title="${escapeHtml(project.localPath)}">${escapeHtml(project.githubHost)}/${escapeHtml(project.repository)}</p>
        </div>
        <div class="board-head-actions">
          <button type="button" class="primary" data-act="new-issue" data-id="${escapeHtml(project.id)}">${escapeHtml(copy.createIssue)}</button>
        </div>
      </div>
    </div>
    ${refreshBar(copy, snap.board)}
    ${issueSearch(copy, snap)}
    ${createIssueForm(copy, snap)}
    ${pendingBar(copy, snap)}
    ${connectionPanel(copy, project)}
    ${boardView(copy, snap, reuseGraphCanvas)}
  </div>`;
}

export function issueSearch(copy: ShellCopy, snap: Snapshot): string {
  const search = ui.issueSearchDraft?.projectId === snap.focusedProjectId
    ? ui.issueSearchDraft
    : snap.board?.search ?? { title: "", triageRole: null, state: "all" as const };
  const key = issueSearchFormKey(snap.focusedProjectId);
  const pending = ui.formOperations.pending.has(key);
  const triageRoles: TriageRole[] = [
    "needs-triage",
    "needs-info",
    "ready-for-agent",
    "ready-for-human",
    "wontfix",
  ];
  return `<form class="issue-search" data-act="issue-search" aria-busy="${pending ? "true" : "false"}">
    <label class="sr-only" for="issue-title-search">${escapeHtml(copy.searchTitle)}</label>
    <input id="issue-title-search" name="title" type="search" value="${escapeHtml(search.title)}" placeholder="${escapeHtml(copy.searchPlaceholder)}" ${pending ? "disabled" : ""} />
    <select name="triageRole" aria-label="${escapeHtml(copy.searchAllTriage)}" ${pending ? "disabled" : ""}>
      <option value="">${escapeHtml(copy.searchAllTriage)}</option>
      ${triageRoles.map((role) => `<option value="${role}" ${search.triageRole === role ? "selected" : ""}>${role}</option>`).join("")}
    </select>
    <select name="state" aria-label="${escapeHtml(copy.searchAllStates)}" ${pending ? "disabled" : ""}>
      <option value="all" ${search.state === "all" ? "selected" : ""}>${escapeHtml(copy.searchAllStates)}</option>
      <option value="open" ${search.state === "open" ? "selected" : ""}>${escapeHtml(copy.searchOpen)}</option>
      <option value="closed" ${search.state === "closed" ? "selected" : ""}>${escapeHtml(copy.searchClosed)}</option>
    </select>
    <button type="submit" ${pending ? "disabled" : ""}>${escapeHtml(pending ? copy.operationPending : copy.searchSubmit)}</button>
    <button type="button" data-act="keyboard-help" aria-label="${escapeHtml(copy.keyboardHelp)}">?</button>
    ${formFeedback(key)}
  </form>`;
}

export function createIssueForm(copy: ShellCopy, snap: Snapshot): string {
  if (!ui.createIssueOpen || ui.createIssueProjectId !== snap.focusedProjectId) return "";
  const key = issueCreateFormKey(ui.createIssueProjectId);
  const pending = ui.formOperations.pending.has(key);
  return `<section class="issue-editor issue-create-editor">
    <form data-act="issue-create" data-form="issue-create" aria-busy="${pending ? "true" : "false"}">
      <div class="issue-editor-title"><h2>${escapeHtml(copy.createIssue)}</h2></div>
      <label class="label" for="issue-create-title">${escapeHtml(copy.issueTitle)}</label>
      <input id="issue-create-title" name="title" required maxlength="240" value="${escapeHtml(ui.createIssueDraft.title)}" ${pending ? "disabled" : ""} />
      <label class="label" for="issue-create-body">${escapeHtml(copy.issueBody)}</label>
      <textarea id="issue-create-body" name="body" rows="5" ${pending ? "disabled" : ""}>${escapeHtml(ui.createIssueDraft.body)}</textarea>
      ${formFeedback(key)}
      <div class="actions">
        <button type="button" data-act="cancel-new-issue" ${pending ? "disabled" : ""}>${escapeHtml(copy.cancel)}</button>
        <button type="submit" class="primary" ${pending ? "disabled" : ""}>${escapeHtml(pending ? copy.operationPending : copy.createIssue)}</button>
      </div>
    </form>
  </section>`;
}

export function boardView(copy: ShellCopy, snap: Snapshot, reuseGraphCanvas = false): string {
  const board = snap.board;
  if (board?.empty === "incomplete-read" || board?.empty === "tracker-error") {
    const detail = board.refresh.kind === "incomplete" || board.refresh.kind === "tracker-error"
      ? board.refresh.detail
      : null;
    const message = board.empty === "tracker-error" ? copy.emptyTrackerError : copy.emptyIncomplete;
    return `<div class="board-empty" data-empty="${board.empty}">
      <b>${escapeHtml(message)}</b>
      ${detail ? `<p>${escapeHtml(detail)}</p>` : ""}
    </div>`;
  }
  if (!board || board.empty === "no-data" || !board.columns) {
    return `<div class="board-empty">${escapeHtml(copy.emptyNoData)}</div>`;
  }
  const onGraph = snap.centerView === "graph";
  const hint = onGraph ? copy.graphHint : board.parentFilter ? copy.childHint : "";
  const inspectorOpen = ui.issueDetailVisible && Boolean(board.selected);
  const inspectorFloating = panelIsFloating("inspector");
  const inspectorWidth = panelWidth("inspector");
  return `<div class="board-shell ${inspectorOpen ? "" : "issue-collapsed"} ${inspectorFloating ? "inspector-floating" : "inspector-docked"}" data-center-view="${onGraph ? "graph" : "board"}" style="--issue-detail-width:${Math.round(inspectorWidth)}px;--inspector-panel-width:${Math.round(inspectorWidth)}px">
    <div class="board-main">
      ${hint || board.parentFilter
        ? `<div class="board-hint">
            ${escapeHtml(hint)}
            ${board.parentFilter
              ? `<button type="button" data-act="clear-filter">${escapeHtml(copy.clearFilter)}</button>`
              : ""}
          </div>`
        : ""}
      ${onGraph ? dependencyGraphView(copy, board, reuseGraphCanvas) : boardLanes(copy, board)}
    </div>
    ${inspectorOpen ? workbenchIssuePanel(copy, board) : ""}
  </div>`;
}

export function boardLanes(copy: ShellCopy, board: BoardSnapshot): string {
  const desktop: Array<["blocked" | "frontier" | "inProgress" | "recentlyCompleted", string, IssueCard[]]> = [
    ["blocked", copy.colBlocked, board.columns?.blocked ?? []],
    ["frontier", copy.colFrontier, board.columns?.frontier ?? []],
    ["inProgress", copy.colInProgress, board.columns?.inProgress ?? []],
    ["recentlyCompleted", copy.colRecent, board.columns?.recentlyCompleted ?? []],
  ];
  const cols = mobileClient()
    ? [desktop[2], desktop[1], desktop[0], desktop[3]] as typeof desktop
    : desktop;
  return `<div class="lanes">
    ${cols
      .map(([key, name, items]) => {
        const empty =
          items.length > 0
            ? ""
            : key === "frontier"
              ? frontierEmptyText(copy, board.frontierEmpty)
              : key === "recentlyCompleted"
                ? copy.noRecent
                : copy.noItems;
        return `<section class="lane" data-lane="${key}">
          <div class="lane-hd">${escapeHtml(name)} <span>${items.length}</span></div>
          ${items.map((issue) => issueCard(copy, issue, board.selected?.id, key)).join("")}
          ${items.length ? "" : `<div class="lane-empty">${escapeHtml(empty)}</div>`}
        </section>`;
      })
      .join("")}
  </div>`;
}

export function dependencyGraphView(copy: ShellCopy, board: BoardSnapshot, reuseCanvas: boolean): string {
  const graph = board.graph;
  if (!graph) {
    return `<div class="board-empty">${escapeHtml(copy.emptyNoData)}</div>`;
  }
  const overview = graph.mode === "overview";
  const legacyGraph = graph.mode == null && graph.centerId == null;
  const canvasNodeLimit = overview ? graph.nodes.length : ui.graphCanvasLimit;
  const projectedNodes = [...graph.nodes]
    .sort((a, b) =>
      (a.distance ?? 0) - (b.distance ?? 0) ||
      a.rank - b.rank ||
      (overview ? b.number - a.number : a.number - b.number),
    )
    .slice(0, canvasNodeLimit);
  const columns = new Map<number, GraphNode[]>();
  for (const node of projectedNodes) {
    const list = columns.get(node.rank) ?? [];
    list.push(node);
    columns.set(node.rank, list);
  }
  const ranks = [...columns.keys()].sort((a, b) => a - b);
  const center = graph.nodes.find((node) => node.id === graph.centerId);
  const totalCount = graph.totalCount ?? graph.nodes.length;
  const centerLabel = copy.graphCenter.replace(
    "{issue}",
    center ? `#${center.number} ${center.title}` : graph.centerId ?? "—",
  );
  const completeLabel = copy.graphShowComplete.replace("{count}", String(totalCount));
  const canvasLimit = copy.graphCanvasLimit
    .replace("{shown}", String(projectedNodes.length))
    .replace("{total}", String(graph.nodes.length));
  const truncated = copy.graphTruncated
    .replace("{shown}", String(graph.nodes.length))
    .replace("{total}", String(totalCount));
  return `<div class="dep-graph">
    ${legacyGraph
      ? `<label class="graph-opt">
          <input type="checkbox" data-field="closedContext" ${board.showClosedGraphContext ? "checked" : ""} />
          ${escapeHtml(completeDependencyGraphLabel(copy, graph))}
        </label>`
      : `<div class="graph-toolbar" data-graph-mode="${overview ? "overview" : "focused"}">
          <span class="graph-center-label">${escapeHtml(overview ? copy.graphOverview : centerLabel)}</span>
          <div class="actions">
            ${overview
              ? ""
              : `<button type="button" data-act="graph-overview">${escapeHtml(copy.graphReturnOverview)}</button>
                ${graph.complete
                  ? `<button type="button" data-act="graph-neighborhood">${escapeHtml(copy.graphShowNeighborhood)}</button>`
                  : totalCount > graph.nodes.length
                    ? `<button type="button" data-act="graph-complete">${escapeHtml(completeLabel)}</button>`
                    : ""}`}
          </div>
        </div>
        ${graph.truncated ? `<div class="graph-limit graph-truncated">${escapeHtml(truncated)}</div>` : ""}
        ${projectedNodes.length < graph.nodes.length
          ? `<div class="graph-limit"><span>${escapeHtml(canvasLimit)}</span><button type="button" data-act="graph-more">${escapeHtml(copy.graphShowMore)}</button></div>`
          : ""}
        ${graph.edges.length === 0 ? `<div class="graph-empty-dependencies">${escapeHtml(copy.graphNoDependencies)}</div>` : ""}`}
    ${reuseCanvas
      ? `<div class="graph-canvas" data-preserve-graph-canvas></div>`
      : `<div class="graph-canvas">
      <svg class="graph-edges" aria-hidden="true"></svg>
      <div class="graph-flow">
        ${ranks
          .map(
            (rank) =>
              `<div class="graph-col" data-rank="${rank}">${(columns.get(rank) ?? [])
                .map((node) =>
                  graphNode(
                    copy,
                    node,
                    board.selected?.id,
                    legacyGraph ? null : graph.centerId ?? null,
                    overview,
                  ),
                )
                .join("")}</div>`,
          )
          .join("")}
      </div>
    </div>`}
    ${!legacyGraph && graph.complete ? dependencyGraphIndex(copy, graph) : ""}
  </div>`;
}

export function dependencyGraphIndex(copy: ShellCopy, graph: DependencyGraph): string {
  const query = ui.graphListQuery.trim().toLowerCase();
  const matches = graph.nodes
    .filter((node) =>
      !query ||
      node.title.toLowerCase().includes(query) ||
      node.id.toLowerCase().includes(query) ||
      `#${node.number}`.includes(query),
    )
    .sort((a, b) =>
      graphRelationMeta(a.relation).order - graphRelationMeta(b.relation).order ||
      (a.distance ?? 0) - (b.distance ?? 0) ||
      a.number - b.number,
    );
  const visible = matches.slice(0, ui.graphListLimit);
  return `<details class="graph-index" open>
    <summary>${escapeHtml(copy.graphCompleteList)} <span>${matches.length}</span></summary>
    <input id="dependency-graph-search" type="search" data-field="graphSearch" value="${escapeHtml(ui.graphListQuery)}" placeholder="${escapeHtml(copy.graphSearchPlaceholder)}" />
    <div class="graph-index-list">
      ${visible.map((node) => graphIndexRow(copy, node, graph.centerId ?? "")).join("")}
    </div>
    ${visible.length < matches.length
      ? `<button type="button" class="graph-index-more" data-act="graph-list-more">${escapeHtml(copy.graphShowMore)}</button>`
      : ""}
  </details>`;
}

export function graphRelationMeta(relation: GraphNode["relation"]): (typeof GRAPH_RELATION_META)["center"] {
  return GRAPH_RELATION_META[relation ?? "center"];
}

export function graphIndexRow(copy: ShellCopy, node: GraphNode, centerId: string): string {
  return `<div class="graph-index-row ${node.open ? "" : "closed"}">
    <button type="button" class="graph-index-main" data-act="focus-issue" data-id="${escapeHtml(node.id)}">
      <span class="graph-relation">${escapeHtml(graphRelationMeta(node.relation).label(copy))}</span>
      <span class="issue-id">#${node.number}</span>
      <span class="issue-title">${escapeHtml(node.title)}</span>
    </button>
    ${node.id === centerId ? "" : graphCenterButton(copy, node)}
  </div>`;
}

export function graphNode(
  copy: ShellCopy,
  node: GraphNode,
  selectedId: string | undefined,
  centerId: string | null,
  overview = false,
): string {
  const selected = node.id === selectedId ? "sel" : "";
  const closed = node.open ? "" : "closed";
  const center = node.id === centerId ? "root" : "";
  return `<article class="graph-node ${selected} ${closed} ${center}" data-id="${escapeHtml(node.id)}">
    <button type="button" class="graph-node-main" data-act="${overview ? "center-graph" : "focus-issue"}" data-id="${escapeHtml(node.id)}">
      <div class="issue-id">#${node.number}</div>
      <div class="issue-title">${escapeHtml(node.title)}</div>
    </button>
    ${center || centerId == null ? "" : graphCenterButton(copy, node)}
  </article>`;
}

export function graphCenterButton(copy: ShellCopy, node: GraphNode): string {
  const label = `${copy.graphCenterHere} #${node.number}`;
  return `<button type="button" class="graph-center-act" data-act="center-graph" data-id="${escapeHtml(node.id)}" aria-label="${escapeHtml(label)}" title="${escapeHtml(label)}">
    ${escapeHtml(copy.graphCenterHere)}
  </button>`;
}

export function issuePanelIcon(open: boolean): string {
  const chevron = open ? "M13 9l3 3-3 3" : "M16 9l-3 3 3 3";
  return `<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
    <rect x="3" y="4" width="18" height="16" rx="2"></rect>
    <path d="M10 4v16"></path>
    <path d="${chevron}"></path>
  </svg>`;
}

export function frontierEmptyText(
  copy: ShellCopy,
  reason: BoardSnapshot["frontierEmpty"],
): string {
  if (reason === "all-blocked") return copy.noFrontierBlocked;
  if (reason === "all-claimed") return copy.noFrontierClaimed;
  if (reason === "no-open") return copy.noFrontierEmpty;
  return copy.noItems;
}

export function issueActivityLabel(copy: ShellCopy, activity: IssueCard["activity"]): string {
  if (activity === "waiting") return copy.waiting;
  if (activity === "execution-stopped") return copy.executionStopped;
  if (activity === "running") return copy.running;
  return "";
}

export function issueMetadataTags(labels: string[] | undefined, includeStatus = true): string {
  return (labels ?? [])
    .filter((label) => label.startsWith("type:") || (includeStatus && label.startsWith("status:")))
    .map((label) => {
      const [kind, ...rest] = label.split(":");
      return `<span class="tag">${escapeHtml(`${kind === "type" ? "Type" : "Status"}: ${rest.join(":")}`)}</span>`;
    })
    .join("");
}

export function issueCard(
  copy: ShellCopy,
  issue: IssueCard,
  selectedId: string | undefined,
  lane: "blocked" | "frontier" | "inProgress" | "recentlyCompleted",
): string {
  const activity = issueActivityLabel(copy, issue.activity);
  const tags = [
    activity ? `<span class="tag">${escapeHtml(activity)}</span>` : "",
    issue.triageRole ? `<span class="tag">${escapeHtml(issue.triageRole)}</span>` : "",
    issueMetadataTags(issue.labels),
    issue.claimedBy.length
      ? `<span class="tag">${escapeHtml(copy.claimed)} ${escapeHtml(issue.claimedBy.join(", "))}</span>`
      : "",
  ]
    .filter(Boolean)
    .join("");
  const cardAction = "focus-issue";
  const actionTargetId = issue.id;
  const actions = lane === "frontier"
    ? `<button type="button" class="primary" data-act="execute-run" data-id="${escapeHtml(issue.id)}">${escapeHtml(copy.executeRun)}</button>`
    : lane === "inProgress" && issue.runId
      ? `<button type="button" data-act="focus-run" data-id="${escapeHtml(issue.runId)}">${escapeHtml(copy.focusRun)}</button>
         <button type="button" data-act="stop-run" data-id="${escapeHtml(issue.runId)}">${escapeHtml(copy.stopRun)}</button>
         ${mobileClient() ? "" : `<button type="button" data-act="view-changes" data-id="${escapeHtml(issue.runId)}">${escapeHtml(copy.viewChanges)}</button>`}`
      : lane === "recentlyCompleted"
        ? `${!mobileClient() && issue.runId ? `<button type="button" data-act="view-changes" data-id="${escapeHtml(issue.runId)}">${escapeHtml(copy.viewChanges)}</button>` : ""}
           <button type="button" data-act="open-issue" data-url="${escapeHtml(issue.url)}">${escapeHtml(copy.openIssue)}</button>`
        : "";
  return `<article class="issue-card ${issue.id === selectedId ? "sel" : ""} ${issue.activity ? escapeHtml(issue.activity) : ""} ${lane === "recentlyCompleted" ? "recently-completed subdued" : ""}" data-issue-id="${escapeHtml(issue.id)}">
    <button type="button" class="issue-card-main" data-act="${cardAction}" data-id="${escapeHtml(actionTargetId)}" data-issue-id="${escapeHtml(issue.id)}">
      <div class="issue-id">#${issue.number}</div>
      <div class="issue-title">${escapeHtml(issue.title)}</div>
      ${tags ? `<div class="issue-tags">${tags}</div>` : ""}
    </button>
    ${actions ? `<div class="issue-card-actions">${actions}</div>` : ""}
  </article>`;
}

export function issueDetail(copy: ShellCopy, board: BoardSnapshot, showPanelToggle = true): string {
  const issue = board.selected;
  if (!issue) {
    return `<div class="lane-empty">${escapeHtml(copy.pickIssue)}</div>`;
  }
  const claim = issue.claimedBy.length
    ? `${copy.claimed} ${issue.claimedBy.join(", ")}`
    : "";
  const hasActive = Boolean(issue.activeRunId) || (ui.snapshot?.runs ?? []).some(
    (run) => run.issueId === issue.id && run.status !== "ended",
  );
  const actions = hasActive
    ? ""
    : issue.executionStopped
      ? `<button type="button" class="primary" data-act="continue-run" data-id="${escapeHtml(issue.id)}">${escapeHtml(copy.continueRun)}</button>
         <button type="button" data-act="release-claim" data-id="${escapeHtml(issue.id)}">${escapeHtml(copy.releaseClaim)}</button>`
      : `<button type="button" class="primary" data-act="execute-run" data-id="${escapeHtml(issue.id)}">${escapeHtml(copy.executeRun)}</button>`;
  const canWrite = board.refresh.kind === "ready"
    && (!board.issueOptions.length || board.issueOptions.some((option) => option.id === issue.id));
  const canEdit = canWrite && issue.document.kind === "ready";
  const openKey = issueOpenFormKey(issue.id);
  const openPending = ui.formOperations.pending.has(openKey);
  const editOpen = ui.issueEditOpenId === issue.id;
  return `
    <header class="detail-sticky">
      <div class="detail-title-row">
        <div class="detail-hd">#${issue.number} ${escapeHtml(issue.title)}</div>
        ${showPanelToggle ? `<button type="button" class="chrome-icon detail-panel-toggle" data-act="toggle-issue" aria-label="${escapeHtml(copy.hideIssueDetail)}" title="${escapeHtml(copy.hideIssueDetail)}">${issuePanelIcon(true)}</button>` : ""}
      </div>
      <div class="detail-meta">
        ${issue.triageRole ? `<span class="tag">${escapeHtml(issue.triageRole)}</span>` : ""}
        ${issueMetadataTags(issue.labels, false)}
        ${claim ? `<span class="tag">${escapeHtml(claim)}</span>` : ""}
        ${issue.waitingForUser ? `<span class="tag">${escapeHtml(copy.waiting)}</span>` : ""}
        ${issue.executionStopped ? `<span class="tag">${escapeHtml(copy.executionStopped)}</span>` : ""}
        ${actions}
        ${canEdit ? `<button type="button" data-act="edit-issue" data-id="${escapeHtml(issue.id)}">${escapeHtml(copy.editIssue)}</button>` : ""}
        ${canWrite ? `
          <button type="button" data-act="toggle-issue-open" data-id="${escapeHtml(issue.id)}" ${openPending ? "disabled" : ""}>${escapeHtml(openPending ? copy.operationPending : issue.open ? copy.closeIssue : copy.reopenIssue)}</button>` : ""}
        <button type="button" data-act="open-issue" data-url="${escapeHtml(issue.url)}">${escapeHtml(copy.openIssue)}</button>
      </div>
      ${canWrite ? formFeedback(openKey) : ""}
    </header>
    <div class="detail-scroll">
      ${issueDocument(copy, issue.document ?? { kind: "unloaded" }, issue.url)}
      ${editOpen && canEdit ? issueEditForm(copy, issue) : ""}
      <section class="detail-block">
      <h4>${escapeHtml(copy.family)}</h4>
      <div class="tiny">${escapeHtml(copy.parent)}</div>
      ${issue.parent ? issueLink(copy, issue.parent) : `<span class="muted">${escapeHtml(copy.noParent)}</span>`}
      <div class="tiny">${escapeHtml(copy.children)}</div>
      ${
        issue.children.length
          ? issue.children.map((child) => issueLink(copy, child)).join("")
          : `<span class="muted">${escapeHtml(copy.noKids)}</span>`
      }
      ${
        issue.children.length
          ? `<div><button type="button" data-act="filter-parent" data-id="${escapeHtml(issue.id)}">${escapeHtml(copy.onlyKids)}</button></div>`
          : ""
      }
      </section>
      <section class="detail-block">
      <h4>${escapeHtml(copy.deps)}</h4>
      ${mobileClient() ? "" : `<button type="button" data-act="view-dependencies" data-id="${escapeHtml(issue.id)}">${escapeHtml(copy.viewDependencies)}</button>`}
      <div class="tiny">${escapeHtml(copy.blockedBy)}</div>
      ${
        issue.blockedBy.length
          ? issue.blockedBy.map((link) => issueLink(copy, link)).join("")
          : `<span class="muted">${escapeHtml(copy.noneBlock)}</span>`
      }
      <div class="tiny">${escapeHtml(copy.blocking)}</div>
      ${
        issue.blocking.length
          ? issue.blocking.map((link) => issueLink(copy, link)).join("")
          : `<span class="muted">${escapeHtml(copy.none)}</span>`
      }
      </section>
      ${canWrite
        ? `<details class="detail-block detail-maintenance" data-section="issue-maintenance" data-id="${escapeHtml(issue.id)}" ${ui.issueMaintenanceOpen.has(issue.id) ? "open" : ""}>
            <summary>${escapeHtml(copy.issueUpdates)}</summary>
            ${issueCommentForm(copy, issue)}
            ${issueRelationsForm(copy, board, issue)}
          </details>`
        : ""}
    </div>`;
}

export function issueEditForm(copy: ShellCopy, issue: IssueDetail): string {
  const key = issueEditFormKey(issue.id);
  const draft = editableIssueDraft(issue);
  const pending = ui.formOperations.pending.has(key);
  return `<section class="detail-block issue-editor issue-edit-editor">
    <form data-act="issue-edit" data-form="issue-edit" data-id="${escapeHtml(issue.id)}" aria-busy="${pending ? "true" : "false"}">
      <h4>${escapeHtml(copy.editIssue)}</h4>
      <label class="label" for="issue-edit-title">${escapeHtml(copy.issueTitle)}</label>
      <input id="issue-edit-title" name="title" required maxlength="240" value="${escapeHtml(draft.title)}" ${pending ? "disabled" : ""} />
      <label class="label" for="issue-edit-body">${escapeHtml(copy.issueBody)}</label>
      <textarea id="issue-edit-body" name="body" rows="8" ${pending ? "disabled" : ""}>${escapeHtml(draft.body)}</textarea>
      ${formFeedback(key)}
      <div class="actions">
        <button type="button" data-act="cancel-edit-issue" data-id="${escapeHtml(issue.id)}" ${pending ? "disabled" : ""}>${escapeHtml(copy.cancel)}</button>
        <button type="submit" class="primary" ${pending ? "disabled" : ""}>${escapeHtml(pending ? copy.operationPending : copy.saveIssue)}</button>
      </div>
    </form>
  </section>`;
}

export function issueCommentForm(copy: ShellCopy, issue: IssueDetail): string {
  const key = issueCommentFormKey(issue.id);
  const pending = ui.formOperations.pending.has(key);
  return `<section class="detail-block issue-editor issue-comment-editor">
    <form data-act="issue-comment" data-form="issue-comment" data-id="${escapeHtml(issue.id)}" aria-busy="${pending ? "true" : "false"}">
      <h4>${escapeHtml(copy.addComment)}</h4>
      <textarea name="body" rows="4" required maxlength="10000" placeholder="${escapeHtml(copy.commentPlaceholder)}" ${pending ? "disabled" : ""}>${escapeHtml(ui.issueCommentDrafts.get(issue.id) ?? "")}</textarea>
      ${formFeedback(key)}
      <div class="actions">
        <button type="submit" class="primary" ${pending ? "disabled" : ""}>${escapeHtml(pending ? copy.operationPending : copy.addComment)}</button>
      </div>
    </form>
  </section>`;
}

export function issueRelationsForm(copy: ShellCopy, board: BoardSnapshot, issue: IssueDetail): string {
  const parentKey = issueParentFormKey(issue.id);
  const blockersKey = issueBlockersFormKey(issue.id);
  const draft = editableIssueRelations(issue);
  const parentPending = ui.formOperations.pending.has(parentKey);
  const blockersPending = ui.formOperations.pending.has(blockersKey);
  const options = issueOptionList(board, issue);
  const parentOptions = options
    .map((option) => `<option value="${escapeHtml(option.id)}" ${draft.parent === option.id ? "selected" : ""}>${escapeHtml(issueOptionLabel(option))}</option>`)
    .join("");
  const blockerOptions = options
    .map((option) => `<option value="${escapeHtml(option.id)}" ${draft.blockedBy.includes(option.id) ? "selected" : ""}>${escapeHtml(issueOptionLabel(option))}</option>`)
    .join("");
  return `<section class="detail-block issue-editor issue-relations-editor">
    <h4>${escapeHtml(copy.family)} / ${escapeHtml(copy.deps)}</h4>
    <form data-act="issue-parent" data-form="issue-parent" data-id="${escapeHtml(issue.id)}" aria-busy="${parentPending ? "true" : "false"}">
      <label class="label" for="issue-parent">${escapeHtml(copy.parentIssue)}</label>
      <select id="issue-parent" name="parent" ${parentPending ? "disabled" : ""}>
        <option value="">${escapeHtml(copy.none)}</option>
        ${parentOptions}
      </select>
      ${formFeedback(parentKey)}
      <div class="actions">
        <button type="submit" class="primary" ${parentPending ? "disabled" : ""}>${escapeHtml(parentPending ? copy.operationPending : copy.saveRelations)}</button>
      </div>
    </form>
    <form data-act="issue-blockers" data-form="issue-blockers" data-id="${escapeHtml(issue.id)}" aria-busy="${blockersPending ? "true" : "false"}">
      <label class="label" for="issue-blocked-by">${escapeHtml(copy.dependencyBlockers)}</label>
      <select id="issue-blocked-by" name="blockedBy" multiple size="${Math.min(6, Math.max(3, options.length))}" ${blockersPending ? "disabled" : ""}>
        ${blockerOptions}
      </select>
      ${formFeedback(blockersKey)}
      <div class="actions">
        <button type="button" data-act="clear-issue-blockers" data-id="${escapeHtml(issue.id)}" ${blockersPending ? "disabled" : ""}>${escapeHtml(copy.clearDependency)}</button>
        <button type="submit" class="primary" ${blockersPending ? "disabled" : ""}>${escapeHtml(blockersPending ? copy.operationPending : copy.saveRelations)}</button>
      </div>
    </form>
  </section>`;
}

export function issueDocument(copy: ShellCopy, state: IssueDocumentState, issueUrl: string): string {
  if (state.kind === "unloaded" || state.kind === "loading") {
    const previous = state.kind === "loading" && state.body != null
      ? `<div class="issue-markdown is-stale">${renderMarkdown(state.body, issueUrl)}</div>`
      : "";
    const asOf = state.kind === "loading" && state.fetchedAtMs != null
      ? ` · ${escapeHtml(copy.refreshAsOf)} ${escapeHtml(formatTime(state.fetchedAtMs))}`
      : "";
    return `<section class="issue-document" data-document-state="${state.kind}" aria-busy="true">
      <div class="document-loading document-status" role="status" aria-live="polite">
        <span class="document-loading-dot" aria-hidden="true"></span>
        <span>${escapeHtml(copy.issueDocumentLoading)}</span>${asOf ? `<span class="document-loading-as-of">${asOf}</span>` : ""}
      </div>
      <div class="document-skeleton" aria-hidden="true"><i></i><i></i><i></i></div>
      ${previous}
    </section>`;
  }
  if (state.kind === "failed") {
    return `<section class="issue-document" data-document-state="failed">
      <p class="notice bad">${escapeHtml(copy.issueDocumentFailed)} ${escapeHtml(state.failure.message)}</p>
      <button type="button" data-act="retry-issue-document">${escapeHtml(copy.issueDocumentRetry)}</button>
    </section>`;
  }
  const stale = state.kind === "stale";
  return `<section class="issue-document" data-document-state="${state.kind}">
    ${stale
      ? `<p class="document-status stale">${escapeHtml(copy.issueDocumentStale)} ${escapeHtml(formatTime(state.fetchedAtMs))}. ${escapeHtml(state.failure.message)}</p>
         <button type="button" data-act="retry-issue-document">${escapeHtml(copy.issueDocumentRetry)}</button>`
      : ""}
    <div class="issue-markdown ${stale ? "is-stale" : ""}">${renderMarkdown(state.body, issueUrl)}</div>
  </section>`;
}

export function issueLink(copy: ShellCopy, link: IssueLink): string {
  if (!link.visible) {
    return `<span class="muted">${escapeHtml(copy.unclearIssue)}</span>`;
  }
  const label = `#${link.number ?? "?"} ${link.title}`.trim();
  return `<button type="button" class="name-btn" data-act="focus-issue" data-id="${escapeHtml(link.id)}">${escapeHtml(label)}</button>`;
}

export function refreshBar(copy: ShellCopy, board: BoardSnapshot | null): string {
  const status = board?.refresh ?? { kind: "never-fetched" as const };
  const kind = ui.refreshing ? "refreshing" : status.kind;
  const parts: string[] = [];
  if (kind === "refreshing") {
    parts.push(copy.refreshRefreshing);
  } else if (status.kind === "never-fetched") {
    parts.push(copy.refreshNever);
  } else if (status.kind === "offline") {
    parts.push(`${copy.refreshOffline} · ${copy.refreshAsOf} ${formatTime(status.fetchedAtMs)}`);
    parts.push(copy.refreshOfflineRecovery);
    if (status.nextRefreshInMs != null) {
      parts.push(`${copy.refreshNext} ${formatCountdown(status.nextRefreshInMs)}`);
    }
  } else if (status.kind === "rate-limited") {
    parts.push(copy.refreshRateLimited);
    if (status.retryAtMs) {
      parts.push(`${copy.refreshRetry} ${formatTime(status.retryAtMs)}`);
    } else {
      parts.push(copy.refreshPaused);
    }
    if (status.fetchedAtMs) {
      parts.push(`${copy.refreshAsOf} ${formatTime(status.fetchedAtMs)}`);
    }
  } else if (status.kind === "auth-failed") {
    parts.push(copy.refreshAuth);
    parts.push(copy.refreshAuthRecovery);
    if (status.fetchedAtMs) {
      parts.push(`${copy.refreshAsOf} ${formatTime(status.fetchedAtMs)}`);
    }
  } else if (status.kind === "incomplete" || status.kind === "tracker-error") {
    parts.push(status.kind === "tracker-error" ? copy.refreshTrackerError : copy.refreshIncomplete);
    if (status.detail) {
      parts.push(status.detail);
    }
    if (status.fetchedAtMs) {
      parts.push(`${copy.refreshAsOf} ${formatTime(status.fetchedAtMs)}`);
    }
    if (status.nextRefreshInMs != null) {
      parts.push(`${copy.refreshNext} ${formatCountdown(status.nextRefreshInMs)}`);
    }
  } else if (status.kind === "ready") {
    parts.push(`${copy.refreshAsOf} ${formatTime(status.fetchedAtMs)}`);
    if (status.nextRefreshInMs != null) {
      parts.push(`${copy.refreshNext} ${formatCountdown(status.nextRefreshInMs)}`);
    }
  }
  return `<div class="refresh-bar" data-kind="${escapeHtml(kind)}">
    <span>${escapeHtml(parts.join(" · "))}</span>
    <button type="button" data-act="refresh">${escapeHtml(copy.refreshNow)}</button>
  </div>`;
}
