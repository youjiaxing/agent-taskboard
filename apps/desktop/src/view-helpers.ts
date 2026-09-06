import { ui } from "./ui";
import type { BoardSnapshot, MobileAppearance, Project, RunSummary, ShellCopy, Snapshot } from "./protocol";
import { mobileMain as renderMobileMain, mobileNavigation as renderMobileNavigation, mobileScopeSheet as renderMobileScopeSheet } from "./mobile-renderers";
import { issueDetail, projectMain } from "./render/board";
import { injectRunForm, telemetryBar } from "./render/shell";

const MOBILE_BREAKPOINT = 640;
const MOBILE_APPEARANCE_KEY = "agent-taskboard-mobile-appearance";

export function mobileClient(): boolean {
  return window.matchMedia(`(max-width: ${MOBILE_BREAKPOINT}px)`).matches;
}

export function systemMobileAppearance(): MobileAppearance {
  const language = navigator.language.toLowerCase().startsWith("zh") ? "zh-CN" : "en";
  const dark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  return {
    language,
    theme: dark ? "plain-night" : "warm-paper",
    lastLightTheme: "warm-paper",
  };
}

export function loadMobileAppearance(): MobileAppearance | null {
  try {
    const raw = localStorage.getItem(MOBILE_APPEARANCE_KEY);
    if (!raw) return null;
    const value = JSON.parse(raw) as Partial<MobileAppearance>;
    if (
      (value.language === "zh-CN" || value.language === "en")
      && (value.theme === "warm-paper" || value.theme === "plain-paper" || value.theme === "plain-night")
      && (value.lastLightTheme === "warm-paper" || value.lastLightTheme === "plain-paper")
    ) {
      return value as MobileAppearance;
    }
  } catch {
    // Use the browser defaults when local settings are invalid.
  }
  return null;
}

export function ensureMobileAppearance(): MobileAppearance {
  if (!ui.mobileAppearance) {
    ui.mobileAppearance = systemMobileAppearance();
    saveMobileAppearance(ui.mobileAppearance);
  }
  return ui.mobileAppearance;
}

export function saveMobileAppearance(appearance: MobileAppearance): void {
  ui.mobileAppearance = appearance;
  localStorage.setItem(MOBILE_APPEARANCE_KEY, JSON.stringify(appearance));
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
  if (mobileClient()) return ensureMobileAppearance().language;
  return ui.snapshot?.appearance.language ?? "en";
}
