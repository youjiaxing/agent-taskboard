import { sendPtyResize } from "./main";
import { effectiveClientLanguage } from "./view-helpers";
import type { BoardSnapshot, ShellCopy, WorkbenchLayout, WorkbenchPanelGeometry, WorkbenchPanelId } from "./protocol";
import { desktopShellAvailable } from "./launch-session";
import { escapeHtml } from "./client-utils";
import { issueDetail } from "./render/board";
import { mobileClient } from "./view-helpers";
import { render } from "./render/app";
import { ui } from "./ui";

export const WORKBENCH_LAYOUT_VERSION = 2;
export const WORKBENCH_LAYOUT_STORAGE_PREFIX = `agent-taskboard-panel-layout:v${WORKBENCH_LAYOUT_VERSION}:`;
export const WORKBENCH_LAYOUT_REGISTRY_KEY = `agent-taskboard-panel-layout-registry:v${WORKBENCH_LAYOUT_VERSION}`;
export const WORKBENCH_LAYOUT_INSTANCE_TTL_MS = 7 * 86_400_000;
export const WORKBENCH_LAYOUT_HEARTBEAT_MS = 5 * 60_000;
export const WORKBENCH_PANEL_DEFAULTS: WorkbenchLayout = {
  inspector: { width: 440, height: 640, x: 2_400, y: 12, floating: true, runFloating: false },
  terminal: { width: 820, height: 360, x: 80, y: 360, floating: false },
  usage: { width: 920, height: 680, x: 48, y: 28, floating: false },
};

export function clonePanelGeometry(geometry: WorkbenchPanelGeometry): WorkbenchPanelGeometry {
  return { ...geometry };
}

export function workbenchLayoutStorageKey(): string {
  const clientKind = desktopShellAvailable() ? "tauri" : "browser";
  let identity = ui.clientId;
  if (clientKind === "tauri") {
    try {
      const tauriInternals = (window as typeof window & {
        __TAURI_INTERNALS__?: { metadata?: { currentWindow?: { label?: string } } };
      }).__TAURI_INTERNALS__;
      identity = tauriInternals?.metadata?.currentWindow?.label ?? "";
    } catch {
      identity = "";
    }
    if (!identity) {
      const key = "agent-taskboard-tauri-layout-client-id";
      identity = localStorage.getItem(key) ?? "";
      if (!identity) {
        identity = `tauri-${crypto.randomUUID?.() ?? Date.now()}`;
        localStorage.setItem(key, identity);
      }
    }
  }
  return `${WORKBENCH_LAYOUT_STORAGE_PREFIX}${clientKind}:${identity}`;
}

export function readWorkbenchLayoutRegistry(): Record<string, number> {
  try {
    const raw = localStorage.getItem(WORKBENCH_LAYOUT_REGISTRY_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    return Object.fromEntries(
      Object.entries(parsed).filter(
        ([key, value]) =>
          key.startsWith(WORKBENCH_LAYOUT_STORAGE_PREFIX)
          && typeof value === "number"
          && Number.isFinite(value),
      ),
    ) as Record<string, number>;
  } catch {
    return {};
  }
}

export function writeWorkbenchLayoutRegistry(registry: Record<string, number>): void {
  try {
    localStorage.setItem(WORKBENCH_LAYOUT_REGISTRY_KEY, JSON.stringify(registry));
  } catch {
    // A restricted Client can still use the layout without the cleanup registry.
  }
}

export function workbenchLayoutStorageKeys(): string[] {
  const keys: string[] = [];
  for (let index = 0; index < localStorage.length; index += 1) {
    const key = localStorage.key(index);
    if (key?.startsWith(WORKBENCH_LAYOUT_STORAGE_PREFIX)) keys.push(key);
  }
  return keys;
}

export function removeHistoricalWorkbenchLayouts(currentKey: string): boolean {
  let changed = false;
  try {
    for (let index = localStorage.length - 1; index >= 0; index -= 1) {
      const key = localStorage.key(index);
      if (
        key
        && key.startsWith("agent-taskboard-panel-layout:v")
        && !key.startsWith(WORKBENCH_LAYOUT_STORAGE_PREFIX)
        && key !== currentKey
      ) {
        localStorage.removeItem(key);
        changed = true;
      }
      if (
        key
        && key.startsWith("agent-taskboard-panel-layout-registry:v")
        && key !== WORKBENCH_LAYOUT_REGISTRY_KEY
      ) {
        localStorage.removeItem(key);
        changed = true;
      }
    }
  } catch {
    // A restricted Client can keep historical entries without affecting this window.
  }
  return changed;
}

export function touchWorkbenchLayoutInstance(currentKey = workbenchLayoutStorageKey()): void {
  try {
    const now = Date.now();
    const registry = readWorkbenchLayoutRegistry();
    let changed = removeHistoricalWorkbenchLayouts(currentKey);
    for (const key of workbenchLayoutStorageKeys()) {
      const lastSeen = registry[key];
    if (
      key !== currentKey
      && (lastSeen == null || now - lastSeen > WORKBENCH_LAYOUT_INSTANCE_TTL_MS)
    ) {
        localStorage.removeItem(key);
        delete registry[key];
        changed = true;
      }
    }
    for (const key of Object.keys(registry)) {
      if (!localStorage.getItem(key) && key !== currentKey) {
        delete registry[key];
        changed = true;
      }
    }
    if (registry[currentKey] !== now) {
      registry[currentKey] = now;
      changed = true;
    }
    if (changed) writeWorkbenchLayoutRegistry(registry);
  } catch {
    // A restricted Client can still use the layout for this window.
  }
}

export function startWorkbenchLayoutHeartbeat(): void {
  window.setInterval(() => touchWorkbenchLayoutInstance(), WORKBENCH_LAYOUT_HEARTBEAT_MS);
}

export function normalizePanelGeometry(
  panelId: WorkbenchPanelId,
  candidate: Partial<WorkbenchPanelGeometry> | undefined,
): WorkbenchPanelGeometry {
  const fallback = WORKBENCH_PANEL_DEFAULTS[panelId];
  const number = (value: unknown, defaultValue: number, minimum: number, maximum: number) =>
    typeof value === "number" && Number.isFinite(value)
      ? Math.min(maximum, Math.max(minimum, value))
      : defaultValue;
  return {
    width: number(candidate?.width, fallback.width, 280, 1_200),
    height: number(candidate?.height, fallback.height, 180, 900),
    x: number(candidate?.x, fallback.x, 0, 2_400),
    y: number(candidate?.y, fallback.y, 0, 1_600),
    floating: typeof candidate?.floating === "boolean" ? candidate.floating : fallback.floating,
    ...(panelId === "inspector"
      ? {
          runFloating: typeof candidate?.runFloating === "boolean" ? candidate.runFloating : false,
          dockedWidth: number(
            candidate?.dockedWidth,
            candidate?.floating === false ? candidate?.width ?? 480 : 480,
            280,
            1_200,
          ),
        }
      : {}),
  };
}

export function loadWorkbenchLayout(): WorkbenchLayout {
  try {
    const key = workbenchLayoutStorageKey();
    touchWorkbenchLayoutInstance(key);
    const raw = localStorage.getItem(key);
    const candidate = raw ? JSON.parse(raw) as Partial<WorkbenchLayout> : {};
    return {
      inspector: normalizePanelGeometry("inspector", candidate.inspector),
      terminal: normalizePanelGeometry("terminal", candidate.terminal),
      usage: normalizePanelGeometry("usage", candidate.usage),
    };
  } catch {
    return {
      inspector: clonePanelGeometry(WORKBENCH_PANEL_DEFAULTS.inspector),
      terminal: clonePanelGeometry(WORKBENCH_PANEL_DEFAULTS.terminal),
      usage: clonePanelGeometry(WORKBENCH_PANEL_DEFAULTS.usage),
    };
  }
}

export function saveWorkbenchLayout(): void {
  try {
    const key = workbenchLayoutStorageKey();
    touchWorkbenchLayoutInstance(key);
    localStorage.setItem(key, JSON.stringify(ui.workbenchLayout));
  } catch {
    // A restricted Client can still use the layout for this window.
  }
}

export function panelCssVariables(panelId: WorkbenchPanelId): string {
  const panel = ui.workbenchLayout[panelId];
  const width = panelWidth(panelId);
  const dimensions = panelIsFloating(panelId)
    ? `width:${Math.round(width)}px;height:${Math.round(panel.height)}px;`
    : "";
  return `--panel-width:${Math.round(width)}px;--panel-height:${Math.round(panel.height)}px;--panel-x:${Math.round(panel.x)}px;--panel-y:${Math.round(panel.y)}px;${dimensions}`;
}

export function panelUiText() {
  const chinese = effectiveClientLanguage() === "zh-CN";
  return chinese
    ? { move: "拖动面板", resize: "调整面板大小", float: "浮窗", dock: "停靠", hide: "收起面板", showTerminal: "显示 Terminal" }
    : { move: "Move panel", resize: "Resize panel", float: "Float", dock: "Dock", hide: "Hide panel", showTerminal: "Show Terminal" };
}

export function panelControls(panelId: WorkbenchPanelId): string {
  if (mobileClient()) return "";
  const panel = ui.workbenchLayout[panelId];
  const floating = panelIsFloating(panelId);
  const text = panelUiText();
  return `<div class="panel-layout-bar">
    <button type="button" class="panel-drag-handle" data-panel-drag="${panelId}" aria-label="${escapeHtml(text.move)}" title="${escapeHtml(text.move)}">⋮⋮</button>
    <output class="panel-size" data-panel-size="${panelId}">${Math.round(panelWidth(panelId))} × ${Math.round(panel.height)}</output>
    <button type="button" class="panel-mode" data-act="panel-mode" data-id="${panelId}" data-panel-mode="${panelId}">${escapeHtml(floating ? text.dock : text.float)}</button>
    ${panelId === "terminal" ? `<button type="button" data-act="hide-terminal" aria-label="${escapeHtml(text.hide)}" title="${escapeHtml(text.hide)}">×</button>` : ""}
  </div>`;
}

export function panelResizeHandle(panelId: WorkbenchPanelId): string {
  const text = panelUiText();
  return `<div class="panel-resize-handle" data-panel-resize="${panelId}" role="separator" aria-label="${escapeHtml(text.resize)}" title="${escapeHtml(text.resize)}"></div>`;
}

export function workbenchIssuePanel(copy: ShellCopy, board: BoardSnapshot): string {
  return `<aside class="issue-detail workbench-panel" data-workbench-panel="inspector" data-floating="${panelIsFloating("inspector")}" data-front="${ui.frontWorkbenchPanel === "inspector"}" style="${panelCssVariables("inspector")}">
    ${panelControls("inspector")}
    ${issueDetail(copy, board)}
    ${panelResizeHandle("inspector")}
  </aside>`;
}

export function panelContainer(panel: HTMLElement): HTMLElement {
  return panel.closest<HTMLElement>(".board-shell, .lifted-run, .workspace") ?? document.documentElement;
}

export function floatingOrigin(panelId: WorkbenchPanelId, panel: HTMLElement): WorkbenchPanelGeometry {
  const current = ui.workbenchLayout[panelId];
  const container = panelContainer(panel).getBoundingClientRect();
  const width = Math.min(panelWidth(panelId), Math.max(280, container.width - 16));
  const height = Math.min(current.height, Math.max(180, container.height - 16));
  return {
    ...current,
    width,
    height,
    x: Math.max(8, Math.min(current.x, container.width - width - 8)),
    y: Math.max(8, Math.min(current.y, container.height - height - 8)),
  };
}

export function panelIsFloating(panelId: WorkbenchPanelId): boolean {
  const panel = ui.workbenchLayout[panelId];
  return panelId === "inspector" && ui.snapshot?.workspaceView === "run"
    ? panel.runFloating ?? false
    : panel.floating;
}

export function panelWidth(panelId: WorkbenchPanelId): number {
  const panel = ui.workbenchLayout[panelId];
  return panelId === "inspector" && !panelIsFloating(panelId)
    ? panel.dockedWidth ?? 480
    : panel.width;
}

export function withPanelFloating(
  panelId: WorkbenchPanelId,
  geometry: WorkbenchPanelGeometry,
  floating: boolean,
): WorkbenchPanelGeometry {
  return panelId === "inspector" && ui.snapshot?.workspaceView === "run"
    ? { ...geometry, runFloating: floating }
    : { ...geometry, floating };
}

export function updatePanelNode(panelId: WorkbenchPanelId): void {
  const panel = ui.app?.querySelector<HTMLElement>(`[data-workbench-panel="${panelId}"]`);
  if (!panel) return;
  panel.dataset.floating = String(panelIsFloating(panelId));
  panel.style.cssText = panelCssVariables(panelId);
  if (panelId === "inspector") {
    const parent = panel.closest<HTMLElement>(".board-shell, .lifted-run");
    const width = `${Math.round(panelWidth(panelId))}px`;
    parent?.style.setProperty("--inspector-panel-width", width);
    parent?.style.setProperty("--issue-detail-width", width);
  }
  const size = panel.querySelector<HTMLOutputElement>(`[data-panel-size="${panelId}"]`);
  const rect = panel.getBoundingClientRect();
  if (size) size.textContent = `${Math.round(rect.width)} × ${Math.round(rect.height)}`;
}

export function positionInspectorAwayFromCard(card: HTMLElement | null): void {
  if (!card || mobileClient() || !panelIsFloating("inspector")) return;
  const panel = ui.app?.querySelector<HTMLElement>(
    '[data-workbench-panel="inspector"]',
  );
  if (!panel) return;
  const container = panelContainer(panel).getBoundingClientRect();
  const cardRect = card.getBoundingClientRect();
  const width = Math.min(panelWidth("inspector"), Math.max(280, container.width - 16));
  const height = Math.min(ui.workbenchLayout.inspector.height, Math.max(180, container.height - 16));
  const gap = 12;
  const leftSpace = cardRect.left - container.left;
  const rightSpace = container.right - cardRect.right;
  let x: number;
  let y = clamp(cardRect.top - container.top, 8, container.height - height - 8);
  const graphAnchor = Boolean(card.closest(".graph-node, .graph-index-row"));
  const cardCenter = cardRect.left - container.left + cardRect.width / 2;
  if (graphAnchor) {
    x = 8;
  } else if (cardCenter >= container.width / 2) {
    x = leftSpace >= width + gap
      ? cardRect.left - container.left - width - gap
      : 8;
  } else {
    x = rightSpace >= width + gap
      ? cardRect.right - container.left + gap
      : container.width - width - 8;
  }
  ui.workbenchLayout.inspector = {
    ...ui.workbenchLayout.inspector,
    x: clamp(x, 8, container.width - width - 8),
    y,
  };
  saveWorkbenchLayout();
  updatePanelNode("inspector");
}

export function inspectorAnchorForIssue(issueId: string): HTMLElement | null {
  const selector = CSS.escape(issueId);
  return ui.app?.querySelector<HTMLElement>(
    `.graph-node[data-id="${selector}"], .graph-index-row button[data-id="${selector}"], .issue-card[data-issue-id="${selector}"]`,
  ) ?? null;
}

export function issueCardAtPoint(clientX: number, clientY: number): HTMLButtonElement | null {
  for (const element of document.elementsFromPoint(clientX, clientY)) {
    const card = element.closest<HTMLButtonElement>(".issue-card-main");
    if (card) return card;
  }
  return null;
}

export function graphActionAtPoint(clientX: number, clientY: number): HTMLButtonElement | null {
  for (const element of document.elementsFromPoint(clientX, clientY)) {
    const action = element.closest<HTMLButtonElement>(".graph-center-act");
    if (action) return action;
  }
  return null;
}

export function refreshPanelSizeFeedback(): void {
  for (const panelId of ["inspector", "terminal", "usage"] as const) updatePanelNode(panelId);
}

ui.workbenchLayout = loadWorkbenchLayout();
startWorkbenchLayoutHeartbeat();

export function workbenchPanelId(value: string | undefined): WorkbenchPanelId | null {
  return value === "inspector" || value === "terminal" || value === "usage" ? value : null;
}

export function clamp(value: number, minimum: number, maximum: number): number {
  return Math.min(Math.max(value, minimum), Math.max(minimum, maximum));
}

export function setPanelFloating(panelId: WorkbenchPanelId, floating: boolean): void {
  const node = ui.app?.querySelector<HTMLElement>(`[data-workbench-panel="${panelId}"]`);
  if (!node) return;
  const geometry = floating ? floatingOrigin(panelId, node) : ui.workbenchLayout[panelId];
  ui.workbenchLayout[panelId] = withPanelFloating(panelId, geometry, floating);
  ui.frontWorkbenchPanel = panelId;
  saveWorkbenchLayout();
  render();
  ui.fitAddon?.fit();
}

export function bringPanelToFront(panelId: WorkbenchPanelId): void {
  ui.frontWorkbenchPanel = panelId;
  for (const panel of ui.app?.querySelectorAll<HTMLElement>("[data-workbench-panel]") ?? []) {
    panel.dataset.front = String(panel.dataset.workbenchPanel === panelId);
  }
}

export function finishPanelPointer(pointerId: number): void {
  if (ui.panelPointerInteraction?.pointerId !== pointerId) return;
  ui.panelPointerInteraction = null;
  saveWorkbenchLayout();
  ui.fitAddon?.fit();
  const runId = ui.snapshot?.focusedRunId;
  if (runId && !mobileClient()) void sendPtyResize(runId);
}
