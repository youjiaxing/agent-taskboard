import { isTauri } from "@tauri-apps/api/core";
import { ui } from "./ui";
import type {
  AppearancePreference,
  BoardViewMemory,
  BrowserAppearance,
  GraphViewportAnchor,
  PrimaryPage,
  Project,
  ResolvedTheme,
  ReturnPoint,
  SystemAppearance,
  RunSummary,
  ShellCopy,
  Snapshot,
} from "./protocol";

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
    const mobileProjectHistoryOpen = mobileClient()
      && ui.mobileRunHistoryScope === "project"
      && ui.mobileWorkspaceSection === "runs";
    if (ui.clientView.page === "focus-workspace" && (snap.board?.selected || mobileProjectHistoryOpen)) return;
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

function graphCanvasNode(id: string): { canvas: HTMLElement; node: HTMLElement } | null {
  const canvas = ui.app.querySelector<HTMLElement>(".graph-canvas");
  const node = canvas
    ? [...canvas.querySelectorAll<HTMLElement>(".graph-node")].find((item) => item.dataset.id === id)
    : null;
  return canvas && node ? { canvas, node } : null;
}

function nodeViewportAnchor(canvas: HTMLElement, node: HTMLElement): GraphViewportAnchor | null {
  if (!node.dataset.id) return null;
  const canvasRect = canvas.getBoundingClientRect();
  const nodeRect = node.getBoundingClientRect();
  return {
    issueId: node.dataset.id,
    viewportX: nodeRect.left - canvasRect.left + nodeRect.width / 2,
    viewportY: nodeRect.top - canvasRect.top + nodeRect.height / 2,
  };
}

/** Keeps one named node at its current viewport position while the graph re-renders. */
export function captureGraphAnchor(issueId: string): GraphViewportAnchor | null {
  const found = graphCanvasNode(issueId);
  return found ? nodeViewportAnchor(found.canvas, found.node) : null;
}

/**
 * The node the viewport is looking at: the center Issue while the graph is focused, otherwise
 * the node nearest the viewport centre. Used to restore the graph after a page round trip.
 */
export function captureGraphViewportAnchor(centerId: string | null | undefined): GraphViewportAnchor | null {
  const canvas = ui.app.querySelector<HTMLElement>(".graph-canvas");
  if (!canvas) return null;
  const nodes = [...canvas.querySelectorAll<HTMLElement>(".graph-node")];
  const centered = centerId ? nodes.find((node) => node.dataset.id === centerId) : undefined;
  if (centered) return nodeViewportAnchor(canvas, centered);
  const canvasRect = canvas.getBoundingClientRect();
  const centreX = canvasRect.left + canvasRect.width / 2;
  const centreY = canvasRect.top + canvasRect.height / 2;
  let nearest: HTMLElement | null = null;
  let nearestDistance = Number.POSITIVE_INFINITY;
  for (const node of nodes) {
    const rect = node.getBoundingClientRect();
    const distance = Math.hypot(rect.left + rect.width / 2 - centreX, rect.top + rect.height / 2 - centreY);
    if (distance < nearestDistance) {
      nearest = node;
      nearestDistance = distance;
    }
  }
  return nearest ? nodeViewportAnchor(canvas, nearest) : null;
}

export function restoreGraphAnchor(canvas: HTMLElement, anchor: GraphViewportAnchor): boolean {
  const node = [...canvas.querySelectorAll<HTMLElement>(".graph-node")]
    .find((item) => item.dataset.id === anchor.issueId);
  if (!node) return false;
  const flow = canvas.querySelector<HTMLElement>(".graph-flow");
  const canvasRect = canvas.getBoundingClientRect();
  const nodeRect = node.getBoundingClientRect();
  const currentX = nodeRect.left - canvasRect.left + nodeRect.width / 2;
  const currentY = nodeRect.top - canvasRect.top + nodeRect.height / 2;
  let nextLeft = canvas.scrollLeft + currentX - anchor.viewportX;
  let nextTop = canvas.scrollTop + currentY - anchor.viewportY;
  if (flow && nextLeft < 0) {
    const padding = Number.parseFloat(getComputedStyle(flow).paddingLeft) || 0;
    flow.style.paddingLeft = `${padding - nextLeft}px`;
    nextLeft = 0;
  }
  if (flow && nextTop < 0) {
    const padding = Number.parseFloat(getComputedStyle(flow).paddingTop) || 0;
    flow.style.paddingTop = `${padding - nextTop}px`;
    nextTop = 0;
  }
  let maxLeft = Math.max(0, canvas.scrollWidth - canvas.clientWidth);
  let maxTop = Math.max(0, canvas.scrollHeight - canvas.clientHeight);
  if (flow && nextLeft > maxLeft) {
    const padding = Number.parseFloat(getComputedStyle(flow).paddingRight) || 0;
    flow.style.paddingRight = `${padding + nextLeft - maxLeft}px`;
    maxLeft = Math.max(0, canvas.scrollWidth - canvas.clientWidth);
  }
  if (flow && nextTop > maxTop) {
    const padding = Number.parseFloat(getComputedStyle(flow).paddingBottom) || 0;
    flow.style.paddingBottom = `${padding + nextTop - maxTop}px`;
    maxTop = Math.max(0, canvas.scrollHeight - canvas.clientHeight);
  }
  canvas.scrollLeft = Math.max(0, Math.min(maxLeft, nextLeft));
  canvas.scrollTop = Math.max(0, Math.min(maxTop, nextTop));
  return true;
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
  const graphAnchor = ui.clientView.page === "dependency-graph"
    ? captureGraphViewportAnchor(snap.board?.graph?.centerId)
    : null;
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
      ? { projectId, viewportAnchor: graphAnchor ?? (ui.pendingGraphAnchor ? { ...ui.pendingGraphAnchor } : null) }
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

/**
 * The Host navigation mirror owns the current Issue and Run, so choosing an Issue or Run moves
 * that mirror after the return point was captured. Keep the return point on the mirror the
 * user actually leaves behind.
 */
export function syncReturnPointNavigation(): void {
  const returnPoint = ui.clientView.returnPoint;
  if (!returnPoint || !ui.snapshot) return;
  returnPoint.issueId = ui.snapshot.board?.selected?.id ?? null;
  returnPoint.runId = ui.snapshot.focusedRunId || null;
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

/** Readable mobile terminal text is cached per Host and Run. */
export function mobileOutputKey(hostId: string, runId: string): string {
  return JSON.stringify([hostId, runId]);
}

/**
 * The mobile client reads readable Run output only while its workspace shows that Run's terminal:
 * the Issue and history sections never poll a PTY.
 */
export function mobileReadableRun(snap: Snapshot): RunSummary | undefined {
  if (!mobileClient() || ui.clientView.page !== "focus-workspace") return undefined;
  if (ui.mobileWorkspaceSection !== "terminal" || ui.mobileLiveTerminal) return undefined;
  const run = workspaceRun(snap);
  return run && run.status !== "ended" ? run : undefined;
}

export function focusedRun(snap: Snapshot): RunSummary | undefined {
  return (snap.runs ?? []).find((run) => run.id === snap.focusedRunId);
}

export function workspaceRun(snap: Snapshot): RunSummary | undefined {
  const focused = focusedRun(snap);
  const issueId = snap.board?.selected?.id;
  if (focused && (!issueId || focused.unbound || focused.issueId === issueId)) return focused;
  if (!issueId) return focused;
  const issueRuns = (snap.runs ?? []).filter((run) => run.issueId === issueId);
  return issueRuns.find((run) => run.status !== "ended") ?? issueRuns[issueRuns.length - 1];
}

export function resetGraphUiState(): void {
  ui.graphCanvasLimit = 48;
  ui.graphListLimit = 50;
  ui.graphListQuery = "";
}

export function clientCopy(language: import("./protocol").Language, fallback: ShellCopy): ShellCopy {
  return ui.snapshot?.copyCatalog?.[language] ?? fallback;
}

/** Every browser Client keeps its own appearance preference; only the desktop app reads the Host's. */
export function effectiveClientLanguage(): import("./protocol").Language {
  if (browserClient()) return ensureBrowserAppearance().language;
  return ui.snapshot?.appearance.language ?? "en";
}
