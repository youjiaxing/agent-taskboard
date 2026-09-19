import { FitAddon } from "@xterm/addon-fit";
import type { Update } from "@tauri-apps/plugin-updater";
import { Terminal } from "@xterm/xterm";
import type { MobileView } from "./mobile-renderers";
import type {
  BoardScrollPosition,
  ChangeScope,
  FormKey,
  FormOperationState,
  GraphViewportAnchor,
  IssueContentDraft,
  IssueRelationDraft,
  IssueSearchDraft,
  LaunchDraft,
  LaunchEnvironmentState,
  BrowserAppearance,
  SystemAppearance,
  ClientLocalViewState,
  PanelPointerInteraction,
  Project,
  ProjectDraft,
  ProjectInferenceState,
  ScrollPosition,
  Snapshot,
  UpdateState,
  UsageCustomDraft,
  ViewChanges,
} from "./protocol";

function sessionClientId(): string {
  const key = "agent-taskboard-client-id";
  const windowMarkerPrefix = "agent-taskboard-client-window:";
  const existing = sessionStorage.getItem(key);
  const windowMarker = window.name.startsWith(windowMarkerPrefix)
    ? window.name.slice(windowMarkerPrefix.length)
    : "";
  const clonedFromOpener = Boolean(window.opener) && !windowMarker;
  if (existing && !clonedFromOpener && windowMarker) {
    if (!windowMarker) window.name = `${windowMarkerPrefix}${existing}`;
    return existing;
  }
  const id =
    typeof crypto !== "undefined" && "randomUUID" in crypto
      ? crypto.randomUUID()
      : `client-${Date.now()}-${Math.random().toString(36).slice(2)}`;
  sessionStorage.setItem(key, id);
  window.name = `${windowMarkerPrefix}${id}`;
  return id;
}

export function emptyDraft(): ProjectDraft {
  return { name: "", localPath: "", githubHost: "github.com", repository: "" };
}

const appEl = document.querySelector<HTMLDivElement>("#app");
if (!appEl) {
  throw new Error("missing #app");
}

export const ui = {
  app: appEl,
  snapshot: null as Snapshot | null,
  clientView: {
    page: "board",
    returnPoint: null,
    panels: {
      sidebarVisible: true,
      sidebarWidth: 248,
      rightSide: "rail",
      rightRailWidth: 320,
      changesPanelWidth: 520,
    },
  } as ClientLocalViewState,
  clientViewInitialized: false,
  returnPointHistory: [] as import("./protocol").ReturnPoint[],
  startAtLogin: null as boolean | null,
  startupSettingsError: "",
  launchEnvironmentState: {
    status: "idle",
    refreshedDirectories: 0,
  } as LaunchEnvironmentState,
  launchEnvironmentError: "",
  updateState: { kind: "idle" } as UpdateState,
  pendingUpdate: null as Update | null,
  pendingUpdateDownloaded: false,
  startupUpdateChecked: false,
  pairingOpen: false,
  hostPickerOpen: false,
  pairingAddress: "",
  pairingPaste: "",
  pairingError: "",
  projectMenuId: "",
  formOpen: null as "register" | "edit" | null,
  formProjectId: "",
  formDraft: emptyDraft(),
  autoFilledProjectName: "",
  projectInference: { status: "idle", requestId: 0 } as ProjectInferenceState,
  formError: "",
  removeError: "",
  projectOperation: null as "save" | "remove" | null,
  removeProject: null as Project | null,
  dangerConfirmation: null as
    | { kind: "stop-run"; runId: string; returnToMobileBoard: boolean }
    | { kind: "revoke-client"; clientId: string; clientName: string }
    | null,
  confirmationError: "",
  confirmationPending: false,
  refreshing: false,
  tickTimer: undefined as number | undefined,
  activePointers: new Set<number>(),
  tickRenderPending: false,
  tickFullRenderPending: false,
  term: null as Terminal | null,
  fitAddon: null as FitAddon | null,
  termHost: null as HTMLDivElement | null,
  ptyOffset: 0,
  ptyRunId: "",
  ptyPumping: false,
  launchDraft: null as LaunchDraft | null,
  launchFolded: false,
  launchPickerProjectId: "",
  launchPickerAgentId: "",
  launchPreviewTimer: undefined as number | undefined,
  launchPreviewSequence: 0,
  changesScope: "this-round" as ChangeScope,
  changesView: null as ViewChanges | null,
  noteDraft: "",
  noteTarget: null as { repo: string; path: string; line: number } | null,
  createIssueOpen: false,
  createIssueProjectId: "",
  createIssueDraft: { title: "", body: "" },
  issueEditOpenIds: new Set<string>(),
  issueEditDrafts: new Map<string, IssueContentDraft>(),
  issueCommentDrafts: new Map<string, string>(),
  terminalInputDrafts: new Map<string, string>(),
  issueSearchDraft: null as IssueSearchDraft | null,
  usageCustomDraft: null as UsageCustomDraft | null,
  issueRelationDrafts: new Map<string, IssueRelationDraft>(),
  issueMaintenanceOpen: new Set<string>(),
  formOperations: {
    pending: new Set<FormKey>(),
    errors: new Map<FormKey, string>(),
    conflicts: new Map(),
  } as FormOperationState,
  telemetryExpanded: false,
  keyboardHelpOpen: false,
  keyboardCursorIssueId: "",
  moreMenuOpen: false,
  terminalPanelVisible: true,
  renderedDetailIssueId: "",
  renderedBoardProjectId: "",
  renderedMobileWorkspaceKey: "",
  issueDetailScrollPositions: new Map<string, ScrollPosition>(),
  boardScrollPositions: new Map<string, BoardScrollPosition>(),
  mobileWorkspaceScrollPositions: new Map<string, ScrollPosition>(),
  renderedGraphKey: "",
  renderedGraphProjectId: "",
  renderedGraphCenterId: "",
  pendingGraphAnchor: null as GraphViewportAnchor | null,
  graphCanvasLimit: 48,
  graphListLimit: 50,
  graphListQuery: "",
  overviewProjectId: "",
  overviewShowEnded: false,
  mobileView: "board" as MobileView,
  mobileScopeOpen: false,
  mobileLiveTerminal: false,
  mobilePtyOffset: 0,
  mobilePtyRunId: "",
  mobilePtyPumping: false,
  mobilePtyText: new Map<string, string>(),
  browserAppearance: null as BrowserAppearance | null,
  systemAppearance: (window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light") as SystemAppearance,
  appearanceMenuOpen: false,
  clientId: sessionClientId(),
  panelPointerInteraction: null as PanelPointerInteraction | null,
  pendingCenterView: null as import("./protocol").CenterView | null,
};
