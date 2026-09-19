import { connectionPanel, pendingBar } from "../main";
import { escapeHtml } from "../client-utils";
import type { MobileWorkspaceSection, RunSummary, ShellCopy, Snapshot } from "../protocol";
import type { StartupCopy } from "../startup-copy";
import { appearancePreferenceLabel } from "../startup-copy";
import { ui } from "../ui";
import { APPEARANCE_DISPLAY_ORDER, currentProject, effectiveAppearancePreference, focusedRun, mobileOutputKey, workspaceRun } from "../view-helpers";
import { boardLanes, boardUnavailable, issueDetail, issueSearch, refreshBar, workspaceRailLabels, workspaceRunHistory } from "./board";
import { loopbackNotice } from "./run";
import { emptyTerminalSurface, injectRunForm, projectTrackerIdentity, readOnlyTerminal, runHeader, runNotices, telemetryBar, terminalPanel } from "./shell";
import { button, optionGroup } from "../components/primitives";
import { dialog, dialogDismissButton, drawer } from "../components/dialog";
import type { IssueLaneState } from "../components/issue";

/** Mobile reads the board top-down: what is being worked on first, then what is claimable. */
const MOBILE_LANE_ORDER: IssueLaneState[] = ["inProgress", "frontier", "blocked", "recentlyCompleted"];

/** Mobile has one bottom area: the views the user switches between, never a setting or a business confirmation. */
export function mobileNav(copy: ShellCopy, localCopy: StartupCopy, snap: Snapshot): string {
  const page = ui.clientView.page;
  const workspaceAvailable = Boolean(workspaceRun(snap) ?? focusedRun(snap) ?? snap.board?.selected);
  const item = (id: "board" | "focus-workspace", label: string, disabled = false) =>
    button(
      { id: "mobile-nav", label, disabled, pressed: page === id, data: { id } },
      { variant: page === id ? "primary" : "ghost", className: "mobile-nav-item" },
    );
  return `<nav class="mobile-nav" aria-label="${escapeHtml(localCopy.mobileNav)}">
    ${item("board", copy.mobileBoard)}
    ${item("focus-workspace", localCopy.mobileWorkspace, !workspaceAvailable)}
  </nav>`;
}

/** The mobile drawer owns every entry the mobile top bar and bottom navigation must not carry. */
export function mobileDrawer(copy: ShellCopy, localCopy: StartupCopy, snap: Snapshot): string {
  const preference = effectiveAppearancePreference(snap);
  const hosts = snap.hosts
    .map((host) => `<button type="button" class="mobile-row ${host.id === snap.focusedHostId ? "active" : ""}" data-act="focus-host" data-id="${escapeHtml(host.id)}"><span class="dot"></span><span class="mobile-row-label">${escapeHtml(host.displayName)}</span>${host.local ? `<span class="tag">${escapeHtml(copy.thisMachine)}</span>` : ""}</button>`)
    .join("");
  const projects = snap.projects
    .map((project) => `<div class="mobile-project-row ${project.id === snap.focusedProjectId ? "active" : ""}">
      <button type="button" class="mobile-row" data-act="focus-project" data-id="${escapeHtml(project.id)}"><span class="mobile-row-label"><b>${escapeHtml(project.name)}</b><small>${escapeHtml(projectTrackerIdentity(project, localCopy.localMarkdownTracker))}</small></span></button>
      <button type="button" class="mobile-row-action" data-act="edit-project" data-id="${escapeHtml(project.id)}">${escapeHtml(copy.editProject)}</button>
      <button type="button" class="mobile-row-action danger" data-act="remove-project" data-id="${escapeHtml(project.id)}">${escapeHtml(copy.removeProject)}</button>
    </div>`)
    .join("");
  const entry = (id: string, label: string, disabled = false) =>
    button({ id, label, disabled }, { variant: "ghost", className: "mobile-entry" });
  const issueSelected = Boolean(snap.board?.selected);
  const body = `<div class="mobile-drawer-body">
    <section class="mobile-drawer-group">
      <div class="group-head"><div class="group-name">${escapeHtml(copy.hosts)}</div></div>
      ${hosts}
      <div class="mobile-drawer-group-actions">
        ${entry("open-overview", copy.hostOverview)}
        ${entry("open-usage", copy.usage)}
      </div>
    </section>
    <section class="mobile-drawer-group">
      <div class="group-head">
        <div class="group-name">${escapeHtml(copy.projects)}</div>
        ${entry("register", copy.addProject)}
      </div>
      ${projects || `<div class="nested">${escapeHtml(copy.noProjectTitle)}</div>`}
    </section>
    <section class="mobile-drawer-group">
      <div class="group-head"><div class="group-name">${escapeHtml(copy.mobileIssue)}</div></div>
      ${entry("mobile-issue-entry", copy.mobileIssue, !issueSelected)}
      ${entry("mobile-history-entry", localCopy.mobileHistory, !issueSelected)}
      ${entry("mobile-search-entry", copy.searchSubmit)}
      <div class="mobile-drawer-appearance">
        ${entry("mobile-appearance-entry", localCopy.appearance)}
        ${ui.mobileDrawerAppearanceOpen
          ? optionGroup({
              label: localCopy.appearance,
              actions: APPEARANCE_DISPLAY_ORDER.map((option) => ({
                id: "appearance",
                label: appearancePreferenceLabel(localCopy, option),
                pressed: option === preference,
                data: { id: option },
              })),
            })
          : ""}
      </div>
      ${entry("mobile-settings-entry", copy.settings)}
      ${entry("pair", copy.pairAnotherHost)}
    </section>
  </div>`;
  return drawer({
    id: "mobile-drawer",
    title: copy.mobileSwitchScope,
    closeLabel: localCopy.close,
    className: "mobile-drawer",
    body,
  });
}

/** Issue search stays a short-lived form; its result lands on the board it filters. */
export function mobileSearchDialog(copy: ShellCopy, localCopy: StartupCopy, snap: Snapshot): string {
  return dialog({
    id: "mobile-search",
    tier: "form",
    title: copy.searchTitle,
    body: issueSearch(copy, snap),
    closeLabel: localCopy.close,
    dismissible: true,
    initialFocus: "first-field",
    actions: dialogDismissButton(copy.cancel),
  });
}

/** Mobile has no dependency-graph or multi-panel page; the board composition is its fallback view. */
export function mobilePage(copy: ShellCopy, localCopy: StartupCopy, snap: Snapshot): string {
  return ui.clientView.page === "focus-workspace"
    ? mobileWorkspacePage(copy, localCopy, snap)
    : mobileBoardPage(copy, snap);
}

/** Mobile draws the board without the desktop panels, graph switch, create form, or inspector. */
export function mobileBoardPage(copy: ShellCopy, snap: Snapshot): string {
  const unavailable = boardUnavailable(copy, snap);
  if (unavailable) return `<section class="mobile-board-view">${loopbackNotice(snap.loopbackPage)}${unavailable}</section>`;
  const lanes = boardLanes(copy, snap.board!, MOBILE_LANE_ORDER);
  const project = currentProject(snap);
  return `<section class="mobile-board-view">
    ${loopbackNotice(snap.loopbackPage)}
    ${pendingBar(copy, snap)}
    <div class="mobile-board-status" data-page-toolbar>
      ${refreshBar(copy, snap.board!)}
      ${project ? connectionPanel(copy, project) : ""}
    </div>
    ${lanes}
  </section>`;
}

/** One panel at a time: the Run's terminal, the Issue it works on, or its Run history. */
export function mobileWorkspacePage(copy: ShellCopy, localCopy: StartupCopy, snap: Snapshot): string {
  const board = snap.board;
  const issue = board?.selected;
  const run = workspaceRun(snap);
  const labels = workspaceRailLabels();
  const sections: Array<[MobileWorkspaceSection, string]> = [
    ["terminal", copy.mobileRun],
    ["issue", labels.issue],
    ["runs", labels.runs],
  ];
  const section = ui.mobileWorkspaceSection;
  const issueRuns = issue ? (snap.runs ?? []).filter((candidate) => candidate.issueId === issue.id) : [];
  const panel = section === "issue"
    ? issue
      ? `<aside class="issue-detail mobile-issue-panel">${issueDetail(copy, board!, { dependencyGraph: false })}</aside>`
      : `<p class="board-empty">${escapeHtml(copy.pickIssue)}</p>`
    : section === "runs"
      ? (issue ? workspaceRunHistory(copy, issueRuns) : `<p class="board-empty">${escapeHtml(copy.pickIssue)}</p>`)
      : mobileTerminalPanel(copy, snap, run);
  return `<section class="mobile-workspace-view">
    ${optionGroup({
      label: localCopy.mobileWorkspace,
      className: "mobile-section-switch",
      actions: sections.map(([id, label]) => ({
        id: "mobile-workspace-section",
        label,
        pressed: section === id,
        data: { id },
      })),
    })}
    <div class="mobile-workspace-panel">${panel}</div>
  </section>`;
}

/** The active Run's input is the last frame row, so it carries the device bottom gap; it stays available across the focus workspace sections, like the desktop terminal. */
export function mobileRunInput(copy: ShellCopy, snap: Snapshot): string {
  if (ui.clientView.page !== "focus-workspace") return "";
  const run = workspaceRun(snap);
  if (!run || run.status === "ended") return "";
  return `<div class="mobile-input-row" data-mobile-run-input>${injectRunForm(copy, run)}</div>`;
}

/** Mobile shows readable Run text and keeps the live official TUI one explicit action away. */
function mobileTerminalPanel(copy: ShellCopy, snap: Snapshot, run: RunSummary | undefined): string {
  if (!run) return emptyTerminalSurface(copy, snap);
  if (run.status === "ended") return readOnlyTerminal(copy, run);
  if (ui.mobileLiveTerminal) return terminalPanel(copy, run, "focus-terminal-surface mobile-live-terminal");
  const readable = ui.mobilePtyText.get(mobileOutputKey(snap.focusedHostId, run.id)) ?? run.recentOutput ?? "";
  return `<div class="mobile-output-panel" data-terminal-surface="readable" data-run="${escapeHtml(run.id)}">
    ${runHeader(copy, run)}
    ${telemetryBar(copy, run)}
    <div class="lane-hd">${escapeHtml(copy.mobileRecentOutput)}</div>
    <pre class="mobile-run-output" data-run="${escapeHtml(run.id)}">${escapeHtml(readable)}</pre>
    ${runNotices(copy, run)}
    <button type="button" class="ghost mobile-terminal-escape" data-act="mobile-live-terminal">${escapeHtml(copy.mobileLiveTerminal)}</button>
  </div>`;
}
