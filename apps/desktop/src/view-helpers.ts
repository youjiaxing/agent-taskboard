import { isTauri } from "@tauri-apps/api/core";
import { ui } from "./ui";
import type {
  AppearancePreference,
  BoardSnapshot,
  BoardViewMemory,
  BrowserAppearance,
  PrimaryPage,
  Project,
  ResolvedTheme,
  ReturnPoint,
  SystemAppearance,
  RunSummary,
  ShellCopy,
  Snapshot,
} from "./protocol";
import { mobileMain as renderMobileMain, mobileNavigation as renderMobileNavigation, mobileScopeSheet as renderMobileScopeSheet } from "./mobile-renderers";
import { issueDetail, projectMain } from "./render/board";
import { injectRunForm, telemetryBar } from "./render/shell";

export const MOBILE_BREAKPOINT = 640;
export const FULL_DESKTOP_BREAKPOINT = 900;
export const APPEARANCE_PREFERENCES: AppearancePreference[] = ["system", "light", "dark", "warm"];
export const APPEARANCE_DISPLAY_ORDER: AppearancePreference[] = ["warm", "light", "dark", "system"];

export function viewportClass(): "mobile" | "compact-desktop" | "full-desktop" {
  if (window.innerWidth < MOBILE_BREAKPOINT) return "mobile";
  if (window.innerWidth < FULL_DESKTOP_BREAKPOINT) return "compact-desktop";
  return "full-desktop";
}
const BROWSER_APPEARANCE_KEY = "agent-taskboard-browser-appearance";

export function mobileClient(): boolean {
  return window.matchMedia(`(max-width: ${MOBILE_BREAKPOINT - 0.02}px)`).matches;
}

export function browserClient(): boolean {
  return !(isTauri() || "__TAURI_INTERNALS__" in window);
}

export function currentSystemAppearance(): SystemAppearance {
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

export function defaultBrowserAppearance(): BrowserAppearance {
  return {
    language: navigator.language.toLowerCase().startsWith("zh") ? "zh-CN" : "en",
    appearancePreference: "system",
  };
}

export function loadBrowserAppearance(): BrowserAppearance | null {
  try {
    const raw = localStorage.getItem(BROWSER_APPEARANCE_KEY);
    if (!raw) return null;
    const value = JSON.parse(raw) as Partial<BrowserAppearance>;
    if (
      (value.language === "zh-CN" || value.language === "en")
      && APPEARANCE_PREFERENCES.includes(value.appearancePreference as AppearancePreference)
    ) {
      return value as BrowserAppearance;
    }
  } catch {
    // Invalid local settings use the new defaults.
  }
  return null;
}

export function ensureBrowserAppearance(): BrowserAppearance {
  if (!ui.browserAppearance) {
    ui.browserAppearance = defaultBrowserAppearance();
    saveBrowserAppearance(ui.browserAppearance);
  }
  return ui.browserAppearance;
}

export function saveBrowserAppearance(appearance: BrowserAppearance): void {
  ui.browserAppearance = appearance;
  localStorage.setItem(BROWSER_APPEARANCE_KEY, JSON.stringify(appearance));
}

export function resolveTheme(
  preference: AppearancePreference,
  systemAppearance: SystemAppearance,
): ResolvedTheme {
  return preference === "system" ? systemAppearance : preference;
}

export function effectiveAppearancePreference(snapshot: Snapshot): AppearancePreference {
  return browserClient()
    ? ensureBrowserAppearance().appearancePreference
    : snapshot.appearance.appearancePreference;
}


export function currentProject(snap: Snapshot): Project | undefined {
  return (
    snap.projects.find((project) => project.id === snap.focusedProjectId) ?? snap.projects[0]
  );
}

export function primaryPageFromSnapshot(snap: Snapshot): PrimaryPage {
  if (snap.usageOpen) return "usage";
  if (snap.workspaceView === "host-overview") return "host-overview";
  if (snap.workspaceView === "run") return "focus-workspace";
  return snap.centerView === "graph" ? "dependency-graph" : "board";
}

export function syncClientPrimaryPage(snap: Snapshot): void {
  if (!ui.clientViewInitialized) {
    ui.clientView.page = primaryPageFromSnapshot(snap);
    ui.clientViewInitialized = true;
    return;
  }
  if (ui.clientView.page !== "settings") {
    ui.clientView.page = primaryPageFromSnapshot(snap);
  }
}

function cloneBoardMemory(memory: BoardViewMemory | undefined): BoardViewMemory | null {
  if (!memory) return null;
  return {
    projectId: memory.projectId,
    scroll: { ...memory.scroll },
    lanes: Object.fromEntries(
      Object.entries(memory.lanes).map(([lane, position]) => [lane, { ...position }]),
    ),
  };
}

export function captureReturnPoint(snap: Snapshot): ReturnPoint {
  const projectId = snap.focusedProjectId || null;
  const lanesNode = ui.app.querySelector<HTMLElement>(".lanes");
  if (projectId && lanesNode) {
    const lanes: Record<string, { scrollTop: number; scrollLeft: number }> = {};
    for (const lane of lanesNode.querySelectorAll<HTMLElement>(".lane[data-lane]")) {
      if (lane.dataset.lane) {
        lanes[lane.dataset.lane] = { scrollTop: lane.scrollTop, scrollLeft: lane.scrollLeft };
      }
    }
    ui.boardScrollPositions.set(projectId, {
      scrollTop: lanesNode.scrollTop,
      scrollLeft: lanesNode.scrollLeft,
      lanes,
    });
  }
  const boardScroll = projectId ? ui.boardScrollPositions.get(projectId) : undefined;
  return {
    page: ui.clientView.page,
    hostId: snap.focusedHostId,
    projectId,
    issueId: snap.board?.selected?.id ?? null,
    runId: snap.focusedRunId || null,
    board: cloneBoardMemory(
      projectId && boardScroll
        ? {
            projectId,
            scroll: { scrollTop: boardScroll.scrollTop, scrollLeft: boardScroll.scrollLeft },
            lanes: boardScroll.lanes,
          }
        : undefined,
    ),
    graph: projectId
      ? { projectId, viewportAnchor: ui.pendingGraphAnchor ? { ...ui.pendingGraphAnchor } : null }
      : null,
  };
}

export function enterPrimaryPage(page: PrimaryPage, snap: Snapshot): void {
  if (ui.clientView.page !== page) {
    if (ui.clientView.returnPoint) ui.returnPointHistory.push(ui.clientView.returnPoint);
    ui.clientView.returnPoint = captureReturnPoint(snap);
  }
  ui.clientView.page = page;
}

export function restoreReturnPointMemory(returnPoint: ReturnPoint): void {
  if (returnPoint.board) {
    ui.boardScrollPositions.set(returnPoint.board.projectId, {
      ...returnPoint.board.scroll,
      lanes: Object.fromEntries(
        Object.entries(returnPoint.board.lanes).map(([lane, position]) => [lane, { ...position }]),
      ),
    });
  }
  if (returnPoint.graph?.viewportAnchor) {
    ui.pendingGraphAnchor = { ...returnPoint.graph.viewportAnchor };
  }
  ui.clientView.page = returnPoint.page;
  ui.clientView.returnPoint = ui.returnPointHistory.pop() ?? null;
}

export function mobileNavigation(copy: ShellCopy, snap: Snapshot): string {
  return renderMobileNavigation(copy, snap, ui.mobileView);
}

export function mobileScopeSheet(copy: ShellCopy, snap: Snapshot): string {
  return renderMobileScopeSheet(copy, snap);
}

export function mobileMain(copy: ShellCopy, snap: Snapshot): string {
  return renderMobileMain({
    copy,
    snapshot: snap,
    mobileView: ui.mobileView,
    mobileLiveTerminal: ui.mobileLiveTerminal,
    mobilePtyText: ui.mobilePtyText,
    focusedRun: (mobileSnapshot) => focusedRun(mobileSnapshot as Snapshot),
    issueDetail: (mobileCopy, board, showPanelToggle) => issueDetail(mobileCopy as ShellCopy, board as BoardSnapshot, showPanelToggle),
    projectMain: (mobileCopy, mobileSnapshot) => projectMain(mobileCopy as ShellCopy, mobileSnapshot as Snapshot),
    telemetryBar: (mobileCopy, run) => telemetryBar(mobileCopy as ShellCopy, run as RunSummary),
    injectRunForm: (mobileCopy, run) => injectRunForm(mobileCopy as ShellCopy, run as RunSummary),
    board: snap.board,
  });
}

export function focusedRun(snap: Snapshot): RunSummary | undefined {
  return (snap.runs ?? []).find((run) => run.id === snap.focusedRunId);
}

export function resetGraphUiState(): void {
  ui.graphCanvasLimit = 48;
  ui.graphListLimit = 50;
  ui.graphListQuery = "";
}

export function clientCopy(language: import("./protocol").Language, fallback: ShellCopy): ShellCopy {
  return ui.snapshot?.copyCatalog?.[language] ?? fallback;
}

export function effectiveClientLanguage(): import("./protocol").Language {
  if (mobileClient()) return ensureBrowserAppearance().language;
  return ui.snapshot?.appearance.language ?? "en";
}
