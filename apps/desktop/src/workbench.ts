import type {
  BoardSnapshot,
  ClientPanelState,
  FixedPanelRegion,
  ShellCopy,
} from "./protocol";
import { desktopShellAvailable } from "./launch-session";
import { escapeHtml } from "./client-utils";
import { issueDetail } from "./render/board";
import { effectiveClientLanguage, mobileClient } from "./view-helpers";
import { ui } from "./ui";

export const CLIENT_PANEL_STATE_STORAGE_PREFIX = "agent-taskboard-client-panels:v1:";

export const CLIENT_PANEL_DEFAULTS: ClientPanelState = {
  sidebarVisible: true,
  sidebarWidth: 248,
  rightSide: "rail",
  rightRailWidth: 320,
  changesPanelWidth: 520,
};

export const CLIENT_PANEL_WIDTHS = {
  sidebar: { minimum: 216, maximum: 320, fallback: 248 },
  "right-rail": { minimum: 280, maximum: 420, fallback: 320 },
  "changes-panel": { minimum: 420, maximum: 640, fallback: 520 },
} as const;

function cloneDefaultPanelState(): ClientPanelState {
  return { ...CLIENT_PANEL_DEFAULTS };
}

export function clientPanelStateStorageKey(): string {
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
      const key = "agent-taskboard-tauri-panel-client-id";
      identity = localStorage.getItem(key) ?? "";
      if (!identity) {
        identity = `tauri-${crypto.randomUUID?.() ?? Date.now()}`;
        localStorage.setItem(key, identity);
      }
    }
  }
  return `${CLIENT_PANEL_STATE_STORAGE_PREFIX}${clientKind}:${identity}`;
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.min(Math.max(value, minimum), maximum);
}

function normalizedWidth(region: FixedPanelRegion, value: number): number {
  const bounds = CLIENT_PANEL_WIDTHS[region];
  return clamp(value, bounds.minimum, bounds.maximum);
}

export function normalizeClientPanelState(candidate: unknown): ClientPanelState {
  if (!candidate || typeof candidate !== "object") return cloneDefaultPanelState();
  const value = candidate as Partial<ClientPanelState>;
  if (
    typeof value.sidebarVisible !== "boolean"
    || !["hidden", "rail", "changes"].includes(value.rightSide ?? "")
    || typeof value.sidebarWidth !== "number"
    || !Number.isFinite(value.sidebarWidth)
    || typeof value.rightRailWidth !== "number"
    || !Number.isFinite(value.rightRailWidth)
    || typeof value.changesPanelWidth !== "number"
    || !Number.isFinite(value.changesPanelWidth)
  ) {
    return cloneDefaultPanelState();
  }
  return {
    sidebarVisible: value.sidebarVisible,
    sidebarWidth: normalizedWidth("sidebar", value.sidebarWidth),
    rightSide: value.rightSide as ClientPanelState["rightSide"],
    rightRailWidth: normalizedWidth("right-rail", value.rightRailWidth),
    changesPanelWidth: normalizedWidth("changes-panel", value.changesPanelWidth),
  };
}

export function loadClientPanelState(): ClientPanelState {
  const key = clientPanelStateStorageKey();
  try {
    const raw = localStorage.getItem(key);
    const normalized = raw
      ? normalizeClientPanelState(JSON.parse(raw))
      : cloneDefaultPanelState();
    localStorage.setItem(key, JSON.stringify(normalized));
    return normalized;
  } catch {
    const fallback = cloneDefaultPanelState();
    try {
      localStorage.setItem(key, JSON.stringify(fallback));
    } catch {
      // Restricted Clients keep the fallback in memory only.
    }
    return fallback;
  }
}

export function saveClientPanelState(): void {
  try {
    localStorage.setItem(
      clientPanelStateStorageKey(),
      JSON.stringify(normalizeClientPanelState(ui.clientView.panels)),
    );
  } catch {
    // Restricted Clients keep the panel state for this window only.
  }
}

export function fixedPanelRegion(value: string | undefined): FixedPanelRegion | null {
  return value === "sidebar" || value === "right-rail" || value === "changes-panel"
    ? value
    : null;
}

export function fixedPanelWidth(region: FixedPanelRegion): number {
  if (region === "sidebar") return ui.clientView.panels.sidebarWidth;
  if (region === "right-rail") return ui.clientView.panels.rightRailWidth;
  return ui.clientView.panels.changesPanelWidth;
}

export function setFixedPanelWidth(region: FixedPanelRegion, width: number): void {
  const normalized = normalizedWidth(region, width);
  if (region === "sidebar") ui.clientView.panels.sidebarWidth = normalized;
  else if (region === "right-rail") ui.clientView.panels.rightRailWidth = normalized;
  else ui.clientView.panels.changesPanelWidth = normalized;
  applyClientPanelWidths();
}

export function applyClientPanelWidths(): void {
  const frame = ui.app?.querySelector<HTMLElement>(".frame");
  if (!frame) return;
  frame.style.setProperty("--client-sidebar-width", `${Math.round(ui.clientView.panels.sidebarWidth)}px`);
  frame.style.setProperty("--client-right-rail-width", `${Math.round(ui.clientView.panels.rightRailWidth)}px`);
  frame.style.setProperty("--client-changes-panel-width", `${Math.round(ui.clientView.panels.changesPanelWidth)}px`);
  const rightRailWidth = `${Math.round(ui.clientView.panels.rightRailWidth)}px`;
  ui.app.querySelector<HTMLElement>(".board-shell")?.style.setProperty("--issue-detail-width", rightRailWidth);
}

export function panelUiText() {
  const chinese = effectiveClientLanguage() === "zh-CN";
  return chinese
    ? { resize: "调整区域宽度", showTerminal: "显示 Terminal" }
    : { resize: "Resize region", showTerminal: "Show Terminal" };
}

export function fixedPanelResizeHandle(region: FixedPanelRegion): string {
  if (mobileClient()) return "";
  const text = panelUiText();
  return `<div class="fixed-panel-resize" data-panel-resize="${region}" role="separator" aria-orientation="vertical" aria-label="${escapeHtml(text.resize)}" title="${escapeHtml(text.resize)}"></div>`;
}

export function workbenchIssuePanel(copy: ShellCopy, board: BoardSnapshot): string {
  return `<aside class="issue-detail fixed-right-rail" data-fixed-panel="right-rail">
    ${fixedPanelResizeHandle("right-rail")}
    ${issueDetail(copy, board)}
  </aside>`;
}

ui.clientView.panels = loadClientPanelState();
