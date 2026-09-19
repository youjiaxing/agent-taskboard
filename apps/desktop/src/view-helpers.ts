import { isTauri } from "@tauri-apps/api/core";
import { ui } from "./ui";
import type {
  AppearancePreference,
  BoardSnapshot,
  BrowserAppearance,
  Project,
  ResolvedTheme,
  SystemAppearance,
  RunSummary,
  ShellCopy,
  Snapshot,
} from "./protocol";
import { mobileMain as renderMobileMain, mobileNavigation as renderMobileNavigation, mobileScopeSheet as renderMobileScopeSheet } from "./mobile-renderers";
import { issueDetail, projectMain } from "./render/board";
import { injectRunForm, telemetryBar } from "./render/shell";

export const MOBILE_BREAKPOINT = 640;
export const APPEARANCE_PREFERENCES: AppearancePreference[] = ["system", "light", "dark", "warm"];
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
