import type { AppearanceState, ChangeFile, ChangeLine, ChangeRepo, Language, Project, ProjectIssueCounts, RunSummary, RunTelemetryLane, ShellCopy, Snapshot, TelemetryLaneKind, TelemetryPoint, TokenCounts, UsageBucket, UsageOption, UsagePage, UsageRange, ViewChanges } from "../protocol";
import { addOpt, escapeHtml, toLocalInput } from "../client-utils";
import { changeNoteFormKey, formFeedback, injectFormKey, revokeClientFormKey, usageCustomFormKey } from "../form-keys";
import { desktopShellAvailable } from "../launch-session";
import { APPEARANCE_DISPLAY_ORDER, effectiveClientLanguage, mobileClient, workspaceRun } from "../view-helpers";
import { fixedPanelResizeHandle } from "../workbench";
import { focusWorkspaceIssueRail, focusWorkspaceProjectRail } from "./board";
import { ui } from "../ui";
import { appearancePreferenceLabel, startupCopy, type StartupCopy } from "../startup-copy";
import { SHELL_SHORTCUTS, shortcutKeyLabels } from "../shortcuts";
import { confirmationDialog, dialog, dialogActionButton, dialogDismissButton } from "../components/dialog";
import { button, checkbox, formField, iconButton, menu, notice, optionGroup, progressFeedback, selectControl, textInput, type ActionDescriptor, type SelectOption } from "../components/primitives";
import { orderRunsForDisplay, runOrganizationActions, runOrganizationLabels, runPersistenceWritesBlocked } from "./run-organization";

export function projectBlock(copy: ShellCopy, snap: Snapshot, project: Project, focusedId: string): string {
  const runs = orderRunsForDisplay((snap.runs ?? []).filter((run) => run.projectId === project.id));
  return `<div class="project-block" data-project="${escapeHtml(project.id)}">
    ${projectRow(copy, project, focusedId)}
    ${runs.map((run) => runRow(copy, run, snap.focusedRunId)).join("")}
  </div>`;
}

export function projectTrackerIdentity(project: Project, localMarkdownLabel: string): string {
  return project.tracker === "local-markdown"
    ? localMarkdownLabel
    : `${project.githubHost}/${project.repository}`;
}

export function projectRow(copy: ShellCopy, project: Project, focusedId: string): string {
  const active = project.id === focusedId;
  const degraded = project.connection.status !== "ready";
  const runWritesBlocked = ui.snapshot ? runPersistenceWritesBlocked(ui.snapshot) : false;
  return `<div class="project-row ${active ? "active" : ""}">
    <button type="button" class="project-main" data-act="focus-project" data-id="${escapeHtml(project.id)}">
      <b>${escapeHtml(project.name)}</b>
      <span>${escapeHtml(projectTrackerIdentity(project, startupCopy(effectiveClientLanguage()).localMarkdownTracker))}</span>
    </button>
    ${degraded ? `<span class="dot warn" title="${escapeHtml(project.connection.status === "unreachable" ? copy.connectionUnavailable : copy.authFailed)}"></span>` : ""}
    <button type="button" class="title-icon" data-act="new-run" data-id="${escapeHtml(project.id)}" aria-label="${escapeHtml(copy.newRun)}" title="${escapeHtml(runWritesBlocked ? copy.runPersistenceWriteBlocked : copy.newRun)}" ${runWritesBlocked ? "disabled" : ""}>＋</button>
    <button type="button" class="more" data-act="project-menu" data-id="${escapeHtml(project.id)}" aria-label="${escapeHtml(copy.projectMenu)} ${escapeHtml(project.name)}">…</button>
    ${
      ui.projectMenuId === project.id
        ? menu({
            className: "project-menu",
            label: `${copy.projectMenu} ${project.name}`,
            actions: [
              { id: "edit-project", label: copy.editProject, data: { id: project.id } },
              { id: "remove-project", label: copy.removeProject, destructive: true, data: { id: project.id } },
            ],
          })
        : ""
    }
  </div>`;
}

export function runIdentity(copy: ShellCopy, run: RunSummary): string {
  return run.unbound || !run.issueId ? copy.unboundIssue : run.issueId;
}

export function runRow(copy: ShellCopy, run: RunSummary, focusedId: string): string {
  const identity = runIdentity(copy, run);
  const localCopy = startupCopy(effectiveClientLanguage());
  const organizationLabels = runOrganizationLabels();
  const action = run.recentAction?.trim() ? escapeHtml(run.recentAction) : "";
  const runWritesBlocked = ui.snapshot ? runPersistenceWritesBlocked(ui.snapshot) : false;
  const stateClass =
    run.waitingForUser && run.status !== "ended"
      ? "waiting"
      : run.endedReason && run.endedReason !== "exited"
        ? "execution-stopped"
        : run.status;
  const stateTag =
    run.waitingForUser && run.status !== "ended"
      ? copy.waiting
      : run.endedReason && run.endedReason !== "exited"
        ? copy.executionStopped
        : run.status === "running"
          ? copy.running
          : "";
  const actions = [
    desktopShellAvailable() && !ui.nativeRunWindowRunId && run.status !== "ended"
      ? iconButton(
          { id: "open-run-window", label: localCopy.openRunWindow, icon: "↗", data: { id: run.id } },
          { className: "run-row-action", attributes: { title: localCopy.openRunWindow } },
        )
      : "",
    iconButton(
      { id: "open-usage-run", label: copy.openHostUsage, icon: "▥", data: { id: run.id } },
      { className: "run-row-action", attributes: { title: copy.openHostUsage } },
    ),
    run.status !== "ended"
      ? iconButton(
          { id: "stop-run", label: copy.stopRun, icon: "■", disabled: runWritesBlocked, data: { id: run.id } },
          { className: "run-row-action danger", attributes: { title: runWritesBlocked ? copy.runPersistenceWriteBlocked : copy.stopRun } },
        )
      : "",
    ui.snapshot ? runOrganizationActions(ui.snapshot, run, "icons") : "",
  ].join("");
  const pinMarker = run.pinnedAtMs != null
    ? `<span class="run-pin-marker" role="img" aria-label="${escapeHtml(organizationLabels.pinnedMarker)}" title="${escapeHtml(organizationLabels.pinnedMarker)}">↑</span>`
    : "";
  return `<div class="run-row ${run.id === focusedId ? "active" : ""} ${escapeHtml(stateClass)} ${run.pinnedAtMs != null ? "pinned" : ""}" data-run="${escapeHtml(run.id)}" data-pinned="${run.pinnedAtMs != null}">
    <button type="button" class="run-main" data-act="focus-run" data-id="${escapeHtml(run.id)}">
      <b>${escapeHtml(run.agentName)}${pinMarker}</b>
      <span>${escapeHtml(identity)}</span>
      ${stateTag ? `<span class="run-state">${escapeHtml(stateTag)}</span>` : ""}
      ${action ? `<span class="run-action">${action}</span>` : ""}
      ${run.failure ? `<span class="run-fail">${escapeHtml(run.failure)}</span>` : ""}
      ${run.isolationNote ? `<span class="run-action">${escapeHtml(run.isolationNote)}</span>` : ""}
    </button>
    <div class="run-row-actions" role="group" aria-label="${escapeHtml(`${run.agentName} · ${identity}`)}">${actions}</div>
  </div>`;
}

export function dash(value?: number | null): string {
  return value == null ? "—" : String(value);
}

export function laneLabel(copy: ShellCopy, lane: TelemetryLaneKind): string {
  if (lane === "subagent") return copy.laneSubagent;
  if (lane === "switched") return copy.laneSwitched;
  return copy.laneMain;
}

export function tokenCells(copy: ShellCopy, tokens: TokenCounts): string {
  const cells: Array<[string, number | null | undefined]> = [
    [copy.tokenInput, tokens.input],
    [copy.tokenOutput, tokens.output],
    [copy.tokenCacheRead, tokens.cacheRead],
    [copy.tokenCacheWrite, tokens.cacheWrite],
    [copy.tokenReasoning, tokens.reasoning],
    [copy.tokenTotal, tokens.total],
  ];
  return cells
    .map(([label, value]) => `<span class="token-cell"><i>${escapeHtml(label)}</i>${dash(value)}</span>`)
    .join("");
}

export function sparkline(points: TelemetryPoint[], field: "ttftMs" | "tokensPerSec"): string {
  const values = points.map((point) => point[field] ?? 0);
  const max = Math.max(...values, 1);
  return `<span class="spark">${points
    .map((point) => {
      const value = point[field] ?? 0;
      const height = Math.max(8, Math.round((value / max) * 28));
      return `<i class="${point.spike ? "slow" : ""}" style="height:${height}px"></i>`;
    })
    .join("")}</span>`;
}

export function telemetryBar(copy: ShellCopy, run: RunSummary): string {
  const lanes = run.telemetry ?? [];
  if (!lanes.length) return "";
  const capsule = (lane: RunTelemetryLane) =>
    `<button type="button" class="capsule ${lane.spike ? "slow" : ""}" data-act="toggle-telemetry">${escapeHtml(lane.model)}<small>${escapeHtml(laneLabel(copy, lane.lane))}</small></button>`;
  const main = lanes.find((lane) => lane.lane === "main") ?? lanes[0];
  const capsules = lanes.map(capsule).join("");
  const simple = `<div class="telemetry-mobile">${capsule(main)}<ul class="telemetry-simple">${lanes
    .map(
      (lane) =>
        `<li>${escapeHtml(lane.model)} · ${escapeHtml(laneLabel(copy, lane.lane))} · ${copy.tokenTotal} ${dash(lane.tokens.total)}</li>`,
    )
    .join("")}</ul></div>`;
  const cards = ui.telemetryExpanded
    ? `<div class="telemetry-cards">${lanes
        .map(
          (lane) => `<article class="telemetry-card ${lane.spike ? "slow" : ""}">
            <header><b>${escapeHtml(lane.model)}</b><span>${escapeHtml(laneLabel(copy, lane.lane))}</span></header>
            <div class="token-row">${tokenCells(copy, lane.tokens)}</div>
            <div class="telemetry-meta">${escapeHtml(copy.ttft)} ${dash(lane.ttftMs)} · ${escapeHtml(copy.genRate)} ${dash(lane.tokensPerSec)}</div>
            ${sparkline(lane.recent, "ttftMs")}
          </article>`,
        )
        .join("")}<p class="tiny">${escapeHtml(copy.proxyDisclaimer)}</p></div>`
    : "";
  return `<div class="telemetry-bar"><div class="telemetry-desktop">${capsules}</div>${simple}${cards}</div>`;
}

export function usagePage(copy: ShellCopy, snap: Snapshot): string {
  const usage = snap.usage;
  if (!usage) return "";
  const range = usage.range;
  const customKey = usageCustomFormKey(snap.focusedHostId);
  const customPending = ui.formOperations.pending.has(customKey);
  const customDraft = ui.usageCustomDraft?.hostId === snap.focusedHostId ? ui.usageCustomDraft : {
    hostId: snap.focusedHostId,
    from: toLocalInput(usage.fromMs),
    to: toLocalInput(usage.toMs),
  };
  const rangeAction = (id: UsageRange, label: string): ActionDescriptor => ({
    id: "usage-range",
    label,
    pressed: range === id,
    data: { id },
  });
  const filterOptions = (items: UsageOption[], selected: string | null | undefined): SelectOption[] => [
    { value: "", label: copy.filterAll },
    ...items.map((item) => ({ value: item.id, label: item.name, selected: item.id === selected })),
  ];
  const modelOptions: SelectOption[] = [
    { value: "", label: copy.filterAll },
    ...usage.models.map((model) => ({ value: model, label: model, selected: model === usage.filter.model })),
  ];
  const rows = usage.runs.length
    ? usage.runs
        .map(
          (row) => `<article class="usage-row ${row.highlighted ? "sel" : ""}">
            <header>
              <div>
                <b>${escapeHtml(row.projectName)}</b>
                <span>${escapeHtml(row.agentName)}${row.models.length ? ` · ${escapeHtml(row.models.join(", "))}` : ""}</span>
              </div>
              ${button({ id: "open-run-usage", label: copy.openThisRun, data: { id: row.runId } })}
            </header>
            <div class="token-row">${tokenCells(copy, row.tokens)}</div>
          </article>`,
        )
        .join("")
    : `<p class="board-empty">${escapeHtml(copy.usageEmpty)}</p>`;
  const trend = `${usageTrend(copy.ttft, usage.buckets, "ttftMs")}${usageTrend(copy.genRate, usage.buckets, "tokensPerSec")}`;
  const hit =
    usage.cacheHitRate == null ? "—" : `${Math.round(usage.cacheHitRate * 1000) / 10}%`;
  return `<div class="usage-page">
    <div class="content-toolbar" data-page-toolbar>
      <div class="board-head">
        <div class="board-head-row">
          <div>
            <h1>${escapeHtml(copy.usage)}</h1>
            <p>${escapeHtml(copy.usageHint)}</p>
          </div>
        </div>
      </div>
    </div>
    ${optionGroup({
      label: copy.usage,
      className: "usage-ranges",
      actions: [
        rangeAction("24-hours", copy.range24Hours),
        rangeAction("today", copy.rangeToday),
        rangeAction("7-days", copy.range7Days),
        rangeAction("30-days", copy.range30Days),
        rangeAction("custom", copy.rangeCustom),
      ],
    })}
    ${
      range === "custom"
        ? `<form class="usage-custom" data-act="usage-custom" aria-busy="${customPending ? "true" : "false"}">
            ${textInput({ name: "from", type: "datetime-local", required: true, value: customDraft.from, disabled: customPending })}
            ${textInput({ name: "to", type: "datetime-local", required: true, value: customDraft.to, disabled: customPending })}
            ${button({ id: "apply-usage-custom", label: customPending ? copy.operationPending : copy.rangeCustom, disabled: customPending, busy: customPending }, { type: "submit", variant: "primary" })}
          </form>${formFeedback(customKey)}`
        : ""
    }
    <div class="usage-filters">
      <label>${escapeHtml(copy.filterProject)}${selectControl({ options: filterOptions(usage.projects, usage.filter.projectId), attributes: { "data-usage-filter": "projectId" } })}</label>
      <label>${escapeHtml(copy.filterAgent)}${selectControl({ options: filterOptions(usage.agents, usage.filter.agentId), attributes: { "data-usage-filter": "agentId" } })}</label>
      <label>${escapeHtml(copy.filterModel)}${selectControl({ options: modelOptions, attributes: { "data-usage-filter": "model" } })}</label>
    </div>
    <div class="token-row totals">${tokenCells(copy, usage.totals)}<span class="token-cell"><i>${escapeHtml(copy.cacheHit)}</i>${hit}</span></div>
    ${trend}
    <p class="tiny">${escapeHtml(copy.proxyDisclaimer)}</p>
    <div class="usage-list usage-full">${rows}</div>
    <div class="usage-list usage-compact">${usageCompact(copy, usage)}</div>
  </div>`;
}

export function hostOverviewPage(copy: ShellCopy, snap: Snapshot, includeOrganization = true): string {
  if (ui.overviewProjectId && !snap.projects.some((project) => project.id === ui.overviewProjectId)) {
    ui.overviewProjectId = "";
  }
  const visibleProjects = snap.projects.filter(
    (project) => !ui.overviewProjectId || project.id === ui.overviewProjectId,
  );
  const projectRuns = (snap.runs ?? []).filter(
    (run) => !ui.overviewProjectId || run.projectId === ui.overviewProjectId,
  );
  const visibleRuns = projectRuns.filter((run) => ui.overviewShowEnded || run.status !== "ended");
  const groups: Array<[string, string, RunSummary[]]> = [
    ["waiting", copy.runGroupWaiting, visibleRuns.filter((run) => run.status !== "ended" && Boolean(run.waitingForUser))],
    ["running", copy.runGroupRunning, visibleRuns.filter((run) => run.status !== "ended" && !run.waitingForUser)],
    ["stopped", copy.runGroupStopped, visibleRuns.filter((run) => run.status === "ended" && Boolean(run.endedReason) && run.endedReason !== "exited")],
    ["ended", copy.runGroupEnded, visibleRuns.filter((run) => run.status === "ended" && (!run.endedReason || run.endedReason === "exited"))],
  ];
  const projectOptions: SelectOption[] = [
    { value: "", label: copy.filterAll },
    ...snap.projects.map((project) => ({
      value: project.id,
      label: project.name,
      selected: project.id === ui.overviewProjectId,
    })),
  ];
  const totalCounts = visibleProjects.reduce(
    (total, project) => {
      const counts = projectIssueCounts(project);
      if (counts.dataAvailable) {
        total.open += counts.open;
        total.frontier += counts.frontier;
        total.available += 1;
      }
      return total;
    },
    { open: 0, frontier: 0, available: 0 },
  );
  const allCountsAvailable = visibleProjects.length > 0 && totalCounts.available === visibleProjects.length;
  const activeRuns = visibleRuns.filter((run) => run.status !== "ended").length;
  return `<div class="overview-page">
    <div class="content-toolbar" data-page-toolbar>
      <div class="board-head">
        <div class="board-head-row">
          <div><h1>${escapeHtml(copy.hostOverview)}</h1><p>${escapeHtml(copy.hostOverviewHint)}</p></div>
        </div>
      </div>
    </div>
    <div class="overview-controls">
      <label>${escapeHtml(copy.filterProject)}
        ${selectControl({ options: projectOptions, attributes: { "data-overview-filter": "project" } })}
      </label>
      ${checkbox({ label: copy.showEndedRuns, checked: ui.overviewShowEnded, attributes: { "data-field": "showEndedRuns" } })}
    </div>
    <div class="overview-stats">
      <div><b>${visibleProjects.length}</b><span>${escapeHtml(copy.projects)}</span></div>
      <div><b>${allCountsAvailable ? totalCounts.open : "—"}</b><span>Open Issue</span></div>
      <div><b>${allCountsAvailable ? totalCounts.frontier : "—"}</b><span>Frontier</span></div>
      <div><b>${activeRuns}</b><span>${escapeHtml(copy.activeRuns)}</span></div>
    </div>
    <section class="overview-project-section">
      <div class="lane-hd">${escapeHtml(copy.projects)} <span>${visibleProjects.length}</span></div>
      <div class="overview-projects">
        ${visibleProjects.map((project) => overviewProjectCard(copy, project)).join("")}
      </div>
    </section>
    <section class="overview-run-section">
      <div class="lane-hd">${escapeHtml(copy.filteredRuns)} <span>${visibleRuns.length}</span></div>
      ${visibleRuns.length === 0
        ? `<div class="overview-runs-empty">${escapeHtml((snap.runs ?? []).length === 0 ? copy.hostOverviewEmpty : copy.noItems)}</div>`
        : `<div class="overview-groups">
          ${groups
            .filter(([id]) => ui.overviewShowEnded || (id !== "stopped" && id !== "ended"))
            .map(
              ([id, title, runs]) => `<section class="overview-group" data-run-group="${id}">
                <div class="lane-hd">${escapeHtml(title)} <span>${runs.length}</span></div>
                <div class="run-thumbnails">${runs.length ? runs.map((run) => runThumbnail(copy, run, snap, includeOrganization)).join("") : `<p class="lane-empty">${escapeHtml(copy.noItems)}</p>`}</div>
              </section>`,
            )
            .join("")}
        </div>`}
    </section>
  </div>`;
}

export function projectIssueCounts(project: Project): ProjectIssueCounts {
  return project.issueCounts ?? {
    dataAvailable: false,
    total: 0,
    open: 0,
    closed: 0,
    blocked: 0,
    frontier: 0,
    inProgress: 0,
  };
}

export function overviewProjectCard(copy: ShellCopy, project: Project): string {
  const counts = projectIssueCounts(project);
  const metric = (label: string, value: number) =>
    `<span><i>${escapeHtml(label)}</i><b>${counts.dataAvailable ? value : "—"}</b></span>`;
  const connection = project.connection.status === "ready"
    ? copy.connectionReady
    : project.connection.status === "unreachable"
      ? copy.connectionUnavailable
      : copy.authFailed;
  return `<button type="button" class="overview-project" data-act="focus-project" data-id="${escapeHtml(project.id)}">
    <span class="overview-project-head"><span><b>${escapeHtml(project.name)}</b><small>${escapeHtml(project.repository)}</small></span><em>${escapeHtml(connection)}</em></span>
    <span class="overview-project-metrics">
      ${metric("Open", counts.open)}
      ${metric(copy.colBlocked, counts.blocked)}
      ${metric(copy.colFrontier, counts.frontier)}
      ${metric(copy.colInProgress, counts.inProgress)}
      ${metric("Closed", counts.closed)}
    </span>
  </button>`;
}

export function runThumbnail(copy: ShellCopy, run: RunSummary, snap: Snapshot, includeOrganization = true): string {
  const project = snap.projects.find((item) => item.id === run.projectId);
  const action = run.recentAction?.trim() || run.failure?.trim() || "";
  return `<article class="run-thumbnail">
    <button type="button" class="run-thumbnail-main" data-act="focus-run" data-id="${escapeHtml(run.id)}">
      <span class="run-project">${escapeHtml(project?.name ?? run.projectId)}</span>
      <b>${escapeHtml(runIdentity(copy, run))}</b>
      <span>${escapeHtml(run.agentName)}${action ? ` · ${escapeHtml(action)}` : ""}</span>
    </button>
    ${includeOrganization ? runOrganizationActions(snap, run) : ""}
  </article>`;
}

export function usageTrend(
  label: string,
  buckets: UsageBucket[],
  field: "ttftMs" | "tokensPerSec",
): string {
  const values = buckets.flatMap((bucket) => {
    const value = bucket[field];
    return value == null ? [] : [value];
  });
  const max = Math.max(...values, 1);
  return `<div class="usage-trend-block"><span class="tiny">${escapeHtml(label)}</span><div class="usage-trend">${buckets
    .map((bucket) => {
      const value = bucket[field];
      if (value == null) return `<span class="usage-trend-missing" title="—">—</span>`;
      const height = Math.max(4, Math.round((value / max) * 48));
      return `<i class="${bucket.slow ? "slow" : ""}" style="height:${height}px" title="${dash(value)}"></i>`;
    })
    .join("")}</div></div>`;
}

export function usageCompact(copy: ShellCopy, usage: UsagePage): string {
  const byProject = new Map<string, { name: string; tokens: TokenCounts }>();
  for (const row of usage.runs) {
    const current = byProject.get(row.projectId);
    if (!current) {
      byProject.set(row.projectId, { name: row.projectName, tokens: row.tokens });
    } else {
      current.tokens = {
        input: addOpt(current.tokens.input, row.tokens.input),
        output: addOpt(current.tokens.output, row.tokens.output),
        cacheRead: addOpt(current.tokens.cacheRead, row.tokens.cacheRead),
        cacheWrite: addOpt(current.tokens.cacheWrite, row.tokens.cacheWrite),
        reasoning: addOpt(current.tokens.reasoning, row.tokens.reasoning),
        total: addOpt(current.tokens.total, row.tokens.total),
      };
    }
  }
  const lines = [...byProject.values()].slice(0, 3);
  if (!lines.length) return `<p class="board-empty">${escapeHtml(copy.usageEmpty)}</p>`;
  return lines
    .map(
      (line) =>
        `<article class="usage-row"><header><b>${escapeHtml(line.name)}</b></header><div class="token-row">${tokenCells(copy, line.tokens)}</div></article>`,
    )
    .join("");
}

export function injectRunForm(copy: ShellCopy, run: RunSummary): string {
  const key = injectFormKey(run.id);
  const pending = ui.formOperations.pending.has(key);
  return `<form class="inject-row" data-act="inject-run" data-id="${escapeHtml(run.id)}" aria-busy="${pending ? "true" : "false"}">
    <input name="text" maxlength="4000" value="${escapeHtml(ui.terminalInputDrafts.get(run.id) ?? "")}" placeholder="${escapeHtml(copy.injectPlaceholder)}" ${pending ? "disabled" : ""} />
    <button type="submit" ${pending ? "disabled" : ""}>${escapeHtml(pending ? copy.operationPending : copy.injectLine)}</button>
    ${formFeedback(key)}
  </form>`;
}

function focusWorkspaceLabels(): { recentOutput: string; emptyTitle: string; emptyBody: string } {
  return effectiveClientLanguage() === "zh-CN"
    ? {
        recentOutput: "最近输出（只读）",
        emptyTitle: "还没有 Run",
        emptyBody: "启动一个 Run 后，Embedded Terminal 会固定显示在这里。",
      }
    : {
        recentOutput: "Recent output (read only)",
        emptyTitle: "No Run yet",
        emptyBody: "Start a Run to keep the Embedded Terminal fixed here.",
      };
}

export function runControls(copy: ShellCopy, run: RunSummary, includeOrganization = false): string {
  const localCopy = startupCopy(effectiveClientLanguage());
  const openWindow = desktopShellAvailable() && !ui.nativeRunWindowRunId && run.status !== "ended"
    ? `<button type="button" data-act="open-run-window" data-id="${escapeHtml(run.id)}">${escapeHtml(localCopy.openRunWindow)}</button>`
    : "";
  return `<div class="actions">
    ${openWindow}
    <button type="button" data-act="open-usage-run" data-id="${escapeHtml(run.id)}">${escapeHtml(copy.openHostUsage)}</button>
    <button type="button" data-act="stop-run" data-id="${escapeHtml(run.id)}" title="${escapeHtml(includeOrganization && runPersistenceWritesBlocked(ui.snapshot!) ? copy.runPersistenceWriteBlocked : copy.stopRun)}" ${run.status === "ended" || (includeOrganization && runPersistenceWritesBlocked(ui.snapshot!)) ? "disabled" : ""}>${escapeHtml(copy.stopRun)}</button>
    ${includeOrganization && ui.snapshot ? runOrganizationActions(ui.snapshot, run) : ""}
  </div>`;
}

/** Run identity and actions, shared by every terminal surface. */
export function runHeader(copy: ShellCopy, run: RunSummary, includeOrganization = false): string {
  return `<header class="run-dock-hd">
      <div><b>${escapeHtml(run.agentName)}</b><span>${escapeHtml(runIdentity(copy, run))}</span></div>
      ${runControls(copy, run, includeOrganization)}
    </header>`;
}

/** Host-reported Run conditions that every terminal surface must keep visible. */
export function runNotices(copy: ShellCopy, run: RunSummary): string {
  return `${run.waitingForUser && run.status !== "ended" ? `<p class="notice">${escapeHtml(copy.waiting)}</p>` : ""}
    ${run.failure ? `<p class="notice bad">${escapeHtml(run.failure)}</p>` : ""}
    ${run.isolationNote ? `<p class="notice">${escapeHtml(run.isolationNote)}</p>` : ""}`;
}

export function terminalPanel(copy: ShellCopy, run: RunSummary, className: string, includeOrganization = false): string {
  return `<div class="${className}" data-terminal-panel data-terminal-surface="live" data-run="${escapeHtml(run.id)}">
    ${runHeader(copy, run, includeOrganization)}
    ${telemetryBar(copy, run)}
    ${runNotices(copy, run)}
    <div class="pty-slot" data-run="${escapeHtml(run.id)}"></div>
  </div>`;
}

export function readOnlyTerminal(copy: ShellCopy, run: RunSummary, includeOrganization = false): string {
  const labels = focusWorkspaceLabels();
  return `<div class="focus-terminal-surface readonly-terminal" data-terminal-panel data-terminal-surface="readonly" data-run="${escapeHtml(run.id)}">
    ${runHeader(copy, run, includeOrganization)}
    <div class="readonly-terminal-label">${escapeHtml(labels.recentOutput)}</div>
    <pre class="readonly-terminal-output" aria-readonly="true">${escapeHtml(run.recentOutput ?? "")}</pre>
  </div>`;
}

export function emptyTerminalSurface(copy: ShellCopy, snap: Snapshot, respectPersistenceWrites = false): string {
  const labels = focusWorkspaceLabels();
  const issue = snap.board?.selected;
  const writesBlocked = respectPersistenceWrites && runPersistenceWritesBlocked(snap);
  return `<div class="focus-terminal-surface empty-terminal" data-terminal-surface="empty">
    <div class="empty-terminal-content">
      <h2>${escapeHtml(labels.emptyTitle)}</h2>
      <p>${escapeHtml(labels.emptyBody)}</p>
      ${issue ? `<button type="button" class="primary" data-act="execute-run" data-id="${escapeHtml(issue.id)}" title="${escapeHtml(writesBlocked ? copy.runPersistenceWriteBlocked : copy.executeRun)}" ${writesBlocked ? "disabled" : ""}>${escapeHtml(copy.executeRun)}</button>` : ""}
    </div>
  </div>`;
}

export function focusWorkspaceView(copy: ShellCopy, snap: Snapshot): string {
  const run = workspaceRun(snap);
  const main = run
    ? run.status === "ended"
      ? readOnlyTerminal(copy, run, true)
      : terminalPanel(copy, run, "focus-terminal-surface lifted-terminal", true)
    : emptyTerminalSurface(copy, snap, true);
  const right = ui.clientView.panels.rightSide === "changes" && run
    ? viewChangesPanel(copy)
    : run?.unbound
      ? focusWorkspaceProjectRail(copy, snap)
      : focusWorkspaceIssueRail(copy, snap) || focusWorkspaceProjectRail(copy, snap);
  return `<section class="lifted-run focus-workspace-layout ${right ? "with-right-side" : "right-side-hidden"}">
    <div class="focus-workspace-main">${main}</div>
    ${right}
  </section>`;
}

export function viewChangesPanel(copy: ShellCopy): string {
  const view = ui.changesView;
  const scope = view?.scope ?? ui.changesScope;
  return `<aside class="fixed-changes-panel" data-fixed-panel="changes-panel">
    ${fixedPanelResizeHandle("changes-panel")}
    <div class="changes-sheet" data-view-state="${view ? "loaded" : "loading"}">
      <h2>${escapeHtml(copy.viewChanges)}</h2>
      <div class="choices">
        <button type="button" class="${scope === "this-round" ? "active" : ""}" data-act="changes-scope" data-id="this-round">${escapeHtml(copy.thisRound)}</button>
        <button type="button" class="${scope === "uncommitted" ? "active" : ""}" data-act="changes-scope" data-id="uncommitted">${escapeHtml(copy.uncommitted)}</button>
      </div>
      ${
        !view
          ? `<p class="notice">${escapeHtml(copy.viewChanges)}</p>`
          : !view.available
            ? `<p class="notice bad">${escapeHtml(view.unavailableReason || copy.viewChanges)}</p>`
            : view.repos
                .map((repo) => changeRepoBlock(copy, view, repo))
                .join("")
      }
    </div>
  </aside>`;
}

export function changeRepoBlock(copy: ShellCopy, view: ViewChanges, repo: ChangeRepo): string {
  const title = repo.displayPath === "." ? view.workingDirectory : repo.displayPath;
  if (!repo.available) {
    return `<section class="change-repo">
      <h3>${escapeHtml(title)}</h3>
      <p class="notice">${escapeHtml(repo.unavailableReason || copy.viewChanges)}</p>
    </section>`;
  }
  if (!repo.files.length) {
    return `<section class="change-repo">
      <h3>${escapeHtml(title)}</h3>
      <p class="muted">${escapeHtml(copy.noItems)}</p>
    </section>`;
  }
  return `<section class="change-repo">
    <h3>${escapeHtml(title)}</h3>
    ${repo.files.map((file) => changeFileBlock(copy, view, repo, file)).join("")}
  </section>`;
}

export function changeFileBlock(
  copy: ShellCopy,
  view: ViewChanges,
  repo: ChangeRepo,
  file: ChangeFile,
): string {
  return `<article class="change-file">
    <h4>${escapeHtml(file.path)}</h4>
    ${file.hunks
      .map(
        (hunk) => `<div class="diff">${hunk.lines
          .map((line) => changeLineRow(copy, view, repo, file, line))
          .join("")}</div>`,
      )
      .join("")}
  </article>`;
}

export function changeLineRow(
  copy: ShellCopy,
  view: ViewChanges,
  repo: ChangeRepo,
  file: ChangeFile,
  line: ChangeLine,
): string {
  const mark = line.kind === "add" ? "+" : line.kind === "delete" ? "-" : " ";
  const number = line.newLine ?? line.oldLine ?? 0;
  const notes = view.notes.filter(
    (note) => note.repo === repo.displayPath && note.path === file.path && note.line === number,
  );
  const canNote = line.kind !== "delete" && line.newLine;
  const active =
    ui.noteTarget &&
    ui.noteTarget.repo === repo.displayPath &&
    ui.noteTarget.path === file.path &&
    ui.noteTarget.line === line.newLine;
  const noteKey = changeNoteFormKey(view.runId);
  const notePending = ui.formOperations.pending.has(noteKey);
  const noteForm = active
    ? `<form class="note-form" data-act="write-note" aria-busy="${notePending ? "true" : "false"}">
        <input name="text" maxlength="400" value="${escapeHtml(ui.noteDraft)}" placeholder="${escapeHtml(copy.changeNotePlaceholder)}" ${notePending ? "disabled" : ""} />
        <button type="submit" ${notePending ? "disabled" : ""}>${escapeHtml(notePending ? copy.operationPending : copy.addChangeNote)}</button>
        ${formFeedback(noteKey)}
      </form>`
    : "";
  const noteList = notes
    .map(
      (note) =>
        `<div class="change-note">${escapeHtml(note.text)} <button type="button" data-act="delete-note" data-id="${escapeHtml(note.id)}">${escapeHtml(copy.deleteChangeNote)}</button></div>`,
    )
    .join("");
  const attrs = canNote
    ? ` data-act="note-line" data-repo="${escapeHtml(repo.displayPath)}" data-path="${escapeHtml(file.path)}" data-line="${line.newLine}"`
    : "";
  return `<span class="diff-line ${line.kind}"${attrs}><span class="diff-no">${number || ""}</span><span class="diff-mark">${mark}</span><span class="diff-text">${escapeHtml(line.text)}</span></span>${noteForm}${noteList}`;
}

export function quitOfferDialog(copy: ShellCopy): string {
  const localCopy = startupCopy(effectiveClientLanguage());
  const activeRunCount = ui.snapshot?.quitOffer?.activeRunCount ?? 0;
  const count = localCopy.interruptedRunCount.replace("{count}", String(activeRunCount));
  return confirmationDialog({
    id: "quit-host",
    title: copy.quitActiveTitle,
    body: `${notice({ status: "danger", message: copy.quitActiveBody })}<p class="hint">${escapeHtml(count)}</p>`,
    closeLabel: localCopy.close,
    cancelLabel: copy.quitReturn,
    confirm: { id: "confirm-quit", label: copy.quitStopAll, destructive: true },
  });
}

export function dangerConfirmationDialog(copy: ShellCopy): string {
  const confirmation = ui.dangerConfirmation;
  if (!confirmation) return "";
  const localCopy = startupCopy(effectiveClientLanguage());
  if (confirmation.kind === "stop-run") {
    const run = ui.snapshot?.runs.find((candidate) => candidate.id === confirmation.runId);
    const runIdentity = run ? `${run.agentName} · ${run.issueId ?? run.id}` : confirmation.runId;
    return confirmationDialog({
      id: "stop-run",
      title: localCopy.stopRunTitle,
      body: `${notice({ status: "danger", message: localCopy.stopRunBody })}<p class="hint"><strong>Run</strong> · ${escapeHtml(runIdentity)}</p>`,
      closeLabel: localCopy.close,
      cancelLabel: copy.cancel,
      confirm: {
        id: "confirm-stop-run",
        label: localCopy.stopRunConfirm,
        destructive: true,
        busy: ui.confirmationPending,
        disabled: Boolean(!mobileClient() && ui.snapshot && runPersistenceWritesBlocked(ui.snapshot)),
      },
      error: ui.confirmationError,
    });
  }
  const body = localCopy.revokeClientBody.replace("{name}", confirmation.clientName);
  return confirmationDialog({
    id: "revoke-client",
    title: localCopy.revokeClientTitle,
    body: notice({ status: "danger", message: body }),
    closeLabel: localCopy.close,
    cancelLabel: copy.cancel,
    confirm: {
      id: "confirm-revoke-client",
      label: localCopy.revokeClientConfirm,
      destructive: true,
      busy: ui.confirmationPending || ui.formOperations.pending.has(revokeClientFormKey(confirmation.clientId)),
    },
    error: ui.confirmationError,
  });
}

export function launchEnvironmentStatus(copy: StartupCopy): string {
  const state = ui.launchEnvironmentState;
  const text = state.status === "ready"
    ? copy.launchEnvironmentReady
    : state.status === "failed"
      ? copy.launchEnvironmentFailed
      : copy.launchEnvironmentIdle;
  const detail = ui.launchEnvironmentError || state.message || "";
  return `<p class="hint ${state.status === "failed" || detail ? "notice bad" : ""}" data-launch-environment-status="${state.status}">${escapeHtml(text)}${detail ? `<br>${escapeHtml(detail)}` : ""}</p>`;
}

const SETTINGS_LANGUAGES: Language[] = ["zh-CN", "en"];

export function settingsPage(
  copy: ShellCopy,
  localCopy: StartupCopy,
  snap: Snapshot,
  appearance: AppearanceState,
  isMobile: boolean,
): string {
  const languageLabel = (language: Language) => language === "zh-CN" ? copy.languageZh : copy.languageEn;
  const project = snap.projects.find((item) => item.id === snap.focusedProjectId);
  return `<section class="settings-page" data-primary-page="settings">
    <div class="content-toolbar" data-page-toolbar>
      <div class="board-head">
        <div class="board-head-row"><div><h1>${escapeHtml(copy.settings)}</h1></div></div>
      </div>
    </div>
    <div class="settings-content">
      <section class="settings-section" data-settings-section="appearance">
        <h2>${escapeHtml(localCopy.appearance)}</h2>
        <div class="field">
          <div class="label">${escapeHtml(copy.language)}</div>
          ${optionGroup({
            label: copy.language,
            actions: SETTINGS_LANGUAGES.map((language) => ({
              id: "language",
              label: languageLabel(language),
              pressed: appearance.language === language,
              data: { id: language },
            })),
          })}
        </div>
        <div class="field">
          <div class="label">${escapeHtml(localCopy.appearance)}</div>
          ${optionGroup({
            label: localCopy.appearance,
            actions: APPEARANCE_DISPLAY_ORDER.map((preference) => ({
              id: "appearance",
              label: appearancePreferenceLabel(localCopy, preference),
              pressed: appearance.appearancePreference === preference,
              data: { id: preference },
            })),
          })}
        </div>
      </section>
      <section class="settings-section" data-settings-section="startup">
        <h2>${escapeHtml(localCopy.hostStartup)}</h2>
        ${startupSettings(localCopy, snap)}
        <div class="field">
          ${button({ id: "refresh-launch-environment", label: localCopy.rereadLaunchEnvironment, disabled: snap.hostMode === "client-only" })}
          ${launchEnvironmentStatus(localCopy)}
        </div>
      </section>
      <section class="settings-section" data-settings-section="updates">
        <h2>${escapeHtml(copy.updates)}</h2>
        ${updateSettings(copy)}
      </section>
      <section class="settings-section" data-settings-section="refresh">
        <h2>${escapeHtml(copy.refreshInterval)}</h2>
        ${formField({
          id: "refresh-interval",
          label: copy.refreshInterval,
          control: textInput({ id: "refresh-interval", type: "number", value: String(Math.round((snap.refreshIntervalMs ?? 60_000) / 1000)), attributes: { min: 15, step: 15, "data-field": "refreshInterval" } }),
          hint: copy.refreshIntervalHelp,
        })}
        ${formField({
          id: "recent-limit",
          label: copy.recentLimit,
          control: textInput({ id: "recent-limit", type: "number", value: String(snap.recentCompletedLimit), attributes: { min: 1, max: 50, "data-field": "recentLimit" } }),
          hint: copy.recentLimitHelp,
        })}
      </section>
      <section class="settings-section" data-settings-section="notifications">
        <h2>${escapeHtml(copy.notifyDesktop)}</h2>
        ${checkbox({ label: copy.showCommandPreview, checked: snap.showCommandPreview, attributes: { "data-field": "commandPreview" } })}
        ${isMobile ? "" : `${checkbox({ label: copy.notifyDesktop, checked: snap.notifyDesktop, attributes: { "data-field": "notifyDesktop" } })}
        ${checkbox({ label: copy.notifySound, checked: snap.notifySound, attributes: { "data-field": "notifySound" } })}`}
      </section>
      <section class="settings-section" data-settings-section="host">
        <h2>${escapeHtml(copy.hosts)}</h2>
        <div class="field paired-clients" data-paired-clients>
          <div class="label">${escapeHtml(copy.pairedClients)}</div>
          ${snap.pairedClients.length
            ? snap.pairedClients.map((client) => `<div class="client-row"><span>${escapeHtml(client.name)}</span>${button({ id: "revoke", label: copy.revokeClient, destructive: true, data: { id: client.id, name: client.name } })}</div>`).join("")
            : `<div class="nested">${escapeHtml(copy.noPairedClients)}</div>`}
        </div>
        ${isMobile ? "" : `${checkbox({ label: copy.autoAdvance, checked: snap.autoAdvance, attributes: { "data-field": "hostAutoAdvance" } })}
        <p class="hint">${escapeHtml(copy.autoAdvanceHelp)}</p>
        ${project ? `${checkbox({ label: copy.projectAutoAdvance, checked: project.autoAdvance, attributes: { "data-field": "projectAutoAdvance" } })}
        ${checkbox({ label: copy.restoreAutoAdvance, checked: project.restoreAutoAdvance, attributes: { "data-field": "restoreAutoAdvance" } })}
        ${formField({
          id: "restore-delay",
          label: copy.restoreDelay,
          control: textInput({ id: "restore-delay", type: "number", value: String(Math.round((project.restoreDelayMs ?? 60000) / 1000)), attributes: { min: 0, max: 600, "data-field": "restoreDelay" } }),
        })}` : ""}
        ${button({ id: "quit", label: copy.quitHost })}`}
      </section>
    </div>
  </section>`;
}

export function startupSettings(copy: StartupCopy, snap: Snapshot): string {
  if (!desktopShellAvailable()) {
    return `<div class="field startup-settings"><div class="label">${escapeHtml(copy.hostStartup)}</div><p class="hint">${escapeHtml(copy.desktopStartupBrowser)}</p></div>`;
  }
  return `<div class="field startup-settings">
    <div class="label">${escapeHtml(copy.hostStartup)}</div>
    ${optionGroup({
      label: copy.hostStartup,
      actions: [
        { id: "host-mode", label: copy.hostAndClient, pressed: snap.hostMode === "host-and-client", data: { id: "host-and-client" } },
        { id: "host-mode", label: copy.clientOnly, pressed: snap.hostMode === "client-only", data: { id: "client-only" } },
      ],
    })}
    <p class="hint">${escapeHtml(copy.hostModeHelp)} ${escapeHtml(copy.restartToApply)}</p>
    ${checkbox({ label: copy.startAtLogin, checked: ui.startAtLogin === true, disabled: ui.startAtLogin == null, attributes: { "data-field": "startAtLogin" } })}
    <p class="hint">${escapeHtml(copy.startAtLoginHelp)}</p>
    ${ui.startupSettingsError ? notice({ status: "danger", message: ui.startupSettingsError }) : ""}
  </div>`;
}

export function updateSettings(copy: ShellCopy): string {
  const status = ui.updateState.kind === "checking"
    ? copy.updateChecking
    : ui.updateState.kind === "current"
      ? copy.updateCurrent
      : ui.updateState.kind === "failed"
        ? `${copy.updateFailed} ${ui.updateState.message}`.trim()
        : ui.updateState.kind === "blocked"
          ? `${copy.updateActiveRuns} (${ui.updateState.activeRunCount})`
          : "";
  const checking = ui.updateState.kind === "checking" || ui.updateState.kind === "installing";
  return `<div class="field update-settings">
    <div class="label">${escapeHtml(copy.updates)}</div>
    ${desktopShellAvailable()
      ? button({ id: "check-updates", label: ui.updateState.kind === "checking" ? copy.updateChecking : copy.checkForUpdates, disabled: checking })
      : `<p class="hint">${escapeHtml(copy.updateUnavailableBrowser)}</p>`}
    ${status ? `<p class="hint update-status">${escapeHtml(status)}</p>` : ""}
  </div>`;
}

export function updateDialog(copy: ShellCopy): string {
  const localCopy = startupCopy(effectiveClientLanguage());
  if (ui.updateState.kind === "available") {
    const body = `${notice({ message: copy.updateReady })}${ui.updateState.notes ? `<div class="field"><div class="label">${escapeHtml(copy.updateNotes)}</div><p class="update-notes">${escapeHtml(ui.updateState.notes)}</p></div>` : ""}`;
    return dialog({
      id: "update",
      tier: "confirm",
      title: `${copy.updateAvailable} ${ui.updateState.version}`,
      body,
      closeLabel: localCopy.close,
      dismissible: true,
      initialFocus: "primary",
      className: "update-dialog",
      actions: `${dialogDismissButton(copy.updateLater)}${dialogActionButton({ id: "install-update", label: copy.updateConfirm }, { primary: true, initialFocus: true })}`,
    });
  }
  if (ui.updateState.kind === "blocked") {
    const count = localCopy.interruptedRunCount.replace("{count}", String(ui.updateState.activeRunCount));
    return confirmationDialog({
      id: "update",
      title: copy.updateAvailable,
      body: `${notice({ status: "danger", message: localCopy.updateBlockedBody })}<p class="hint">${escapeHtml(count)}</p>`,
      closeLabel: localCopy.close,
      cancelLabel: copy.updateLater,
      confirm: { id: "install-update", label: localCopy.updateRetry, destructive: true },
    });
  }
  if (ui.updateState.kind === "installing") {
    const progress = ui.updateState.progress;
    const progressLabel = progress == null ? copy.updateInstalling : `${copy.updateInstalling} ${progress}%`;
    return dialog({
      id: "update",
      tier: "confirm",
      title: copy.updateInstalling,
      body: progressFeedback({ message: `${localCopy.updateInstallingBody} ${progressLabel}`, progress }),
      closeLabel: localCopy.close,
      dismissible: false,
      initialFocus: "none",
      busy: true,
      className: "update-dialog",
    });
  }
  return "";
}

export function keyboardHelpDialog(copy: ShellCopy): string {
  const localCopy = startupCopy(effectiveClientLanguage());
  return dialog({
    id: "keyboard-help",
    tier: "confirm",
    title: copy.keyboardHelp,
    body: `${shortcutList(localCopy)}<p class="hint">${escapeHtml(localCopy.shortcutsScope)}</p>`,
    closeLabel: localCopy.close,
    dismissible: true,
    initialFocus: "primary",
    className: "keyboard-help",
    actions: dialogDismissButton(copy.gotIt, { initialFocus: true }),
  });
}

/** Renders the same table the keydown handler dispatches from, so the help list cannot drift. */
function shortcutList(copy: StartupCopy): string {
  return `<ul class="shortcut-list">${SHELL_SHORTCUTS.map((shortcut) => `<li data-shortcut="${shortcut.id}">
      <span class="shortcut-keys">${shortcutKeyLabels(shortcut.id).map((key) => `<kbd>${escapeHtml(key)}</kbd>`).join("")}</span>
      <span class="shortcut-label">${escapeHtml(copy.shortcuts[shortcut.id])}</span>
    </li>`).join("")}</ul>`;
}
