import {
  currentSystemAppearance,
  effectiveAppearancePreference,
  ensureBrowserAppearance,
  focusedRun,
  loadBrowserAppearance,
  mobileClient,
  viewportClass,
} from "./view-helpers";
import { loadSelectedIssueDocument, loadViewChanges, protocolBase, rpc, rpcDetached } from "./rpc";
import { FitAddon } from "@xterm/addon-fit";
import {
  onAction as onNativeNotificationAction,
  sendNotification as sendNativeNotification,
} from "@tauri-apps/plugin-notification";

import { listen } from "@tauri-apps/api/event";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import "./shell.css";
import "./shell-board.css";
import "./shell-issue.css";
import "./shell-graph.css";
import "./shell-run.css";
import "./shell-dialogs.css";
import "./shell-mobile.css";

import {
  escapeHtml,
  formatCountdown,
} from "./client-utils";
import { ui } from "./ui";
import { handleAppClick, openKeyboardHelp, openSettingsPanel } from "./events/click";
import { bindNativeMenuBridge, hookTerminalEditMenu } from "./edit-menu";
import { bindFormEvents } from "./events/forms";
import {
  fixedPanelRegion,
  fixedPanelWidth,
  saveClientPanelState,
  setFixedPanelWidth,
} from "./workbench";
import {
  desktopShellAvailable,
  checkForUpdates,
  requestDesktopNotificationPermission,
} from "./launch-session";
import {
  refreshBar,
} from "./render/board";
import {
  render,
} from "./render/app";
import type {
  Language,
  ShellCopy,
  Project,
  DependencyGraph,
  BoardSnapshot,
  Snapshot,
  NotificationKind,
  HostEvent,
  RpcResult,
  GraphViewportAnchor,
} from "./protocol";



ui.browserAppearance = loadBrowserAppearance();
export function resetGraphUiState(): void {
  ui.graphCanvasLimit = 48;
  ui.graphListLimit = 50;
  ui.graphListQuery = "";
}



export function notificationTitle(copy: ShellCopy, kind: NotificationKind): string {
  if (kind === "waiting") return copy.notifyWaiting;
  if (kind === "completed") return copy.notifyCompleted;
  if (kind === "abnormal-stop") return copy.notifyAbnormal;
  return copy.notifyCrash;
}

export function playNotifySound(): void {
  const AudioCtx =
    window.AudioContext ||
    (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
  if (!AudioCtx) return;
  const ctx = new AudioCtx();
  const oscillator = ctx.createOscillator();
  const gain = ctx.createGain();
  oscillator.type = "sine";
  oscillator.frequency.value = 880;
  gain.gain.value = 0.05;
  oscillator.connect(gain);
  gain.connect(ctx.destination);
  oscillator.start();
  oscillator.stop(ctx.currentTime + 0.12);
  oscillator.onended = () => {
    void ctx.close();
  };
}

export async function jumpToNotification(event: Extract<HostEvent, { type: "notification" }>): Promise<void> {
  await rpc("showWindow");
  if (event.projectId) {
    await rpc("focusProject", { projectId: event.projectId });
  }
  if (event.issueId) {
    ui.clientView.panels.rightSide = "rail";
    await rpc("focusIssue", { issueId: event.issueId });
  }
  if (event.runId) {
    await rpc("focusRun", { runId: event.runId });
  }
  render();
}

export function deliverHostEvents(events: HostEvent[], snap: Snapshot): void {
  if (mobileClient()) return;
  for (const event of events) {
    if (event.type !== "notification") continue;
    const title = notificationTitle(snap.copy, event.kind);
    const body = event.issueId || event.runId;
    if (snap.notifyDesktop) {
      void requestDesktopNotificationPermission().then((granted) => {
        if (!granted) return;
        if (desktopShellAvailable()) {
          sendNativeNotification({
            title,
            body,
            group: event.runId,
            extra: {
              kind: event.kind,
              runId: event.runId,
              issueId: event.issueId ?? "",
              projectId: event.projectId,
            },
          });
          return;
        }
        if (typeof Notification === "undefined") return;
        const note = new Notification(title, { body, tag: event.runId });
        note.onclick = () => {
          void jumpToNotification(event);
        };
      });
    }
    if (snap.notifySound) {
      playNotifySound();
    }
  }
}

export function emptyActionAct(action: Snapshot["emptyActions"][number]): string {
  return action === "register-first-project" ? "register" : "pair";
}

export function emptyActionLabel(copy: ShellCopy, action: Snapshot["emptyActions"][number]): string {
  return action === "register-first-project"
    ? copy.registerFirstProject
    : copy.pairAnotherHost;
}

export function clientCopy(language: Language, fallback: ShellCopy): ShellCopy {
  return ui.snapshot?.copyCatalog?.[language] ?? fallback;
}

export function captureActiveField(): {
  selector: string;
  start: number | null;
  end: number | null;
  direction: "forward" | "backward" | "none" | null;
  scrollLeft: number;
} | null {
  const active = document.activeElement as HTMLInputElement | HTMLTextAreaElement | null;
  if (!active || !ui.app?.contains(active)) return null;
  if (!("value" in active)) return null;
  let selector = active.id ? `#${CSS.escape(active.id)}` : "";
  if (!selector) {
    const form = active.closest<HTMLFormElement>("form[data-form]");
    const formKind = form?.dataset.form;
    const name = active.getAttribute("name");
    if (form && formKind && name) {
      const identity = form.dataset.id
        ? `[data-id="${CSS.escape(form.dataset.id)}"]`
        : "";
      selector = `form[data-form="${CSS.escape(formKind)}"]${identity} [name="${CSS.escape(name)}"]`;
    }
  }
  if (!selector) return null;
  return {
    selector,
    start: active.selectionStart,
    end: active.selectionEnd,
    direction: active.selectionDirection,
    scrollLeft: active.scrollLeft,
  };
}

export function restoreActiveField(field: {
  selector: string;
  start: number | null;
  end: number | null;
  direction: "forward" | "backward" | "none" | null;
  scrollLeft: number;
} | null): void {
  if (!field) return;
  const next = ui.app?.querySelector<HTMLInputElement | HTMLTextAreaElement>(field.selector);
  if (!next) return;
  next.focus();
  if (field.start != null && field.end != null) {
    next.setSelectionRange(field.start, field.end, field.direction ?? "none");
  }
  next.scrollLeft = field.scrollLeft;
}

export function dependencyGraphRenderKey(board: BoardSnapshot | null | undefined): string {
  if (!board?.graph) return "";
  return JSON.stringify([board.graph, ui.graphCanvasLimit]);
}

export function completeDependencyGraphLabel(copy: ShellCopy, graph: DependencyGraph): string {
  if (typeof graph.closedCount === "number") {
    return copy.showClosedContext.replace("{count}", String(graph.closedCount));
  }
  return copy.showClosedContext
    .replace(/\s*（[^）]*\{count\}[^）]*）/, "")
    .replace(/\s*\([^)]*\{count\}[^)]*\)/, "");
}

export function paintGraphEdges(): void {
  const canvas = ui.app?.querySelector<HTMLElement>(".graph-canvas");
  const svg = ui.app?.querySelector<SVGSVGElement>(".graph-edges");
  const graph = ui.snapshot?.board?.graph;
  if (!canvas || !svg || !graph) return;
  const origin = canvas.getBoundingClientRect();
  const width = Math.max(canvas.scrollWidth, canvas.clientWidth);
  const height = Math.max(canvas.scrollHeight, canvas.clientHeight);
  svg.setAttribute("viewBox", `0 0 ${width} ${height}`);
  svg.setAttribute("width", String(width));
  svg.setAttribute("height", String(height));
  const nodes = [...canvas.querySelectorAll<HTMLElement>(".graph-node")];
  const byId = new Map(nodes.map((node) => [node.dataset.id ?? "", node]));
  const paths = graph.edges
    .map((edge) => {
      const from = byId.get(edge.from);
      const to = byId.get(edge.to);
      if (!from || !to) return "";
      const a = from.getBoundingClientRect();
      const b = to.getBoundingClientRect();
      const x1 = a.right - origin.left + canvas.scrollLeft;
      const y1 = a.top + a.height / 2 - origin.top + canvas.scrollTop;
      const x2 = b.left - origin.left + canvas.scrollLeft;
      const y2 = b.top + b.height / 2 - origin.top + canvas.scrollTop;
      const mid = (x1 + x2) / 2;
      return `<path data-from="${escapeHtml(edge.from)}" data-to="${escapeHtml(edge.to)}" d="M ${x1} ${y1} C ${mid} ${y1}, ${mid} ${y2}, ${x2} ${y2}" />`;
    })
    .join("");
  svg.innerHTML = `<defs><marker id="graph-arrow" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path d="M0,0 L8,4 L0,8 Z"></path></marker></defs>${paths}`;
}

export function syncGraphSelection(canvas: HTMLElement | null, selectedId: string | undefined): void {
  if (!canvas) return;
  for (const node of canvas.querySelectorAll<HTMLElement>(".graph-node")) {
    node.classList.toggle("sel", node.dataset.id === selectedId);
  }
}

export function centerGraphViewport(canvas: HTMLElement, centerId: string): void {
  if (!centerId) return;
  const center = [...canvas.querySelectorAll<HTMLElement>(".graph-node")]
    .find((node) => node.dataset.id === centerId);
  if (!center) return;
  const canvasRect = canvas.getBoundingClientRect();
  const centerRect = center.getBoundingClientRect();
  const centerX = centerRect.left - canvasRect.left + canvas.scrollLeft + centerRect.width / 2;
  const centerY = centerRect.top - canvasRect.top + canvas.scrollTop + centerRect.height / 2;
  canvas.scrollLeft = Math.max(0, centerX - canvas.clientWidth / 2);
  canvas.scrollTop = Math.max(0, centerY - canvas.clientHeight / 2);
}

export function captureGraphAnchor(issueId: string): GraphViewportAnchor | null {
  const canvas = ui.app?.querySelector<HTMLElement>(".graph-canvas");
  const node = canvas
    ? [...canvas.querySelectorAll<HTMLElement>(".graph-node")]
      .find((item) => item.dataset.id === issueId)
    : null;
  if (!canvas || !node) return null;
  const canvasRect = canvas.getBoundingClientRect();
  const nodeRect = node.getBoundingClientRect();
  return {
    issueId,
    viewportX: nodeRect.left - canvasRect.left + nodeRect.width / 2,
    viewportY: nodeRect.top - canvasRect.top + nodeRect.height / 2,
  };
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

export function renderStatusBarsOnly(): void {
  if (!ui.snapshot) return;
  const language = mobileClient()
    ? ensureBrowserAppearance().language
    : ui.snapshot.appearance.language;
  const copy = language !== ui.snapshot.appearance.language
    ? clientCopy(language, ui.snapshot.copy)
    : ui.snapshot.copy;
  const current = ui.app?.querySelector<HTMLElement>(".project-board [data-page-toolbar] > .refresh-bar");
  if (current) current.outerHTML = refreshBar(copy, ui.snapshot.board);
  const pending = ui.app?.querySelector<HTMLElement>('.project-board > .refresh-bar[data-kind="pending"]');
  if (pending) pending.outerHTML = pendingBar(copy, ui.snapshot);
}

export function eventsNeedFullRender(events: HostEvent[]): boolean {
  return events.some((event) => {
    if (event.type !== "refresh-status-changed") return true;
    return event.status.kind !== "refreshing" && event.status.kind !== "ready";
  });
}

export function pendingBar(copy: ShellCopy, snap: Snapshot): string {
  const pending = snap.pendingConfirmation;
  if (!pending) return "";
  return `<div class="refresh-bar" data-kind="pending">
    <span>${escapeHtml(copy.pendingConfirmation)} · ${escapeHtml(pending.issueId)} · ${formatCountdown(pending.remainingMs)}</span>
    <button type="button" data-act="veto-advance" data-id="${escapeHtml(pending.projectId)}">${escapeHtml(copy.vetoAdvance)}</button>
  </div>`;
}

export function connectionPanel(copy: ShellCopy, project: Project): string {
  if (project.connection.status === "ready") {
    return "";
  }
  if (project.connection.status === "unreachable") {
    return `<div class="notice bad">
      <b>${escapeHtml(copy.connectionUnavailable)}</b>
      <p>${escapeHtml(project.connection.message)}</p>
    </div>`;
  }
  const repair = project.connection.repair;
  return `<div class="notice bad">
    <b>${escapeHtml(copy.authFailed)}</b>
    <p>${escapeHtml(project.connection.message)}</p>
    <ul class="repair">
      <li>${escapeHtml(copy.repairCli)}${repair.cliDetected ? "" : ` — ${escapeHtml(copy.noGhDetected)}`}</li>
      <li>${escapeHtml(copy.repairSecrets)}：<code>${escapeHtml(repair.secretsPath)}</code></li>
      <li>${escapeHtml(copy.repairEnv)}：<code>${escapeHtml(repair.appEnv)} / ${escapeHtml(repair.genericEnv)}</code></li>
    </ul>
    <p class="tiny">${escapeHtml(repair.suggestedScope)}</p>
  </div>`;
}

function designToken(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

export function termTheme(): ConstructorParameters<typeof Terminal>[0] {
  return {
    cursorBlink: true,
    fontFamily: designToken("--font-family-mono"),
    fontSize: Number.parseFloat(designToken("--font-size-md")),
    theme: {
      background: designToken("--terminal-canvas"),
      foreground: designToken("--terminal-text"),
      cursor: designToken("--terminal-cursor"),
      selectionBackground: designToken("--terminal-selection"),
    },
  };
}

export function ensureTerminal(): void {
  if (ui.term && ui.termHost && ui.fitAddon) return;
  ui.fitAddon = new FitAddon();
  ui.term = new Terminal(termTheme());
  ui.term.loadAddon(ui.fitAddon);
  ui.termHost = document.createElement("div");
  ui.termHost.className = "pty-host";
  ui.term.open(ui.termHost);
  hookTerminalEditMenu(ui.term);
  ui.term.onData((data) => {
    const runId = ui.snapshot?.focusedRunId;
    if (!runId) return;
    void sendPtyInput(runId, data);
  });
}

export function attachTerminal(snap: Snapshot): void {
  const run = focusedRun(snap);
  const slot = ui.app?.querySelector<HTMLElement>(".pty-slot");
  if (!run || !slot) {
    ui.ptyPumping = false;
    return;
  }
  ensureTerminal();
  if (ui.termHost && ui.termHost.parentElement !== slot) {
    slot.appendChild(ui.termHost);
  }
  ui.fitAddon?.fit();
  void sendPtyResize(run.id);
  if (ui.ptyRunId !== run.id) {
    ui.ptyRunId = run.id;
    ui.ptyOffset = 0;
    ui.term?.reset();
  }
  if (run.status === "ended") {
    ui.ptyPumping = false;
    return;
  }
  if (!ui.ptyPumping) {
    ui.ptyPumping = true;
    void pumpPty();
  }
}

export async function pumpMobileOutput(snap: Snapshot): Promise<void> {
  const run = ui.mobileView === "run" ? focusedRun(snap) : undefined;
  const hostId = snap.focusedHostId;
  const outputKey = JSON.stringify([hostId, run?.id]);
  if (!run || run.status === "ended" || ui.mobileLiveTerminal) {
    if (run?.status === "ended" && run.recentOutput) {
      ui.mobilePtyText.set(outputKey, run.recentOutput);
    }
    ui.mobilePtyPumping = false;
    return;
  }
  if (ui.mobilePtyRunId !== outputKey) {
    ui.mobilePtyRunId = outputKey;
    ui.mobilePtyOffset = 0;
  }
  if (ui.mobilePtyPumping) return;
  ui.mobilePtyPumping = true;
  const runId = run.id;
  try {
    while (
      mobileClient()
      && ui.mobileView === "run"
      && !ui.mobileLiveTerminal
      && ui.snapshot?.focusedHostId === hostId
      && ui.snapshot?.focusedRunId === runId
    ) {
      const response = await fetch(
        `${await protocolBase()}/runs/${encodeURIComponent(runId)}/output?after=${ui.mobilePtyOffset}`,
      );
      if (!response.ok) break;
      const json = (await response.json()) as { offset: number; recentOutput?: string; exited: number | null };
      if (ui.mobilePtyRunId !== outputKey || ui.snapshot?.focusedHostId !== hostId || ui.snapshot?.focusedRunId !== runId) break;
      if (typeof json.recentOutput === "string") {
        const recent = json.recentOutput;
        ui.mobilePtyText.set(outputKey, recent);
        const output = ui.app?.querySelector<HTMLElement>(`.mobile-run-output[data-run="${CSS.escape(runId)}"]`);
        if (output) {
          output.textContent = recent;
          output.scrollTop = output.scrollHeight;
        }
      }
      ui.mobilePtyOffset = json.offset;
      if (json.exited != null) {
        await rpc("snapshot");
        render();
        break;
      }
    }
  } catch {
    // Keep the last readable output when the Run or Host disconnects.
  } finally {
    ui.mobilePtyPumping = false;
  }
}

export async function sendPtyInput(runId: string, data: string): Promise<void> {
  try {
    await fetch(`${await protocolBase()}/runs/${encodeURIComponent(runId)}/input`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ data }),
    });
  } catch {
    // Host may have stopped the Run.
  }
}

export async function sendPtyResize(runId: string): Promise<void> {
  const cols = ui.term?.cols ?? 80;
  const rows = ui.term?.rows ?? 24;
  try {
    await fetch(`${await protocolBase()}/runs/${encodeURIComponent(runId)}/resize`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ cols, rows }),
    });
  } catch {
    // ignore
  }
}

export async function pumpPty(): Promise<void> {
  while (ui.ptyPumping && ui.snapshot?.focusedRunId && ui.ptyRunId === ui.snapshot.focusedRunId) {
    const runId = ui.ptyRunId;
    try {
      const response = await fetch(
        `${await protocolBase()}/runs/${encodeURIComponent(runId)}/output?after=${ui.ptyOffset}`,
      );
      if (response.ok) {
        const json = (await response.json()) as {
          offset: number;
          data: string;
          exited: number | null;
        };
        if (ui.ptyRunId !== runId || ui.snapshot?.focusedRunId !== runId) {
          break;
        }
        if (json.data) {
          const raw = atob(json.data);
          const bytes = new Uint8Array(raw.length);
          for (let i = 0; i < raw.length; i += 1) bytes[i] = raw.charCodeAt(i);
          ui.term?.write(bytes);
        }
        ui.ptyOffset = json.offset;
        if (json.exited != null) {
          await rpc("snapshot");
          render();
          break;
        }
      }
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 400));
    }
  }
  ui.ptyPumping = false;
}

ui.app.addEventListener("pointerdown", (event) => {
  if (mobileClient()) return;
  const target = (event.target as HTMLElement).closest<HTMLElement>("[data-panel-resize]");
  const region = fixedPanelRegion(target?.dataset.panelResize);
  if (!target || !region) return;
  event.preventDefault();
  ui.panelPointerInteraction = {
    pointerId: event.pointerId,
    region,
    startClientX: event.clientX,
    startWidth: fixedPanelWidth(region),
  };
  target.setPointerCapture?.(event.pointerId);
});

document.addEventListener("pointermove", (event) => {
  const interaction = ui.panelPointerInteraction;
  if (!interaction || interaction.pointerId !== event.pointerId) return;
  event.preventDefault();
  const dx = event.clientX - interaction.startClientX;
  const nextWidth = interaction.region === "sidebar"
    ? interaction.startWidth + dx
    : interaction.startWidth - dx;
  setFixedPanelWidth(interaction.region, nextWidth);
}, true);

function finishFixedPanelResize(pointerId: number): void {
  if (ui.panelPointerInteraction?.pointerId !== pointerId) return;
  ui.panelPointerInteraction = null;
  saveClientPanelState();
  ui.fitAddon?.fit();
  const runId = ui.snapshot?.focusedRunId;
  if (runId && !mobileClient()) void sendPtyResize(runId);
}

window.addEventListener("pointerup", (event) => finishFixedPanelResize(event.pointerId));
window.addEventListener("pointercancel", (event) => finishFixedPanelResize(event.pointerId));

export function shouldReportClientView(): boolean {
  return true;
}

let hostWindowVisible = true;
let lastReportedView = { projectId: "", visible: false };

export function clientIsVisible(): boolean {
  return hostWindowVisible && document.visibilityState === "visible";
}

export async function reportClientView(
  visible = clientIsVisible(),
): Promise<{ changed: boolean; result: RpcResult | null }> {
  if (!shouldReportClientView()) return { changed: false, result: null };
  const projectId = visible ? ui.snapshot?.focusedProjectId ?? "" : "";
  const changed = visible !== lastReportedView.visible || projectId !== lastReportedView.projectId;
  lastReportedView = { projectId, visible };
  const result = await rpc("setClientView", { clientId: ui.clientId, projectId, visible });
  return { changed, result };
}

let foregroundRefresh: Promise<void> | null = null;

export function onClientForegroundOrHidden(): void {
  if (!shouldReportClientView()) return;
  if (!clientIsVisible()) {
    void reportClientView(false).then(render).catch(() => {});
    return;
  }
  if (foregroundRefresh) return;
  foregroundRefresh = (async () => {
    try {
      const reported = await reportClientView(true);
      if (!clientIsVisible()) {
        await reportClientView(false);
        return;
      }
      const result = reported.changed
        ? reported.result
        : await rpcDetached("refresh", { projectId: ui.snapshot?.focusedProjectId ?? "" });
      if (eventsNeedFullRender(result?.events ?? [])) render();
      else renderStatusBarsOnly();
    } finally {
      foregroundRefresh = null;
    }
  })();
  void foregroundRefresh.catch(() => {});
}

export function ensureTick(): void {
  if (ui.tickTimer != null) return;
  ui.tickTimer = window.setInterval(() => {
    const extra = shouldReportClientView()
      ? {
          clientId: ui.clientId,
          projectId: ui.snapshot?.focusedProjectId ?? "",
          visible: clientIsVisible(),
        }
      : {};
    rpc("tick", extra)
      .then((result) => renderAfterTick(eventsNeedFullRender(result.events ?? [])))
      .catch(() => {});
  }, 1000);
}

export async function renderAfterTick(fullRender: boolean): Promise<void> {
  if (ui.activePointers.size > 0) {
    ui.tickRenderPending = true;
    ui.tickFullRenderPending ||= fullRender;
    return;
  }
  if (ui.snapshot?.board?.selected?.document.kind === "unloaded") {
    await loadSelectedIssueDocument();
    fullRender = true;
  }
  if (fullRender) render();
  else renderStatusBarsOnly();
}

export function finishPointerInteraction(pointerId: number): void {
  ui.activePointers.delete(pointerId);
  if (ui.activePointers.size > 0 || !ui.tickRenderPending) return;
  window.setTimeout(() => {
    if (ui.activePointers.size > 0 || !ui.tickRenderPending) return;
    ui.tickRenderPending = false;
    const fullRender = ui.tickFullRenderPending;
    ui.tickFullRenderPending = false;
    void renderAfterTick(fullRender).catch(() => {});
  }, 0);
}

document.addEventListener("pointerdown", (event) => ui.activePointers.add(event.pointerId), true);
document.addEventListener("pointerup", (event) => finishPointerInteraction(event.pointerId), true);
document.addEventListener("pointercancel", (event) => finishPointerInteraction(event.pointerId), true);
window.addEventListener("blur", () => {
  for (const pointerId of ui.activePointers) finishPointerInteraction(pointerId);
});

document.addEventListener("visibilitychange", onClientForegroundOrHidden);

export function terminalHasFocus(): boolean {
  const active = document.activeElement as HTMLElement | null;
  return Boolean(active?.closest(".pty-host"));
}

export function typingTarget(target: EventTarget | null): boolean {
  const element = target as HTMLElement | null;
  return Boolean(element?.closest("input, textarea, select, [contenteditable='true']"));
}

document.addEventListener("keydown", (event) => {
  if (!ui.snapshot || terminalHasFocus()) return;
  if (ui.appearanceMenuOpen && ["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) {
    event.preventDefault();
    const items = [...ui.app.querySelectorAll<HTMLButtonElement>(".appearance-menu button")];
    if (!items.length) return;
    const current = items.findIndex((item) => item === document.activeElement);
    const next = event.key === "Home"
      ? 0
      : event.key === "End"
        ? items.length - 1
        : (current + (event.key === "ArrowUp" ? -1 : 1) + items.length) % items.length;
    items[next]?.focus();
    return;
  }
  if (event.key === "?" && !typingTarget(event.target)) {
    event.preventDefault();
    ui.keyboardHelpOpen = !ui.keyboardHelpOpen;
    render();
    return;
  }
  if (event.key === "Escape") {
    if (ui.appearanceMenuOpen) {
      event.preventDefault();
      ui.appearanceMenuOpen = false;
      render();
      ui.app.querySelector<HTMLButtonElement>("button[data-act='appearance-menu']")?.focus();
    } else if (ui.moreMenuOpen) {
      event.preventDefault();
      ui.moreMenuOpen = false;
      render();
      ui.app.querySelector<HTMLButtonElement>("button[data-act='more-menu']")?.focus();
    } else if (ui.keyboardHelpOpen) {
      ui.keyboardHelpOpen = false;
      render();
    }
    return;
  }
  if (event.key === "/" && !typingTarget(event.target)) {
    event.preventDefault();
    ui.app.querySelector<HTMLInputElement>("#issue-title-search")?.focus();
    return;
  }
  if (
    typingTarget(event.target)
    || ui.keyboardHelpOpen
    || ui.clientView.page === "settings"
    || ui.pairingOpen
    || ui.formOpen
    || Boolean(ui.removeProject)
    || Boolean(ui.snapshot.launchForm)
    || Boolean(ui.snapshot.quitOffer)
    || ui.clientView.panels.rightSide === "changes"
  ) return;
  const cards = [...ui.app.querySelectorAll<HTMLButtonElement>(".issue-card-main")];
  if (!cards.length) return;
  if (["j", "J", "ArrowDown", "k", "K", "ArrowUp"].includes(event.key)) {
    event.preventDefault();
    const direction = ["k", "K", "ArrowUp"].includes(event.key) ? -1 : 1;
    const focusedIndex = cards.findIndex((card) => card === document.activeElement);
    const rememberedIndex = cards.findIndex((card) => card.dataset.issueId === ui.keyboardCursorIssueId);
    const currentIndex = focusedIndex >= 0 ? focusedIndex : rememberedIndex;
    const nextIndex = (currentIndex + direction + cards.length) % cards.length;
    const nextCard = cards[nextIndex];
    ui.keyboardCursorIssueId = nextCard?.dataset.issueId ?? "";
    nextCard?.focus();
    return;
  }
  if (event.key === "Enter" && document.activeElement?.classList.contains("issue-card-main")) {
    event.preventDefault();
    (document.activeElement as HTMLButtonElement).click();
  }
});

window.addEventListener("focus", () => {
  if (!clientIsVisible()) return;
  onClientForegroundOrHidden();
});

window.addEventListener("pagehide", () => {
  void reportClientView(false).catch(() => {});
});

window.addEventListener("agent-taskboard:host-window-hidden", () => {
  hostWindowVisible = false;
  void reportClientView(false).then(render).catch(() => {});
});

window.addEventListener("agent-taskboard:host-window-shown", () => {
  hostWindowVisible = true;
  onClientForegroundOrHidden();
});

window.addEventListener("agent-taskboard:check-update", () => {
  void checkForUpdates(false);
});

window.addEventListener("agent-taskboard:open-settings", () => {
  void openSettingsPanel();
});

window.addEventListener("agent-taskboard:open-keyboard-help", () => {
  openKeyboardHelp();
});

bindNativeMenuBridge();

let wasMobileClient = mobileClient();

function updateSystemAppearance(appearance: "light" | "dark"): void {
  if (ui.systemAppearance === appearance) return;
  ui.systemAppearance = appearance;
  if (ui.snapshot && effectiveAppearancePreference(ui.snapshot) === "system") render();
}

window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
  updateSystemAppearance(currentSystemAppearance());
});

if (desktopShellAvailable()) {
  void listen<"light" | "dark">("system-appearance-changed", (event) => {
    updateSystemAppearance(event.payload);
  }).catch((error: unknown) => {
    console.warn("system appearance events unavailable", error);
  });
}

if (desktopShellAvailable()) {
  void onNativeNotificationAction((notification) => {
    const runId = String(notification.extra?.runId ?? "");
    const projectId = String(notification.extra?.projectId ?? "");
    if (!runId || !projectId) return;
    const rawKind = String(notification.extra?.kind ?? "waiting");
    const kind: NotificationKind = ["waiting", "completed", "abnormal-stop", "crash-recovered"].includes(rawKind)
      ? rawKind as NotificationKind
      : "waiting";
    void jumpToNotification({
      type: "notification",
      kind,
      runId,
      issueId: String(notification.extra?.issueId ?? "") || null,
      projectId,
    });
  }).catch((error: unknown) => {
    console.warn("native notification actions unavailable", error);
  });
}

window.addEventListener("resize", () => {
  document.documentElement.dataset.viewport = viewportClass();
  if (!ui.snapshot) return;
  const isMobile = mobileClient();
  if (isMobile !== wasMobileClient) {
    wasMobileClient = isMobile;
    ui.mobileLiveTerminal = false;
    render();
  }
  ui.fitAddon?.fit();
  const runId = ui.snapshot.focusedRunId;
  if (runId && (!isMobile || ui.mobileLiveTerminal)) void sendPtyResize(runId);
});

rpc("snapshot")
  .then(async () => {
    render();
    const restoredChangesRun = ui.snapshot ? focusedRun(ui.snapshot) : undefined;
    if (!mobileClient() && ui.clientView.panels.rightSide === "changes" && restoredChangesRun) {
      try {
        await loadViewChanges(restoredChangesRun.id, ui.changesScope);
        render();
      } catch {
        // Keep the restored panel visible so its scope controls can retry the read.
      }
    }
    ensureTick();
    await reportClientView();
    render();
    if (desktopShellAvailable() && !ui.startupUpdateChecked && ui.snapshot?.windowVisible) {
      ui.startupUpdateChecked = true;
      window.setTimeout(() => {
        if (ui.updateState.kind === "idle") void checkForUpdates(false);
      }, 250);
    }
  })
  .catch((error: unknown) => {
    if (ui.app) {
      ui.app.textContent = error instanceof Error ? error.message : String(error);
    }
  });

ui.app.addEventListener("click", (event) => {
  void handleAppClick(event);
});
bindFormEvents();
