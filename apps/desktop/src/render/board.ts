import { connectionPanel, pendingBar } from "../main";
import type { BoardSnapshot, DependencyGraph, FormKey, GraphNode, IssueCard, IssueDetail, IssueDocumentState, IssueLink, RunSummary, ShellCopy, Snapshot, TriageRole } from "../protocol";
import { currentProject, effectiveClientLanguage } from "../view-helpers";
import { issueDraftKey, editableIssueDraft, editableIssueRelations, formFeedback, issueBlockersFormKey, issueCommentFormKey, issueCreateFormKey, issueEditFormKey, issueOpenFormKey, issueOptionLabel, issueOptionList, issueParentFormKey, issueSearchFormKey } from "../form-keys";
import { escapeHtml, formatCountdown, formatTime, renderMarkdown } from "../client-utils";
import { loopbackNotice } from "./run";
import { fixedPanelResizeHandle, fixedPanelWidth, workbenchIssuePanel } from "../workbench";
import { ui } from "../ui";
import { GRAPH_RELATION_META } from "../graph-meta";
import { issueCard, issueIdentity, issueStateBadge, issueTags, type IssueCardAction, type IssueDisplayTag, type IssueLaneState } from "../components/issue";
import { button, formField, selectControl, textArea, textInput } from "../components/primitives";
import { refreshStatus } from "../components/refresh-status";

export function projectMain(copy: ShellCopy, snap: Snapshot, reuseGraphCanvas = false): string {
  const project = currentProject(snap);
  if (!project) return loopbackNotice(snap.loopbackPage);
  return `<div class="project-board">
    ${loopbackNotice(snap.loopbackPage)}
    <div class="content-toolbar" data-page-toolbar>
      <div class="board-head">
        <div class="board-head-row">
          <div class="project-heading">
            <h1>${escapeHtml(project.name)}</h1>
            <p title="${escapeHtml(project.localPath)}">${escapeHtml(project.githubHost)}/${escapeHtml(project.repository)}</p>
          </div>
          <div class="board-head-actions">
            <div class="view-switch" role="tablist">
              <button type="button" class="${snap.centerView === "board" ? "active" : ""}" data-act="center-view" data-id="board">${escapeHtml(copy.viewBoard)}</button>
              <button type="button" class="${snap.centerView === "graph" ? "active" : ""}" data-act="center-view" data-id="graph">${escapeHtml(copy.viewGraph)}</button>
            </div>
            <button type="button" class="primary" data-act="new-issue" data-id="${escapeHtml(project.id)}">${escapeHtml(copy.createIssue)}</button>
          </div>
        </div>
      </div>
      ${refreshBar(copy, snap.board)}
      ${issueSearch(copy, snap)}
    </div>
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
      ${formField({
        id: "issue-create-title",
        label: copy.issueTitle,
        required: true,
        control: textInput({ id: "issue-create-title", name: "title", value: ui.createIssueDraft.title, required: true, disabled: pending, attributes: { maxlength: 240 } }),
      })}
      ${formField({
        id: "issue-create-body",
        label: copy.issueBody,
        control: textArea({ id: "issue-create-body", name: "body", value: ui.createIssueDraft.body, rows: 5, disabled: pending }),
      })}
      ${formFeedback(key)}
      <div class="actions">
        ${button({ id: "cancel-new-issue", label: copy.cancel, disabled: pending })}
        ${button({ id: "submit-issue-create", label: pending ? copy.operationPending : copy.createIssue, disabled: pending, busy: pending }, { type: "submit", variant: "primary" })}
      </div>
    </form>
  </section>`;
}

export function boardView(copy: ShellCopy, snap: Snapshot, reuseGraphCanvas = false): string {
  const unavailable = boardUnavailable(copy, snap);
  if (unavailable) return unavailable;
  const board = snap.board!;
  const onGraph = snap.centerView === "graph";
  const hint = onGraph ? copy.graphHint : board.parentFilter ? copy.childHint : "";
  const inspectorOpen = ui.clientView.panels.rightSide === "rail" && Boolean(board.selected);
  const inspectorWidth = fixedPanelWidth("right-rail");
  return `<div class="board-shell ${inspectorOpen ? "" : "issue-collapsed"}" data-center-view="${onGraph ? "graph" : "board"}" style="--issue-detail-width:${Math.round(inspectorWidth)}px">
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

/** The desktop order: blocked, Frontier, in progress, recently completed. */
const DEFAULT_LANE_ORDER: IssueLaneState[] = ["blocked", "frontier", "inProgress", "recentlyCompleted"];

/**
 * The Board's own unreadable-data states, shared by every view that would draw its lanes.
 * Returns null only when `snap.board` carries drawable columns.
 */
export function boardUnavailable(copy: ShellCopy, snap: Snapshot): string | null {
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
  return null;
}

export function boardLanes(
  copy: ShellCopy,
  board: BoardSnapshot,
  order: IssueLaneState[] = DEFAULT_LANE_ORDER,
): string {
  const byLane: Record<IssueLaneState, [IssueLaneState, string, IssueCard[]]> = {
    blocked: ["blocked", copy.colBlocked, board.columns?.blocked ?? []],
    frontier: ["frontier", copy.colFrontier, board.columns?.frontier ?? []],
    inProgress: ["inProgress", copy.colInProgress, board.columns?.inProgress ?? []],
    recentlyCompleted: ["recentlyCompleted", copy.colRecent, board.columns?.recentlyCompleted ?? []],
  };
  const cols = order.map((lane) => byLane[lane]);
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
          ${items.map((issue) => laneCard(copy, issue, board.selected?.id, key)).join("")}
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
  const totalCount = graph.totalCount;
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
    <div class="graph-toolbar" data-graph-mode="${overview ? "overview" : "focused"}">
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
    ${graph.edges.length === 0 ? `<div class="graph-empty-dependencies">${escapeHtml(copy.graphNoDependencies)}</div>` : ""}
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
                    graph.centerId,
                    overview,
                  ),
                )
                .join("")}</div>`,
          )
          .join("")}
      </div>
    </div>`}
    ${graph.complete ? dependencyGraphIndex(copy, graph) : ""}
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
      ${issueIdentity(node)}
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
      ${issueIdentity(node)}
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

export function issueMetadataTags(labels: string[] | undefined, includeStatus = true): IssueDisplayTag[] {
  return (labels ?? [])
    .filter((label) => label.startsWith("type:") || (includeStatus && label.startsWith("status:")))
    .map((label) => {
      const [kind, ...rest] = label.split(":");
      return { label: `${kind === "type" ? "Type" : "Status"}: ${rest.join(":")}` };
    });
}

function laneActions(copy: ShellCopy, issue: IssueCard, lane: IssueLaneState): IssueCardAction[] {
  if (lane === "frontier") {
    return [{ action: { id: "execute-run", label: copy.executeRun, data: { id: issue.id } }, variant: "primary" }];
  }
  if (lane === "inProgress" && issue.runId) {
    return [
      { action: { id: "focus-run", label: copy.focusRun, data: { id: issue.runId } } },
      { action: { id: "stop-run", label: copy.stopRun, data: { id: issue.runId } } },
    ];
  }
  if (lane === "recentlyCompleted") {
    return [{ action: { id: "open-issue", label: copy.openIssue, data: { url: issue.url } } }];
  }
  return [];
}

function laneTags(copy: ShellCopy, issue: IssueCard): IssueDisplayTag[] {
  const activity = issueActivityLabel(copy, issue.activity);
  const tags: IssueDisplayTag[] = [];
  if (activity) {
    tags.push({ label: activity, tone: issue.activity === "execution-stopped" ? "danger" : "info" });
  }
  if (issue.triageRole) tags.push({ label: issue.triageRole });
  tags.push(...issueMetadataTags(issue.labels));
  if (issue.claimedBy.length) tags.push({ label: `${copy.claimed} ${issue.claimedBy.join(", ")}` });
  return tags;
}

function laneCard(
  copy: ShellCopy,
  issue: IssueCard,
  selectedId: string | undefined,
  lane: IssueLaneState,
): string {
  return issueCard({
    id: issue.id,
    identity: { number: issue.number, title: issue.title },
    state: lane,
    activity: issue.activity,
    selected: issue.id === selectedId,
    tags: laneTags(copy, issue),
    entry: { id: "focus-issue", label: issue.title, data: { id: issue.id } },
    actions: laneActions(copy, issue, lane),
  });
}

/** The page resolves which board column owns the selected Issue. */
export function selectedIssueLane(board: BoardSnapshot): IssueLaneState | null {
  const id = board.selected?.id;
  if (!id || !board.columns) return null;
  const lanes: Array<[IssueLaneState, IssueCard[]]> = [
    ["blocked", board.columns.blocked],
    ["frontier", board.columns.frontier],
    ["inProgress", board.columns.inProgress],
    ["recentlyCompleted", board.columns.recentlyCompleted],
  ];
  return lanes.find(([, items]) => items.some((item) => item.id === id))?.[0] ?? null;
}

function issueCanWrite(board: BoardSnapshot, issue: IssueDetail): boolean {
  return !board.issueOptions.length || board.issueOptions.some((option) => option.id === issue.id);
}

function issueActions(copy: ShellCopy, board: BoardSnapshot, issue: IssueDetail): string {
  const claim = issue.claimedBy.length ? `${copy.claimed} ${issue.claimedBy.join(", ")}` : "";
  const hasActive = Boolean(issue.activeRunId);
  const lane = selectedIssueLane(board);
  const primaryActions = hasActive
    ? ""
    : issue.executionStopped
      ? `<button type="button" class="primary" data-act="continue-run" data-id="${escapeHtml(issue.id)}">${escapeHtml(copy.continueRun)}</button>
         <button type="button" data-act="release-claim" data-id="${escapeHtml(issue.id)}">${escapeHtml(copy.releaseClaim)}</button>`
      : `<button type="button" class="primary" data-act="execute-run" data-id="${escapeHtml(issue.id)}">${escapeHtml(copy.executeRun)}</button>`;
  const canWrite = issueCanWrite(board, issue);
  const canStartEdit = canWrite && issue.document.kind === "ready";
  const openKey = issueOpenFormKey(issue.id);
  const openPending = ui.formOperations.pending.has(openKey);
  return `<div class="detail-meta">
    ${lane ? issueStateBadge(copy, lane) : ""}
    ${issue.triageRole ? `<span class="tag">${escapeHtml(issue.triageRole)}</span>` : ""}
    ${issueTags(issueMetadataTags(issue.labels, false))}
    ${claim ? `<span class="tag">${escapeHtml(claim)}</span>` : ""}
    ${issue.waitingForUser ? `<span class="tag">${escapeHtml(copy.waiting)}</span>` : ""}
    ${issue.executionStopped ? `<span class="tag">${escapeHtml(copy.executionStopped)}</span>` : ""}
    ${primaryActions}
    ${canStartEdit ? `<button type="button" data-act="edit-issue" data-id="${escapeHtml(issue.id)}">${escapeHtml(copy.editIssue)}</button>` : ""}
    ${canWrite ? `<button type="button" data-act="toggle-issue-open" data-id="${escapeHtml(issue.id)}" ${openPending ? "disabled" : ""}>${escapeHtml(openPending ? copy.operationPending : issue.open ? copy.closeIssue : copy.reopenIssue)}</button>` : ""}
    <button type="button" data-act="open-issue" data-url="${escapeHtml(issue.url)}">${escapeHtml(copy.openIssue)}</button>
  </div>${canWrite ? formFeedback(openKey) : ""}`;
}

function issueBody(copy: ShellCopy, board: BoardSnapshot, issue: IssueDetail, showDependencyGraph: boolean): string {
  const canWrite = issueCanWrite(board, issue);
  const canStartEdit = canWrite && issue.document.kind === "ready";
  const editOpen = ui.issueEditOpenIds.has(issueDraftKey(issue.id));
  const showEditForm = editOpen && (canStartEdit || ui.issueEditDrafts.has(issueDraftKey(issue.id)));
  return `${issueDocument(copy, issue.document ?? { kind: "unloaded" }, issue.url)}
    ${showEditForm ? issueEditForm(copy, issue) : ""}
    <section class="detail-block">
      <h4>${escapeHtml(copy.family)}</h4>
      <div class="tiny">${escapeHtml(copy.parent)}</div>
      ${issue.parent ? issueLink(copy, issue.parent) : `<span class="muted">${escapeHtml(copy.noParent)}</span>`}
      <div class="tiny">${escapeHtml(copy.children)}</div>
      ${issue.children.length ? issue.children.map((child) => issueLink(copy, child)).join("") : `<span class="muted">${escapeHtml(copy.noKids)}</span>`}
      ${issue.children.length ? `<div><button type="button" data-act="filter-parent" data-id="${escapeHtml(issue.id)}">${escapeHtml(copy.onlyKids)}</button></div>` : ""}
    </section>
    <section class="detail-block">
      <h4>${escapeHtml(copy.deps)}</h4>
      ${showDependencyGraph ? `<button type="button" data-act="view-dependencies" data-id="${escapeHtml(issue.id)}">${escapeHtml(copy.viewDependencies)}</button>` : ""}
      <div class="tiny">${escapeHtml(copy.blockedBy)}</div>
      ${issue.blockedBy.length ? issue.blockedBy.map((link) => issueLink(copy, link)).join("") : `<span class="muted">${escapeHtml(copy.noneBlock)}</span>`}
      <div class="tiny">${escapeHtml(copy.blocking)}</div>
      ${issue.blocking.length ? issue.blocking.map((link) => issueLink(copy, link)).join("") : `<span class="muted">${escapeHtml(copy.none)}</span>`}
    </section>
    ${canWrite
      ? `<details class="detail-block detail-maintenance" data-section="issue-maintenance" data-id="${escapeHtml(issue.id)}" ${ui.issueMaintenanceOpen.has(issueDraftKey(issue.id)) ? "open" : ""}>
          <summary>${escapeHtml(copy.issueUpdates)}</summary>
          ${issueCommentForm(copy, issue)}
          ${issueRelationsForm(copy, board, issue)}
        </details>`
      : ""}`;
}

/** The page decides whether this Issue surface includes dependency controls. */
export type IssueDetailOptions = {
  dependencyGraph?: boolean;
};

export function issueDetail(copy: ShellCopy, board: BoardSnapshot, options: IssueDetailOptions = {}): string {
  const issue = board.selected;
  if (!issue) return `<div class="lane-empty">${escapeHtml(copy.pickIssue)}</div>`;
  return `<header class="detail-sticky">
      <div class="detail-title-row">
        <div class="detail-hd">#${issue.number} ${escapeHtml(issue.title)}</div>
      </div>
      ${issueActions(copy, board, issue)}
    </header>
    <div class="detail-scroll">${issueBody(copy, board, issue, options.dependencyGraph ?? true)}</div>`;
}

export function workspaceRailLabels(): { actions: string; issue: string; runs: string; emptyRuns: string } {
  return effectiveClientLanguage() === "zh-CN"
    ? { actions: "认领与操作", issue: "Issue 正文", runs: "运行记录", emptyRuns: "还没有运行记录" }
    : { actions: "Claim and actions", issue: "Issue body", runs: "Run history", emptyRuns: "No Run history yet" };
}

export function workspaceRunHistory(copy: ShellCopy, runs: RunSummary[]): string {
  const labels = workspaceRailLabels();
  if (!runs.length) return `<p class="muted">${escapeHtml(labels.emptyRuns)}</p>`;
  return `<div class="workspace-run-history">${[...runs].reverse().map((run) => {
    const status = run.status === "ended" ? copy.runGroupEnded : run.waitingForUser ? copy.waiting : copy.running;
    return `<button type="button" class="workspace-run-history-item" data-act="focus-run" data-id="${escapeHtml(run.id)}">
      <span><b>${escapeHtml(run.agentName)}</b><small>${escapeHtml(status)}</small></span>
      ${run.recentAction ? `<span>${escapeHtml(run.recentAction)}</span>` : ""}
    </button>`;
  }).join("")}</div>`;
}

export function focusWorkspaceIssueRail(copy: ShellCopy, snap: Snapshot): string {
  const board = snap.board;
  const issue = board?.selected;
  if (!board || !issue || ui.clientView.panels.rightSide !== "rail") return "";
  const runs = (snap.runs ?? []).filter((run) => run.issueId === issue.id);
  const hasActive = runs.some((run) => run.status !== "ended");
  let open = ui.workspaceRailOpenSections.get(issue.id);
  if (!open) {
    open = new Set<"actions" | "issue" | "runs">(hasActive ? ["runs"] : ["issue"]);
    ui.workspaceRailOpenSections.set(issue.id, open);
  }
  const labels = workspaceRailLabels();
  const section = (id: "actions" | "issue" | "runs", label: string, body: string, bodyClass = "") =>
    `<details class="workspace-rail-section" data-section="workspace-rail" data-id="${escapeHtml(issue.id)}" data-workspace-section="${id}" ${open?.has(id) ? "open" : ""}>
      <summary>${escapeHtml(label)}</summary>
      <div class="workspace-rail-section-body ${bodyClass}">${body}</div>
    </details>`;
  return `<aside class="issue-detail fixed-right-rail workspace-right-rail" data-fixed-panel="right-rail">
    ${fixedPanelResizeHandle("right-rail")}
    ${section("actions", labels.actions, `<div class="detail-hd">#${issue.number} ${escapeHtml(issue.title)}</div>${issueActions(copy, board, issue)}`)}
    ${section("issue", labels.issue, issueBody(copy, board, issue, true), "detail-scroll")}
    ${section("runs", labels.runs, workspaceRunHistory(copy, runs))}
  </aside>`;
}

function issueConflictFieldLabel(copy: ShellCopy, field: string): string {
  const labels: Record<string, string> = {
    title: copy.issueTitle,
    body: copy.issueBody,
    parent: copy.parentIssue,
    blockedBy: copy.dependencyBlockers,
  };
  return labels[field] ?? field;
}

function issueConflictFeedback(copy: ShellCopy, key: FormKey): string {
  const conflict = ui.formOperations.conflicts.get(key);
  if (!conflict) return "";
  const latest = conflict.fields
    .map((field) => {
      const value = conflict.latest[field as keyof typeof conflict.latest];
      const text = Array.isArray(value) ? value.join(", ") : value ?? copy.none;
      return `<div><strong>${escapeHtml(issueConflictFieldLabel(copy, field))}</strong><pre>${escapeHtml(String(text))}</pre></div>`;
    })
    .join("");
  return `<section class="notice bad issue-conflict" role="alert" data-form-key="${escapeHtml(key)}">
    <p>${escapeHtml(copy.issueConflict)}</p>
    <div class="issue-conflict-latest"><span class="tiny">${escapeHtml(copy.issueConflictLatest)}</span>${latest}</div>
    <div class="actions">
      <button type="button" data-act="use-latest-issue-conflict" data-form-key="${escapeHtml(key)}">${escapeHtml(copy.issueConflictUseLatest)}</button>
      <button type="submit" data-conflict-policy="overwrite">${escapeHtml(copy.issueConflictOverwrite)}</button>
    </div>
  </section>`;
}

export function issueEditForm(copy: ShellCopy, issue: IssueDetail): string {
  const key = issueEditFormKey(issue.id);
  const draft = editableIssueDraft(issue);
  const pending = ui.formOperations.pending.has(key);
  return `<section class="detail-block issue-editor issue-edit-editor">
    <form data-act="issue-edit" data-form="issue-edit" data-id="${escapeHtml(issue.id)}" aria-busy="${pending ? "true" : "false"}">
      <h4>${escapeHtml(copy.editIssue)}</h4>
      ${formField({
        id: "issue-edit-title",
        label: copy.issueTitle,
        required: true,
        control: textInput({ id: "issue-edit-title", name: "title", value: draft.title, required: true, disabled: pending, attributes: { maxlength: 240 } }),
      })}
      ${formField({
        id: "issue-edit-body",
        label: copy.issueBody,
        control: textArea({ id: "issue-edit-body", name: "body", value: draft.body, rows: 8, disabled: pending }),
      })}
      ${formFeedback(key)}
      ${issueConflictFeedback(copy, key)}
      <div class="actions">
        ${button({ id: "cancel-edit-issue", label: copy.cancel, disabled: pending, data: { id: issue.id } })}
        ${button({ id: "submit-issue-edit", label: pending ? copy.operationPending : copy.saveIssue, disabled: pending, busy: pending }, { type: "submit", variant: "primary" })}
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
      ${formField({
        label: copy.addComment,
        required: true,
        control: textArea({ name: "body", value: ui.issueCommentDrafts.get(issueDraftKey(issue.id)) ?? "", rows: 4, required: true, disabled: pending, placeholder: copy.commentPlaceholder, attributes: { maxlength: 10000 } }),
      })}
      ${formFeedback(key)}
      <div class="actions">${button({ id: "submit-issue-comment", label: pending ? copy.operationPending : copy.addComment, disabled: pending, busy: pending }, { type: "submit", variant: "primary" })}</div>
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
  return `<section class="detail-block issue-editor issue-relations-editor">
    <h4>${escapeHtml(copy.family)} / ${escapeHtml(copy.deps)}</h4>
    <form data-act="issue-parent" data-form="issue-parent" data-id="${escapeHtml(issue.id)}" aria-busy="${parentPending ? "true" : "false"}">
      ${formField({
        id: "issue-parent",
        label: copy.parentIssue,
        control: selectControl({
          id: "issue-parent",
          name: "parent",
          disabled: parentPending,
          options: [
            { value: "", label: copy.none, selected: !draft.parent },
            ...options.map((option) => ({ value: option.id, label: issueOptionLabel(option), selected: draft.parent === option.id })),
          ],
        }),
      })}
      ${formFeedback(parentKey)}
      ${issueConflictFeedback(copy, parentKey)}
      <div class="actions">${button({ id: "submit-issue-parent", label: parentPending ? copy.operationPending : copy.saveRelations, disabled: parentPending, busy: parentPending }, { type: "submit", variant: "primary" })}</div>
    </form>
    <form data-act="issue-blockers" data-form="issue-blockers" data-id="${escapeHtml(issue.id)}" aria-busy="${blockersPending ? "true" : "false"}">
      ${formField({
        id: "issue-blocked-by",
        label: copy.dependencyBlockers,
        control: selectControl({
          id: "issue-blocked-by",
          name: "blockedBy",
          disabled: blockersPending,
          multiple: true,
          size: Math.min(6, Math.max(3, options.length)),
          options: options.map((option) => ({ value: option.id, label: issueOptionLabel(option), selected: draft.blockedBy.includes(option.id) })),
        }),
      })}
      ${formFeedback(blockersKey)}
      ${issueConflictFeedback(copy, blockersKey)}
      <div class="actions">
        ${button({ id: "clear-issue-blockers", label: copy.clearDependency, disabled: blockersPending, data: { id: issue.id } })}
        ${button({ id: "submit-issue-blockers", label: blockersPending ? copy.operationPending : copy.saveRelations, disabled: blockersPending, busy: blockersPending }, { type: "submit", variant: "primary" })}
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
  const parts: string[] = [];
  // 手动刷新期间沿用 Host 已确认过的「数据截至」，不隐藏上一次成功读取的时间。
  const asOf = status.kind === "never-fetched" || status.fetchedAtMs == null
    ? ""
    : `${copy.refreshAsOf} ${formatTime(status.fetchedAtMs)}`;
  if (ui.refreshing || status.kind === "refreshing") {
    parts.push(copy.refreshRefreshing);
    if (asOf) parts.push(asOf);
  } else if (status.kind === "never-fetched") {
    parts.push(copy.refreshNever);
  } else if (status.kind === "offline") {
    parts.push(`${copy.refreshOffline} · ${asOf}`);
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
    if (asOf) parts.push(asOf);
  } else if (status.kind === "auth-failed") {
    parts.push(copy.refreshAuth);
    parts.push(copy.refreshAuthRecovery);
    if (asOf) parts.push(asOf);
  } else if (status.kind === "incomplete" || status.kind === "tracker-error") {
    parts.push(status.kind === "tracker-error" ? copy.refreshTrackerError : copy.refreshIncomplete);
    if (status.detail) {
      parts.push(status.detail);
    }
    if (asOf) parts.push(asOf);
    if (status.nextRefreshInMs != null) {
      parts.push(`${copy.refreshNext} ${formatCountdown(status.nextRefreshInMs)}`);
    }
  } else if (status.kind === "ready") {
    parts.push(asOf);
    if (status.nextRefreshInMs != null) {
      parts.push(`${copy.refreshNext} ${formatCountdown(status.nextRefreshInMs)}`);
    }
  }
  return refreshStatus({
    kind: ui.refreshing ? "refreshing" : status.kind,
    message: parts.filter(Boolean).join(" · "),
    actions: [{ id: "refresh", label: copy.refreshNow }],
  });
}
