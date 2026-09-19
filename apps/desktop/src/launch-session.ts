import { effectiveClientLanguage } from "./view-helpers";
import { openUrl } from "@tauri-apps/plugin-opener";
import { startupCopy } from "./startup-copy";
import type { LaunchDraft, RunLaunchForm, ShellCopy, Snapshot, UpdateInstallGate } from "./protocol";
import { syncLaunchPicker } from "./launch-picker";
import { launchFieldDefault, launchFieldOptions, launchSelectOptions, launchSelectState, CUSTOM_VALUE } from "./render/run";
import { render } from "./render/app";
import { rpc } from "./rpc";
import { ui } from "./ui";
import { check } from "@tauri-apps/plugin-updater";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { open as openDirectory } from "@tauri-apps/plugin-dialog";
import {
  isPermissionGranted as isNativeNotificationPermissionGranted,
  requestPermission as requestNativeNotificationPermission,
} from "@tauri-apps/plugin-notification";
import { relaunch } from "@tauri-apps/plugin-process";

export function syncLaunchDraft(snap: Snapshot): void {
  const form = snap.launchForm;
  if (!form) {
    ui.launchDraft = null;
    ui.launchPickerProjectId = "";
    ui.launchPickerAgentId = "";
    return;
  }
  if (!form.skipAgentPicker) {
    ui.launchDraft = null;
    const picker = syncLaunchPicker(
      { projectId: form.projectId, selectedAgentId: form.selectedAgentId },
      { projectId: ui.launchPickerProjectId, agentId: ui.launchPickerAgentId },
    );
    ui.launchPickerProjectId = picker.projectId;
    ui.launchPickerAgentId = picker.agentId;
    return;
  }
  ui.launchPickerProjectId = form.projectId;
  ui.launchPickerAgentId = form.selectedAgentId;
  if (
    !ui.launchDraft
    || ui.launchDraft.projectId !== form.projectId
    || ui.launchDraft.agentId !== form.selectedAgentId
  ) {
    ui.launchDraft = {
      projectId: form.projectId,
      issueId: form.issueId,
      agentId: form.selectedAgentId,
      values: { ...form.values },
      openingText: form.openingText,
      intentId: "",
      custom: false,
    };
    coerceLaunchFieldValues();
  }
}

export function prefillHint(copy: ShellCopy, source: RunLaunchForm["prefillSource"]): string {
  if (source === "current-project") return copy.prefillCurrent;
  if (source === "other-project") return copy.prefillOther;
  return copy.prefillSeed;
}

export function applyLaunchDependentDefaults(changedId: string): void {
  if (!ui.snapshot?.launchForm || !ui.launchDraft) return;
  const parentValue = ui.launchDraft.values[changedId] ?? "";
  if (!parentValue || parentValue === "__custom__") return;
  for (const field of ui.snapshot.launchForm.fields) {
    const filter = field.optionFilter;
    if (!filter || filter.fieldId !== changedId) continue;
    const options = launchFieldOptions(field, ui.launchDraft.values);
    const defaultValue = launchFieldDefault(field, ui.launchDraft.values);
    if (defaultValue) {
      ui.launchDraft.values[field.id] = defaultValue;
    } else if (!options.includes(ui.launchDraft.values[field.id] ?? "")) {
      ui.launchDraft.values[field.id] = options[0] ?? "";
    }
  }
}

export function coerceLaunchFieldValues(): void {
  if (!ui.snapshot?.launchForm || !ui.launchDraft) return;
  for (const field of ui.snapshot.launchForm.fields) {
    if (field.kind !== "select" || !field.optionFilter) continue;
    const options = launchFieldOptions(field, ui.launchDraft.values);
    const current = ui.launchDraft.values[field.id] ?? "";
    if (!current || options.includes(current) || options.length === 0) continue;
    if (!(field.options ?? []).includes(current)) continue;
    ui.launchDraft.values[field.id] = launchFieldDefault(field, ui.launchDraft.values) ?? options[0] ?? current;
  }
}

export function refreshLaunchFieldOptions(): void {
  if (!ui.snapshot?.launchForm || !ui.launchDraft) return;
  for (const field of ui.snapshot.launchForm.fields) {
    if (field.kind !== "select") continue;
    const select = ui.app?.querySelector<HTMLSelectElement>(
      `[data-launch-select="${CSS.escape(field.id)}"]`,
    );
    if (!select) continue;
    const options = launchFieldOptions(field, ui.launchDraft.values);
    const current = ui.launchDraft.values[field.id] ?? "";
    const { customValue, customEntry } = launchSelectState(options, current);
    select.innerHTML = launchSelectOptions(options, current);
    select.value = customEntry ? CUSTOM_VALUE : current;
    const custom = ui.app?.querySelector<HTMLInputElement>(
      `[data-launch-custom="${CSS.escape(field.id)}"]`,
    );
    if (custom) {
      custom.hidden = !customValue && !customEntry;
      custom.value = customValue;
    }
  }
}

export function launchValuesForHost(draft: LaunchDraft): Record<string, string> {
  return Object.fromEntries(
    Object.entries(draft.values).filter(([, value]) => value !== CUSTOM_VALUE),
  );
}

export function scheduleLaunchPreview(): void {
  if (!ui.snapshot?.launchForm || !ui.launchDraft) return;
  const sequence = ++ui.launchPreviewSequence;
  if (ui.launchPreviewTimer != null) window.clearTimeout(ui.launchPreviewTimer);
  ui.launchPreviewTimer = window.setTimeout(() => {
    ui.launchPreviewTimer = undefined;
    const draft = ui.launchDraft;
    if (!draft) return;
    void rpc("updateRunLaunch", {
      projectId: draft.projectId,
      agentId: draft.agentId,
      values: launchValuesForHost(draft),
      openingText: draft.openingText,
      language: effectiveClientLanguage(),
    }).then(() => {
      if (sequence !== ui.launchPreviewSequence) return;
      const preview = ui.app?.querySelector<HTMLElement>(".launch-command-preview");
      if (preview && ui.snapshot?.launchForm) preview.textContent = ui.snapshot.launchForm.commandPreview;
      refreshLaunchWarnings();
    }).catch(() => {});
  }, 120);
}

export function refreshLaunchWarnings(): void {
  const node = ui.app?.querySelector<HTMLElement>(".launch-warnings");
  if (!node || !ui.snapshot?.launchForm) return;
  const warnings = ui.snapshot.launchForm.warnings ?? [];
  node.textContent = warnings.join(" ");
  node.hidden = warnings.length === 0;
}

export function refreshIntentChoices(): void {
  if (!ui.app || !ui.launchDraft) return;
  for (const button of ui.app.querySelectorAll<HTMLButtonElement>(".launch-sheet button[data-act='intent']")) {
    button.classList.toggle(
      "active",
      !ui.launchDraft.custom && (button.dataset.id ?? "") === ui.launchDraft.intentId,
    );
  }
  const custom = ui.app.querySelector<HTMLButtonElement>(".launch-sheet button[data-act='intent-custom']");
  if (custom) {
    custom.hidden = !ui.launchDraft.custom;
    custom.classList.toggle("active", ui.launchDraft.custom);
  }
}

export function expectedOpening(form: RunLaunchForm, draft: LaunchDraft): string {
  const prefix = form.intents.find((intent) => intent.id === draft.intentId)?.prefix ?? "";
  const body = (draft.values["initial-instruction"] ?? "").trim();
  const notes = (form.changeNotesText ?? "").trim();
  const core = prefix && body ? `${prefix}\n${body}` : prefix || body;
  if (core && notes) return `${core}\n\n${notes}`;
  return core || notes;
}

export async function openExternalUrl(url: string): Promise<void> {
  if (desktopShellAvailable()) {
    await openUrl(url);
    return;
  }
  window.open(url, "_blank", "noopener,noreferrer");
}

export function isLoopbackPage(): boolean {
  const { hostname, port } = window.location;
  return (
    (hostname === "127.0.0.1" || hostname === "localhost" || hostname === "[::1]") &&
    port === "10529"
  );
}

export function desktopShellAvailable(): boolean {
  return isTauri() || "__TAURI_INTERNALS__" in window;
}

export async function openRunWindow(
  runId: string,
  hostId: string,
  projectId: string,
  title: string,
): Promise<void> {
  if (!desktopShellAvailable()) return;
  await invoke("open_run_window", { runId, hostId, projectId, title });
}

export function focusedHostIsLocal(): boolean {
  const snap = ui.snapshot;
  if (!snap) return false;
  const focused = snap.hosts.find((host) => host.id === snap.focusedHostId);
  if (focused) return focused.local;
  return snap.hosts.some((host) => host.local) && snap.hosts.every((host) => host.local);
}

export function directoryName(path: string): string {
  const trimmed = path.replace(/[\\/]+$/, "");
  const parts = trimmed.split(/[\\/]/).filter(Boolean);
  return parts.length ? parts[parts.length - 1] : "";
}

export function supersedeProjectInference(): void {
  ui.projectInference = { status: "idle", requestId: ui.projectInference.requestId + 1 };
}

export function applyLocalPath(path: string, prefillName: boolean): void {
  const nextPath = path.trim();
  const nextName = directoryName(nextPath);
  const shouldPrefillName =
    prefillName &&
    Boolean(nextName) &&
    (!ui.formDraft.name.trim() || ui.formDraft.name.trim() === ui.autoFilledProjectName);
  ui.formDraft = {
    ...ui.formDraft,
    localPath: path,
    name: shouldPrefillName ? nextName : ui.formDraft.name,
  };
  if (shouldPrefillName) ui.autoFilledProjectName = nextName;
  supersedeProjectInference();
  ui.formError = "";
  void inferFromLocalPath(nextPath);
}

export async function inferFromLocalPath(path: string): Promise<void> {
  const requestedPath = path.trim();
  if (!requestedPath || !focusedHostIsLocal()) return;
  const requestId = ui.projectInference.requestId + 1;
  ui.projectInference = { status: "pending", requestId };
  render();
  try {
    const result = await rpc("inferProject", { localPath: requestedPath });
    if (requestId !== ui.projectInference.requestId || ui.formDraft.localPath.trim() !== requestedPath) return;
    ui.projectInference = result.inference
      ? { status: "candidate", requestId, candidate: result.inference }
      : { status: "failed", requestId, message: ui.snapshot?.copy.inferenceFailed ?? "" };
  } catch (error) {
    if (requestId !== ui.projectInference.requestId || ui.formDraft.localPath.trim() !== requestedPath) return;
    ui.projectInference = {
      status: "failed",
      requestId,
      message: error instanceof Error ? error.message : String(error),
    };
  }
  render();
}

export async function chooseProjectDirectory(): Promise<void> {
  if (!focusedHostIsLocal()) return;
  ui.formError = "";
  if (!desktopShellAvailable()) {
    ui.formError = ui.snapshot?.copy.chooseDirectoryDesktopOnly ?? "";
    render();
    return;
  }
  try {
    const selected = await openDirectory({
      directory: true,
      multiple: false,
      title: ui.snapshot?.copy.localDirectory,
      defaultPath: ui.formDraft.localPath.trim() || undefined,
      canCreateDirectories: false,
    });
    if (typeof selected !== "string" || !selected) return;
    applyLocalPath(selected, true);
  } catch (error) {
    ui.formError = error instanceof Error ? error.message : String(error);
    render();
  }
}

export async function loadStartupSettings(): Promise<void> {
  ui.startupSettingsError = "";
  if (!desktopShellAvailable()) {
    ui.startAtLogin = null;
    return;
  }
  try {
    ui.startAtLogin = await isEnabled();
  } catch (error) {
    ui.startupSettingsError = error instanceof Error ? error.message : String(error);
  }
}

export async function setHostMode(mode: Snapshot["hostMode"]): Promise<void> {
  ui.startupSettingsError = "";
  try {
    if (mode === "client-only") {
      const gate = await readUpdateInstallGate("updateInstallGate");
      if (!gate.allowed) {
        ui.startupSettingsError = startupCopy(ui.snapshot?.appearance.language ?? "en").hostModeActiveRuns;
        return;
      }
    }
    await invoke("set_host_mode", { mode });
    await relaunch();
  } catch (error) {
    ui.startupSettingsError = error instanceof Error ? error.message : String(error);
  }
}

export async function setStartAtLogin(enabled: boolean): Promise<void> {
  ui.startupSettingsError = "";
  try {
    if (enabled) await enable();
    else await disable();
    ui.startAtLogin = await isEnabled();
  } catch (error) {
    ui.startupSettingsError = error instanceof Error ? error.message : String(error);
  }
}

export async function requestDesktopNotificationPermission(): Promise<boolean> {
  if (desktopShellAvailable()) {
    if (await isNativeNotificationPermissionGranted()) return true;
    return (await requestNativeNotificationPermission()) === "granted";
  }
  if (typeof Notification === "undefined") return false;
  if (Notification.permission === "granted") return true;
  if (Notification.permission === "denied") return false;
  return (await Notification.requestPermission()) === "granted";
}

export async function checkForUpdates(manual: boolean): Promise<void> {
  if (!desktopShellAvailable()) {
    ui.updateState = { kind: "failed", message: ui.snapshot?.copy.updateUnavailableBrowser ?? "" };
    render();
    return;
  }
  if (ui.updateState.kind === "checking" || ui.updateState.kind === "installing") return;
  ui.updateState = { kind: "checking", manual };
  render();
  try {
    ui.pendingUpdate?.close().catch(() => {});
    ui.pendingUpdate = await check({ timeout: 30_000 });
    ui.pendingUpdateDownloaded = false;
    ui.updateState = ui.pendingUpdate
      ? {
          kind: "available",
          version: ui.pendingUpdate.version,
          notes: ui.pendingUpdate.body ?? "",
        }
      : { kind: "current" };
  } catch (error) {
    if (manual) {
      ui.updateState = {
        kind: "failed",
        message: error instanceof Error ? error.message : String(error),
      };
    } else {
      ui.updateState = { kind: "idle" };
    }
  }
  render();
}

export async function readUpdateInstallGate(op = "updateInstallGate"): Promise<UpdateInstallGate> {
  const result = await rpc(op);
  if (!result.updateInstallGate) throw new Error("Host returned no update install gate");
  return result.updateInstallGate;
}

export async function installPendingUpdate(): Promise<void> {
  if (!ui.pendingUpdate || ui.updateState.kind === "installing") return;
  let gate: UpdateInstallGate;
  try {
    gate = await readUpdateInstallGate();
  } catch (error) {
    ui.updateState = {
      kind: "failed",
      message: error instanceof Error ? error.message : String(error),
    };
    render();
    return;
  }
  if (!gate.allowed) {
    ui.updateState = { kind: "blocked", activeRunCount: gate.activeRunCount };
    render();
    return;
  }
  ui.updateState = { kind: "installing", progress: null };
  render();
  let downloaded = 0;
  let contentLength: number | undefined;
  try {
    if (!ui.pendingUpdateDownloaded) {
      await ui.pendingUpdate.download((event) => {
        if (event.event === "Started") {
          contentLength = event.data.contentLength;
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
        }
        const progress = contentLength && contentLength > 0
          ? Math.min(100, Math.round((downloaded / contentLength) * 100))
          : null;
        ui.updateState = { kind: "installing", progress };
        render();
      });
      ui.pendingUpdateDownloaded = true;
    }
    const finalGate = await readUpdateInstallGate("beginUpdateInstall");
    if (!finalGate.allowed) {
      ui.updateState = { kind: "blocked", activeRunCount: finalGate.activeRunCount };
      render();
      return;
    }
    await ui.pendingUpdate.install();
    await relaunch();
  } catch (error) {
    await rpc("cancelUpdateInstall").catch(() => {});
    ui.updateState = {
      kind: "failed",
      message: error instanceof Error ? error.message : String(error),
    };
    render();
  }
}
