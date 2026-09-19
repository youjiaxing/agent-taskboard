import { escapeHtml } from "./client-utils";
import { drawer } from "./components/dialog";

type MobileCopy = {
  mobileSwitchScope: string;
  mobileBoard: string;
  mobileIssue: string;
  mobileRun: string;
  mobileRecentOutput: string;
  mobileLiveTerminal: string;
  editProject: string;
  removeProject: string;
  addProject: string;
  pairAnotherHost: string;
  usage: string;
  thisMachine: string;
  noItems: string;
  stopRun: string;
  unboundIssue: string;
  cancel: string;
};

type MobileHost = { id: string; displayName: string; local: boolean };
type MobileProject = { id: string; name: string; repository: string };
type MobileRun = {
  id: string;
  agentName: string;
  issueId?: string | null;
  unbound: boolean;
  status: "starting" | "running" | "ended";
  recentOutput?: string;
};

type MobileSnapshot = {
  runs: MobileRun[];
  focusedHostId: string;
  focusedProjectId: string;
  hosts: MobileHost[];
  projects: MobileProject[];
  focusedRunId: string;
};

export type MobileView = "board" | "issue" | "run";

export type MobileRendererOptions = {
  copy: MobileCopy;
  snapshot: MobileSnapshot;
  mobileView: MobileView;
  mobileLiveTerminal: boolean;
  mobilePtyText: Map<string, string>;
  focusedRun: (snapshot: MobileSnapshot) => MobileRun | undefined;
  issueDetail: (copy: MobileCopy, board: unknown, showPanelToggle?: boolean) => string;
  projectMain: (copy: MobileCopy, snapshot: MobileSnapshot) => string;
  telemetryBar: (copy: MobileCopy, run: MobileRun) => string;
  injectRunForm: (copy: MobileCopy, run: MobileRun) => string;
  board: unknown;
};

export function mobileNavigation(copy: MobileCopy, snapshot: MobileSnapshot, mobileView: MobileView): string {
  const run = snapshot.runs?.find((candidate) => candidate.id === snapshot.focusedRunId);
  return `<nav class="mobile-nav" aria-label="${escapeHtml([copy.mobileBoard, copy.mobileIssue, copy.mobileRun].join(" / "))}">
    <button type="button" class="${mobileView === "board" ? "active" : ""}" data-act="mobile-board">${escapeHtml(copy.mobileBoard)}</button>
    <button type="button" class="${mobileView === "issue" ? "active" : ""}" data-act="mobile-issue">${escapeHtml(copy.mobileIssue)}</button>
    <button type="button" class="${mobileView === "run" ? "active" : ""}" data-act="mobile-run" ${run ? "" : "disabled"}>${escapeHtml(copy.mobileRun)}</button>
  </nav>`;
}

export function mobileScopeSheet(copy: MobileCopy, snapshot: MobileSnapshot): string {
  const hosts = snapshot.hosts
    .map((host) => `<button type="button" class="item ${host.id === snapshot.focusedHostId ? "active" : ""}" data-act="focus-host" data-id="${escapeHtml(host.id)}"><span class="dot"></span>${escapeHtml(host.displayName)}${host.local ? `<span class="tag">${escapeHtml(copy.thisMachine)}</span>` : ""}</button>`)
    .join("");
  const projects = snapshot.projects
    .map((project) => `<div class="mobile-scope-project ${project.id === snapshot.focusedProjectId ? "active" : ""}">
      <button type="button" class="project-main" data-act="focus-project" data-id="${escapeHtml(project.id)}"><b>${escapeHtml(project.name)}</b><span>${escapeHtml(project.repository)}</span></button>
      <button type="button" data-act="edit-project" data-id="${escapeHtml(project.id)}">${escapeHtml(copy.editProject)}</button>
      <button type="button" class="danger" data-act="remove-project" data-id="${escapeHtml(project.id)}">${escapeHtml(copy.removeProject)}</button>
    </div>`)
    .join("");
  return drawer({
    id: "mobile-scope",
    title: copy.mobileSwitchScope,
    closeLabel: copy.cancel,
    className: "mobile-scope-sheet",
    body: `<div class="mobile-scope-hosts">${hosts}</div>
      <div class="mobile-scope-projects">${projects}</div>
      <div class="actions">
        <button type="button" data-act="register">${escapeHtml(copy.addProject)}</button>
        <button type="button" data-act="pair">${escapeHtml(copy.pairAnotherHost)}</button>
        <button type="button" data-act="open-usage">${escapeHtml(copy.usage)}</button>
      </div>`,
  });
}

export function mobileMain(options: MobileRendererOptions): string {
  const { copy, snapshot, mobileView } = options;
  if (mobileView === "run") return mobileRunView(options);
  if (mobileView === "issue") {
    return `<section class="mobile-issue-view"><aside class="issue-detail">${options.board ? options.issueDetail(copy, options.board, false) : ""}</aside></section>`;
  }
  return `<section class="mobile-board-view">${options.projectMain(copy, snapshot)}</section>`;
}

function mobileRunView(options: MobileRendererOptions): string {
  const { copy, snapshot, mobileLiveTerminal, mobilePtyText, focusedRun } = options;
  const run = focusedRun(snapshot);
  if (!run) {
    return `<section class="mobile-run-view"><p class="board-empty">${escapeHtml(copy.noItems)}</p></section>`;
  }
  const identity = run.unbound || !run.issueId ? copy.unboundIssue : run.issueId;
  const liveTerminal = mobileLiveTerminal && run.status !== "ended";
  const outputKey = JSON.stringify([snapshot.focusedHostId, run.id]);
  return `<section class="mobile-run-view">
    <header class="run-dock-hd">
      <div><b>${escapeHtml(run.agentName)}</b><span>${escapeHtml(identity)}</span></div>
      <div class="actions">
        <button type="button" class="mobile-usage-entry" data-act="open-usage-run" data-id="${escapeHtml(run.id)}">${escapeHtml(copy.usage)}</button>
        <button type="button" data-act="stop-run" data-id="${escapeHtml(run.id)}" ${run.status === "ended" ? "disabled" : ""}>${escapeHtml(copy.stopRun)}</button>
      </div>
    </header>
    ${options.telemetryBar(copy, run)}
    <section class="mobile-output-panel">
      <div class="lane-hd">${escapeHtml(copy.mobileRecentOutput)}</div>
      ${liveTerminal
        ? `<div class="pty-slot" data-run="${escapeHtml(run.id)}"></div>`
        : `<pre class="mobile-run-output" data-run="${escapeHtml(run.id)}">${escapeHtml(run.status === "ended" ? run.recentOutput ?? mobilePtyText.get(outputKey) ?? "" : mobilePtyText.get(outputKey) ?? run.recentOutput ?? "")}</pre>`}
    </section>
    ${run.status !== "ended" ? options.injectRunForm(copy, run) : ""}
    ${run.status !== "ended" && !liveTerminal ? `<button type="button" class="ghost mobile-terminal-escape" data-act="mobile-live-terminal">${escapeHtml(copy.mobileLiveTerminal)}</button>` : ""}
  </section>`;
}
