import { FitAddon } from "@xterm/addon-fit";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { open as openDirectory } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import "./shell.css";
import { startupCopy, type StartupCopy } from "./startup-copy";

type Language = "zh-CN" | "en";
type Theme = "warm-paper" | "plain-paper" | "plain-night";

type ShellCopy = {
  appName: string;
  registerFirstProject: string;
  pairAnotherHost: string;
  noProjectTitle: string;
  noProjectBody: string;
  quitHost: string;
  showWindow: string;
  settings: string;
  updates: string;
  checkForUpdates: string;
  updateChecking: string;
  updateAvailable: string;
  updateReady: string;
  updateNotes: string;
  updateConfirm: string;
  updateLater: string;
  updateCurrent: string;
  updateUnavailableBrowser: string;
  updateActiveRuns: string;
  updateInstalling: string;
  updateFailed: string;
  language: string;
  theme: string;
  languageZh: string;
  languageEn: string;
  themeWarmPaper: string;
  themePlainPaper: string;
  themePlainNight: string;
  hosts: string;
  projects: string;
  thisMachine: string;
  nextStep: string;
  forgetHost: string;
  forgetHostConfirmTitle: string;
  forgetHostConfirmBody: string;
  shadeLight: string;
  shadeDark: string;
  editMenu: string;
  pairingRequired: string;
  pairingTitle: string;
  pairingThisHost: string;
  pairingToAnother: string;
  pairingAddress: string;
  pairingShow: string;
  pairingCopy: string;
  pairingSamePayload: string;
  pairingPaste: string;
  pairingConnect: string;
  pairedClients: string;
  revokeClient: string;
  noPairedClients: string;
  addProject: string;
  editProject: string;
  removeProject: string;
  registerProjectTitle: string;
  editProjectTitle: string;
  displayName: string;
  localDirectory: string;
  chooseDirectory: string;
  chooseDirectoryDesktopOnly: string;
  inferringFromDirectory: string;
  inferenceFailed: string;
  activeProjectEditHint: string;
  remoteProjectHint: string;
  operationPending: string;
  inferencePending: string;
  retryInference: string;
  removalPending: string;
  githubHost: string;
  repository: string;
  inferFromDirectory: string;
  useInference: string;
  inferenceHint: string;
  saveRegistration: string;
  cancel: string;
  removeConfirmTitle: string;
  removeConfirmBody: string;
  removeConfirm: string;
  cannotRemoveActiveRun: string;
  cannotRemoveActiveRunBody: string;
  removeKeepClaimsBody: string;
  continueRun: string;
  releaseClaim: string;
  executionStopped: string;
  waiting: string;
  running: string;
  injectLine: string;
  injectPlaceholder: string;
  notifyDesktop: string;
  notifySound: string;
  notifyWaiting: string;
  notifyCompleted: string;
  notifyAbnormal: string;
  notifyCrash: string;
  gotIt: string;
  authFailed: string;
  connectionUnavailable: string;
  repairCli: string;
  repairSecrets: string;
  repairEnv: string;
  noGhDetected: string;
  connectionReady: string;
  projectMenu: string;
  boardHint: string;
  childHint: string;
  graphHint: string;
  viewBoard: string;
  viewGraph: string;
  viewDependencies: string;
  graphOverview: string;
  graphReturnOverview: string;
  graphTruncated: string;
  graphNoDependencies: string;
  showClosedContext: string;
  graphCenter: string;
  graphCenterHere: string;
  graphShowComplete: string;
  graphShowNeighborhood: string;
  graphShowMore: string;
  graphCanvasLimit: string;
  graphCompleteList: string;
  graphSearchPlaceholder: string;
  graphUpstream: string;
  graphDownstream: string;
  graphBoth: string;
  clearFilter: string;
  colBlocked: string;
  colFrontier: string;
  colInProgress: string;
  colRecent: string;
  noItems: string;
  noFrontierBlocked: string;
  noFrontierClaimed: string;
  noFrontierEmpty: string;
  noRecent: string;
  recentNote: string;
  emptyNoData: string;
  emptyIncomplete: string;
  emptyTrackerError: string;
  issueDocument: string;
  issueDocumentLoading: string;
  issueDocumentRetry: string;
  issueDocumentStale: string;
  issueDocumentFailed: string;
  family: string;
  deps: string;
  parent: string;
  children: string;
  noParent: string;
  noKids: string;
  onlyKids: string;
  blockedBy: string;
  blocking: string;
  noneBlock: string;
  none: string;
  claimed: string;
  unclaimed: string;
  pickIssue: string;
  recentLimit: string;
  recentLimitHelp: string;
  refreshInterval: string;
  refreshIntervalHelp: string;
  unclearIssue: string;
  refreshNow: string;
  refreshRefreshing: string;
  refreshAsOf: string;
  refreshNext: string;
  refreshOffline: string;
  refreshOfflineRecovery: string;
  refreshNever: string;
  refreshRateLimited: string;
  refreshRetry: string;
  refreshPaused: string;
  refreshAuth: string;
  refreshAuthRecovery: string;
  refreshIncomplete: string;
  refreshTrackerError: string;
  newRun: string;
  executeRun: string;
  startRun: string;
  startRunPending: string;
  switchAgent: string;
  pickAgent: string;
  launchTitle: string;
  prefillCurrent: string;
  prefillOther: string;
  prefillSeed: string;
  isolation: string;
  isolationOffReason: string;
  isolationHint: string;
  runIntent: string;
  intentNone: string;
  intentModify: string;
  intentContinue: string;
  intentAnswer: string;
  intentReview: string;
  intentCustom: string;
  openingPlaceholder: string;
  foldedOptions: string;
  commandPreview: string;
  showCommandPreview: string;
  instructionRequired: string;
  workingDirectory: string;
  unboundIssue: string;
  stopRun: string;
  quitActiveTitle: string;
  quitActiveBody: string;
  quitReturn: string;
  quitStopAll: string;
  viewChanges: string;
  focusRun: string;
  openIssue: string;
  searchTitle: string;
  searchPlaceholder: string;
  searchAllTriage: string;
  searchAllStates: string;
  searchOpen: string;
  searchClosed: string;
  searchSubmit: string;
  keyboardHelp: string;
  keyboardHelpBody: string;
  thisRound: string;
  uncommitted: string;
  addChangeNote: string;
  changeNotePlaceholder: string;
  deleteChangeNote: string;
  autoAdvance: string;
  autoAdvanceHelp: string;
  projectAutoAdvance: string;
  restoreAutoAdvance: string;
  restoreDelay: string;
  pendingConfirmation: string;
  vetoAdvance: string;
  usage: string;
  usageHint: string;
  hostOverview: string;
  hostOverviewHint: string;
  hostOverviewEmpty: string;
  returnToBoard: string;
  showSidebar: string;
  hideSidebar: string;
  showIssueDetail: string;
  hideIssueDetail: string;
  showEndedRuns: string;
  runGroupWaiting: string;
  runGroupRunning: string;
  runGroupStopped: string;
  runGroupEnded: string;
  range24Hours: string;
  rangeToday: string;
  range7Days: string;
  range30Days: string;
  rangeCustom: string;
  filterAll: string;
  filterProject: string;
  filterAgent: string;
  filterModel: string;
  tokenInput: string;
  tokenOutput: string;
  tokenCacheRead: string;
  tokenCacheWrite: string;
  tokenReasoning: string;
  tokenTotal: string;
  ttft: string;
  genRate: string;
  cacheHit: string;
  spike: string;
  proxyDisclaimer: string;
  openHostUsage: string;
  openThisRun: string;
  laneMain: string;
  laneSubagent: string;
  laneSwitched: string;
  usageEmpty: string;
  closeUsage: string;
  mobileSwitchScope: string;
  mobileBoard: string;
  mobileIssue: string;
  mobileRun: string;
  mobileRecentOutput: string;
  mobileLiveTerminal: string;
};

type CredentialSource = "app-env" | "secrets-file" | "cli" | "generic-env" | "local-file";

type Repair = {
  cliDetected: boolean;
  secretsPath: string;
  appEnv: string;
  genericEnv: string;
  suggestedScope: string;
};

type ProjectConnection =
  | { status: "ready"; source: CredentialSource }
  | {
      status: "auth-failed";
      source?: CredentialSource;
      kind: "missing-credentials" | "rejected" | "unreachable";
      repair: Repair;
      message: string;
    }
  | {
      status: "unreachable";
      source?: CredentialSource;
      repair: Repair;
      message: string;
    };

type Project = {
  id: string;
  name: string;
  localPath: string;
  tracker: "github" | "local-markdown";
  githubHost: string;
  repository: string;
  connection: ProjectConnection;
  hasActiveRun: boolean;
  hasExecutionStopped?: boolean;
  trackerSynced: boolean;
  autoAdvance?: boolean;
  restoreAutoAdvance?: boolean;
  restoreDelayMs?: number;
  issueCounts?: ProjectIssueCounts;
};

type ProjectIssueCounts = {
  dataAvailable: boolean;
  total: number;
  open: number;
  closed: number;
  blocked: number;
  frontier: number;
  inProgress: number;
};

type ProjectDraft = {
  name: string;
  localPath: string;
  githubHost: string;
  repository: string;
  tracker?: "github" | "local-markdown";
};

type IssueSearchDraft = {
  title: string;
  triageRole: string;
  state: string;
};

type UsageCustomDraft = {
  from: string;
  to: string;
};

type FormKey =
  | `issue-search:${string}`
  | `inject-run:${string}`
  | `change-note:${string}`
  | `usage-custom:${string}`
  | `launch:${string}`
  | `pairing:${string}`;

type FormOperationState = {
  pending: Set<FormKey>;
  errors: Map<FormKey, string>;
};

type ProjectInferenceState =
  | { status: "idle"; requestId: number }
  | { status: "pending"; requestId: number }
  | { status: "candidate"; requestId: number; candidate: ProjectDraft }
  | { status: "failed"; requestId: number; message: string };

type TriageRole =
  | "needs-triage"
  | "needs-info"
  | "ready-for-agent"
  | "ready-for-human"
  | "wontfix";

type IssueCard = {
  id: string;
  repository: string;
  number: number;
  title: string;
  url: string;
  claimedBy: string[];
  triageRole: TriageRole | null;
  open: boolean;
  activity?: "running" | "waiting" | "execution-stopped" | null;
  runId?: string | null;
};

type IssueLink = {
  id: string;
  repository: string;
  number: number | null;
  title: string;
  open: boolean | null;
  visible: boolean;
};

type IssueDetail = {
  id: string;
  repository: string;
  number: number;
  title: string;
  url: string;
  open: boolean;
  claimedBy: string[];
  triageRole: TriageRole | null;
  labels: string[];
  parent: IssueLink | null;
  children: IssueLink[];
  blockedBy: IssueLink[];
  blocking: IssueLink[];
  document: IssueDocumentState;
  executionStopped?: boolean;
  waitingForUser?: boolean;
  activeRunId?: string | null;
};

type IssueDocumentFailure = {
  kind: "offline" | "rate-limited" | "auth" | "tracker";
  message: string;
  retryAfterMs?: number | null;
};

type IssueDocumentState =
  | { kind: "unloaded" }
  | { kind: "loading"; body?: string | null; fetchedAtMs?: number | null }
  | { kind: "ready"; body: string; fetchedAtMs: number }
  | { kind: "stale"; body: string; fetchedAtMs: number; failure: IssueDocumentFailure }
  | { kind: "failed"; failure: IssueDocumentFailure };

type BoardColumns = {
  blocked: IssueCard[];
  frontier: IssueCard[];
  inProgress: IssueCard[];
  recentlyCompleted: IssueCard[];
};

type RefreshStatus =
  | { kind: "refreshing"; fetchedAtMs?: number | null }
  | { kind: "ready"; fetchedAtMs: number; nextRefreshInMs?: number | null }
  | { kind: "offline"; fetchedAtMs: number; nextRefreshInMs?: number | null }
  | { kind: "never-fetched" }
  | { kind: "rate-limited"; fetchedAtMs?: number | null; retryAtMs?: number | null }
  | { kind: "auth-failed"; fetchedAtMs?: number | null }
  | {
      kind: "incomplete" | "tracker-error";
      fetchedAtMs?: number | null;
      dataComplete?: boolean;
      nextRefreshInMs?: number | null;
      detail?: string | null;
    };

type GraphNode = {
  id: string;
  repository: string;
  number: number;
  title: string;
  open: boolean;
  rank: number;
  distance?: number;
  relation?: "center" | "upstream" | "downstream" | "both";
};

type GraphEdge = {
  from: string;
  to: string;
};

type DependencyGraph = {
  nodes: GraphNode[];
  edges: GraphEdge[];
  mode?: "overview" | "focused";
  centerId?: string | null;
  totalCount?: number;
  complete?: boolean;
  maxDistance?: number;
  truncated?: boolean;
  closedCount?: number;
};

type CenterView = "board" | "graph";
type WorkspaceView = "project" | "host-overview" | "run";
type ClientGraphMode = "overview" | "focused";
type ClientViewState = {
  focusedHostId: string;
  focusedProjectId: string;
  selectedIssueId: string | null;
  focusedRunId: string;
  centerView: CenterView;
  workspaceView: WorkspaceView;
  parentFilterId: string | null;
  search: BoardSnapshot["search"];
  graphMode: ClientGraphMode;
  graphCenterIssueId: string | null;
  completeDependencyGraph: boolean;
  usageOpen: boolean;
  usageQuery: {
    range: UsageRange;
    customFromMs?: number | null;
    customToMs?: number | null;
    filter: UsageFilter;
    highlightedRunId?: string | null;
  };
};

type BoardSnapshot = {
  projectId: string;
  columns: BoardColumns | null;
  empty: "no-data" | "incomplete-read" | "tracker-error" | null;
  frontierEmpty: "all-blocked" | "all-claimed" | "no-open" | null;
  parentFilter: IssueCard | null;
  selected: IssueDetail | null;
  labelMappingActive: boolean;
  recentLimit: number;
  refresh: RefreshStatus;
  graph: DependencyGraph | null;
  showClosedGraphContext: boolean;
  search: {
    title: string;
    triageRole: TriageRole | null;
    state: "all" | "open" | "closed";
  };
};

type PairingOffer = {
  address: string;
  code: string;
  text: string;
  qrText: string;
  qrSvg: string;
};

type PairedClient = { id: string; name: string };

type LoopbackPage =
  | { status: "serving"; url: string }
  | { status: "occupied"; url: string; reason: string }
  | { status: "host-not-running"; url: string; reason: string };

type Snapshot = {
  running: boolean;
  windowVisible: boolean;
  hostMode: "host-and-client" | "client-only";
  focusedHostId: string;
  focusedProjectId: string;
  hosts: { id: string; displayName: string; local: boolean }[];
  projects: Project[];
  appearance: {
    language: Language;
    theme: Theme;
    lastLightTheme: Theme;
    languages: Language[];
    themes: Theme[];
  };
  copy: ShellCopy;
  copyCatalog: Record<Language, ShellCopy>;
  emptyActions: Array<"register-first-project" | "pair-another-host">;
  loopbackPage: LoopbackPage;
  pairingOffer: PairingOffer | null;
  pairedClients: PairedClient[];
  board: BoardSnapshot | null;
  recentCompletedLimit: number;
  refreshIntervalMs: number;
  centerView: CenterView;
  workspaceView: WorkspaceView;
  runs: RunSummary[];
  focusedRunId: string;
  quitOffer: QuitOffer | null;
  launchForm?: RunLaunchForm | null;
  showCommandPreview?: boolean;
  notifyDesktop?: boolean;
  notifySound?: boolean;
  autoAdvance?: boolean;
  pendingConfirmation?: PendingConfirmation | null;
  usageOpen?: boolean;
  usage?: UsagePage;
};

type PendingConfirmation = {
  projectId: string;
  issueId: string;
  runId: string;
  agentId: string;
  deadlineMs: number;
  remainingMs: number;
};

type AgentFieldKind = "text" | "select" | "boolean" | "multiline";

type AgentField = {
  id: string;
  label: string;
  kind: AgentFieldKind;
  options?: string[];
  required: boolean;
  folded: boolean;
};

type AgentSummary = {
  id: string;
  name: string;
  installed: boolean;
  unavailableReason?: string | null;
  fields: AgentField[];
};

type IntentOption = {
  id: string;
  label: string;
  prefix: string;
};

type RunLaunchForm = {
  projectId: string;
  issueId?: string | null;
  agents: AgentSummary[];
  selectedAgentId: string;
  skipAgentPicker: boolean;
  fields: AgentField[];
  values: Record<string, string>;
  prefillSource: "current-project" | "other-project" | "cli-seed";
  workingDirectory: string;
  isolationSupported: boolean;
  isolationReason: string;
  openingText: string;
  changeNotesText?: string;
  commandPreview: string;
  intents: IntentOption[];
  warnings?: string[];
  error?: string | null;
};

type ChangeScope = "this-round" | "uncommitted";

type ChangeLine = {
  kind: "context" | "add" | "delete";
  oldLine?: number | null;
  newLine?: number | null;
  text: string;
};

type ChangeHunk = {
  header: string;
  lines: ChangeLine[];
};

type ChangeFile = {
  path: string;
  hunks: ChangeHunk[];
};

type ChangeRepo = {
  path: string;
  displayPath: string;
  available: boolean;
  unavailableReason?: string | null;
  startCommit?: string | null;
  files: ChangeFile[];
};

type ChangeNote = {
  id: string;
  runId: string;
  projectId: string;
  issueId?: string | null;
  repo: string;
  path: string;
  line: number;
  text: string;
};

type ViewChanges = {
  runId: string;
  issueId?: string | null;
  workingDirectory: string;
  isolated: boolean;
  scope: ChangeScope;
  available: boolean;
  unavailableReason?: string | null;
  repos: ChangeRepo[];
  notes: ChangeNote[];
};

type LaunchDraft = {
  projectId: string;
  issueId?: string | null;
  agentId: string;
  values: Record<string, string>;
  openingText: string;
  intentId: string;
  custom: boolean;
};

type RunStatus = "starting" | "running" | "ended";

type RunSummary = {
  id: string;
  projectId: string;
  agentId: string;
  agentName: string;
  issueId?: string | null;
  unbound: boolean;
  status: RunStatus;
  waitingForUser?: boolean;
  recentAction?: string | null;
  failure?: string | null;
  previousRunId?: string | null;
  nativeSessionId?: string | null;
  endedReason?: "exited" | "stopped" | "abnormal" | "crash" | null;
  workingDirectory?: string;
  isolated?: boolean;
  isolationNote?: string | null;
  startedAtMs?: number;
  telemetry?: RunTelemetryLane[];
  recentOutput?: string;
};

type TokenCounts = {
  input?: number | null;
  output?: number | null;
  cacheRead?: number | null;
  cacheWrite?: number | null;
  reasoning?: number | null;
  total?: number | null;
};

type TelemetryLaneKind = "main" | "subagent" | "switched";

type TelemetryPoint = {
  atMs: number;
  ttftMs?: number | null;
  tokensPerSec?: number | null;
  spike: boolean;
};

type RunTelemetryLane = {
  model: string;
  lane: TelemetryLaneKind;
  tokens: TokenCounts;
  ttftMs?: number | null;
  tokensPerSec?: number | null;
  recent: TelemetryPoint[];
  spike: boolean;
};

type UsageRange = "today" | "24-hours" | "7-days" | "30-days" | "custom";

type UsageFilter = {
  projectId?: string | null;
  agentId?: string | null;
  model?: string | null;
};

type UsageOption = { id: string; name: string };

type UsageRunRow = {
  runId: string;
  projectId: string;
  projectName: string;
  agentId: string;
  agentName: string;
  issueId?: string | null;
  startedAtMs: number;
  models: string[];
  tokens: TokenCounts;
  highlighted: boolean;
};

type UsageBucket = {
  startMs: number;
  tokens: TokenCounts;
  ttftMs?: number | null;
  tokensPerSec?: number | null;
  slow: boolean;
};

type UsagePage = {
  range: UsageRange;
  customFromMs?: number | null;
  customToMs?: number | null;
  filter: UsageFilter;
  bucketKind: "hour" | "day";
  fromMs: number;
  toMs: number;
  runs: UsageRunRow[];
  buckets: UsageBucket[];
  totals: TokenCounts;
  cacheHitRate?: number | null;
  highlightedRunId?: string | null;
  projects: UsageOption[];
  agents: UsageOption[];
  models: string[];
};

type QuitOffer = {
  activeRunCount: number;
};

type UpdateInstallGate = {
  allowed: boolean;
  activeRunCount: number;
};

type UpdateState =
  | { kind: "idle" }
  | { kind: "checking"; manual: boolean }
  | { kind: "current" }
  | { kind: "available"; version: string; notes: string }
  | { kind: "blocked"; activeRunCount: number }
  | { kind: "installing"; progress: number | null }
  | { kind: "failed"; message: string };

type NotificationKind = "waiting" | "completed" | "abnormal-stop" | "crash-recovered";

type HostEvent =
  | { type: "refresh-status-changed"; projectId: string; status: RefreshStatus }
  | { type: "board-updated"; projectId: string }
  | { type: "run-status-changed"; runId: string; status: RunStatus }
  | { type: "waiting"; runId: string }
  | { type: "execution-stopped"; issueId: string; runId: string }
  | { type: "host-crashed-recovered"; runIds: string[] }
  | {
      type: "pending-confirmation-started";
      projectId: string;
      issueId: string;
      runId: string;
    }
  | {
      type: "pending-confirmation-ended";
      projectId: string;
      issueId: string;
      runId: string;
      advanced: boolean;
    }
  | {
      type: "notification";
      kind: NotificationKind;
      runId: string;
      issueId?: string | null;
      projectId: string;
    }
  | { type: "telemetry"; runId: string };

type LaunchEnvironmentState = {
  status: "idle" | "ready" | "failed";
  refreshedDirectories: number;
  message?: string | null;
};

type RpcResult = {
  snapshot: Snapshot;
  process: "keep-running" | "exit";
  launchEnvironment?: LaunchEnvironmentState;
  inference?: ProjectDraft;
  updateInstallGate?: UpdateInstallGate;
  events?: HostEvent[];
  viewChanges?: ViewChanges;
};

const app = document.querySelector<HTMLDivElement>("#app");
if (!app) {
  throw new Error("missing #app");
}

let snapshot: Snapshot | null = null;
let settingsOpen = false;
let startAtLogin: boolean | null = null;
let startupSettingsError = "";
let launchEnvironmentState: LaunchEnvironmentState = {
  status: "idle",
  refreshedDirectories: 0,
};
let launchEnvironmentError = "";
let updateState: UpdateState = { kind: "idle" };
let pendingUpdate: Update | null = null;
let pendingUpdateDownloaded = false;
let startupUpdateChecked = false;
let pairingOpen = false;
let hostPickerOpen = false;
let pairingAddress = "";
let pairingPaste = "";
let pairingError = "";
let projectMenuId = "";
let formOpen: "register" | "edit" | null = null;
let formProjectId = "";
let formDraft: ProjectDraft = emptyDraft();
let autoFilledProjectName = "";
let projectInference: ProjectInferenceState = { status: "idle", requestId: 0 };
let formError = "";
let removeError = "";
let projectOperation: "save" | "remove" | null = null;
let removeProject: Project | null = null;
let forgetHostId = "";
let forgetHostError = "";
let forgetHostPending = false;
type GraphViewportAnchor = {
  issueId: string;
  viewportX: number;
  viewportY: number;
};
let pendingGraphAnchor: GraphViewportAnchor | null = null;
let refreshing = false;
let tickTimer: number | undefined;
const activePointers = new Set<number>();
let tickRenderPending = false;
let term: Terminal | null = null;
let fitAddon: FitAddon | null = null;
let termHost: HTMLDivElement | null = null;
let ptyOffset = 0;
let ptyRunId = "";
let ptyPumping = false;
let launchDraft: LaunchDraft | null = null;
let launchFolded = false;
let agentPickerSelection = "";
let changesOpen = false;
let changesScope: ChangeScope = "this-round";
let changesView: ViewChanges | null = null;
let noteDraft = "";
let noteTarget: { repo: string; path: string; line: number } | null = null;
let issueSearchDraft: IssueSearchDraft | null = null;
let usageCustomDraft: UsageCustomDraft | null = null;
const injectDrafts = new Map<string, string>();
const formOperations: FormOperationState = {
  pending: new Set<FormKey>(),
  errors: new Map<FormKey, string>(),
};
let telemetryExpanded = false;
let keyboardHelpOpen = false;
let keyboardCursorIssueId = "";
let sidebarVisible = true;
let issueDetailVisible = true;
let renderedDetailIssueId = "";
let renderedSnapshotKey = "";
let renderedGraphKey = "";
let renderedGraphProjectId = "";
let renderedGraphCenterId = "";
let graphCanvasLimit = 48;
let graphBatchTimer: number | undefined;
let graphListLimit = 50;
let graphListQuery = "";
let overviewProjectId = "";
let overviewShowEnded = false;
let sidebarBeforeLift = true;
type MobileView = "board" | "issue" | "run";
type MobileAppearance = { language: Language; theme: Theme; lastLightTheme: Theme };
let mobileView: MobileView = "board";
let mobileScopeOpen = false;
let mobileLiveTerminal = false;
let mobilePtyOffset = 0;
let mobilePtyRunId = "";
let mobilePtyPumping = false;
const mobilePtyText = new Map<string, string>();
let mobileAppearance = loadMobileAppearance();
const clientId = sessionClientId();
let clientView = loadClientView(clientId);
let clientViewRevision = 0;

const GRAPH_RELATION_META: Record<
  NonNullable<GraphNode["relation"]>,
  { order: number; label: (copy: ShellCopy) => string }
> = {
  upstream: { order: 0, label: (copy) => copy.graphUpstream },
  center: {
    order: 1,
    label: (copy) => copy.graphCenter.replace("：{issue}", "").replace(": {issue}", ""),
  },
  both: { order: 2, label: (copy) => copy.graphBoth },
  downstream: { order: 3, label: (copy) => copy.graphDownstream },
};

function sessionClientId(): string {
  const key = "agent-taskboard-client-id";
  const existing = sessionStorage.getItem(key);
  if (existing) return existing;
  const id =
    typeof crypto !== "undefined" && "randomUUID" in crypto
      ? crypto.randomUUID()
      : `client-${Date.now()}`;
  sessionStorage.setItem(key, id);
  return id;
}

function defaultClientView(): ClientViewState {
  return {
    focusedHostId: "local",
    focusedProjectId: "",
    selectedIssueId: null,
    focusedRunId: "",
    centerView: "board",
    workspaceView: "project",
    parentFilterId: null,
    search: { title: "", triageRole: null, state: "all" },
    graphMode: "overview",
    graphCenterIssueId: null,
    completeDependencyGraph: false,
    usageOpen: false,
    usageQuery: {
      range: "today",
      customFromMs: null,
      customToMs: null,
      filter: {},
      highlightedRunId: null,
    },
  };
}

function clientViewStorageKey(id: string): string {
  return `agent-taskboard-client-view:${id}`;
}

function loadClientView(id: string): ClientViewState {
  const fallback = defaultClientView();
  try {
    const raw = localStorage.getItem(clientViewStorageKey(id));
    if (!raw) return fallback;
    const stored = JSON.parse(raw) as Partial<ClientViewState>;
    return {
      ...fallback,
      focusedHostId: typeof stored.focusedHostId === "string" ? stored.focusedHostId : fallback.focusedHostId,
      focusedProjectId: typeof stored.focusedProjectId === "string" ? stored.focusedProjectId : "",
      selectedIssueId: typeof stored.selectedIssueId === "string" ? stored.selectedIssueId : null,
      focusedRunId: typeof stored.focusedRunId === "string" ? stored.focusedRunId : "",
      centerView: stored.centerView === "graph" ? "graph" : "board",
      workspaceView:
        stored.workspaceView === "host-overview" || stored.workspaceView === "run"
          ? stored.workspaceView
          : "project",
      parentFilterId: typeof stored.parentFilterId === "string" ? stored.parentFilterId : null,
      search: {
        title: typeof stored.search?.title === "string" ? stored.search.title : "",
        triageRole: stored.search?.triageRole ?? null,
        state:
          stored.search?.state === "open" || stored.search?.state === "closed"
            ? stored.search.state
            : "all",
      },
      graphMode: stored.graphMode === "focused" ? "focused" : "overview",
      graphCenterIssueId:
        typeof stored.graphCenterIssueId === "string" ? stored.graphCenterIssueId : null,
      completeDependencyGraph: Boolean(stored.completeDependencyGraph),
      usageOpen: Boolean(stored.usageOpen),
      usageQuery: {
        range:
          stored.usageQuery?.range === "24-hours"
          || stored.usageQuery?.range === "7-days"
          || stored.usageQuery?.range === "30-days"
          || stored.usageQuery?.range === "custom"
            ? stored.usageQuery.range
            : "today",
        customFromMs:
          typeof stored.usageQuery?.customFromMs === "number" ? stored.usageQuery.customFromMs : null,
        customToMs:
          typeof stored.usageQuery?.customToMs === "number" ? stored.usageQuery.customToMs : null,
        filter: {
          projectId:
            typeof stored.usageQuery?.filter?.projectId === "string"
              ? stored.usageQuery.filter.projectId
              : null,
          agentId:
            typeof stored.usageQuery?.filter?.agentId === "string"
              ? stored.usageQuery.filter.agentId
              : null,
          model:
            typeof stored.usageQuery?.filter?.model === "string"
              ? stored.usageQuery.filter.model
              : null,
        },
        highlightedRunId:
          typeof stored.usageQuery?.highlightedRunId === "string"
            ? stored.usageQuery.highlightedRunId
            : null,
      },
    };
  } catch {
    return fallback;
  }
}

function saveClientView(): void {
  localStorage.setItem(clientViewStorageKey(clientId), JSON.stringify(clientView));
}

function commitClientView(next: ClientViewState): void {
  if (JSON.stringify(next) !== JSON.stringify(clientView)) clientViewRevision += 1;
  clientView = next;
  saveClientView();
}

function updateClientView(patch: Partial<ClientViewState>): void {
  commitClientView({ ...clientView, ...patch });
}

function reconcileClientView(snap: Snapshot): void {
  const board = snap.board;
  const projectChanged = snap.focusedProjectId !== clientView.focusedProjectId;
  const selectedIssueConfirmedMissing = Boolean(
    !projectChanged
      && clientView.selectedIssueId
      && board
      && board.refresh.kind === "ready"
      && !board.selected,
  );
  const next: ClientViewState = {
    ...clientView,
    focusedHostId: snap.focusedHostId,
    focusedProjectId: snap.focusedProjectId,
    selectedIssueId:
      board?.selected?.id
      ?? (projectChanged || selectedIssueConfirmedMissing ? null : clientView.selectedIssueId),
    focusedRunId: snap.focusedRunId,
    centerView: snap.centerView,
    workspaceView: snap.workspaceView,
    parentFilterId: board?.parentFilter?.id ?? null,
    search: board?.search ?? (projectChanged ? defaultClientView().search : clientView.search),
    graphCenterIssueId:
      clientView.graphMode === "focused"
        ? board?.graph?.centerId ?? clientView.graphCenterIssueId
        : null,
    completeDependencyGraph: Boolean(board?.graph?.complete),
    usageOpen: Boolean(snap.usageOpen),
    usageQuery: {
      range: snap.usage?.range ?? clientView.usageQuery.range,
      customFromMs: snap.usage?.customFromMs ?? clientView.usageQuery.customFromMs ?? null,
      customToMs: snap.usage?.customToMs ?? clientView.usageQuery.customToMs ?? null,
      filter: { ...(snap.usage?.filter ?? clientView.usageQuery.filter) },
      highlightedRunId:
        snap.usage?.highlightedRunId ?? clientView.usageQuery.highlightedRunId ?? null,
    },
  };
  commitClientView(next);
}

function emptyDraft(): ProjectDraft {
  return { name: "", localPath: "", githubHost: "github.com", repository: "", tracker: "github" };
}

function resetGraphUiState(): void {
  if (graphBatchTimer != null) window.clearTimeout(graphBatchTimer);
  graphBatchTimer = undefined;
  graphCanvasLimit = 48;
  graphListLimit = 50;
  graphListQuery = "";
}

function resetInlineFormDrafts(): void {
  issueSearchDraft = null;
  usageCustomDraft = null;
  injectDrafts.clear();
  formOperations.errors.clear();
}

const MOBILE_BREAKPOINT = 640;
const MOBILE_APPEARANCE_KEY = "agent-taskboard-mobile-appearance";

function mobileClient(): boolean {
  return window.matchMedia(`(max-width: ${MOBILE_BREAKPOINT}px)`).matches;
}

function systemMobileAppearance(): MobileAppearance {
  const language = navigator.language.toLowerCase().startsWith("zh") ? "zh-CN" : "en";
  const dark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  return {
    language,
    theme: dark ? "plain-night" : "warm-paper",
    lastLightTheme: "warm-paper",
  };
}

function loadMobileAppearance(): MobileAppearance | null {
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

function ensureMobileAppearance(): MobileAppearance {
  if (!mobileAppearance) {
    mobileAppearance = systemMobileAppearance();
    saveMobileAppearance(mobileAppearance);
  }
  return mobileAppearance;
}

function saveMobileAppearance(appearance: MobileAppearance): void {
  mobileAppearance = appearance;
  localStorage.setItem(MOBILE_APPEARANCE_KEY, JSON.stringify(appearance));
}

function syncLaunchDraft(snap: Snapshot): void {
  const form = snap.launchForm;
  if (!form) {
    launchDraft = null;
    return;
  }
  if (
    !launchDraft
    || launchDraft.projectId !== form.projectId
    || launchDraft.agentId !== form.selectedAgentId
  ) {
    launchDraft = {
      projectId: form.projectId,
      issueId: form.issueId,
      agentId: form.selectedAgentId,
      values: { ...form.values },
      openingText: form.openingText,
      intentId: "",
      custom: false,
    };
  }
  if (!form.skipAgentPicker && !agentPickerSelection) {
    agentPickerSelection = "";
  }
}

function prefillHint(copy: ShellCopy, source: RunLaunchForm["prefillSource"]): string {
  if (source === "current-project") return copy.prefillCurrent;
  if (source === "other-project") return copy.prefillOther;
  return copy.prefillSeed;
}

function liveEnumWarnings(form: RunLaunchForm, draft: LaunchDraft, language: Language): string[] {
  const warnings: string[] = [];
  for (const field of form.fields) {
    if (field.kind !== "select" || !field.options?.length) continue;
    const value = (draft.values[field.id] ?? "").trim();
    if (!value || field.options.includes(value)) continue;
    warnings.push(
      language === "zh-CN"
        ? `${value} 不是已知的 ${field.label}，仍可启动。`
        : `${value} is not a known ${field.label}; launch is still allowed.`,
    );
  }
  return warnings;
}

function refreshLaunchWarnings(): void {
  const node = app?.querySelector<HTMLElement>(".launch-warnings");
  if (!node || !snapshot?.launchForm || !launchDraft) return;
  const live = liveEnumWarnings(
    snapshot.launchForm,
    launchDraft,
    snapshot.appearance.language,
  );
  const preserved = (snapshot.launchForm.warnings ?? []).filter(
    (warning) => !warning.includes("不是已知的") && !warning.includes("is not a known"),
  );
  const warnings = [...preserved, ...live];
  node.textContent = warnings.join(" ");
  node.hidden = warnings.length === 0;
}

function refreshIntentChoices(): void {
  if (!app || !launchDraft) return;
  for (const button of app.querySelectorAll<HTMLButtonElement>(".launch-sheet button[data-act='intent']")) {
    button.classList.toggle(
      "active",
      !launchDraft.custom && (button.dataset.id ?? "") === launchDraft.intentId,
    );
  }
  const custom = app.querySelector<HTMLButtonElement>(".launch-sheet button[data-act='intent-custom']");
  if (custom) {
    custom.hidden = !launchDraft.custom;
    custom.classList.toggle("active", launchDraft.custom);
  }
}

function expectedOpening(form: RunLaunchForm, draft: LaunchDraft): string {
  const prefix = form.intents.find((intent) => intent.id === draft.intentId)?.prefix ?? "";
  const body = (draft.values["initial-instruction"] ?? "").trim();
  const notes = (form.changeNotesText ?? "").trim();
  const core = prefix && body ? `${prefix}\n${body}` : prefix || body;
  if (core && notes) return `${core}\n\n${notes}`;
  return core || notes;
}

async function openExternalUrl(url: string): Promise<void> {
  if (desktopShellAvailable()) {
    await openUrl(url);
    return;
  }
  window.open(url, "_blank", "noopener,noreferrer");
}

function isLoopbackPage(): boolean {
  const { hostname, port } = window.location;
  return (
    (hostname === "127.0.0.1" || hostname === "localhost" || hostname === "[::1]") &&
    port === "10529"
  );
}

function desktopShellAvailable(): boolean {
  return isTauri() || "__TAURI_INTERNALS__" in window;
}

function focusedHostIsLocal(): boolean {
  const snap = snapshot;
  if (!snap) return false;
  const focused = snap.hosts.find((host) => host.id === snap.focusedHostId);
  if (focused) return focused.local;
  return snap.hosts.some((host) => host.local) && snap.hosts.every((host) => host.local);
}

function directoryName(path: string): string {
  const trimmed = path.replace(/[\\/]+$/, "");
  const parts = trimmed.split(/[\\/]/).filter(Boolean);
  return parts.length ? parts[parts.length - 1] : "";
}

function supersedeProjectInference(): void {
  projectInference = { status: "idle", requestId: projectInference.requestId + 1 };
}

function applyLocalPath(path: string, prefillName: boolean): void {
  const nextPath = path.trim();
  const nextName = directoryName(nextPath);
  const shouldPrefillName =
    prefillName &&
    Boolean(nextName) &&
    (!formDraft.name.trim() || formDraft.name.trim() === autoFilledProjectName);
  formDraft = {
    ...formDraft,
    localPath: path,
    name: shouldPrefillName ? nextName : formDraft.name,
  };
  if (shouldPrefillName) autoFilledProjectName = nextName;
  supersedeProjectInference();
  formError = "";
  void inferFromLocalPath(nextPath);
}

async function inferFromLocalPath(path: string): Promise<void> {
  const requestedPath = path.trim();
  if (!requestedPath || !focusedHostIsLocal()) return;
  const requestId = projectInference.requestId + 1;
  projectInference = { status: "pending", requestId };
  render();
  try {
    const result = await rpc("inferProject", { localPath: requestedPath });
    if (requestId !== projectInference.requestId || formDraft.localPath.trim() !== requestedPath) return;
    if (result.inference?.tracker === "local-markdown") {
      const candidate = result.inference;
      const useCandidateName = !formDraft.name.trim() || formDraft.name.trim() === autoFilledProjectName;
      formDraft = {
        ...formDraft,
        name: useCandidateName ? candidate.name : formDraft.name,
        localPath: candidate.localPath,
        githubHost: candidate.githubHost,
        repository: candidate.repository,
        tracker: candidate.tracker,
      };
      autoFilledProjectName = useCandidateName ? candidate.name : autoFilledProjectName;
      projectInference = { status: "idle", requestId };
    } else {
      projectInference = result.inference
        ? { status: "candidate", requestId, candidate: result.inference }
        : { status: "failed", requestId, message: snapshot?.copy.inferenceFailed ?? "" };
    }
  } catch (error) {
    if (requestId !== projectInference.requestId || formDraft.localPath.trim() !== requestedPath) return;
    projectInference = {
      status: "failed",
      requestId,
      message: error instanceof Error ? error.message : String(error),
    };
  }
  render();
}

async function chooseProjectDirectory(): Promise<void> {
  if (!focusedHostIsLocal()) return;
  formError = "";
  if (!desktopShellAvailable()) {
    formError = snapshot?.copy.chooseDirectoryDesktopOnly ?? "";
    render();
    return;
  }
  try {
    const selected = await openDirectory({
      directory: true,
      multiple: false,
      title: snapshot?.copy.localDirectory,
      defaultPath: formDraft.localPath.trim() || undefined,
      canCreateDirectories: false,
    });
    if (typeof selected !== "string" || !selected) return;
    applyLocalPath(selected, true);
  } catch (error) {
    formError = error instanceof Error ? error.message : String(error);
    render();
  }
}

async function loadStartupSettings(): Promise<void> {
  startupSettingsError = "";
  if (!desktopShellAvailable()) {
    startAtLogin = null;
    return;
  }
  try {
    startAtLogin = await isEnabled();
  } catch (error) {
    startupSettingsError = error instanceof Error ? error.message : String(error);
  }
}

async function setHostMode(mode: Snapshot["hostMode"]): Promise<void> {
  startupSettingsError = "";
  try {
    if (mode === "client-only") {
      const gate = await readUpdateInstallGate("updateInstallGate");
      if (!gate.allowed) {
        startupSettingsError = startupCopy(snapshot?.appearance.language ?? "en").hostModeActiveRuns;
        return;
      }
    }
    await invoke("set_host_mode", { mode });
    await relaunch();
  } catch (error) {
    startupSettingsError = error instanceof Error ? error.message : String(error);
  }
}

async function setStartAtLogin(enabled: boolean): Promise<void> {
  startupSettingsError = "";
  try {
    if (enabled) await enable();
    else await disable();
    startAtLogin = await isEnabled();
  } catch (error) {
    startupSettingsError = error instanceof Error ? error.message : String(error);
  }
}

async function checkForUpdates(manual: boolean): Promise<void> {
  if (!desktopShellAvailable()) {
    updateState = { kind: "failed", message: snapshot?.copy.updateUnavailableBrowser ?? "" };
    render();
    return;
  }
  if (updateState.kind === "checking" || updateState.kind === "installing") return;
  updateState = { kind: "checking", manual };
  render();
  try {
    pendingUpdate?.close().catch(() => {});
    pendingUpdate = await check({ timeout: 30_000 });
    pendingUpdateDownloaded = false;
    updateState = pendingUpdate
      ? {
          kind: "available",
          version: pendingUpdate.version,
          notes: pendingUpdate.body ?? "",
        }
      : { kind: "current" };
  } catch (error) {
    if (manual) {
      updateState = {
        kind: "failed",
        message: error instanceof Error ? error.message : String(error),
      };
    } else {
      updateState = { kind: "idle" };
    }
  }
  render();
}

async function readUpdateInstallGate(op = "updateInstallGate"): Promise<UpdateInstallGate> {
  const result = await rpc(op);
  if (!result.updateInstallGate) throw new Error("Host returned no update install gate");
  return result.updateInstallGate;
}

async function installPendingUpdate(): Promise<void> {
  if (!pendingUpdate || updateState.kind === "installing") return;
  let gate: UpdateInstallGate;
  try {
    gate = await readUpdateInstallGate();
  } catch (error) {
    updateState = {
      kind: "failed",
      message: error instanceof Error ? error.message : String(error),
    };
    render();
    return;
  }
  if (!gate.allowed) {
    updateState = { kind: "blocked", activeRunCount: gate.activeRunCount };
    render();
    return;
  }
  updateState = { kind: "installing", progress: null };
  render();
  let downloaded = 0;
  let contentLength: number | undefined;
  try {
    if (!pendingUpdateDownloaded) {
      await pendingUpdate.download((event) => {
        if (event.event === "Started") {
          contentLength = event.data.contentLength;
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
        }
        const progress = contentLength && contentLength > 0
          ? Math.min(100, Math.round((downloaded / contentLength) * 100))
          : null;
        updateState = { kind: "installing", progress };
        render();
      });
      pendingUpdateDownloaded = true;
    }
    const finalGate = await readUpdateInstallGate("beginUpdateInstall");
    if (!finalGate.allowed) {
      updateState = { kind: "blocked", activeRunCount: finalGate.activeRunCount };
      render();
      return;
    }
    await pendingUpdate.install();
    await relaunch();
  } catch (error) {
    await rpc("cancelUpdateInstall").catch(() => {});
    updateState = {
      kind: "failed",
      message: error instanceof Error ? error.message : String(error),
    };
    render();
  }
}

async function protocolBase(): Promise<string> {
  if (window.__HOST_PROTOCOL__) {
    return window.__HOST_PROTOCOL__;
  }
  if (isLoopbackPage()) {
    return "";
  }
  for (let i = 0; i < 50; i += 1) {
    if (window.__HOST_PROTOCOL__) {
      return window.__HOST_PROTOCOL__;
    }
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  throw new Error("Host protocol is not available");
}

let rpcQueue: Promise<void> = Promise.resolve();

async function rpc(op: string, extra: Record<string, unknown> = {}): Promise<RpcResult> {
  const requestClientViewRevision = clientViewRevision;
  const requestClientView: ClientViewState = {
    ...clientView,
    search: { ...clientView.search },
  };
  const request = rpcQueue.then(async () => {
    const response = await fetch(`${await protocolBase()}/rpc`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ op, ...extra, clientId, clientView: requestClientView }),
    });
    if (!response.ok) {
      const text = await response.text();
      let message = text || `Host protocol ${response.status}`;
      try {
        const parsed = JSON.parse(text) as { error?: string; message?: string };
        message = parsed.message || parsed.error || message;
      } catch {
        // keep raw body
      }
      throw new Error(message);
    }
    const result = (await response.json()) as RpcResult;
    result.snapshot.runs = result.snapshot.runs ?? [];
    result.snapshot.focusedRunId = result.snapshot.focusedRunId ?? "";
    result.snapshot.workspaceView = result.snapshot.workspaceView ?? "project";
    result.snapshot.showCommandPreview = result.snapshot.showCommandPreview ?? true;
    result.snapshot.notifyDesktop = result.snapshot.notifyDesktop ?? true;
    result.snapshot.notifySound = result.snapshot.notifySound ?? true;
    result.snapshot.usageOpen = result.snapshot.usageOpen ?? false;
    result.snapshot.refreshIntervalMs = result.snapshot.refreshIntervalMs ?? 300_000;
    result.events = result.events ?? [];
    deliverHostEvents(result.events, result.snapshot);
    if (requestClientViewRevision === clientViewRevision) {
      reconcileClientView(result.snapshot);
      syncLaunchDraft(result.snapshot);
      snapshot = result.snapshot;
      if (result.viewChanges) {
        changesView = result.viewChanges;
        changesOpen = true;
      }
    }
    return result;
  });
  rpcQueue = request.then(() => undefined, () => undefined);
  return request;
}

async function navigateClient(
  patch: Partial<ClientViewState>,
  request: Record<string, unknown> = {},
): Promise<void> {
  updateClientView(patch);
  await rpc("snapshot", request);
}

async function refreshProject(projectId = clientView.focusedProjectId): Promise<void> {
  let result = await rpc("refresh", projectId ? { projectId } : {});
  for (let attempt = 0; attempt < 200; attempt += 1) {
    const board = result.snapshot.board;
    if (board?.projectId !== projectId || board.refresh.kind !== "refreshing") return;
    await new Promise((resolve) => window.setTimeout(resolve, 50));
    result = await rpc("snapshot");
  }
}

async function loadViewChanges(runId: string, scope: ChangeScope): Promise<void> {
  const result = await rpc("viewChanges", { runId, scope });
  changesView = result.viewChanges ?? null;
}

async function loadSelectedIssueDocument(force = false): Promise<void> {
  const issue = snapshot?.board?.selected;
  if (!issue) return;
  const state = issue.document ?? { kind: "unloaded" as const };
  if (!force && state.kind !== "unloaded" && state.kind !== "loading") return;
  issue.document = state.kind === "ready" || state.kind === "stale" || state.kind === "loading"
    ? { kind: "loading", body: state.body, fetchedAtMs: state.fetchedAtMs }
    : { kind: "loading" };
  render();
  const issueId = issue.id;
  let result = await rpc("loadIssueDocument", { issueId });
  for (let attempt = 0; attempt < 200; attempt += 1) {
    const selected = result.snapshot.board?.selected;
    if (selected?.id !== issueId || selected.document?.kind !== "loading") return;
    await new Promise((resolve) => window.setTimeout(resolve, 50));
    result = await rpc("snapshot");
  }
}

function notificationTitle(copy: ShellCopy, kind: NotificationKind): string {
  if (kind === "waiting") return copy.notifyWaiting;
  if (kind === "completed") return copy.notifyCompleted;
  if (kind === "abnormal-stop") return copy.notifyAbnormal;
  return copy.notifyCrash;
}

function playNotifySound(): void {
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

async function jumpToNotification(event: Extract<HostEvent, { type: "notification" }>): Promise<void> {
  await rpc("showWindow");
  issueDetailVisible = Boolean(event.issueId) || issueDetailVisible;
  await navigateClient({
    focusedProjectId: event.projectId || clientView.focusedProjectId,
    selectedIssueId: event.issueId ?? clientView.selectedIssueId,
    focusedRunId: event.runId || "",
    workspaceView: event.runId ? "run" : "project",
  });
  render();
}

function deliverHostEvents(events: HostEvent[], snap: Snapshot): void {
  if (mobileClient()) return;
  for (const event of events) {
    if (event.type !== "notification") continue;
    const title = notificationTitle(snap.copy, event.kind);
    const body = event.issueId || event.runId;
    if (snap.notifyDesktop && typeof Notification !== "undefined") {
      const show = () => {
        const note = new Notification(title, { body, tag: event.runId });
        note.onclick = () => {
          void jumpToNotification(event);
        };
      };
      if (Notification.permission === "granted") {
        show();
      } else if (Notification.permission === "default") {
        void Notification.requestPermission().then((permission) => {
          if (permission === "granted") show();
        });
      }
    }
    if (snap.notifySound) {
      playNotifySound();
    }
  }
}

function emptyActionAct(action: Snapshot["emptyActions"][number]): string {
  return action === "register-first-project" ? "register" : "pair";
}

function emptyActionLabel(copy: ShellCopy, action: Snapshot["emptyActions"][number]): string {
  return action === "register-first-project"
    ? copy.registerFirstProject
    : copy.pairAnotherHost;
}

function themeLabel(copy: ShellCopy, theme: Theme): string {
  if (theme === "warm-paper") return copy.themeWarmPaper;
  if (theme === "plain-paper") return copy.themePlainPaper;
  return copy.themePlainNight;
}

function clientCopy(language: Language, fallback: ShellCopy): ShellCopy {
  return snapshot?.copyCatalog?.[language] ?? fallback;
}

function languageLabel(copy: ShellCopy, language: Language): string {
  return language === "zh-CN" ? copy.languageZh : copy.languageEn;
}

function captureActiveField(): {
  selector: string;
  start: number | null;
  end: number | null;
  direction: "forward" | "backward" | "none" | null;
  scrollLeft: number;
  scrollTop: number;
} | null {
  const active = document.activeElement;
  if (
    !(active instanceof HTMLInputElement)
    && !(active instanceof HTMLTextAreaElement)
    && !(active instanceof HTMLSelectElement)
  ) return null;
  if (!app?.contains(active)) return null;
  let selector = active.id ? `#${CSS.escape(active.id)}` : "";
  if (!selector) {
    const name = active.getAttribute("name");
    const form = active.closest<HTMLFormElement>("form[data-act]");
    const action = form?.dataset.act;
    if (name && action) {
      const formId = form.dataset.id;
      selector = `form[data-act="${CSS.escape(action)}"]${
        formId ? `[data-id="${CSS.escape(formId)}"]` : ""
      } [name="${CSS.escape(name)}"]`;
    }
  }
  for (const attribute of ["data-field", "data-usage-filter"] as const) {
    if (selector) break;
    const value = active.getAttribute(attribute);
    if (value) {
      selector = `${active.tagName.toLowerCase()}[${attribute}="${CSS.escape(value)}"]`;
    }
  }
  if (!selector) return null;
  const textControl = active instanceof HTMLTextAreaElement
    || (active instanceof HTMLInputElement
      && ["text", "search", "url", "tel", "password"].includes(active.type));
  return {
    selector,
    start: textControl ? active.selectionStart : null,
    end: textControl ? active.selectionEnd : null,
    direction: textControl ? active.selectionDirection : null,
    scrollLeft: active.scrollLeft,
    scrollTop: active.scrollTop,
  };
}

function restoreActiveField(field: {
  selector: string;
  start: number | null;
  end: number | null;
  direction: "forward" | "backward" | "none" | null;
  scrollLeft: number;
  scrollTop: number;
} | null): void {
  if (!field) return;
  const next = app?.querySelector<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>(
    field.selector,
  );
  if (!next) return;
  next.focus();
  if (
    field.start != null
    && field.end != null
    && (next instanceof HTMLInputElement || next instanceof HTMLTextAreaElement)
  ) {
    next.setSelectionRange(field.start, field.end, field.direction ?? "none");
  }
  next.scrollLeft = field.scrollLeft;
  next.scrollTop = field.scrollTop;
}

function dependencyGraphRenderKey(board: BoardSnapshot | null | undefined): string {
  if (!board?.graph) return "";
  return JSON.stringify([board.graph, graphCanvasLimit]);
}

function snapshotRenderKey(snap: Snapshot): string {
  return JSON.stringify(snap, (key, value) => {
    if (key === "nextRefreshInMs" || key === "remainingMs") return undefined;
    return value;
  });
}

function completeDependencyGraphLabel(copy: ShellCopy, graph: DependencyGraph): string {
  if (typeof graph.closedCount === "number") {
    return copy.showClosedContext.replace("{count}", String(graph.closedCount));
  }
  return copy.showClosedContext
    .replace(/\s*（[^）]*\{count\}[^）]*）/, "")
    .replace(/\s*\([^)]*\{count\}[^)]*\)/, "");
}

function render(): void {
  if (!snapshot || !app) return;
  const snap = snapshot;
  const isMobile = mobileClient();
  const activeField = captureActiveField();
  const appearance = isMobile
    ? { ...snap.appearance, ...ensureMobileAppearance() }
    : snap.appearance;
  const copy = isMobile && appearance.language !== snap.appearance.language
    ? clientCopy(appearance.language, snap.copy)
    : snap.copy;
  const { hosts, projects } = snap;
  const project = currentProject(snap);
  document.documentElement.lang = appearance.language === "zh-CN" ? "zh-CN" : "en";
  document.documentElement.dataset.theme = appearance.theme;
  document.documentElement.dataset.mobile = isMobile ? "true" : "false";
  document.title = copy.appName;

  const host = hosts.find((item) => item.id === snapshot?.focusedHostId) ?? hosts[0];
  const empty = snapshot.emptyActions.length > 0;
  const runLifted = !isMobile && snap.workspaceView === "run" && Boolean(focusedRun(snap));
  const showSidebar = !isMobile && sidebarVisible && !runLifted;
  const selectedIssue = snap.board?.selected;
  const previousDetailScrollNode = app.querySelector<HTMLElement>(".detail-scroll");
  const previousDetailScroll = previousDetailScrollNode
    ? {
        issueId: renderedDetailIssueId,
        scrollTop: previousDetailScrollNode.scrollTop,
        scrollLeft: previousDetailScrollNode.scrollLeft,
      }
    : null;
  const previousLaunchSheet = app.querySelector<HTMLElement>(".launch-sheet");
  const previousLaunchScroll = previousLaunchSheet
    ? {
        key: snapshot.launchForm ? `${snapshot.launchForm.projectId}:${snapshot.launchForm.selectedAgentId}` : "",
        scrollTop: previousLaunchSheet.scrollTop,
        scrollLeft: previousLaunchSheet.scrollLeft,
      }
    : null;
  const inspectorOpen = issueDetailVisible && Boolean(selectedIssue);
  const showIssueToggle = !isMobile && Boolean(selectedIssue) && (snap.workspaceView === "project" || runLifted);
  const previousGraphToolbar = app.querySelector<HTMLElement>(".graph-toolbar");
  const previousGraphCanvas = app.querySelector<HTMLElement>(".graph-canvas");
  const previousGraph = previousGraphCanvas
    ? {
        canvas: previousGraphCanvas,
        projectId: renderedGraphProjectId,
        centerId: renderedGraphCenterId,
        renderKey: renderedGraphKey,
        scrollLeft: previousGraphCanvas.scrollLeft,
        scrollTop: previousGraphCanvas.scrollTop,
        clientWidth: previousGraphCanvas.clientWidth,
        clientHeight: previousGraphCanvas.clientHeight,
        scrollWidth: previousGraphCanvas.scrollWidth,
        scrollHeight: previousGraphCanvas.scrollHeight,
      }
    : null;
  const desktopProjectGraph =
    !isMobile &&
    !empty &&
    !snap.usageOpen &&
    snap.workspaceView === "project" &&
    !runLifted &&
    snap.centerView === "graph" &&
    Boolean(snap.board?.graph);
  const nextGraphKey = desktopProjectGraph ? dependencyGraphRenderKey(snap.board) : "";
  const nextGraphCenterId = desktopProjectGraph ? snap.board?.graph?.centerId ?? "" : "";
  const graphContentChanged = Boolean(previousGraph && previousGraph.renderKey !== nextGraphKey);
  const reuseGraphCanvas = Boolean(
    previousGraph &&
      previousGraph.projectId === snap.focusedProjectId &&
      previousGraph.renderKey === nextGraphKey,
  );
  if (!pairingAddress) {
    pairingAddress = (snapshot.loopbackPage.url || "http://127.0.0.1:10529/").replace(/\/$/, "");
  }

  app.innerHTML = `
    <div class="frame">
      <header class="chrome ${showSidebar ? "with-side" : "side-hidden"}">
        <div class="chrome-lead">
          ${isMobile
            ? `<button type="button" class="chrome-button" data-act="mobile-scope">${escapeHtml(copy.mobileSwitchScope)}</button>`
            : `<button type="button" class="chrome-icon" data-act="toggle-sidebar" aria-label="${escapeHtml(showSidebar ? copy.hideSidebar : copy.showSidebar)}" title="${escapeHtml(showSidebar ? copy.hideSidebar : copy.showSidebar)}">☰</button>
               ${showSidebar ? `<span class="chrome-app">${escapeHtml(copy.appName)}</span>` : ""}`}
        </div>
        <div class="chrome-main">
          <div class="chrome-primary">
            ${!isMobile && !empty && !snap.usageOpen && snap.workspaceView === "project" && !runLifted
              ? `<div class="view-switch" role="tablist">
                  <button type="button" class="${snap.centerView === "board" ? "active" : ""}" data-act="center-view" data-id="board">${escapeHtml(copy.viewBoard)}</button>
                  <button type="button" class="${snap.centerView === "graph" ? "active" : ""}" data-act="center-view" data-id="graph">${escapeHtml(copy.viewGraph)}</button>
                </div>`
              : ""}
            ${runLifted ? `<button type="button" class="chrome-button" data-act="return-board">← ${escapeHtml(copy.returnToBoard)}</button>` : ""}
            ${!isMobile && snap.workspaceView === "host-overview" ? `<span class="chrome-title">${escapeHtml(copy.hostOverview)}</span>` : ""}
            ${!isMobile && snap.usageOpen ? `<span class="chrome-title">${escapeHtml(copy.usage)}</span>` : ""}
            ${!isMobile && !showSidebar ? `<button type="button" class="chrome-button ${snap.workspaceView === "host-overview" ? "active" : ""}" data-act="open-overview">${escapeHtml(copy.hostOverview)}</button>` : ""}
          </div>
          ${!isMobile && project ? `<span class="chrome-context">${escapeHtml(host?.displayName ?? "")} · ${escapeHtml(project.name)}</span>` : ""}
          <div class="chrome-trail">
            ${showIssueToggle
              ? `<button type="button" class="chrome-icon ${inspectorOpen ? "active" : ""}" data-act="toggle-issue" aria-label="${escapeHtml(inspectorOpen ? copy.hideIssueDetail : copy.showIssueDetail)}" title="${escapeHtml(inspectorOpen ? copy.hideIssueDetail : copy.showIssueDetail)}">${issuePanelIcon(inspectorOpen)}</button>`
              : ""}
            <button type="button" class="chrome-button" data-act="settings">${escapeHtml(copy.settings)}</button>
            <button type="button" class="chrome-button ${appearance.theme !== "plain-night" ? "active" : ""}" data-act="shade" data-id="light">${escapeHtml(copy.shadeLight)}</button>
            <button type="button" class="chrome-button ${appearance.theme === "plain-night" ? "active" : ""}" data-act="shade" data-id="dark">${escapeHtml(copy.shadeDark)}</button>
          </div>
        </div>
      </header>
      <div class="body ${showSidebar ? "" : "side-collapsed"}">
        ${showSidebar ? `<aside class="side">
          <div>
            <div class="group-name">${escapeHtml(copy.hosts)}</div>
            ${
              host
                ? `<div class="host-line">
                    <button type="button" class="item active" data-act="toggle-hosts"><span class="dot"></span>${escapeHtml(host.displayName)}${host.local ? `<span class="tag">${escapeHtml(copy.thisMachine)}</span>` : ""}</button>
                    <button type="button" class="title-icon" data-act="pair" aria-label="${escapeHtml(copy.pairAnotherHost)}">⊕</button>
                  </div>
                  <button type="button" class="item ${snap.workspaceView === "host-overview" ? "active" : ""}" data-act="open-overview">${escapeHtml(copy.hostOverview)}</button>
                  <button type="button" class="item ${snap.usageOpen ? "active" : ""}" data-act="open-usage">${escapeHtml(copy.usage)}</button>`
                : ""
            }
            ${
              hostPickerOpen && hosts.length > 1
                ? `<div class="host-picker">${hosts
                    .map(
                      (item) =>
                        `<div class="host-picker-row">
                          <button type="button" class="item ${item.id === host?.id ? "active" : ""}" data-act="focus-host" data-id="${escapeHtml(item.id)}">${escapeHtml(item.displayName)}${item.local ? `<span class="tag">${escapeHtml(copy.thisMachine)}</span>` : ""}</button>
                          ${item.local ? "" : `<button type="button" class="host-forget" data-act="forget-host" data-id="${escapeHtml(item.id)}" aria-label="${escapeHtml(copy.forgetHost)}" title="${escapeHtml(copy.forgetHost)}">×</button>`}
                        </div>`,
                    )
                    .join("")}</div>`
                : ""
            }
          </div>
          <div>
            <div class="group-head">
              <div class="group-name">${escapeHtml(copy.projects)}</div>
              <button type="button" class="title-icon" data-act="register" aria-label="${escapeHtml(copy.addProject)}">＋</button>
            </div>
            ${
              projects.length
                ? projects
                    .map((project) => projectBlock(copy, snap, project, snap.focusedProjectId))
                    .join("")
                : `<div class="nested">${escapeHtml(copy.noProjectTitle)}</div>`
            }
          </div>
        </aside>` : ""}
        <main class="workspace ${empty ? "" : "board-open"}${!snap.usageOpen && snap.workspaceView === "project" && focusedRun(snap) ? " has-run" : ""}">
          ${
            empty
              ? `<div class="empty">
                  ${loopbackNotice(snap.loopbackPage)}
                  <h1>${escapeHtml(copy.noProjectTitle)}</h1>
                  <p>${escapeHtml(copy.noProjectBody)}</p>
                  <div class="actions">
                    ${snap.emptyActions
                      .map(
                        (action, index) =>
                          `<button type="button" class="${index === 0 ? "primary" : ""}" data-act="${emptyActionAct(action)}">${escapeHtml(emptyActionLabel(copy, action))}</button>`,
                      )
                      .join("")}
                  </div>
                </div>`
              : snap.usageOpen
                ? usagePage(copy, snap)
                : isMobile
                  ? mobileMain(copy, snap)
                  : snap.workspaceView === "host-overview"
                    ? hostOverviewPage(copy, snap)
                    : runLifted
                      ? liftedRunView(copy, snap)
                      : `${projectMain(copy, snap, reuseGraphCanvas)}${runDock(copy, snap)}`
          }
        </main>
      </div>
      ${isMobile && !empty && !snap.usageOpen ? mobileNavigation(copy, snap) : ""}
    </div>
    ${isMobile && mobileScopeOpen ? mobileScopeSheet(copy, snap) : ""}
    ${
      settingsOpen
        ? `<div class="overlay" data-act="close-settings">
            <div class="sheet" data-stop="true">
              <h2>${escapeHtml(copy.settings)}</h2>
              <div class="field">
                <div class="label">${escapeHtml(copy.language)}</div>
                <div class="choices">
                  ${appearance.languages
                    .map(
                      (language) =>
                        `<button type="button" class="${appearance.language === language ? "active" : ""}" data-act="language" data-id="${language}">${escapeHtml(languageLabel(copy, language))}</button>`,
                    )
                    .join("")}
                </div>
              </div>
              <div class="field">
                <div class="label">${escapeHtml(copy.theme)}</div>
                <div class="choices">
                  ${appearance.themes
                    .map(
                      (theme) =>
                        `<button type="button" class="${appearance.theme === theme ? "active" : ""}" data-act="theme" data-id="${theme}">${escapeHtml(themeLabel(copy, theme))}</button>`,
                    )
                    .join("")}
                </div>
              </div>
              ${startupSettings(startupCopy(appearance.language), snap)}
              <div class="field">
                <button type="button" data-act="refresh-launch-environment" ${snap.hostMode === "client-only" ? "disabled" : ""}>${escapeHtml(startupCopy(appearance.language).rereadLaunchEnvironment)}</button>
                ${launchEnvironmentStatus(startupCopy(appearance.language))}
              </div>
              ${updateSettings(copy)}
              <div class="field">
                <label class="label" for="refresh-interval">${escapeHtml(copy.refreshInterval)}</label>
                <input id="refresh-interval" type="number" min="15" step="15" data-field="refreshInterval" value="${Math.round((snap.refreshIntervalMs ?? 300_000) / 1000)}" />
                <p class="hint">${escapeHtml(copy.refreshIntervalHelp)}</p>
              </div>
              <div class="field">
                <label class="label" for="recent-limit">${escapeHtml(copy.recentLimit)}</label>
                <input id="recent-limit" type="number" min="1" max="50" data-field="recentLimit" value="${snap.recentCompletedLimit}" />
                <p class="hint">${escapeHtml(copy.recentLimitHelp)}</p>
              </div>
              <label class="graph-opt">
                <input type="checkbox" data-field="commandPreview" ${snap.showCommandPreview ? "checked" : ""} />
                ${escapeHtml(copy.showCommandPreview)}
              </label>
              ${isMobile ? "" : `<label class="graph-opt">
                <input type="checkbox" data-field="notifyDesktop" ${snap.notifyDesktop ? "checked" : ""} />
                ${escapeHtml(copy.notifyDesktop)}
              </label>
              <label class="graph-opt">
                <input type="checkbox" data-field="notifySound" ${snap.notifySound ? "checked" : ""} />
                ${escapeHtml(copy.notifySound)}
              </label>
              <label class="graph-opt">
                <input type="checkbox" data-field="hostAutoAdvance" ${snap.autoAdvance ? "checked" : ""} />
                ${escapeHtml(copy.autoAdvance)}
              </label>
              <p class="hint">${escapeHtml(copy.autoAdvanceHelp)}</p>
              ${
                currentProject(snap)
                  ? `<label class="graph-opt">
                <input type="checkbox" data-field="projectAutoAdvance" ${currentProject(snap)?.autoAdvance ? "checked" : ""} />
                ${escapeHtml(copy.projectAutoAdvance)}
              </label>
              <label class="graph-opt">
                <input type="checkbox" data-field="restoreAutoAdvance" ${currentProject(snap)?.restoreAutoAdvance ? "checked" : ""} />
                ${escapeHtml(copy.restoreAutoAdvance)}
              </label>
              <div class="field">
                <label class="label" for="restore-delay">${escapeHtml(copy.restoreDelay)}</label>
                <input id="restore-delay" type="number" min="0" max="600" data-field="restoreDelay" value="${Math.round((currentProject(snap)?.restoreDelayMs ?? 60000) / 1000)}" />
              </div>`
                  : ""
              }
              <button type="button" data-act="quit">${escapeHtml(copy.quitHost)}</button>`}
            </div>
          </div>`
        : ""
    }
    ${
      pairingOpen
        ? `<div class="overlay" data-act="close-pairing">
            <div class="sheet pairing-sheet" data-act="pairing-noop">
              <h2>${escapeHtml(copy.pairingTitle)}</h2>
              <p class="hint">${escapeHtml(copy.pairingSamePayload)}</p>
              <div class="field">
                <div class="label">${escapeHtml(copy.pairingThisHost)}</div>
                <label class="label" for="pairing-address">${escapeHtml(copy.pairingAddress)}</label>
                <input id="pairing-address" data-field="address" value="${escapeHtml(pairingAddress)}" />
                <div class="actions">
                  <button type="button" class="primary" data-act="show-offer" ${formOperations.pending.has(pairingFormKey("offer")) ? "disabled" : ""}>${escapeHtml(formOperations.pending.has(pairingFormKey("offer")) ? copy.operationPending : copy.pairingShow)}</button>
                </div>
                ${formFeedback(pairingFormKey("offer"))}
                ${
                  snapshot.pairingOffer
                    ? `<div class="offer">
                        <div class="qr">${snapshot.pairingOffer.qrSvg}</div>
                        <pre class="payload">${escapeHtml(snapshot.pairingOffer.text)}</pre>
                        <button type="button" data-act="copy-offer">${escapeHtml(copy.pairingCopy)}</button>
                      </div>`
                    : ""
                }
              </div>
              <div class="field">
                <div class="label">${escapeHtml(copy.pairedClients)}</div>
                ${
                  snapshot.pairedClients.length
                    ? snapshot.pairedClients
                        .map(
                          (client) =>
                            `<div class="client-row"><span>${escapeHtml(client.name)}</span><button type="button" data-act="revoke" data-id="${escapeHtml(client.id)}" ${formOperations.pending.has(pairingFormKey(`revoke:${client.id}`)) ? "disabled" : ""}>${escapeHtml(formOperations.pending.has(pairingFormKey(`revoke:${client.id}`)) ? copy.operationPending : copy.revokeClient)}</button>${formFeedback(pairingFormKey(`revoke:${client.id}`))}</div>`,
                        )
                        .join("")
                    : `<div class="nested">${escapeHtml(copy.noPairedClients)}</div>`
                }
              </div>
              <div class="field">
                <div class="label">${escapeHtml(copy.pairingToAnother)}</div>
                <textarea data-field="paste" rows="4" placeholder="${escapeHtml(copy.pairingPaste)}">${escapeHtml(pairingPaste)}</textarea>
                <div class="actions">
                  <button type="button" class="primary" data-act="connect-host" ${formOperations.pending.has(pairingFormKey("connect")) ? "disabled" : ""}>${escapeHtml(formOperations.pending.has(pairingFormKey("connect")) ? copy.operationPending : copy.pairingConnect)}</button>
                </div>
                ${formFeedback(pairingFormKey("connect"))}
              </div>
              ${pairingError ? `<p class="notice">${escapeHtml(pairingError)}</p>` : ""}
            </div>
          </div>`
        : ""
    }
    ${formOpen ? projectForm(copy) : ""}
    ${snap.launchForm ? launchForm(copy, snap) : ""}
    ${removeProject ? removeDialog(copy, removeProject) : ""}
    ${forgetHostId ? forgetHostDialog(copy, hosts.find((item) => item.id === forgetHostId)) : ""}
    ${snap.quitOffer ? quitOfferDialog(copy) : ""}
    ${updateDialog(copy)}
    ${changesOpen ? viewChangesPanel(copy) : ""}
    ${keyboardHelpOpen ? keyboardHelpDialog(copy) : ""}
  `;
  const graphToolbar = app.querySelector<HTMLElement>(".graph-toolbar");
  if (
    previousGraphToolbar
    && graphToolbar
    && previousGraphToolbar.outerHTML === graphToolbar.outerHTML
  ) {
    graphToolbar.replaceWith(previousGraphToolbar);
  }
  const graphPlaceholder = app.querySelector<HTMLElement>("[data-preserve-graph-canvas]");
  if (reuseGraphCanvas && previousGraph && graphPlaceholder) {
    graphPlaceholder.replaceWith(previousGraph.canvas);
  }
  const graphCanvas = app.querySelector<HTMLElement>(".graph-canvas");
  const sameGraphCenter = Boolean(
    previousGraph &&
      previousGraph.projectId === snap.focusedProjectId &&
      previousGraph.centerId === nextGraphCenterId,
  );
  if (graphCanvas && sameGraphCenter && !graphContentChanged && previousGraph) {
    graphCanvas.scrollLeft = previousGraph.scrollLeft;
    graphCanvas.scrollTop = previousGraph.scrollTop;
  }
  renderedGraphKey = graphCanvas ? nextGraphKey : "";
  renderedGraphProjectId = graphCanvas ? snap.focusedProjectId : "";
  renderedGraphCenterId = graphCanvas ? nextGraphCenterId : "";
  const graphLayoutChanged = Boolean(
    graphCanvas &&
      previousGraph &&
      (graphCanvas.clientWidth !== previousGraph.clientWidth ||
        graphCanvas.clientHeight !== previousGraph.clientHeight ||
        graphCanvas.scrollWidth !== previousGraph.scrollWidth ||
        graphCanvas.scrollHeight !== previousGraph.scrollHeight),
  );
  if (!reuseGraphCanvas || graphLayoutChanged) {
    paintGraphEdges();
  }
  syncGraphSelection(graphCanvas, snap.board?.selected?.id);
  if (graphCanvas && (!sameGraphCenter || graphContentChanged)) {
    const restored = pendingGraphAnchor
      ? restoreGraphAnchor(graphCanvas, pendingGraphAnchor)
      : false;
    if (!restored) centerGraphViewport(graphCanvas, nextGraphCenterId);
    pendingGraphAnchor = null;
  }
  restoreActiveField(activeField);
  const nextDetailScroll = app.querySelector<HTMLElement>(".detail-scroll");
  if (nextDetailScroll && previousDetailScroll?.issueId === (selectedIssue?.id ?? "")) {
    nextDetailScroll.scrollTop = previousDetailScroll.scrollTop;
    nextDetailScroll.scrollLeft = previousDetailScroll.scrollLeft;
  }
  const nextLaunchSheet = app.querySelector<HTMLElement>(".launch-sheet");
  if (
    nextLaunchSheet
    && previousLaunchScroll
    && previousLaunchScroll.key === (snap.launchForm ? `${snap.launchForm.projectId}:${snap.launchForm.selectedAgentId}` : "")
  ) {
    nextLaunchSheet.scrollTop = previousLaunchScroll.scrollTop;
    nextLaunchSheet.scrollLeft = previousLaunchScroll.scrollLeft;
  }
  renderedDetailIssueId = selectedIssue?.id ?? "";
  renderedSnapshotKey = snapshotRenderKey(snap);
  if (isMobile && !mobileLiveTerminal) {
    ptyPumping = false;
    void pumpMobileOutput(snap);
  } else {
    mobilePtyPumping = false;
    attachTerminal(snap);
  }
  scheduleGraphBatch(snap);
}

function scheduleGraphBatch(snap: Snapshot): void {
  if (graphBatchTimer != null) return;
  const graph = snap.centerView === "graph" ? snap.board?.graph : null;
  if (!graph || graphCanvasLimit >= graph.nodes.length) return;
  const expectedProject = snap.focusedProjectId;
  const expectedCenter = graph.centerId ?? "";
  graphBatchTimer = window.setTimeout(() => {
    graphBatchTimer = undefined;
    if (
      snapshot?.focusedProjectId !== expectedProject
      || snapshot.centerView !== "graph"
      || (snapshot.board?.graph?.centerId ?? "") !== expectedCenter
    ) return;
    graphCanvasLimit = Math.min(graphCanvasLimit + 48, snapshot.board?.graph?.nodes.length ?? 0);
    renderGraphBatch(snapshot);
  }, 50);
}

function renderGraphBatch(snap: Snapshot): void {
  const board = snap.board;
  const graph = board?.graph;
  const currentGraph = app?.querySelector<HTMLElement>(".dep-graph");
  const currentCanvas = currentGraph?.querySelector<HTMLElement>(".graph-canvas");
  const currentFlow = currentCanvas?.querySelector<HTMLElement>(".graph-flow");
  if (!board || !graph || !currentGraph || !currentCanvas || !currentFlow) {
    render();
    return;
  }

  const template = document.createElement("template");
  template.innerHTML = dependencyGraphView(snap.copy, board, false).trim();
  const nextGraph = template.content.firstElementChild as HTMLElement | null;
  const nextFlow = nextGraph?.querySelector<HTMLElement>(".graph-flow");
  if (!nextGraph || !nextFlow) {
    render();
    return;
  }

  for (const nextColumn of [...nextFlow.querySelectorAll<HTMLElement>(":scope > .graph-col")]) {
    const rank = nextColumn.dataset.rank ?? "";
    let currentColumn = currentFlow.querySelector<HTMLElement>(
      `:scope > .graph-col[data-rank="${CSS.escape(rank)}"]`,
    );
    if (!currentColumn) {
      const numericRank = Number(rank);
      const before = [...currentFlow.querySelectorAll<HTMLElement>(":scope > .graph-col")]
        .find((column) => Number(column.dataset.rank) > numericRank);
      currentFlow.insertBefore(nextColumn, before ?? null);
      currentColumn = nextColumn;
    } else {
      for (const nextNode of [...nextColumn.querySelectorAll<HTMLElement>(":scope > .graph-node")]) {
        const id = nextNode.dataset.id ?? "";
        if (!currentColumn.querySelector(`:scope > .graph-node[data-id="${CSS.escape(id)}"]`)) {
          currentColumn.append(nextNode);
        }
      }
    }
  }

  const currentLimit = currentGraph.querySelector<HTMLElement>(
    ":scope > .graph-limit:not(.graph-truncated)",
  );
  const nextLimit = nextGraph.querySelector<HTMLElement>(
    ":scope > .graph-limit:not(.graph-truncated)",
  );
  if (nextLimit && currentLimit) {
    currentLimit.textContent = nextLimit.textContent;
  } else if (nextLimit) {
    currentCanvas.before(nextLimit);
  } else {
    currentLimit?.remove();
  }

  renderedGraphKey = dependencyGraphRenderKey(board);
  paintGraphEdges();
  syncGraphSelection(currentCanvas, board.selected?.id);
  scheduleGraphBatch(snap);
}

function paintGraphEdges(): void {
  const canvas = app?.querySelector<HTMLElement>(".graph-canvas");
  const svg = app?.querySelector<SVGSVGElement>(".graph-edges");
  const graph = snapshot?.board?.graph;
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

function syncGraphSelection(canvas: HTMLElement | null, selectedId: string | undefined): void {
  if (!canvas) return;
  for (const node of canvas.querySelectorAll<HTMLElement>(".graph-node")) {
    node.classList.toggle("sel", node.dataset.id === selectedId);
  }
}

function centerGraphViewport(canvas: HTMLElement, centerId: string): void {
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

function captureGraphAnchor(issueId: string): GraphViewportAnchor | null {
  const canvas = app?.querySelector<HTMLElement>(".graph-canvas");
  const node = canvas
    ? [...canvas.querySelectorAll<HTMLElement>(".graph-node")].find((item) => item.dataset.id === issueId)
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

function restoreGraphAnchor(canvas: HTMLElement, anchor: GraphViewportAnchor): boolean {
  const node = [...canvas.querySelectorAll<HTMLElement>(".graph-node")]
    .find((item) => item.dataset.id === anchor.issueId);
  if (!node) return false;
  const canvasRect = canvas.getBoundingClientRect();
  const nodeRect = node.getBoundingClientRect();
  const currentX = nodeRect.left - canvasRect.left + nodeRect.width / 2;
  const currentY = nodeRect.top - canvasRect.top + nodeRect.height / 2;
  canvas.scrollLeft = Math.max(0, Math.min(
    canvas.scrollWidth - canvas.clientWidth,
    canvas.scrollLeft + currentX - anchor.viewportX,
  ));
  canvas.scrollTop = Math.max(0, Math.min(
    canvas.scrollHeight - canvas.clientHeight,
    canvas.scrollTop + currentY - anchor.viewportY,
  ));
  return true;
}

function currentProject(snap: Snapshot): Project | undefined {
  return (
    snap.projects.find((project) => project.id === snap.focusedProjectId) ?? snap.projects[0]
  );
}

function projectTrackerLabel(project: Project): string {
  return project.tracker === "local-markdown"
    ? "Local Markdown"
    : `${project.githubHost}/${project.repository}`;
}

function mobileNavigation(copy: ShellCopy, snap: Snapshot): string {
  const run = focusedRun(snap);
  return `<nav class="mobile-nav" aria-label="${escapeHtml([copy.mobileBoard, copy.mobileIssue, copy.mobileRun].join(" / "))}">
    <button type="button" class="${mobileView === "board" ? "active" : ""}" data-act="mobile-board">${escapeHtml(copy.mobileBoard)}</button>
    <button type="button" class="${mobileView === "issue" ? "active" : ""}" data-act="mobile-issue">${escapeHtml(copy.mobileIssue)}</button>
    <button type="button" class="${mobileView === "run" ? "active" : ""}" data-act="mobile-run" ${run ? "" : "disabled"}>${escapeHtml(copy.mobileRun)}</button>
  </nav>`;
}

function mobileScopeSheet(copy: ShellCopy, snap: Snapshot): string {
  const hosts = snap.hosts
    .map((host) => `<button type="button" class="item ${host.id === snap.focusedHostId ? "active" : ""}" data-act="focus-host" data-id="${escapeHtml(host.id)}"><span class="dot"></span>${escapeHtml(host.displayName)}${host.local ? `<span class="tag">${escapeHtml(copy.thisMachine)}</span>` : ""}</button>`)
    .join("");
  const projects = snap.projects
    .map((project) => `<div class="mobile-scope-project ${project.id === snap.focusedProjectId ? "active" : ""}">
      <button type="button" class="project-main" data-act="focus-project" data-id="${escapeHtml(project.id)}"><b>${escapeHtml(project.name)}</b><span>${escapeHtml(projectTrackerLabel(project))}</span></button>
      <button type="button" data-act="edit-project" data-id="${escapeHtml(project.id)}">${escapeHtml(copy.editProject)}</button>
      <button type="button" class="danger" data-act="remove-project" data-id="${escapeHtml(project.id)}">${escapeHtml(copy.removeProject)}</button>
    </div>`)
    .join("");
  return `<div class="overlay modal" data-act="close-mobile-scope">
    <section class="sheet mobile-scope-sheet" data-act="form-noop">
      <h2>${escapeHtml(copy.mobileSwitchScope)}</h2>
      <div class="mobile-scope-hosts">${hosts}</div>
      <div class="mobile-scope-projects">${projects}</div>
      <div class="actions">
        <button type="button" data-act="register">${escapeHtml(copy.addProject)}</button>
        <button type="button" data-act="pair">${escapeHtml(copy.pairAnotherHost)}</button>
        <button type="button" data-act="open-usage">${escapeHtml(copy.usage)}</button>
      </div>
    </section>
  </div>`;
}

function mobileMain(copy: ShellCopy, snap: Snapshot): string {
  if (mobileView === "run") return mobileRunView(copy, snap);
  if (mobileView === "issue") {
    return `<section class="mobile-issue-view"><aside class="issue-detail">${snap.board ? issueDetail(copy, snap.board, false) : ""}</aside></section>`;
  }
  return `<section class="mobile-board-view">${projectMain(copy, snap)}</section>`;
}

function mobileRunView(copy: ShellCopy, snap: Snapshot): string {
  const run = focusedRun(snap);
  if (!run) {
    return `<section class="mobile-run-view"><p class="board-empty">${escapeHtml(copy.noItems)}</p></section>`;
  }
  const identity = runIdentity(copy, run);
  return `<section class="mobile-run-view">
    <header class="run-dock-hd">
      <div><b>${escapeHtml(run.agentName)}</b><span>${escapeHtml(identity)}</span></div>
      <div class="actions">
        <button type="button" class="mobile-usage-entry" data-act="open-usage-run" data-id="${escapeHtml(run.id)}">${escapeHtml(copy.usage)}</button>
        <button type="button" data-act="stop-run" data-id="${escapeHtml(run.id)}" ${run.status === "ended" ? "disabled" : ""}>${escapeHtml(copy.stopRun)}</button>
      </div>
    </header>
    ${telemetryBar(copy, run)}
    <section class="mobile-output-panel">
      <div class="lane-hd">${escapeHtml(copy.mobileRecentOutput)}</div>
      ${mobileLiveTerminal
        ? `<div class="pty-slot" data-run="${escapeHtml(run.id)}"></div>`
        : `<pre class="mobile-run-output" data-run="${escapeHtml(run.id)}">${escapeHtml(run.status === "ended" ? run.recentOutput ?? mobilePtyText.get(run.id) ?? "" : mobilePtyText.get(run.id) ?? run.recentOutput ?? "")}</pre>`}
    </section>
    ${run.status === "ended" ? "" : injectRunForm(copy, run.id)}
    ${mobileLiveTerminal ? "" : `<button type="button" class="ghost mobile-terminal-escape" data-act="mobile-live-terminal">${escapeHtml(copy.mobileLiveTerminal)}</button>`}
  </section>`;
}

function focusedRun(snap: Snapshot): RunSummary | undefined {
  return (snap.runs ?? []).find((run) => run.id === snap.focusedRunId);
}

function issueSearchFormKey(projectId: string): FormKey {
  return `issue-search:${projectId}`;
}

function editableIssueSearchDraft(): IssueSearchDraft {
  if (issueSearchDraft) return issueSearchDraft;
  const search = snapshot?.board?.search;
  issueSearchDraft = {
    title: search?.title ?? "",
    triageRole: search?.triageRole ?? "",
    state: search?.state ?? "all",
  };
  return issueSearchDraft;
}

function injectFormKey(runId: string): FormKey {
  return `inject-run:${runId}`;
}

function changeNoteFormKey(runId: string): FormKey {
  return `change-note:${runId}`;
}

function usageCustomFormKey(hostId: string): FormKey {
  return `usage-custom:${hostId}`;
}

function launchFormKey(projectId: string): FormKey {
  return `launch:${projectId}`;
}

function pairingFormKey(action: string): FormKey {
  return `pairing:${action}`;
}

function formFeedback(key: FormKey): string {
  const error = formOperations.errors.get(key);
  return error ? `<p class="notice bad form-feedback">${escapeHtml(error)}</p>` : "";
}

function clearFormOperation(key: FormKey): void {
  formOperations.errors.delete(key);
}

async function runFormOperation(key: FormKey, operation: () => Promise<void>): Promise<boolean> {
  if (formOperations.pending.has(key)) return false;
  formOperations.pending.add(key);
  formOperations.errors.delete(key);
  render();
  try {
    await operation();
    return true;
  } catch (error) {
    formOperations.errors.set(key, error instanceof Error ? error.message : String(error));
    return false;
  } finally {
    formOperations.pending.delete(key);
    render();
  }
}

function injectRunForm(copy: ShellCopy, runId: string): string {
  const key = injectFormKey(runId);
  const pending = formOperations.pending.has(key);
  return `<form class="inject-row" data-act="inject-run" data-id="${escapeHtml(runId)}" aria-busy="${pending ? "true" : "false"}">
    <input name="text" maxlength="4000" required value="${escapeHtml(injectDrafts.get(runId) ?? "")}" placeholder="${escapeHtml(copy.injectPlaceholder)}" ${pending ? "disabled" : ""} />
    <button type="submit" ${pending ? "disabled" : ""}>${escapeHtml(pending ? copy.operationPending : copy.injectLine)}</button>
  </form>${formFeedback(key)}`;
}

function projectBlock(copy: ShellCopy, snap: Snapshot, project: Project, focusedId: string): string {
  const runs = (snap.runs ?? []).filter((run) => run.projectId === project.id);
  return `<div class="project-block">
    ${projectRow(copy, project, focusedId)}
    ${runs.map((run) => runRow(copy, run, snap.focusedRunId)).join("")}
  </div>`;
}

function projectRow(copy: ShellCopy, project: Project, focusedId: string): string {
  const active = project.id === focusedId;
  const degraded = project.connection.status !== "ready";
  return `<div class="project-row ${active ? "active" : ""}">
    <button type="button" class="project-main" data-act="focus-project" data-id="${escapeHtml(project.id)}">
      <b>${escapeHtml(project.name)}</b>
      <span>${escapeHtml(projectTrackerLabel(project))}</span>
    </button>
    ${degraded ? `<span class="dot warn" title="${escapeHtml(project.connection.status === "unreachable" ? copy.connectionUnavailable : copy.authFailed)}"></span>` : ""}
    <button type="button" class="title-icon" data-act="new-run" data-id="${escapeHtml(project.id)}" aria-label="${escapeHtml(copy.newRun)}">＋</button>
    <button type="button" class="more" data-act="project-menu" data-id="${escapeHtml(project.id)}" aria-label="${escapeHtml(copy.projectMenu)} ${escapeHtml(project.name)}">…</button>
    ${
      projectMenuId === project.id
        ? `<div class="project-menu">
            <button type="button" data-act="edit-project" data-id="${escapeHtml(project.id)}">${escapeHtml(copy.editProject)}</button>
            <button type="button" class="danger" data-act="remove-project" data-id="${escapeHtml(project.id)}">${escapeHtml(copy.removeProject)}</button>
          </div>`
        : ""
    }
  </div>`;
}

function runIdentity(copy: ShellCopy, run: RunSummary): string {
  return run.unbound || !run.issueId ? copy.unboundIssue : run.issueId;
}

function runRow(copy: ShellCopy, run: RunSummary, focusedId: string): string {
  const identity = runIdentity(copy, run);
  const action = run.recentAction?.trim() ? escapeHtml(run.recentAction) : "";
  const stateClass =
    run.waitingForUser && run.status !== "ended"
      ? "waiting"
      : run.endedReason && run.endedReason !== "exited"
        ? "execution-stopped"
        : run.status;
  const stateTag =
    run.waitingForUser && run.status !== "ended"
      ? copy.waiting
      : run.endedReason && run.endedReason !== "exited"
        ? copy.executionStopped
        : run.status === "running"
          ? copy.running
          : "";
  return `<button type="button" class="run-row ${run.id === focusedId ? "active" : ""} ${escapeHtml(stateClass)}" data-act="focus-run" data-id="${escapeHtml(run.id)}">
    <b>${escapeHtml(run.agentName)}</b>
    <span>${escapeHtml(identity)}</span>
    ${stateTag ? `<span class="run-state">${escapeHtml(stateTag)}</span>` : ""}
    ${action ? `<span class="run-action">${action}</span>` : ""}
    ${run.failure ? `<span class="run-fail">${escapeHtml(run.failure)}</span>` : ""}
    ${run.isolationNote ? `<span class="run-action">${escapeHtml(run.isolationNote)}</span>` : ""}
  </button>`;
}

function dash(value?: number | null): string {
  return value == null ? "—" : String(value);
}

function laneLabel(copy: ShellCopy, lane: TelemetryLaneKind): string {
  if (lane === "subagent") return copy.laneSubagent;
  if (lane === "switched") return copy.laneSwitched;
  return copy.laneMain;
}

function tokenCells(copy: ShellCopy, tokens: TokenCounts): string {
  const cells: Array<[string, number | null | undefined]> = [
    [copy.tokenInput, tokens.input],
    [copy.tokenOutput, tokens.output],
    [copy.tokenCacheRead, tokens.cacheRead],
    [copy.tokenCacheWrite, tokens.cacheWrite],
    [copy.tokenReasoning, tokens.reasoning],
    [copy.tokenTotal, tokens.total],
  ];
  return cells
    .map(([label, value]) => `<span class="token-cell"><i>${escapeHtml(label)}</i>${dash(value)}</span>`)
    .join("");
}

function sparkline(points: TelemetryPoint[], field: "ttftMs" | "tokensPerSec"): string {
  const values = points.map((point) => point[field] ?? 0);
  const max = Math.max(...values, 1);
  return `<span class="spark">${points
    .map((point) => {
      const value = point[field] ?? 0;
      const height = Math.max(8, Math.round((value / max) * 28));
      return `<i class="${point.spike ? "slow" : ""}" style="height:${height}px"></i>`;
    })
    .join("")}</span>`;
}

function telemetryBar(copy: ShellCopy, run: RunSummary): string {
  const lanes = run.telemetry ?? [];
  if (!lanes.length) return "";
  const capsule = (lane: RunTelemetryLane) =>
    `<button type="button" class="capsule ${lane.spike ? "slow" : ""}" data-act="toggle-telemetry">${escapeHtml(lane.model)}<small>${escapeHtml(laneLabel(copy, lane.lane))}</small></button>`;
  const main = lanes.find((lane) => lane.lane === "main") ?? lanes[0];
  const capsules = lanes.map(capsule).join("");
  const simple = `<div class="telemetry-mobile">${capsule(main)}<ul class="telemetry-simple">${lanes
    .map(
      (lane) =>
        `<li>${escapeHtml(lane.model)} · ${escapeHtml(laneLabel(copy, lane.lane))} · ${copy.tokenTotal} ${dash(lane.tokens.total)}</li>`,
    )
    .join("")}</ul></div>`;
  const cards = telemetryExpanded
    ? `<div class="telemetry-cards">${lanes
        .map(
          (lane) => `<article class="telemetry-card ${lane.spike ? "slow" : ""}">
            <header><b>${escapeHtml(lane.model)}</b><span>${escapeHtml(laneLabel(copy, lane.lane))}</span></header>
            <div class="token-row">${tokenCells(copy, lane.tokens)}</div>
            <div class="telemetry-meta">${escapeHtml(copy.ttft)} ${dash(lane.ttftMs)} · ${escapeHtml(copy.genRate)} ${dash(lane.tokensPerSec)}</div>
            ${sparkline(lane.recent, "ttftMs")}
          </article>`,
        )
        .join("")}<p class="tiny">${escapeHtml(copy.proxyDisclaimer)}</p></div>`
    : "";
  return `<div class="telemetry-bar"><div class="telemetry-desktop">${capsules}</div>${simple}${cards}</div>`;
}

function usagePage(copy: ShellCopy, snap: Snapshot): string {
  const usage = snap.usage;
  if (!usage) return "";
  const range = usage.range;
  const customKey = usageCustomFormKey(snap.focusedHostId);
  const customPending = formOperations.pending.has(customKey);
  const customDraft = usageCustomDraft ?? {
    from: toLocalInput(usage.fromMs),
    to: toLocalInput(usage.toMs),
  };
  const rangeBtn = (id: UsageRange, label: string) =>
    `<button type="button" class="${range === id ? "active" : ""}" data-act="usage-range" data-id="${id}">${escapeHtml(label)}</button>`;
  const optionList = (items: UsageOption[], selected: string | null | undefined) =>
    `<option value="">${escapeHtml(copy.filterAll)}</option>${items
      .map(
        (item) =>
          `<option value="${escapeHtml(item.id)}" ${item.id === selected ? "selected" : ""}>${escapeHtml(item.name)}</option>`,
      )
      .join("")}`;
  const rows = usage.runs.length
    ? usage.runs
        .map(
          (row) => `<article class="usage-row ${row.highlighted ? "sel" : ""}">
            <header>
              <div>
                <b>${escapeHtml(row.projectName)}</b>
                <span>${escapeHtml(row.agentName)}${row.models.length ? ` · ${escapeHtml(row.models.join(", "))}` : ""}</span>
              </div>
              <button type="button" data-act="open-run-usage" data-id="${escapeHtml(row.runId)}">${escapeHtml(copy.openThisRun)}</button>
            </header>
            <div class="token-row">${tokenCells(copy, row.tokens)}</div>
          </article>`,
        )
        .join("")
    : `<p class="board-empty">${escapeHtml(copy.usageEmpty)}</p>`;
  const trend = `${usageTrend(copy.ttft, usage.buckets, "ttftMs")}${usageTrend(copy.genRate, usage.buckets, "tokensPerSec")}`;
  const hit =
    usage.cacheHitRate == null ? "—" : `${Math.round(usage.cacheHitRate * 1000) / 10}%`;
  return `<div class="usage-page">
    <div class="board-head">
      <div class="board-head-row">
        <div>
          <h1>${escapeHtml(copy.usage)}</h1>
          <p>${escapeHtml(copy.usageHint)}</p>
        </div>
        <button type="button" data-act="close-usage">${escapeHtml(copy.closeUsage)}</button>
      </div>
    </div>
    <div class="choices usage-ranges">
      ${rangeBtn("24-hours", copy.range24Hours)}
      ${rangeBtn("today", copy.rangeToday)}
      ${rangeBtn("7-days", copy.range7Days)}
      ${rangeBtn("30-days", copy.range30Days)}
      ${rangeBtn("custom", copy.rangeCustom)}
    </div>
    ${
      range === "custom"
        ? `<form class="usage-custom" data-act="usage-custom" aria-busy="${customPending ? "true" : "false"}">
            <input type="datetime-local" name="from" required value="${escapeHtml(customDraft.from)}" ${customPending ? "disabled" : ""} />
            <input type="datetime-local" name="to" required value="${escapeHtml(customDraft.to)}" ${customPending ? "disabled" : ""} />
            <button type="submit" ${customPending ? "disabled" : ""}>${escapeHtml(customPending ? copy.operationPending : copy.rangeCustom)}</button>
          </form>${formFeedback(customKey)}`
        : ""
    }
    <div class="usage-filters">
      <label>${escapeHtml(copy.filterProject)}<select data-usage-filter="projectId">${optionList(usage.projects, usage.filter.projectId)}</select></label>
      <label>${escapeHtml(copy.filterAgent)}<select data-usage-filter="agentId">${optionList(usage.agents, usage.filter.agentId)}</select></label>
      <label>${escapeHtml(copy.filterModel)}<select data-usage-filter="model"><option value="">${escapeHtml(copy.filterAll)}</option>${usage.models
        .map(
          (model) =>
            `<option value="${escapeHtml(model)}" ${model === usage.filter.model ? "selected" : ""}>${escapeHtml(model)}</option>`,
        )
        .join("")}</select></label>
    </div>
    <div class="token-row totals">${tokenCells(copy, usage.totals)}<span class="token-cell"><i>${escapeHtml(copy.cacheHit)}</i>${hit}</span></div>
    ${trend}
    <p class="tiny">${escapeHtml(copy.proxyDisclaimer)}</p>
    <div class="usage-list usage-full">${rows}</div>
    <div class="usage-list usage-compact">${usageCompact(copy, usage)}</div>
  </div>`;
}

function hostOverviewPage(copy: ShellCopy, snap: Snapshot): string {
  if (overviewProjectId && !snap.projects.some((project) => project.id === overviewProjectId)) {
    overviewProjectId = "";
  }
  const visibleProjects = snap.projects.filter(
    (project) => !overviewProjectId || project.id === overviewProjectId,
  );
  const visibleRuns = (snap.runs ?? []).filter(
    (run) => !overviewProjectId || run.projectId === overviewProjectId,
  );
  const groups: Array<[string, string, RunSummary[]]> = [
    ["waiting", copy.runGroupWaiting, visibleRuns.filter((run) => run.status !== "ended" && Boolean(run.waitingForUser))],
    ["running", copy.runGroupRunning, visibleRuns.filter((run) => run.status !== "ended" && !run.waitingForUser)],
    ["stopped", copy.runGroupStopped, visibleRuns.filter((run) => run.status === "ended" && Boolean(run.endedReason) && run.endedReason !== "exited")],
    ["ended", copy.runGroupEnded, visibleRuns.filter((run) => run.status === "ended" && (!run.endedReason || run.endedReason === "exited"))],
  ];
  const projectOptions = snap.projects
    .map(
      (project) => `<option value="${escapeHtml(project.id)}" ${project.id === overviewProjectId ? "selected" : ""}>${escapeHtml(project.name)}</option>`,
    )
    .join("");
  const totalCounts = visibleProjects.reduce(
    (total, project) => {
      const counts = projectIssueCounts(project);
      if (counts.dataAvailable) {
        total.open += counts.open;
        total.frontier += counts.frontier;
        total.available += 1;
      }
      return total;
    },
    { open: 0, frontier: 0, available: 0 },
  );
  const allCountsAvailable = visibleProjects.length > 0 && totalCounts.available === visibleProjects.length;
  const activeRuns = visibleRuns.filter((run) => run.status !== "ended").length;
  return `<div class="overview-page">
    <div class="board-head">
      <div class="board-head-row">
        <div><h1>${escapeHtml(copy.hostOverview)}</h1><p>${escapeHtml(copy.hostOverviewHint)}</p></div>
        <button type="button" data-act="return-board">${escapeHtml(copy.returnToBoard)}</button>
      </div>
    </div>
    <div class="overview-controls">
      <label>${escapeHtml(copy.filterProject)}
        <select data-overview-filter="project"><option value="">${escapeHtml(copy.filterAll)}</option>${projectOptions}</select>
      </label>
      <label class="graph-opt"><input type="checkbox" data-field="showEndedRuns" ${overviewShowEnded ? "checked" : ""} />${escapeHtml(copy.showEndedRuns)}</label>
    </div>
    <div class="overview-stats">
      <div><b>${visibleProjects.length}</b><span>${escapeHtml(copy.projects)}</span></div>
      <div><b>${allCountsAvailable ? totalCounts.open : "—"}</b><span>Open Issue</span></div>
      <div><b>${allCountsAvailable ? totalCounts.frontier : "—"}</b><span>Frontier</span></div>
      <div><b>${activeRuns}</b><span>Run</span></div>
    </div>
    <section class="overview-project-section">
      <div class="lane-hd">${escapeHtml(copy.projects)} <span>${visibleProjects.length}</span></div>
      <div class="overview-projects">
        ${visibleProjects.map((project) => overviewProjectCard(copy, project)).join("")}
      </div>
    </section>
    <section class="overview-run-section">
      <div class="lane-hd">Run <span>${visibleRuns.length}</span></div>
      ${visibleRuns.length === 0
        ? `<div class="overview-runs-empty">${escapeHtml((snap.runs ?? []).length === 0 ? copy.hostOverviewEmpty : copy.noItems)}</div>`
        : `<div class="overview-groups">
          ${groups
            .filter(([id]) => id !== "ended" || overviewShowEnded)
            .map(
              ([id, title, runs]) => `<section class="overview-group" data-run-group="${id}">
                <div class="lane-hd">${escapeHtml(title)} <span>${runs.length}</span></div>
                <div class="run-thumbnails">${runs.length ? runs.map((run) => runThumbnail(copy, run, snap)).join("") : `<p class="lane-empty">${escapeHtml(copy.noItems)}</p>`}</div>
              </section>`,
            )
            .join("")}
        </div>`}
    </section>
  </div>`;
}

function projectIssueCounts(project: Project): ProjectIssueCounts {
  return project.issueCounts ?? {
    dataAvailable: false,
    total: 0,
    open: 0,
    closed: 0,
    blocked: 0,
    frontier: 0,
    inProgress: 0,
  };
}

function overviewProjectCard(copy: ShellCopy, project: Project): string {
  const counts = projectIssueCounts(project);
  const metric = (label: string, value: number) =>
    `<span><i>${escapeHtml(label)}</i><b>${counts.dataAvailable ? value : "—"}</b></span>`;
  const connection = project.connection.status === "ready"
    ? copy.connectionReady
    : project.connection.status === "unreachable"
      ? copy.connectionUnavailable
      : copy.authFailed;
  return `<button type="button" class="overview-project" data-act="focus-project" data-id="${escapeHtml(project.id)}">
    <span class="overview-project-head"><span><b>${escapeHtml(project.name)}</b><small>${escapeHtml(projectTrackerLabel(project))}</small></span><em>${escapeHtml(connection)}</em></span>
    <span class="overview-project-metrics">
      ${metric("Open", counts.open)}
      ${metric(copy.colBlocked, counts.blocked)}
      ${metric(copy.colFrontier, counts.frontier)}
      ${metric(copy.colInProgress, counts.inProgress)}
      ${metric("Closed", counts.closed)}
    </span>
  </button>`;
}

function runThumbnail(copy: ShellCopy, run: RunSummary, snap: Snapshot): string {
  const project = snap.projects.find((item) => item.id === run.projectId);
  const action = run.recentAction?.trim() || run.failure?.trim() || "";
  return `<button type="button" class="run-thumbnail" data-act="focus-run" data-id="${escapeHtml(run.id)}">
    <span class="run-project">${escapeHtml(project?.name ?? run.projectId)}</span>
    <b>${escapeHtml(runIdentity(copy, run))}</b>
    <span>${escapeHtml(run.agentName)}${action ? ` · ${escapeHtml(action)}` : ""}</span>
  </button>`;
}

function usageTrend(
  label: string,
  buckets: UsageBucket[],
  field: "ttftMs" | "tokensPerSec",
): string {
  const max = Math.max(...buckets.map((bucket) => bucket[field] ?? 0), 1);
  return `<div class="usage-trend-block"><span class="tiny">${escapeHtml(label)}</span><div class="usage-trend">${buckets
    .map((bucket) => {
      const height = Math.max(4, Math.round(((bucket[field] ?? 0) / max) * 48));
      return `<i class="${bucket.slow ? "slow" : ""}" style="height:${height}px" title="${dash(bucket[field])}"></i>`;
    })
    .join("")}</div></div>`;
}

function usageCompact(copy: ShellCopy, usage: UsagePage): string {
  const byProject = new Map<string, { name: string; tokens: TokenCounts }>();
  for (const row of usage.runs) {
    const current = byProject.get(row.projectId);
    if (!current) {
      byProject.set(row.projectId, { name: row.projectName, tokens: row.tokens });
    } else {
      current.tokens = {
        input: addOpt(current.tokens.input, row.tokens.input),
        output: addOpt(current.tokens.output, row.tokens.output),
        cacheRead: addOpt(current.tokens.cacheRead, row.tokens.cacheRead),
        cacheWrite: addOpt(current.tokens.cacheWrite, row.tokens.cacheWrite),
        reasoning: addOpt(current.tokens.reasoning, row.tokens.reasoning),
        total: addOpt(current.tokens.total, row.tokens.total),
      };
    }
  }
  const lines = [...byProject.values()].slice(0, 3);
  if (!lines.length) return `<p class="board-empty">${escapeHtml(copy.usageEmpty)}</p>`;
  return lines
    .map(
      (line) =>
        `<article class="usage-row"><header><b>${escapeHtml(line.name)}</b></header><div class="token-row">${tokenCells(copy, line.tokens)}</div></article>`,
    )
    .join("");
}

function addOpt(left?: number | null, right?: number | null): number | null {
  if (left == null || right == null) return null;
  return left + right;
}

function toLocalInput(ms: number): string {
  const date = new Date(ms);
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

function runControls(copy: ShellCopy, run: RunSummary): string {
  return `<div class="actions">
    <button type="button" data-act="open-usage-run" data-id="${escapeHtml(run.id)}">${escapeHtml(copy.openHostUsage)}</button>
    ${mobileClient() ? "" : `<button type="button" data-act="view-changes" data-id="${escapeHtml(run.id)}">${escapeHtml(copy.viewChanges)}</button>`}
    <button type="button" data-act="stop-run" data-id="${escapeHtml(run.id)}" ${run.status === "ended" ? "disabled" : ""}>${escapeHtml(copy.stopRun)}</button>
  </div>`;
}

function terminalPanel(copy: ShellCopy, run: RunSummary, className: string): string {
  const identity = runIdentity(copy, run);
  return `<div class="${className}">
    <header class="run-dock-hd">
      <div><b>${escapeHtml(run.agentName)}</b><span>${escapeHtml(identity)}</span></div>
      ${runControls(copy, run)}
    </header>
    ${telemetryBar(copy, run)}
    ${run.waitingForUser && run.status !== "ended" ? `<p class="notice">${escapeHtml(copy.waiting)}</p>` : ""}
    ${run.failure ? `<p class="notice bad">${escapeHtml(run.failure)}</p>` : ""}
    ${run.isolationNote ? `<p class="notice">${escapeHtml(run.isolationNote)}</p>` : ""}
    <div class="pty-slot" data-run="${escapeHtml(run.id)}"></div>
    ${run.status === "ended" ? "" : injectRunForm(copy, run.id)}
  </div>`;
}

function runDock(copy: ShellCopy, snap: Snapshot): string {
  const run = focusedRun(snap);
  if (!run || run.status === "ended") return "";
  const selectedIssueId = snap.board?.selected?.id;
  if (!run.unbound && run.issueId !== selectedIssueId) return "";
  return terminalPanel(copy, run, "run-dock");
}

function liftedRunView(copy: ShellCopy, snap: Snapshot): string {
  const run = focusedRun(snap);
  if (!run) return projectMain(copy, snap);
  const inspectorOpen = issueDetailVisible && Boolean(snap.board?.selected);
  return `<section class="lifted-run ${inspectorOpen ? "" : "issue-collapsed"}">
    ${terminalPanel(copy, run, "lifted-terminal")}
    ${inspectorOpen && snap.board ? `<aside class="issue-detail">${issueDetail(copy, snap.board)}</aside>` : ""}
  </section>`;
}

function viewChangesPanel(copy: ShellCopy): string {
  const view = changesView;
  const scope = view?.scope ?? changesScope;
  return `<div class="overlay modal" data-act="close-changes">
    <div class="sheet form-sheet changes-sheet" data-act="form-noop">
      <h2>${escapeHtml(copy.viewChanges)}</h2>
      <div class="choices">
        <button type="button" class="${scope === "this-round" ? "active" : ""}" data-act="changes-scope" data-id="this-round">${escapeHtml(copy.thisRound)}</button>
        <button type="button" class="${scope === "uncommitted" ? "active" : ""}" data-act="changes-scope" data-id="uncommitted">${escapeHtml(copy.uncommitted)}</button>
      </div>
      ${
        !view
          ? `<p class="notice">${escapeHtml(copy.viewChanges)}</p>`
          : !view.available
            ? `<p class="notice bad">${escapeHtml(view.unavailableReason || copy.viewChanges)}</p>`
            : view.repos
                .map((repo) => changeRepoBlock(copy, view, repo))
                .join("")
      }
      <div class="actions">
        <button type="button" data-act="close-changes">${escapeHtml(copy.cancel)}</button>
      </div>
    </div>
  </div>`;
}

function changeRepoBlock(copy: ShellCopy, view: ViewChanges, repo: ChangeRepo): string {
  const title = repo.displayPath === "." ? view.workingDirectory : repo.displayPath;
  if (!repo.available) {
    return `<section class="change-repo">
      <h3>${escapeHtml(title)}</h3>
      <p class="notice">${escapeHtml(repo.unavailableReason || copy.viewChanges)}</p>
    </section>`;
  }
  if (!repo.files.length) {
    return `<section class="change-repo">
      <h3>${escapeHtml(title)}</h3>
      <p class="muted">${escapeHtml(copy.noItems)}</p>
    </section>`;
  }
  return `<section class="change-repo">
    <h3>${escapeHtml(title)}</h3>
    ${repo.files.map((file) => changeFileBlock(copy, view, repo, file)).join("")}
  </section>`;
}

function changeFileBlock(
  copy: ShellCopy,
  view: ViewChanges,
  repo: ChangeRepo,
  file: ChangeFile,
): string {
  return `<article class="change-file">
    <h4>${escapeHtml(file.path)}</h4>
    ${file.hunks
      .map(
        (hunk) => `<div class="diff">${hunk.lines
          .map((line) => changeLineRow(copy, view, repo, file, line))
          .join("")}</div>`,
      )
      .join("")}
  </article>`;
}

function changeLineRow(
  copy: ShellCopy,
  view: ViewChanges,
  repo: ChangeRepo,
  file: ChangeFile,
  line: ChangeLine,
): string {
  const mark = line.kind === "add" ? "+" : line.kind === "delete" ? "-" : " ";
  const number = line.newLine ?? line.oldLine ?? 0;
  const notes = view.notes.filter(
    (note) => note.repo === repo.displayPath && note.path === file.path && note.line === number,
  );
  const canNote = line.kind !== "delete" && line.newLine;
  const active =
    noteTarget &&
    noteTarget.repo === repo.displayPath &&
    noteTarget.path === file.path &&
    noteTarget.line === line.newLine;
  const noteKey = changeNoteFormKey(view.runId);
  const notePending = formOperations.pending.has(noteKey);
  const noteForm = active
    ? `<form class="note-form" data-act="write-note" aria-busy="${notePending ? "true" : "false"}">
        <input name="text" maxlength="400" required value="${escapeHtml(noteDraft)}" placeholder="${escapeHtml(copy.changeNotePlaceholder)}" ${notePending ? "disabled" : ""} />
        <button type="submit" ${notePending ? "disabled" : ""}>${escapeHtml(notePending ? copy.operationPending : copy.addChangeNote)}</button>
      </form>${formFeedback(noteKey)}`
    : "";
  const noteList = notes
    .map(
      (note) =>
        `<div class="change-note">${escapeHtml(note.text)} <button type="button" data-act="delete-note" data-id="${escapeHtml(note.id)}">${escapeHtml(copy.deleteChangeNote)}</button></div>`,
    )
    .join("");
  const attrs = canNote
    ? ` data-act="note-line" data-repo="${escapeHtml(repo.displayPath)}" data-path="${escapeHtml(file.path)}" data-line="${line.newLine}"`
    : "";
  return `<span class="diff-line ${line.kind}"${attrs}><span class="diff-no">${number || ""}</span><span class="diff-mark">${mark}</span><span class="diff-text">${escapeHtml(line.text)}</span></span>${noteForm}${noteList}`;
}

function quitOfferDialog(copy: ShellCopy): string {
  return `<div class="overlay modal" data-act="cancel-quit">
    <div class="sheet" data-act="form-noop">
      <h2>${escapeHtml(copy.quitActiveTitle)}</h2>
      <p class="notice">${escapeHtml(copy.quitActiveBody)}</p>
      <div class="actions">
        <button type="button" data-act="cancel-quit">${escapeHtml(copy.quitReturn)}</button>
        <button type="button" class="danger primary" data-act="confirm-quit">${escapeHtml(copy.quitStopAll)}</button>
      </div>
    </div>
  </div>`;
}

function launchEnvironmentStatus(copy: StartupCopy): string {
  const state = launchEnvironmentState;
  const text = state.status === "ready"
    ? copy.launchEnvironmentReady
    : state.status === "failed"
      ? copy.launchEnvironmentFailed
      : copy.launchEnvironmentIdle;
  const detail = launchEnvironmentError || state.message || "";
  return `<p class="hint ${state.status === "failed" || detail ? "notice bad" : ""}" data-launch-environment-status="${state.status}">${escapeHtml(text)}${detail ? `<br>${escapeHtml(detail)}` : ""}</p>`;
}

function startupSettings(copy: StartupCopy, snap: Snapshot): string {
  if (!desktopShellAvailable()) {
    return `<div class="field startup-settings"><div class="label">${escapeHtml(copy.hostStartup)}</div><p class="hint">${escapeHtml(copy.desktopStartupBrowser)}</p></div>`;
  }
  return `<div class="field startup-settings">
    <div class="label">${escapeHtml(copy.hostStartup)}</div>
    <div class="choices">
      <button type="button" class="${snap.hostMode === "host-and-client" ? "active" : ""}" data-act="host-mode" data-id="host-and-client">${escapeHtml(copy.hostAndClient)}</button>
      <button type="button" class="${snap.hostMode === "client-only" ? "active" : ""}" data-act="host-mode" data-id="client-only">${escapeHtml(copy.clientOnly)}</button>
    </div>
    <p class="hint">${escapeHtml(copy.hostModeHelp)} ${escapeHtml(copy.restartToApply)}</p>
    <label class="graph-opt">
      <input type="checkbox" data-field="startAtLogin" ${startAtLogin ? "checked" : ""} ${startAtLogin == null ? "disabled" : ""} />
      ${escapeHtml(copy.startAtLogin)}
    </label>
    <p class="hint">${escapeHtml(copy.startAtLoginHelp)}</p>
    ${startupSettingsError ? `<p class="notice bad">${escapeHtml(startupSettingsError)}</p>` : ""}
  </div>`;
}

function updateSettings(copy: ShellCopy): string {
  const status = updateState.kind === "checking"
    ? copy.updateChecking
    : updateState.kind === "current"
      ? copy.updateCurrent
      : updateState.kind === "failed"
        ? `${copy.updateFailed} ${updateState.message}`.trim()
        : updateState.kind === "blocked"
          ? `${copy.updateActiveRuns} (${updateState.activeRunCount})`
          : "";
  return `<div class="field update-settings">
    <div class="label">${escapeHtml(copy.updates)}</div>
    ${desktopShellAvailable()
      ? `<button type="button" data-act="check-updates" ${updateState.kind === "checking" || updateState.kind === "installing" ? "disabled" : ""}>${escapeHtml(updateState.kind === "checking" ? copy.updateChecking : copy.checkForUpdates)}</button>`
      : `<p class="hint">${escapeHtml(copy.updateUnavailableBrowser)}</p>`}
    ${status ? `<p class="hint update-status">${escapeHtml(status)}</p>` : ""}
  </div>`;
}

function updateDialog(copy: ShellCopy): string {
  if (updateState.kind === "available") {
    return `<div class="overlay modal update-dialog" data-act="update-later">
      <div class="sheet" data-act="form-noop" role="dialog" aria-modal="true">
        <h2>${escapeHtml(copy.updateAvailable)} ${escapeHtml(updateState.version)}</h2>
        <p class="notice">${escapeHtml(copy.updateReady)}</p>
        ${updateState.notes ? `<div class="field"><div class="label">${escapeHtml(copy.updateNotes)}</div><p class="update-notes">${escapeHtml(updateState.notes)}</p></div>` : ""}
        <div class="actions">
          <button type="button" data-act="update-later">${escapeHtml(copy.updateLater)}</button>
          <button type="button" class="primary" data-act="install-update">${escapeHtml(copy.updateConfirm)}</button>
        </div>
      </div>
    </div>`;
  }
  if (updateState.kind === "blocked") {
    return `<div class="overlay modal update-dialog" data-act="update-later">
      <div class="sheet" data-act="form-noop" role="dialog" aria-modal="true">
        <h2>${escapeHtml(copy.updateAvailable)}</h2>
        <p class="notice bad">${escapeHtml(copy.updateActiveRuns)} (${updateState.activeRunCount})</p>
        <div class="actions">
          <button type="button" data-act="update-later">${escapeHtml(copy.updateLater)}</button>
          <button type="button" data-act="install-update">${escapeHtml(copy.updateConfirm)}</button>
        </div>
      </div>
    </div>`;
  }
  if (updateState.kind === "installing") {
    const progress = updateState.progress == null ? "" : ` ${updateState.progress}%`;
    return `<div class="overlay modal update-dialog">
      <div class="sheet" data-act="form-noop" role="dialog" aria-modal="true">
        <h2>${escapeHtml(copy.updateInstalling)}${progress}</h2>
        ${updateState.progress == null ? "" : `<progress max="100" value="${updateState.progress}"></progress>`}
      </div>
    </div>`;
  }
  return "";
}

function keyboardHelpDialog(copy: ShellCopy): string {
  return `<div class="overlay modal keyboard-help" data-act="close-keyboard-help">
    <div class="sheet" data-act="form-noop" role="dialog" aria-modal="true" aria-label="${escapeHtml(copy.keyboardHelp)}">
      <h2>${escapeHtml(copy.keyboardHelp)}</h2>
      <p class="hint">${escapeHtml(copy.keyboardHelpBody)}</p>
      <div class="actions"><button type="button" data-act="close-keyboard-help">${escapeHtml(copy.gotIt)}</button></div>
    </div>
  </div>`;
}

function projectMain(copy: ShellCopy, snap: Snapshot, reuseGraphCanvas = false): string {
  const project = currentProject(snap);
  if (!project) return loopbackNotice(snap.loopbackPage);
  return `<div class="project-board">
    ${loopbackNotice(snap.loopbackPage)}
    <div class="board-head">
      <div class="board-head-row">
        <div class="project-heading">
          <h1>${escapeHtml(project.name)}</h1>
          <p title="${escapeHtml(project.localPath)}">${escapeHtml(projectTrackerLabel(project))}</p>
        </div>
      </div>
    </div>
    ${refreshBar(copy, snap.board)}
    ${issueSearch(copy, snap)}
    ${pendingBar(copy, snap)}
    ${connectionPanel(copy, project)}
    ${boardView(copy, snap, reuseGraphCanvas)}
  </div>`;
}

function issueSearch(copy: ShellCopy, snap: Snapshot): string {
  const search = issueSearchDraft ?? snap.board?.search ?? { title: "", triageRole: null, state: "all" as const };
  const key = issueSearchFormKey(snap.focusedProjectId);
  const pending = formOperations.pending.has(key);
  const triageRoles: TriageRole[] = [
    "needs-triage",
    "needs-info",
    "ready-for-agent",
    "ready-for-human",
    "wontfix",
  ];
  return `<form class="issue-search" data-act="issue-search" aria-busy="${pending ? "true" : "false"}">
    <label class="sr-only" for="issue-title-search">${escapeHtml(copy.searchTitle)}</label>
    <input id="issue-title-search" name="title" type="search" value="${escapeHtml(search.title)}" placeholder="${escapeHtml(copy.searchPlaceholder)}" ${pending ? "disabled" : ""} />
    <select name="triageRole" aria-label="${escapeHtml(copy.searchAllTriage)}" ${pending ? "disabled" : ""}>
      <option value="">${escapeHtml(copy.searchAllTriage)}</option>
      ${triageRoles.map((role) => `<option value="${role}" ${search.triageRole === role ? "selected" : ""}>${role}</option>`).join("")}
    </select>
    <select name="state" aria-label="${escapeHtml(copy.searchAllStates)}" ${pending ? "disabled" : ""}>
      <option value="all" ${search.state === "all" ? "selected" : ""}>${escapeHtml(copy.searchAllStates)}</option>
      <option value="open" ${search.state === "open" ? "selected" : ""}>${escapeHtml(copy.searchOpen)}</option>
      <option value="closed" ${search.state === "closed" ? "selected" : ""}>${escapeHtml(copy.searchClosed)}</option>
    </select>
    <button type="submit" ${pending ? "disabled" : ""}>${escapeHtml(pending ? copy.operationPending : copy.searchSubmit)}</button>
    <button type="button" data-act="keyboard-help" aria-label="${escapeHtml(copy.keyboardHelp)}">?</button>
  </form>${formFeedback(key)}`;
}

function boardView(copy: ShellCopy, snap: Snapshot, reuseGraphCanvas = false): string {
  const board = snap.board;
  if (board?.empty === "incomplete-read" || board?.empty === "tracker-error") {
    const detail = board.refresh.kind === "incomplete" || board.refresh.kind === "tracker-error"
      ? board.refresh.detail
      : null;
    const message = board.empty === "tracker-error" ? copy.emptyTrackerError : copy.emptyIncomplete;
    return `<div class="board-empty" data-empty="${board.empty}">
      <b>${escapeHtml(message)}</b>
      ${detail ? `<p>${escapeHtml(detail)}</p>` : ""}
    </div>`;
  }
  if (!board || board.empty === "no-data" || !board.columns) {
    return `<div class="board-empty">${escapeHtml(copy.emptyNoData)}</div>`;
  }
  const onGraph = snap.centerView === "graph";
  const hint = onGraph ? copy.graphHint : board.parentFilter ? copy.childHint : copy.boardHint;
  const inspectorOpen = issueDetailVisible && Boolean(board.selected);
  return `<div class="board-shell ${inspectorOpen ? "" : "issue-collapsed"}" data-center-view="${onGraph ? "graph" : "board"}">
    <div class="board-main">
      <div class="board-hint">
        ${escapeHtml(hint)}
        ${
          board.parentFilter
            ? `<button type="button" data-act="clear-filter">${escapeHtml(copy.clearFilter)}</button>`
            : ""
        }
      </div>
      ${onGraph ? dependencyGraphView(copy, board, reuseGraphCanvas) : boardLanes(copy, board)}
    </div>
    ${inspectorOpen ? `<aside class="issue-detail">${issueDetail(copy, board)}</aside>` : ""}
  </div>`;
}

function boardLanes(copy: ShellCopy, board: BoardSnapshot): string {
  const desktop: Array<["blocked" | "frontier" | "inProgress" | "recentlyCompleted", string, IssueCard[]]> = [
    ["blocked", copy.colBlocked, board.columns?.blocked ?? []],
    ["frontier", copy.colFrontier, board.columns?.frontier ?? []],
    ["inProgress", copy.colInProgress, board.columns?.inProgress ?? []],
    ["recentlyCompleted", copy.colRecent, board.columns?.recentlyCompleted ?? []],
  ];
  const cols = mobileClient()
    ? [desktop[2], desktop[1], desktop[0], desktop[3]] as typeof desktop
    : desktop;
  return `<div class="lanes">
    ${cols
      .map(([key, name, items]) => {
        const empty =
          items.length > 0
            ? ""
            : key === "frontier"
              ? frontierEmptyText(copy, board.frontierEmpty)
              : key === "recentlyCompleted"
                ? copy.noRecent
                : copy.noItems;
        return `<section class="lane" data-lane="${key}">
          <div class="lane-hd">${escapeHtml(name)} <span>${items.length}</span></div>
          ${items.map((issue) => issueCard(copy, issue, board.selected?.id, key)).join("")}
          ${items.length ? "" : `<div class="lane-empty">${escapeHtml(empty)}</div>`}
          ${key === "recentlyCompleted" ? `<p class="lane-note">${escapeHtml(copy.recentNote)}</p>` : ""}
        </section>`;
      })
      .join("")}
  </div>`;
}

function dependencyGraphView(copy: ShellCopy, board: BoardSnapshot, reuseCanvas: boolean): string {
  const graph = board.graph;
  if (!graph) {
    return `<div class="board-empty">${escapeHtml(copy.emptyNoData)}</div>`;
  }
  const overview = graph.mode === "overview";
  const legacyGraph = graph.mode == null && graph.centerId == null;
  const projectedNodes = [...graph.nodes]
    .sort((a, b) =>
      (a.distance ?? 0) - (b.distance ?? 0) ||
      a.rank - b.rank ||
      (overview ? b.number - a.number : a.number - b.number),
    )
    .slice(0, graphCanvasLimit);
  const columns = new Map<number, GraphNode[]>();
  for (const node of projectedNodes) {
    const list = columns.get(node.rank) ?? [];
    list.push(node);
    columns.set(node.rank, list);
  }
  const ranks = [...columns.keys()].sort((a, b) => a - b);
  const center = graph.nodes.find((node) => node.id === graph.centerId);
  const totalCount = graph.totalCount ?? graph.nodes.length;
  const centerLabel = copy.graphCenter.replace(
    "{issue}",
    center ? `#${center.number} ${center.title}` : graph.centerId ?? "—",
  );
  const completeLabel = copy.graphShowComplete.replace("{count}", String(totalCount));
  const canvasLimit = copy.graphCanvasLimit
    .replace("{shown}", String(projectedNodes.length))
    .replace("{total}", String(graph.nodes.length));
  const truncated = copy.graphTruncated
    .replace("{shown}", String(graph.nodes.length))
    .replace("{total}", String(totalCount));
  return `<div class="dep-graph">
    ${legacyGraph
      ? `<label class="graph-opt">
          <input type="checkbox" data-field="closedContext" ${board.showClosedGraphContext ? "checked" : ""} />
          ${escapeHtml(completeDependencyGraphLabel(copy, graph))}
        </label>`
      : `<div class="graph-toolbar" data-graph-mode="${overview ? "overview" : "focused"}">
          <span class="graph-center-label">${escapeHtml(overview ? copy.graphOverview : centerLabel)}</span>
          <div class="actions">
            ${overview
              ? ""
              : `<button type="button" data-act="graph-overview">${escapeHtml(copy.graphReturnOverview)}</button>
                ${graph.complete
                  ? `<button type="button" data-act="graph-neighborhood">${escapeHtml(copy.graphShowNeighborhood)}</button>`
                  : totalCount > graph.nodes.length
                    ? `<button type="button" data-act="graph-complete">${escapeHtml(completeLabel)}</button>`
                    : ""}`}
          </div>
        </div>
        ${graph.truncated ? `<div class="graph-limit graph-truncated">${escapeHtml(truncated)}</div>` : ""}
        ${projectedNodes.length < graph.nodes.length ? `<div class="graph-limit">${escapeHtml(canvasLimit)}</div>` : ""}
        ${graph.edges.length === 0 ? `<div class="graph-empty-dependencies">${escapeHtml(copy.graphNoDependencies)}</div>` : ""}`}
    ${reuseCanvas
      ? `<div class="graph-canvas" data-preserve-graph-canvas></div>`
      : `<div class="graph-canvas">
      <svg class="graph-edges" aria-hidden="true"></svg>
      <div class="graph-flow">
        ${ranks
          .map(
            (rank) =>
              `<div class="graph-col" data-rank="${rank}">${(columns.get(rank) ?? [])
                .map((node) =>
                  graphNode(
                    copy,
                    node,
                    board.selected?.id,
                    legacyGraph ? null : graph.centerId ?? null,
                    overview,
                  ),
                )
                .join("")}</div>`,
          )
          .join("")}
      </div>
    </div>`}
    ${!legacyGraph && graph.complete ? dependencyGraphIndex(copy, graph) : ""}
  </div>`;
}

function dependencyGraphIndex(copy: ShellCopy, graph: DependencyGraph): string {
  const query = graphListQuery.trim().toLowerCase();
  const matches = graph.nodes
    .filter((node) =>
      !query ||
      node.title.toLowerCase().includes(query) ||
      node.id.toLowerCase().includes(query) ||
      `#${node.number}`.includes(query),
    )
    .sort((a, b) =>
      graphRelationMeta(a.relation).order - graphRelationMeta(b.relation).order ||
      (a.distance ?? 0) - (b.distance ?? 0) ||
      a.number - b.number,
    );
  const visible = matches.slice(0, graphListLimit);
  return `<details class="graph-index" open>
    <summary>${escapeHtml(copy.graphCompleteList)} <span>${matches.length}</span></summary>
    <input id="dependency-graph-search" type="search" data-field="graphSearch" value="${escapeHtml(graphListQuery)}" placeholder="${escapeHtml(copy.graphSearchPlaceholder)}" />
    <div class="graph-index-list">
      ${visible.map((node) => graphIndexRow(copy, node, graph.centerId ?? "")).join("")}
    </div>
    ${visible.length < matches.length
      ? `<button type="button" class="graph-index-more" data-act="graph-list-more">${escapeHtml(copy.graphShowMore)}</button>`
      : ""}
  </details>`;
}

function graphRelationMeta(relation: GraphNode["relation"]): (typeof GRAPH_RELATION_META)["center"] {
  return GRAPH_RELATION_META[relation ?? "center"];
}

function graphIndexRow(copy: ShellCopy, node: GraphNode, centerId: string): string {
  return `<div class="graph-index-row ${node.open ? "" : "closed"}">
    <button type="button" class="graph-index-main" data-act="focus-issue" data-id="${escapeHtml(node.id)}">
      <span class="graph-relation">${escapeHtml(graphRelationMeta(node.relation).label(copy))}</span>
      <span class="issue-id">#${node.number}</span>
      <span class="issue-title">${escapeHtml(node.title)}</span>
    </button>
    ${node.id === centerId ? "" : graphCenterButton(copy, node)}
  </div>`;
}

function graphNode(
  copy: ShellCopy,
  node: GraphNode,
  selectedId: string | undefined,
  centerId: string | null,
  overview = false,
): string {
  const selected = node.id === selectedId ? "sel" : "";
  const closed = node.open ? "" : "closed";
  const center = node.id === centerId ? "root" : "";
  return `<article class="graph-node ${selected} ${closed} ${center}" data-id="${escapeHtml(node.id)}">
    <button type="button" class="graph-node-main" data-act="${overview ? "center-graph" : "focus-issue"}" data-id="${escapeHtml(node.id)}">
      <div class="issue-id">#${node.number}</div>
      <div class="issue-title">${escapeHtml(node.title)}</div>
    </button>
    ${center || centerId == null ? "" : graphCenterButton(copy, node)}
  </article>`;
}

function graphCenterButton(copy: ShellCopy, node: GraphNode): string {
  const label = `${copy.graphCenterHere} #${node.number}`;
  return `<button type="button" class="graph-center-act" data-act="center-graph" data-id="${escapeHtml(node.id)}" aria-label="${escapeHtml(label)}" title="${escapeHtml(label)}">
    ${escapeHtml(copy.graphCenterHere)}
  </button>`;
}

function issuePanelIcon(open: boolean): string {
  const chevron = open ? "M13 9l3 3-3 3" : "M16 9l-3 3 3 3";
  return `<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
    <rect x="3" y="4" width="18" height="16" rx="2"></rect>
    <path d="M10 4v16"></path>
    <path d="${chevron}"></path>
  </svg>`;
}

function frontierEmptyText(
  copy: ShellCopy,
  reason: BoardSnapshot["frontierEmpty"],
): string {
  if (reason === "all-blocked") return copy.noFrontierBlocked;
  if (reason === "all-claimed") return copy.noFrontierClaimed;
  if (reason === "no-open") return copy.noFrontierEmpty;
  return copy.noItems;
}

function issueActivityLabel(copy: ShellCopy, activity: IssueCard["activity"]): string {
  if (activity === "waiting") return copy.waiting;
  if (activity === "execution-stopped") return copy.executionStopped;
  if (activity === "running") return copy.running;
  return "";
}

function issueCard(
  copy: ShellCopy,
  issue: IssueCard,
  selectedId: string | undefined,
  lane: "blocked" | "frontier" | "inProgress" | "recentlyCompleted",
): string {
  const activity = issueActivityLabel(copy, issue.activity);
  const tags = [
    activity ? `<span class="tag">${escapeHtml(activity)}</span>` : "",
    issue.triageRole ? `<span class="tag">${escapeHtml(issue.triageRole)}</span>` : "",
    issue.claimedBy.length
      ? `<span class="tag">${escapeHtml(copy.claimed)} ${escapeHtml(issue.claimedBy.join(", "))}</span>`
      : "",
  ]
    .filter(Boolean)
    .join("");
  const cardAction = "focus-issue";
  const actionTargetId = issue.id;
  const laneActions = lane === "frontier"
    ? `<button type="button" class="primary" data-act="execute-run" data-id="${escapeHtml(issue.id)}">${escapeHtml(copy.executeRun)}</button>`
    : lane === "inProgress" && issue.runId
      ? `<button type="button" data-act="focus-run" data-id="${escapeHtml(issue.runId)}">${escapeHtml(copy.focusRun)}</button>
         <button type="button" data-act="stop-run" data-id="${escapeHtml(issue.runId)}">${escapeHtml(copy.stopRun)}</button>
         ${mobileClient() ? "" : `<button type="button" data-act="view-changes" data-id="${escapeHtml(issue.runId)}">${escapeHtml(copy.viewChanges)}</button>`}`
      : lane === "recentlyCompleted"
        ? `${!mobileClient() && issue.runId ? `<button type="button" data-act="view-changes" data-id="${escapeHtml(issue.runId)}">${escapeHtml(copy.viewChanges)}</button>` : ""}
           <button type="button" data-act="open-issue" data-url="${escapeHtml(issue.url)}">${escapeHtml(copy.openIssue)}</button>`
        : "";
  const dependencyAction = mobileClient()
    ? ""
    : `<button type="button" data-act="view-dependencies" data-id="${escapeHtml(issue.id)}">${escapeHtml(copy.viewDependencies)}</button>`;
  const actions = `${dependencyAction}${laneActions}`;
  return `<article class="issue-card ${issue.id === selectedId ? "sel" : ""} ${issue.activity ? escapeHtml(issue.activity) : ""} ${lane === "recentlyCompleted" ? "recently-completed subdued" : ""}" data-issue-id="${escapeHtml(issue.id)}">
    <button type="button" class="issue-card-main" data-act="${cardAction}" data-id="${escapeHtml(actionTargetId)}" data-issue-id="${escapeHtml(issue.id)}">
      <div class="issue-id">#${issue.number}</div>
      <div class="issue-title">${escapeHtml(issue.title)}</div>
      ${tags ? `<div class="issue-tags">${tags}</div>` : ""}
    </button>
    ${actions ? `<div class="issue-card-actions">${actions}</div>` : ""}
  </article>`;
}

function issueDetail(copy: ShellCopy, board: BoardSnapshot, showPanelToggle = true): string {
  const issue = board.selected;
  if (!issue) {
    return `<div class="lane-empty">${escapeHtml(copy.pickIssue)}</div>`;
  }
  const claim = issue.claimedBy.length
    ? `${copy.claimed} ${issue.claimedBy.join(", ")}`
    : copy.unclaimed;
  const hasActive = Boolean(issue.activeRunId) || (snapshot?.runs ?? []).some(
    (run) => run.issueId === issue.id && run.status !== "ended",
  );
  const actions = hasActive
    ? ""
    : issue.executionStopped
      ? `<button type="button" class="primary" data-act="continue-run" data-id="${escapeHtml(issue.id)}">${escapeHtml(copy.continueRun)}</button>
         <button type="button" data-act="release-claim" data-id="${escapeHtml(issue.id)}">${escapeHtml(copy.releaseClaim)}</button>`
      : `<button type="button" class="primary" data-act="execute-run" data-id="${escapeHtml(issue.id)}">${escapeHtml(copy.executeRun)}</button>`;
  return `
    <header class="detail-sticky">
      <div class="detail-title-row">
        <div class="detail-hd">#${issue.number} ${escapeHtml(issue.title)}</div>
        ${showPanelToggle ? `<button type="button" class="chrome-icon detail-panel-toggle" data-act="toggle-issue" aria-label="${escapeHtml(copy.hideIssueDetail)}" title="${escapeHtml(copy.hideIssueDetail)}">${issuePanelIcon(true)}</button>` : ""}
      </div>
      <div class="detail-meta">
        ${issue.triageRole ? `<span class="tag">${escapeHtml(issue.triageRole)}</span>` : ""}
        <span class="tag">${escapeHtml(claim)}</span>
        ${issue.waitingForUser ? `<span class="tag">${escapeHtml(copy.waiting)}</span>` : ""}
        ${issue.executionStopped ? `<span class="tag">${escapeHtml(copy.executionStopped)}</span>` : ""}
        ${actions}
        <button type="button" data-act="open-issue" data-url="${escapeHtml(issue.url)}">${escapeHtml(copy.openIssue)}</button>
      </div>
    </header>
    <div class="detail-scroll">
      ${issueDocument(copy, issue.document ?? { kind: "unloaded" }, issue.url)}
      <section class="detail-block">
      <h4>${escapeHtml(copy.family)}</h4>
      <div class="tiny">${escapeHtml(copy.parent)}</div>
      ${issue.parent ? issueLink(copy, issue.parent) : `<span class="muted">${escapeHtml(copy.noParent)}</span>`}
      <div class="tiny">${escapeHtml(copy.children)}</div>
      ${
        issue.children.length
          ? issue.children.map((child) => issueLink(copy, child)).join("")
          : `<span class="muted">${escapeHtml(copy.noKids)}</span>`
      }
      ${
        issue.children.length
          ? `<div><button type="button" data-act="filter-parent" data-id="${escapeHtml(issue.id)}">${escapeHtml(copy.onlyKids)}</button></div>`
          : ""
      }
      </section>
      <section class="detail-block">
      <h4>${escapeHtml(copy.deps)}</h4>
      <div class="tiny">${escapeHtml(copy.blockedBy)}</div>
      ${
        issue.blockedBy.length
          ? issue.blockedBy.map((link) => issueLink(copy, link)).join("")
          : `<span class="muted">${escapeHtml(copy.noneBlock)}</span>`
      }
      <div class="tiny">${escapeHtml(copy.blocking)}</div>
      ${
        issue.blocking.length
          ? issue.blocking.map((link) => issueLink(copy, link)).join("")
          : `<span class="muted">${escapeHtml(copy.none)}</span>`
      }
      </section>
    </div>`;
}

function issueDocument(copy: ShellCopy, state: IssueDocumentState, issueUrl: string): string {
  if (state.kind === "unloaded" || state.kind === "loading") {
    const previous = state.kind === "loading" && state.body != null
      ? `<div class="issue-markdown is-stale">${renderMarkdown(state.body, issueUrl)}</div>`
      : "";
    const asOf = state.kind === "loading" && state.fetchedAtMs != null
      ? ` · ${escapeHtml(copy.refreshAsOf)} ${escapeHtml(formatTime(state.fetchedAtMs))}`
      : "";
    return `<section class="issue-document" data-document-state="${state.kind}">
      <h4>${escapeHtml(copy.issueDocument)}</h4>
      <p class="document-status">${escapeHtml(copy.issueDocumentLoading)}${asOf}</p>
      ${previous}
    </section>`;
  }
  if (state.kind === "failed") {
    return `<section class="issue-document" data-document-state="failed">
      <h4>${escapeHtml(copy.issueDocument)}</h4>
      <p class="notice bad">${escapeHtml(copy.issueDocumentFailed)} ${escapeHtml(state.failure.message)}</p>
      <button type="button" data-act="retry-issue-document">${escapeHtml(copy.issueDocumentRetry)}</button>
    </section>`;
  }
  const stale = state.kind === "stale";
  return `<section class="issue-document" data-document-state="${state.kind}">
    <h4>${escapeHtml(copy.issueDocument)}</h4>
    ${stale
      ? `<p class="document-status stale">${escapeHtml(copy.issueDocumentStale)} ${escapeHtml(formatTime(state.fetchedAtMs))}. ${escapeHtml(state.failure.message)}</p>
         <button type="button" data-act="retry-issue-document">${escapeHtml(copy.issueDocumentRetry)}</button>`
      : ""}
    <div class="issue-markdown ${stale ? "is-stale" : ""}">${renderMarkdown(state.body, issueUrl)}</div>
  </section>`;
}

function renderMarkdown(markdown: string, baseUrl: string): string {
  const lines = markdown.replace(/\r\n?/g, "\n").split("\n");
  const blocks: string[] = [];
  let paragraph: string[] = [];
  let list: { ordered: boolean; items: string[] } | null = null;
  const flushParagraph = () => {
    if (!paragraph.length) return;
    blocks.push(`<p>${renderInlineMarkdown(paragraph.join(" "), baseUrl)}</p>`);
    paragraph = [];
  };
  const flushList = () => {
    if (!list) return;
    const tag = list.ordered ? "ol" : "ul";
    blocks.push(`<${tag}>${list.items.map((item) => `<li>${renderInlineMarkdown(item, baseUrl)}</li>`).join("")}</${tag}>`);
    list = null;
  };
  for (const line of lines) {
    const heading = /^(#{1,6})\s+(.+)$/.exec(line);
    const item = /^\s*([-*+] |\d+\. )(.+)$/.exec(line);
    if (heading) {
      flushParagraph();
      flushList();
      const level = heading[1].length;
      blocks.push(`<h${level}>${renderInlineMarkdown(heading[2], baseUrl)}</h${level}>`);
    } else if (item) {
      flushParagraph();
      const ordered = /^\d/.test(item[1]);
      if (list && list.ordered !== ordered) flushList();
      list ??= { ordered, items: [] };
      list.items.push(item[2]);
    } else if (!line.trim()) {
      flushParagraph();
      flushList();
    } else {
      flushList();
      paragraph.push(line.trim());
    }
  }
  flushParagraph();
  flushList();
  return blocks.join("");
}

function renderInlineMarkdown(source: string, baseUrl: string): string {
  const token = /(`[^`\n]+`|\*\*[^*\n]+\*\*|\[[^\]\n]+\]\([^\n)]+\))/g;
  let html = "";
  let offset = 0;
  for (const match of source.matchAll(token)) {
    const index = match.index ?? 0;
    html += escapeHtml(source.slice(offset, index));
    const value = match[0];
    if (value.startsWith("`")) {
      html += `<code>${escapeHtml(value.slice(1, -1))}</code>`;
    } else if (value.startsWith("**")) {
      html += `<strong>${escapeHtml(value.slice(2, -2))}</strong>`;
    } else {
      const link = /^\[([^\]]+)\]\(([^)]+)\)$/.exec(value);
      const href = link ? safeHttpUrl(link[2], baseUrl) : null;
      html += href
        ? `<a href="${escapeHtml(href)}" data-act="open-external" data-url="${escapeHtml(href)}">${escapeHtml(link?.[1] ?? "")}</a>`
        : `<span class="unsafe-link">${escapeHtml(link?.[1] ?? value)}</span>`;
    }
    offset = index + value.length;
  }
  return html + escapeHtml(source.slice(offset));
}

function safeHttpUrl(raw: string, baseUrl?: string): string | null {
  try {
    const url = baseUrl ? new URL(raw.trim(), baseUrl) : new URL(raw.trim());
    return url.protocol === "http:" || url.protocol === "https:" ? url.toString() : null;
  } catch {
    return null;
  }
}

function issueLink(copy: ShellCopy, link: IssueLink): string {
  if (!link.visible) {
    return `<span class="muted">${escapeHtml(copy.unclearIssue)}</span>`;
  }
  const label = `#${link.number ?? "?"} ${link.title}`.trim();
  return `<button type="button" class="name-btn" data-act="focus-issue" data-id="${escapeHtml(link.id)}">${escapeHtml(label)}</button>`;
}

function refreshBar(copy: ShellCopy, board: BoardSnapshot | null): string {
  const status = board?.refresh ?? { kind: "never-fetched" as const };
  const kind = refreshing ? "refreshing" : status.kind;
  const parts: string[] = [];
  if (kind === "refreshing") {
    parts.push(copy.refreshRefreshing);
  } else if (status.kind === "never-fetched") {
    parts.push(copy.refreshNever);
  } else if (status.kind === "offline") {
    parts.push(`${copy.refreshOffline} · ${copy.refreshAsOf} ${formatTime(status.fetchedAtMs)}`);
    parts.push(copy.refreshOfflineRecovery);
    if (status.nextRefreshInMs != null) {
      parts.push(`${copy.refreshNext} ${formatCountdown(status.nextRefreshInMs)}`);
    }
  } else if (status.kind === "rate-limited") {
    parts.push(copy.refreshRateLimited);
    if (status.retryAtMs) {
      parts.push(`${copy.refreshRetry} ${formatTime(status.retryAtMs)}`);
    } else {
      parts.push(copy.refreshPaused);
    }
    if (status.fetchedAtMs) {
      parts.push(`${copy.refreshAsOf} ${formatTime(status.fetchedAtMs)}`);
    }
  } else if (status.kind === "auth-failed") {
    parts.push(copy.refreshAuth);
    parts.push(copy.refreshAuthRecovery);
    if (status.fetchedAtMs) {
      parts.push(`${copy.refreshAsOf} ${formatTime(status.fetchedAtMs)}`);
    }
  } else if (status.kind === "incomplete" || status.kind === "tracker-error") {
    parts.push(status.kind === "tracker-error" ? copy.refreshTrackerError : copy.refreshIncomplete);
    if (status.detail) {
      parts.push(status.detail);
    }
    if (status.fetchedAtMs) {
      parts.push(`${copy.refreshAsOf} ${formatTime(status.fetchedAtMs)}`);
    }
    if (status.nextRefreshInMs != null) {
      parts.push(`${copy.refreshNext} ${formatCountdown(status.nextRefreshInMs)}`);
    }
  } else if (status.kind === "ready") {
    parts.push(`${copy.refreshAsOf} ${formatTime(status.fetchedAtMs)}`);
    if (status.nextRefreshInMs != null) {
      parts.push(`${copy.refreshNext} ${formatCountdown(status.nextRefreshInMs)}`);
    }
  }
  return `<div class="refresh-bar" data-kind="${escapeHtml(kind)}">
    <span>${escapeHtml(parts.join(" · "))}</span>
    <button type="button" data-act="refresh">${escapeHtml(copy.refreshNow)}</button>
  </div>`;
}

function pendingBar(copy: ShellCopy, snap: Snapshot): string {
  const pending = snap.pendingConfirmation;
  if (!pending) return "";
  return `<div class="refresh-bar" data-kind="pending">
    <span>${escapeHtml(copy.pendingConfirmation)} · ${escapeHtml(pending.issueId)} · ${formatCountdown(pending.remainingMs)}</span>
    <button type="button" data-act="veto-advance" data-id="${escapeHtml(pending.projectId)}">${escapeHtml(copy.vetoAdvance)}</button>
  </div>`;
}

function formatTime(ms: number): string {
  try {
    return new Date(ms).toLocaleString();
  } catch {
    return String(ms);
  }
}

function formatCountdown(ms: number): string {
  const seconds = Math.max(0, Math.ceil(ms / 1000));
  if (seconds >= 60) {
    const minutes = Math.floor(seconds / 60);
    const rest = seconds % 60;
    return rest ? `${minutes}m ${rest}s` : `${minutes}m`;
  }
  return `${seconds}s`;
}

function connectionPanel(copy: ShellCopy, project: Project): string {
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

function launchForm(copy: ShellCopy, snap: Snapshot): string {
  const form = snap.launchForm;
  if (!form || !launchDraft) return "";
  const draft = launchDraft;
  if (!form.skipAgentPicker) {
    return `<div class="overlay modal" data-act="close-launch">
      <div class="sheet form-sheet launch-sheet" data-act="form-noop">
        <h2>${escapeHtml(copy.pickAgent)}</h2>
        <div class="choices agent-picks">
          ${form.agents
            .map(
              (agent) =>
                `<div class="agent-choice ${agent.installed ? "" : "agent-choice-unavailable"}">
                  <button type="button" class="${agent.id === agentPickerSelection ? "active" : ""}" data-act="pick-agent" data-id="${escapeHtml(agent.id)}" ${agent.installed ? "" : "disabled"}>${escapeHtml(agent.name)}</button>
                  ${agent.installed || !agent.unavailableReason ? "" : `<p class="notice bad">${escapeHtml(agent.unavailableReason)}</p>`}
                </div>`,
            )
            .join("")}
        </div>
        <div class="actions">
          <button type="button" data-act="close-launch">${escapeHtml(copy.cancel)}</button>
          <button type="button" class="primary" data-act="confirm-agent" ${agentPickerSelection ? "" : "disabled"}>${escapeHtml(copy.nextStep)}</button>
        </div>
      </div>
    </div>`;
  }
  const first = form.fields.filter((field) => !field.folded && field.id !== "initial-instruction");
  const folded = form.fields.filter((field) => field.folded);
  const intentActive = draft.custom ? "" : draft.intentId;
  const key = launchFormKey(form.projectId);
  const pending = formOperations.pending.has(key);
  const error = formOperations.errors.get(key) || form.error || "";
  return `<div class="overlay modal" data-act="close-launch">
    <form class="sheet form-sheet launch-sheet" data-act="form-noop" data-form="launch" aria-busy="${pending ? "true" : "false"}">
      <h2>${escapeHtml(copy.launchTitle)}</h2>
      <fieldset class="launch-fields" ${pending ? "disabled" : ""}>
        <div class="launch-agent">
          <b>${escapeHtml(form.agents.find((agent) => agent.id === form.selectedAgentId)?.name ?? form.selectedAgentId)}</b>
          <button type="button" data-act="switch-agent">${escapeHtml(copy.switchAgent)}</button>
        </div>
        <p class="hint">${escapeHtml(prefillHint(copy, form.prefillSource))}</p>
        <div class="field">
          <div class="label">${escapeHtml(copy.runIntent)}</div>
          <div class="choices">
            <button type="button" class="${intentActive === "" && !draft.custom ? "active" : ""}" data-act="intent" data-id="">${escapeHtml(copy.intentNone)}</button>
            ${form.intents
              .map(
                (intent) =>
                  `<button type="button" class="${intentActive === intent.id ? "active" : ""}" data-act="intent" data-id="${escapeHtml(intent.id)}">${escapeHtml(intent.label)}</button>`,
              )
              .join("")}
            <button type="button" class="active" data-act="intent-custom" ${draft.custom ? "" : "hidden"}>${escapeHtml(copy.intentCustom)}</button>
          </div>
        </div>
        <div class="field">
          <label class="label" for="opening-text">${escapeHtml(copy.openingPlaceholder)}</label>
          <textarea id="opening-text" data-field="openingText" rows="4" required placeholder="${escapeHtml(copy.openingPlaceholder)}">${escapeHtml(draft.openingText)}</textarea>
        </div>
        ${first.map((field) => launchField(field, draft.values[field.id] ?? "")).join("")}
        <div class="field">
          <div class="label">${escapeHtml(copy.workingDirectory)}</div>
          <input value="${escapeHtml(form.workingDirectory)}" readonly />
        </div>
        <label class="graph-opt ${form.isolationSupported ? "" : "isolation-off"}">
          <input type="checkbox" data-launch="isolation" ${draft.values.isolation === "true" ? "checked" : ""} ${form.isolationSupported ? "" : "disabled"} />
          ${escapeHtml(copy.isolation)}
        </label>
        <p class="hint">${escapeHtml(copy.isolationHint)}</p>
        ${
          form.isolationSupported
            ? ""
            : `<details class="isolation-why"><summary>${escapeHtml(copy.isolationOffReason)}</summary><p class="hint">${escapeHtml(form.isolationReason)}</p></details>`
        }
        <details class="folded" ${launchFolded ? "open" : ""}>
          <summary data-act="toggle-folded">${escapeHtml(copy.foldedOptions)}</summary>
          ${folded.map((field) => launchField(field, draft.values[field.id] ?? "")).join("")}
          ${
            snap.showCommandPreview
              ? `<div class="field"><div class="label">${escapeHtml(copy.commandPreview)}</div><pre class="payload">${escapeHtml(form.commandPreview)}</pre></div>`
              : ""
          }
        </details>
        <p class="notice launch-warnings" ${form.warnings?.length ? "" : "hidden"}>${escapeHtml((form.warnings ?? []).join(" "))}</p>
        ${error ? `<p class="notice bad">${escapeHtml(error)}</p>` : ""}
        <div class="actions">
          <button type="button" data-act="close-launch">${escapeHtml(copy.cancel)}</button>
          <button type="submit" class="primary">${escapeHtml(pending ? copy.startRunPending : copy.startRun)}</button>
        </div>
      </fieldset>
    </form>
  </div>`;
}

function launchField(field: AgentField, value: string): string {
  const id = `launch-${field.id}`;
  if (field.kind === "boolean") {
    return `<label class="graph-opt">
      <input type="checkbox" data-launch="${escapeHtml(field.id)}" ${value === "true" ? "checked" : ""} />
      ${escapeHtml(field.label)}
    </label>`;
  }
  if (field.kind === "select") {
    const options = field.options ?? [];
    const listId = `${id}-list`;
    return `<div class="field">
      <label class="label" for="${id}">${escapeHtml(field.label)}</label>
      <input id="${id}" list="${listId}" data-launch="${escapeHtml(field.id)}" value="${escapeHtml(value)}" ${field.required ? "required" : ""} />
      <datalist id="${listId}">${options.map((option) => `<option value="${escapeHtml(option)}"></option>`).join("")}</datalist>
    </div>`;
  }
  if (field.kind === "multiline") {
    return `<div class="field">
      <label class="label" for="${id}">${escapeHtml(field.label)}</label>
      <textarea id="${id}" data-launch="${escapeHtml(field.id)}" rows="3">${escapeHtml(value)}</textarea>
    </div>`;
  }
  return `<div class="field">
    <label class="label" for="${id}">${escapeHtml(field.label)}</label>
    <input id="${id}" data-launch="${escapeHtml(field.id)}" value="${escapeHtml(value)}" ${field.required ? "required" : ""} />
  </div>`;
}

function projectForm(copy: ShellCopy): string {
  const editing = formOpen === "edit";
  const activeRun = Boolean(
    snapshot?.projects.find((project) => project.id === formProjectId)?.hasActiveRun,
  );
  const lockedRegistration = editing && activeRun;
  const saving = projectOperation === "save";
  const inferenceCandidate = projectInference.status === "candidate" ? projectInference.candidate : null;
  const inferenceMessage = projectInference.status === "failed" ? projectInference.message : "";
  return `<div class="overlay modal" data-act="close-form">
    <form class="sheet form-sheet" data-act="form-noop" data-form="project">
      <h2>${escapeHtml(editing ? copy.editProjectTitle : copy.registerProjectTitle)}</h2>
      <p class="hint">${escapeHtml(focusedHostIsLocal() ? copy.inferenceHint : copy.remoteProjectHint)}</p>
      ${lockedRegistration ? `<p class="notice">${escapeHtml(copy.activeProjectEditHint)}</p>` : ""}
      <div class="field">
        <label class="label" for="project-name">${escapeHtml(copy.displayName)}</label>
        <input id="project-name" data-field="name" ${saving ? "disabled" : "required"} value="${escapeHtml(formDraft.name)}" />
      </div>
      <div class="field">
        <label class="label" for="project-path">${escapeHtml(copy.localDirectory)}</label>
        <div class="path-picker">
          <input id="project-path" class="path-input" data-field="localPath" ${lockedRegistration || saving ? "disabled" : "required"} value="${escapeHtml(formDraft.localPath)}" title="${escapeHtml(formDraft.localPath)}" dir="ltr" />
          ${focusedHostIsLocal() && !lockedRegistration
            ? `<button type="button" data-act="choose-project-directory" ${saving ? "disabled" : ""}>${escapeHtml(copy.chooseDirectory)}</button>`
            : ""}
        </div>
      </div>
      <div class="field">
        <label class="label" for="project-host">${escapeHtml(copy.githubHost)}</label>
        <input id="project-host" data-field="githubHost" ${lockedRegistration || saving ? "disabled" : ""} value="${escapeHtml(formDraft.githubHost)}" />
      </div>
      <div class="field">
        <label class="label" for="project-repo">${escapeHtml(copy.repository)}</label>
        <input id="project-repo" data-field="repository" ${lockedRegistration || saving ? "disabled" : "required"} placeholder="owner/repo" value="${escapeHtml(formDraft.repository)}" />
      </div>
      ${!lockedRegistration && projectInference.status !== "idle"
        ? `<div class="inference">
            ${projectInference.status === "pending"
              ? `<p class="hint" data-inference="pending">${escapeHtml(copy.inferringFromDirectory)}</p>`
              : ""}
            ${inferenceCandidate
              ? `<div class="notice ok inference-candidate" data-inference="candidate">
                  <div><b>${escapeHtml(inferenceCandidate.name)}</b></div>
                  <div>${escapeHtml(inferenceCandidate.tracker === "local-markdown" ? "Local Markdown" : `${inferenceCandidate.githubHost}/${inferenceCandidate.repository}`)}</div>
                  <div class="actions">
                    <button type="button" data-act="apply-infer">${escapeHtml(copy.useInference)}</button>
                  </div>
                </div>`
              : ""}
            ${inferenceMessage
              ? `<div class="notice bad" data-inference="failed">
                  <div>${escapeHtml(inferenceMessage)}</div>
                  <div class="actions">
                    <button type="button" data-act="retry-infer">${escapeHtml(copy.retryInference)}</button>
                  </div>
                </div>`
              : ""}
          </div>`
        : ""}
      ${formError ? `<p class="notice bad">${escapeHtml(formError)}</p>` : ""}
      <div class="actions">
        <button type="button" data-act="close-form" ${saving ? "disabled" : ""}>${escapeHtml(copy.cancel)}</button>
        <button type="submit" class="primary" ${saving ? "disabled" : ""}>${escapeHtml(saving ? copy.operationPending : editing ? copy.saveRegistration : copy.addProject)}</button>
      </div>
    </form>
  </div>`;
}

function removeDialog(copy: ShellCopy, project: Project): string {
  if (project.hasActiveRun) {
    return `<div class="overlay modal" data-act="close-remove">
      <div class="sheet" data-act="form-noop">
        <h2>${escapeHtml(copy.cannotRemoveActiveRun)} ${escapeHtml(project.name)}</h2>
        <p class="notice bad">${escapeHtml(copy.cannotRemoveActiveRunBody)}</p>
        <div class="actions">
          <button type="button" class="primary" data-act="close-remove">${escapeHtml(copy.gotIt)}</button>
        </div>
      </div>
    </div>`;
  }
  return `<div class="overlay modal" data-act="close-remove">
    <div class="sheet" data-act="form-noop">
      <h2>${escapeHtml(copy.removeConfirmTitle)}</h2>
      <p class="notice">${escapeHtml(copy.removeConfirmBody)}</p>
      ${project.hasExecutionStopped ? `<p class="notice">${escapeHtml(copy.removeKeepClaimsBody)}</p>` : ""}
      ${removeError ? `<p class="notice bad">${escapeHtml(removeError)}</p>` : ""}
      <div class="actions">
        <button type="button" data-act="close-remove" ${projectOperation === "remove" ? "disabled" : ""}>${escapeHtml(copy.cancel)}</button>
        <button type="button" class="danger primary" data-act="confirm-remove" ${projectOperation === "remove" ? "disabled" : ""}>${escapeHtml(projectOperation === "remove" ? copy.removalPending : copy.removeConfirm)}</button>
      </div>
    </div>
  </div>`;
}

function forgetHostDialog(copy: ShellCopy, host?: { id: string; displayName: string; local: boolean }): string {
  if (!host || host.local) return "";
  return `<div class="overlay modal" data-act="close-forget-host">
    <div class="sheet" data-act="form-noop">
      <h2>${escapeHtml(copy.forgetHostConfirmTitle)}</h2>
      <p class="notice">${escapeHtml(copy.forgetHostConfirmBody)}</p>
      <p class="hint">${escapeHtml(host.displayName)}</p>
      ${forgetHostError ? `<p class="notice bad">${escapeHtml(forgetHostError)}</p>` : ""}
      <div class="actions">
        <button type="button" data-act="close-forget-host" ${forgetHostPending ? "disabled" : ""}>${escapeHtml(copy.cancel)}</button>
        <button type="button" class="danger primary" data-act="confirm-forget-host" ${forgetHostPending ? "disabled" : ""}>${escapeHtml(forgetHostPending ? copy.operationPending : copy.forgetHost)}</button>
      </div>
    </div>
  </div>`;
}

function loopbackNotice(page: LoopbackPage): string {
  if (page.status === "serving") return "";
  return `<p class="notice">${escapeHtml(page.reason)}</p>`;
}

function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function termTheme(theme: Theme): ConstructorParameters<typeof Terminal>[0] {
  if (theme === "plain-night") {
    return {
      cursorBlink: true,
      fontSize: 13,
      theme: { background: "#181817", foreground: "#f4f2ee", cursor: "#e86a5c" },
    };
  }
  return {
    cursorBlink: true,
    fontSize: 13,
    theme: { background: "#1c1b19", foreground: "#f4f2ee", cursor: "#c45c26" },
  };
}

function ensureTerminal(theme: Theme): void {
  if (term && termHost && fitAddon) return;
  fitAddon = new FitAddon();
  term = new Terminal(termTheme(theme));
  term.loadAddon(fitAddon);
  termHost = document.createElement("div");
  termHost.className = "pty-host";
  term.open(termHost);
  term.onData((data) => {
    const runId = snapshot?.focusedRunId;
    if (!runId) return;
    void sendPtyInput(runId, data);
  });
}

function attachTerminal(snap: Snapshot): void {
  const run = focusedRun(snap);
  const slot = app?.querySelector<HTMLElement>(".pty-slot");
  if (!run || !slot) {
    ptyPumping = false;
    return;
  }
  ensureTerminal(snap.appearance.theme);
  if (termHost && termHost.parentElement !== slot) {
    slot.appendChild(termHost);
  }
  fitAddon?.fit();
  void sendPtyResize(run.id);
  if (ptyRunId !== run.id) {
    ptyRunId = run.id;
    ptyOffset = 0;
    term?.reset();
  }
  if (run.status === "ended") {
    ptyPumping = false;
    return;
  }
  if (!ptyPumping) {
    ptyPumping = true;
    void pumpPty();
  }
}

async function pumpMobileOutput(snap: Snapshot): Promise<void> {
  const run = mobileView === "run" ? focusedRun(snap) : undefined;
  if (!run || run.status === "ended" || mobileLiveTerminal) {
    if (run?.status === "ended" && run.recentOutput) {
      mobilePtyText.set(run.id, run.recentOutput);
    }
    mobilePtyPumping = false;
    return;
  }
  if (mobilePtyRunId !== run.id) {
    mobilePtyRunId = run.id;
    mobilePtyOffset = 0;
  }
  if (mobilePtyPumping) return;
  mobilePtyPumping = true;
  const runId = run.id;
  try {
    while (
      mobileClient()
      && mobileView === "run"
      && !mobileLiveTerminal
      && snapshot?.focusedRunId === runId
    ) {
      const response = await fetch(
        `${await protocolBase()}/runs/${encodeURIComponent(runId)}/output?after=${mobilePtyOffset}`,
      );
      if (!response.ok || mobilePtyRunId !== runId) break;
      const json = (await response.json()) as { offset: number; data: string; exited: number | null };
      if (json.data) {
        const raw = atob(json.data);
        const bytes = Uint8Array.from(raw, (byte) => byte.charCodeAt(0));
        const text = new TextDecoder().decode(bytes);
        const recent = `${mobilePtyText.get(runId) ?? ""}${text}`.slice(-16_000);
        mobilePtyText.set(runId, recent);
        const output = app?.querySelector<HTMLElement>(`.mobile-run-output[data-run="${CSS.escape(runId)}"]`);
        if (output) {
          output.textContent = recent;
          output.scrollTop = output.scrollHeight;
        }
      }
      mobilePtyOffset = json.offset;
      if (json.exited != null) {
        await rpc("snapshot");
        render();
        break;
      }
    }
  } catch {
    // Keep the last readable output when the Run or Host disconnects.
  } finally {
    mobilePtyPumping = false;
  }
}

async function sendPtyInput(runId: string, data: string): Promise<void> {
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

async function sendPtyResize(runId: string): Promise<void> {
  const cols = term?.cols ?? 80;
  const rows = term?.rows ?? 24;
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

async function pumpPty(): Promise<void> {
  while (ptyPumping && snapshot?.focusedRunId && ptyRunId === snapshot.focusedRunId) {
    const runId = ptyRunId;
    try {
      const response = await fetch(
        `${await protocolBase()}/runs/${encodeURIComponent(runId)}/output?after=${ptyOffset}`,
      );
      if (response.ok) {
        const json = (await response.json()) as {
          offset: number;
          data: string;
          exited: number | null;
        };
        if (ptyRunId !== runId || snapshot?.focusedRunId !== runId) {
          break;
        }
        if (json.data) {
          const raw = atob(json.data);
          const bytes = new Uint8Array(raw.length);
          for (let i = 0; i < raw.length; i += 1) bytes[i] = raw.charCodeAt(i);
          term?.write(bytes);
        }
        ptyOffset = json.offset;
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
  ptyPumping = false;
}

app.addEventListener("click", async (event) => {
  const target = (event.target as HTMLElement).closest<HTMLElement>("[data-act]");
  if (!target || !snapshot) return;
  if (target.dataset.stop) event.stopPropagation();
  const act = target.dataset.act;
  if (act === "mobile-scope") {
    mobileScopeOpen = true;
    render();
    return;
  }
  if (act === "close-mobile-scope" && event.target === target) {
    mobileScopeOpen = false;
    render();
    return;
  }
  if (act === "mobile-board") {
    mobileView = "board";
    mobileLiveTerminal = false;
    render();
    return;
  }
  if (act === "mobile-issue") {
    mobileView = "issue";
    mobileLiveTerminal = false;
    render();
    return;
  }
  if (act === "mobile-run") {
    if (focusedRun(snapshot)) mobileView = "run";
    render();
    return;
  }
  if (act === "mobile-live-terminal") {
    mobileLiveTerminal = true;
    render();
    return;
  }
  if (act === "close-settings" && event.target === target) {
    settingsOpen = false;
    render();
    return;
  }
  if (act === "toggle-sidebar") {
    sidebarVisible = !sidebarVisible;
    render();
    return;
  }
  if (act === "toggle-issue") {
    issueDetailVisible = !issueDetailVisible;
    render();
    return;
  }
  if (act === "open-overview") {
    sidebarVisible = true;
    await navigateClient({ workspaceView: "host-overview", focusedRunId: "" });
    render();
    return;
  }
  if (act === "return-board") {
    sidebarVisible = sidebarBeforeLift;
    await navigateClient({ workspaceView: "project" });
    render();
    return;
  }
  if (act === "settings") {
    settingsOpen = true;
    await loadStartupSettings();
    pairingOpen = false;
    formOpen = null;
    removeProject = null;
    projectMenuId = "";
    render();
    return;
  }
  if (act === "host-mode" && target.dataset.id) {
    const mode = target.dataset.id as Snapshot["hostMode"];
    if (mode !== snapshot.hostMode) {
      await setHostMode(mode);
    }
    render();
    return;
  }
  if (act === "refresh-launch-environment") {
    launchEnvironmentError = "";
    try {
      const result = await rpc("refreshLaunchEnvironment");
      launchEnvironmentState = result.launchEnvironment ?? launchEnvironmentState;
    } catch (error) {
      launchEnvironmentState = {
        status: "failed",
        refreshedDirectories: 0,
      };
      launchEnvironmentError = error instanceof Error ? error.message : String(error);
    }
    render();
    return;
  }
  if (act === "check-updates") {
    await checkForUpdates(true);
    return;
  }
  if (act === "install-update") {
    await installPendingUpdate();
    return;
  }
  if (act === "update-later") {
    if (event.target !== target && target.closest(".sheet")) return;
    updateState = { kind: "idle" };
    render();
    return;
  }
  if (act === "pairing-noop") {
    return;
  }
  if (act === "close-pairing" && event.target === target) {
    pairingOpen = false;
    pairingError = "";
    render();
    return;
  }
  if (act === "pair") {
    mobileScopeOpen = false;
    pairingOpen = true;
    settingsOpen = false;
    hostPickerOpen = false;
    formOpen = null;
    removeProject = null;
    projectMenuId = "";
    pairingError = "";
    for (const key of formOperations.errors.keys()) {
      if (key.startsWith("pairing:")) formOperations.errors.delete(key);
    }
    render();
    return;
  }
  if (act === "register") {
    mobileScopeOpen = false;
    formOpen = "register";
    formProjectId = "";
    formDraft = emptyDraft();
    autoFilledProjectName = "";
    supersedeProjectInference();
    formError = "";
    removeError = "";
    projectMenuId = "";
    pairingOpen = false;
    settingsOpen = false;
    removeProject = null;
    render();
    return;
  }
  if (act === "form-noop") {
    return;
  }
  if (act === "keyboard-help") {
    keyboardHelpOpen = true;
    render();
    return;
  }
  if (act === "close-keyboard-help") {
    keyboardHelpOpen = false;
    render();
    return;
  }
  if (act === "open-issue" && target.dataset.url) {
    await openExternalUrl(target.dataset.url);
    return;
  }
  if (act === "open-external" && target.dataset.url) {
    event.preventDefault();
    const url = safeHttpUrl(target.dataset.url);
    if (url) await openExternalUrl(url);
    return;
  }
  if (act === "retry-issue-document") {
    await loadSelectedIssueDocument(true);
    render();
    return;
  }
  if (act === "close-form" && (event.target === target || target.tagName === "BUTTON")) {
    if (projectOperation) return;
    if (target.tagName !== "BUTTON") return;
    formOpen = null;
    supersedeProjectInference();
    formError = "";
    render();
    return;
  }
  if (act === "project-menu" && target.dataset.id) {
    projectMenuId = projectMenuId === target.dataset.id ? "" : target.dataset.id;
    render();
    return;
  }
  if (act === "focus-project" && target.dataset.id) {
    projectMenuId = "";
    mobileScopeOpen = false;
    mobileView = "board";
    sidebarVisible = true;
    await navigateClient({
      focusedProjectId: target.dataset.id,
      selectedIssueId: null,
      focusedRunId: "",
      workspaceView: "project",
      parentFilterId: null,
      search: { title: "", triageRole: null, state: "all" },
      graphMode: "overview",
      graphCenterIssueId: null,
      completeDependencyGraph: false,
    });
    resetInlineFormDrafts();
    await reportClientView();
    render();
    return;
  }
  if (act === "new-run" && target.dataset.id) {
    projectMenuId = "";
    settingsOpen = false;
    pairingOpen = false;
    formOpen = null;
    launchDraft = null;
    agentPickerSelection = "";
    clearFormOperation(launchFormKey(target.dataset.id));
    await rpc("prepareRunLaunch", { projectId: target.dataset.id });
    render();
    return;
  }
  if (act === "execute-run" && target.dataset.id && snapshot.focusedProjectId) {
    settingsOpen = false;
    pairingOpen = false;
    formOpen = null;
    launchDraft = null;
    clearFormOperation(launchFormKey(snapshot.focusedProjectId));
    await rpc("prepareRunLaunch", {
      projectId: snapshot.focusedProjectId,
      issueId: target.dataset.id,
    });
    render();
    return;
  }
  if (act === "continue-run" && target.dataset.id) {
    await rpc("continueRun", { issueId: target.dataset.id });
    render();
    return;
  }
  if (act === "release-claim" && target.dataset.id) {
    await rpc("releaseIssue", { issueId: target.dataset.id });
    render();
    return;
  }
  if (act === "close-launch" && (event.target === target || target.tagName === "BUTTON")) {
    const projectId = snapshot.launchForm?.projectId ?? launchDraft?.projectId;
    if (projectId && formOperations.pending.has(launchFormKey(projectId))) return;
    await rpc("cancelRunLaunch");
    launchDraft = null;
    agentPickerSelection = "";
    if (projectId) clearFormOperation(launchFormKey(projectId));
    render();
    return;
  }
  if (act === "switch-agent") {
    const form = snapshot.launchForm;
    if (!form) return;
    agentPickerSelection = form.selectedAgentId;
    clearFormOperation(launchFormKey(form.projectId));
    await rpc("prepareRunLaunch", {
      projectId: form.projectId,
      issueId: form.issueId,
      pickAgent: true,
    });
    render();
    return;
  }
  if (act === "pick-agent" && target.dataset.id) {
    agentPickerSelection = target.dataset.id;
    render();
    return;
  }
  if (act === "confirm-agent") {
    const form = snapshot.launchForm;
    if (!form || !agentPickerSelection) return;
    launchDraft = null;
    clearFormOperation(launchFormKey(form.projectId));
    await rpc("prepareRunLaunch", {
      projectId: form.projectId,
      issueId: form.issueId,
      agentId: agentPickerSelection,
      pickAgent: false,
    });
    agentPickerSelection = "";
    render();
    return;
  }
  if (act === "intent") {
    if (!launchDraft || !snapshot.launchForm) return;
    const intentId = target.dataset.id ?? "";
    launchDraft.intentId = intentId;
    launchDraft.custom = false;
    launchDraft.openingText = expectedOpening(snapshot.launchForm, launchDraft);
    render();
    return;
  }
  if (act === "intent-custom") {
    return;
  }
  if (act === "toggle-folded") {
    launchFolded = !launchFolded;
    return;
  }
  if (act === "focus-run" && target.dataset.id) {
    sidebarBeforeLift = sidebarVisible;
    issueDetailVisible = true;
    const run = snapshot.runs.find((item) => item.id === target.dataset.id);
    await navigateClient({
      focusedProjectId: run?.projectId ?? clientView.focusedProjectId,
      selectedIssueId: run?.issueId ?? clientView.selectedIssueId,
      focusedRunId: target.dataset.id,
      workspaceView: "run",
    });
    if (mobileClient()) {
      mobileView = "run";
      mobileLiveTerminal = false;
    } else {
      sidebarVisible = false;
    }
    render();
    await loadSelectedIssueDocument();
    render();
    return;
  }
  if (act === "open-usage") {
    mobileScopeOpen = false;
    settingsOpen = false;
    pairingOpen = false;
    formOpen = null;
    await navigateClient({ usageOpen: true });
    render();
    return;
  }
  if (act === "close-usage") {
    await navigateClient({
      usageOpen: false,
      usageQuery: { ...clientView.usageQuery, highlightedRunId: null },
    });
    render();
    return;
  }
  if (act === "usage-range" && target.dataset.id) {
    usageCustomDraft = null;
    clearFormOperation(usageCustomFormKey(snapshot.focusedHostId));
    await navigateClient({
      usageQuery: {
        ...clientView.usageQuery,
        range: target.dataset.id as UsageRange,
      },
    });
    render();
    return;
  }
  if (act === "open-usage-run" && target.dataset.id) {
    await navigateClient({
      usageOpen: true,
      usageQuery: { ...clientView.usageQuery, highlightedRunId: target.dataset.id },
    });
    render();
    return;
  }
  if (act === "open-run-usage" && target.dataset.id) {
    const run = snapshot.runs.find((item) => item.id === target.dataset.id);
    if (!run) return;
    await navigateClient({
      focusedProjectId: run.projectId,
      selectedIssueId: run.issueId ?? null,
      focusedRunId: run.id,
      workspaceView: "run",
      usageOpen: false,
      usageQuery: { ...clientView.usageQuery, highlightedRunId: null },
    });
    render();
    return;
  }
  if (act === "toggle-telemetry") {
    telemetryExpanded = !telemetryExpanded;
    render();
    return;
  }
  if (act === "stop-run" && target.dataset.id) {
    await rpc("stopRun", { runId: target.dataset.id });
    if (mobileClient()) {
      mobileView = "board";
      mobileLiveTerminal = false;
    }
    render();
    return;
  }
  if (act === "view-changes" && target.dataset.id) {
    changesOpen = true;
    changesScope = "this-round";
    noteTarget = null;
    noteDraft = "";
    clearFormOperation(changeNoteFormKey(target.dataset.id));
    await loadViewChanges(target.dataset.id, changesScope);
    render();
    return;
  }
  if (act === "close-changes") {
    const runId = changesView?.runId;
    changesOpen = false;
    changesView = null;
    noteTarget = null;
    noteDraft = "";
    if (runId) clearFormOperation(changeNoteFormKey(runId));
    render();
    return;
  }
  if (act === "changes-scope" && target.dataset.id) {
    const scope = target.dataset.id === "uncommitted" ? "uncommitted" : "this-round";
    changesScope = scope;
    const runId = changesView?.runId ?? snapshot.focusedRunId;
    if (runId) await loadViewChanges(runId, scope);
    render();
    return;
  }
  if (act === "note-line" && target.dataset.repo && target.dataset.path && target.dataset.line) {
    noteTarget = {
      repo: target.dataset.repo,
      path: target.dataset.path,
      line: Number(target.dataset.line),
    };
    noteDraft = "";
    if (changesView) clearFormOperation(changeNoteFormKey(changesView.runId));
    render();
    const input = app.querySelector<HTMLInputElement>(".note-form input");
    input?.focus();
    return;
  }
  if (act === "delete-note" && target.dataset.id) {
    await rpc("deleteChangeNote", { noteId: target.dataset.id });
    const runId = changesView?.runId ?? snapshot.focusedRunId;
    if (runId) await loadViewChanges(runId, changesScope);
    render();
    return;
  }
  if (act === "cancel-quit") {
    await rpc("cancelQuit");
    render();
    return;
  }
  if (act === "confirm-quit") {
    await rpc("confirmQuitStopAll");
    render();
    return;
  }
  if (act === "refresh") {
    refreshing = true;
    render();
    try {
      await refreshProject();
    } finally {
      refreshing = false;
    }
    render();
    return;
  }
  if (act === "edit-project" && target.dataset.id) {
    mobileScopeOpen = false;
    const project = snapshot.projects.find((item) => item.id === target.dataset.id);
    if (!project) return;
    formOpen = "edit";
    formProjectId = project.id;
    formDraft = {
      name: project.name,
      localPath: project.localPath,
      githubHost: project.githubHost,
      repository: project.repository,
    };
    autoFilledProjectName = "";
    supersedeProjectInference();
    formError = "";
    removeError = "";
    projectMenuId = "";
    render();
    return;
  }
  if (act === "remove-project" && target.dataset.id) {
    mobileScopeOpen = false;
    removeProject = snapshot.projects.find((item) => item.id === target.dataset.id) ?? null;
    removeError = "";
    projectMenuId = "";
    render();
    return;
  }
  if (act === "close-remove" && (event.target === target || target.tagName === "BUTTON")) {
    if (projectOperation) return;
    removeProject = null;
    removeError = "";
    render();
    return;
  }
  if (act === "confirm-remove" && removeProject) {
    if (projectOperation) return;
    removeError = "";
    projectOperation = "remove";
    render();
    try {
      await rpc("removeProject", { projectId: removeProject.id });
      removeProject = null;
    } catch (error) {
      removeError = error instanceof Error ? error.message : String(error);
    } finally {
      projectOperation = null;
    }
    render();
    return;
  }
  if (act === "choose-project-directory") {
    event.preventDefault();
    event.stopPropagation();
    await chooseProjectDirectory();
    return;
  }
  if (act === "apply-infer" && projectInference.status === "candidate") {
    const candidate = projectInference.candidate;
    const useCandidateName = !formDraft.name.trim() || formDraft.name.trim() === autoFilledProjectName;
    formDraft = {
      name: useCandidateName ? candidate.name : formDraft.name,
      localPath: candidate.localPath,
      githubHost: candidate.githubHost,
      repository: candidate.repository,
      tracker: candidate.tracker,
    };
    autoFilledProjectName = useCandidateName ? candidate.name : "";
    projectInference = { status: "idle", requestId: projectInference.requestId };
    render();
    return;
  }
  if (act === "retry-infer") {
    await inferFromLocalPath(formDraft.localPath);
    return;
  }
  if (act === "toggle-hosts") {
    hostPickerOpen = snapshot.hosts.length > 1 ? !hostPickerOpen : false;
    render();
    return;
  }
  if (act === "forget-host" && target.dataset.id) {
    forgetHostId = target.dataset.id;
    forgetHostError = "";
    hostPickerOpen = false;
    render();
    return;
  }
  if (act === "close-forget-host" && (event.target === target || target.tagName === "BUTTON")) {
    if (forgetHostPending) return;
    forgetHostId = "";
    forgetHostError = "";
    render();
    return;
  }
  if (act === "confirm-forget-host" && forgetHostId) {
    if (forgetHostPending) return;
    forgetHostPending = true;
    forgetHostError = "";
    render();
    try {
      await rpc("forgetRemoteHost", { hostId: forgetHostId });
      forgetHostId = "";
      hostPickerOpen = false;
    } catch (error) {
      forgetHostError = error instanceof Error ? error.message : String(error);
    } finally {
      forgetHostPending = false;
    }
    render();
    return;
  }
  if (act === "focus-host" && target.dataset.id) {
    await reportClientView(false);
    await navigateClient({
      focusedHostId: target.dataset.id,
      focusedProjectId: "",
      selectedIssueId: null,
      focusedRunId: "",
      workspaceView: "project",
      parentFilterId: null,
      search: { title: "", triageRole: null, state: "all" },
      graphMode: "overview",
      graphCenterIssueId: null,
      completeDependencyGraph: false,
    });
    resetInlineFormDrafts();
    hostPickerOpen = false;
    await reportClientView();
    render();
    return;
  }
  if (act === "show-offer") {
    pairingError = "";
    const addressInput = app.querySelector<HTMLInputElement>("[data-field='address']");
    pairingAddress = addressInput?.value ?? pairingAddress;
    await runFormOperation(pairingFormKey("offer"), async () => {
      await rpc("beginPairingOffer", { address: pairingAddress });
    });
    return;
  }
  if (act === "copy-offer" && snapshot.pairingOffer) {
    await navigator.clipboard.writeText(snapshot.pairingOffer.text);
    return;
  }
  if (act === "revoke" && target.dataset.id) {
    pairingError = "";
    await runFormOperation(pairingFormKey(`revoke:${target.dataset.id}`), async () => {
      await rpc("revokeClient", { clientId: target.dataset.id });
    });
    return;
  }
  if (act === "connect-host") {
    pairingError = "";
    clearFormOperation(pairingFormKey("connect"));
    const pasteInput = app.querySelector<HTMLTextAreaElement>("[data-field='paste']");
    pairingPaste = pasteInput?.value ?? pairingPaste;
    const parsed = parsePairingPayload(pairingPaste);
    if (!parsed) {
      pairingError = snapshot.copy.pairingPaste;
      render();
      return;
    }
    await runFormOperation(pairingFormKey("connect"), async () => {
      await rpc("pairRemoteHost", parsed);
      pairingPaste = "";
      pairingOpen = false;
    });
    return;
  }
  if (act === "language" && target.dataset.id) {
    if (mobileClient()) {
      const appearance = ensureMobileAppearance();
      saveMobileAppearance({ ...appearance, language: target.dataset.id as Language });
    } else {
      await rpc("setLanguage", { language: target.dataset.id });
    }
    render();
    return;
  }
  if (act === "theme" && target.dataset.id) {
    if (mobileClient()) {
      const appearance = ensureMobileAppearance();
      const theme = target.dataset.id as Theme;
      saveMobileAppearance({
        ...appearance,
        theme,
        lastLightTheme: theme === "plain-night" ? appearance.lastLightTheme : theme,
      });
    } else {
      await rpc("setTheme", { theme: target.dataset.id });
    }
    render();
    return;
  }
  if (act === "shade") {
    const current = mobileClient() ? ensureMobileAppearance() : snapshot.appearance;
    const next = target.dataset.id === "dark" ? "plain-night" : current.lastLightTheme;
    if (mobileClient()) {
      saveMobileAppearance({ ...current, theme: next });
    } else {
      await rpc("setTheme", { theme: next });
    }
    render();
    return;
  }
  if (act === "quit") {
    settingsOpen = false;
    await rpc("quitHost");
    render();
    return;
  }
  if (act === "veto-advance" && target.dataset.id) {
    await rpc("vetoPendingConfirmation", { projectId: target.dataset.id });
    render();
    return;
  }
  if (act === "center-view" && target.dataset.id) {
    if (target.dataset.id === "graph") {
      resetGraphUiState();
      await navigateClient({
        centerView: "graph",
        selectedIssueId: null,
        graphMode: "overview",
        graphCenterIssueId: null,
        completeDependencyGraph: false,
      });
    } else {
      await navigateClient({ centerView: "board" });
    }
    render();
    return;
  }
  if (act === "center-graph" && target.dataset.id) {
    pendingGraphAnchor = captureGraphAnchor(target.dataset.id);
    const fromOverview = snapshot.board?.graph?.mode === "overview";
    await navigateClient({
      selectedIssueId: target.dataset.id,
      graphMode: "focused",
      graphCenterIssueId: target.dataset.id,
      completeDependencyGraph: fromOverview ? false : clientView.completeDependencyGraph,
    });
    render();
    await loadSelectedIssueDocument();
    render();
    return;
  }
  if (act === "graph-overview") {
    resetGraphUiState();
    await navigateClient({
      selectedIssueId: null,
      graphMode: "overview",
      graphCenterIssueId: null,
      completeDependencyGraph: false,
    });
    render();
    return;
  }
  if (act === "view-dependencies" && target.dataset.id) {
    pendingGraphAnchor = captureGraphAnchor(target.dataset.id);
    resetGraphUiState();
    issueDetailVisible = true;
    await navigateClient({
      selectedIssueId: target.dataset.id,
      focusedRunId: "",
      centerView: "graph",
      workspaceView: "project",
      graphMode: "focused",
      graphCenterIssueId: target.dataset.id,
      completeDependencyGraph: false,
    });
    render();
    await loadSelectedIssueDocument();
    render();
    return;
  }
  if (act === "graph-complete") {
    resetGraphUiState();
    await navigateClient({ completeDependencyGraph: true });
    render();
    return;
  }
  if (act === "graph-neighborhood") {
    resetGraphUiState();
    await navigateClient({ completeDependencyGraph: false });
    render();
    return;
  }
  if (act === "graph-more") {
    graphCanvasLimit += 48;
    render();
    return;
  }
  if (act === "graph-list-more") {
    graphListLimit += 50;
    render();
    return;
  }
  if (act === "focus-issue" && target.dataset.id) {
    issueDetailVisible = true;
    await navigateClient({
      selectedIssueId: target.dataset.id,
      focusedRunId: "",
      workspaceView: "project",
    });
    render();
    await loadSelectedIssueDocument();
    if (mobileClient()) {
      mobileView = "issue";
      mobileLiveTerminal = false;
    } else if (target.closest(".issue-card") && snapshot.focusedRunId) {
      sidebarBeforeLift = sidebarVisible;
      await navigateClient({ workspaceView: "run" });
      sidebarVisible = false;
    }
    render();
    return;
  }
  if (act === "filter-parent" && target.dataset.id) {
    await navigateClient({ parentFilterId: target.dataset.id });
    render();
    return;
  }
  if (act === "clear-filter") {
    await navigateClient({ parentFilterId: null });
    render();
  }
});

app.addEventListener("submit", async (event) => {
  const search = (event.target as HTMLElement | null)?.closest<HTMLFormElement>("form[data-act='issue-search']");
  if (search && snapshot) {
    event.preventDefault();
    const data = new FormData(search);
    const draft = {
      title: String(data.get("title") ?? ""),
      triageRole: String(data.get("triageRole") ?? ""),
      state: String(data.get("state") ?? "all"),
    };
    issueSearchDraft = draft;
    const key = issueSearchFormKey(snapshot.focusedProjectId);
    const success = await runFormOperation(key, async () => {
      await navigateClient(
        {
          search: {
            title: draft.title,
            triageRole: (draft.triageRole || null) as TriageRole | null,
            state: draft.state === "open" || draft.state === "closed" ? draft.state : "all",
          },
        },
        { clientAction: "searchIssues" },
      );
    });
    if (success) {
      issueSearchDraft = null;
      keyboardCursorIssueId = "";
      render();
    }
    return;
  }
  const inject = (event.target as HTMLElement | null)?.closest<HTMLFormElement>("form[data-act='inject-run']");
  if (inject && snapshot) {
    event.preventDefault();
    const runId = inject.dataset.id;
    const input = inject.querySelector<HTMLInputElement>("input[name='text']");
    const text = input?.value ?? "";
    if (!runId || !text.trim()) return;
    injectDrafts.set(runId, text);
    const success = await runFormOperation(injectFormKey(runId), async () => {
      await rpc("injectRunInput", { runId, text });
    });
    if (success) {
      injectDrafts.delete(runId);
      render();
    }
    return;
  }
  const noteForm = (event.target as HTMLElement | null)?.closest<HTMLFormElement>("form[data-act='write-note']");
  if (!noteForm || !snapshot || !noteTarget || !changesView) return;
  event.preventDefault();
  const input = noteForm.querySelector<HTMLInputElement>("input[name='text']");
  const text = input?.value ?? noteDraft;
  if (!text.trim()) return;
  noteDraft = text;
  const target = { ...noteTarget };
  const runId = changesView.runId;
  const success = await runFormOperation(changeNoteFormKey(runId), async () => {
    await rpc("writeChangeNote", {
      runId,
      repo: target.repo,
      path: target.path,
      line: target.line,
      text,
    });
    await loadViewChanges(runId, changesScope);
  });
  if (success) {
    noteDraft = "";
    noteTarget = null;
    render();
  }
});

app.addEventListener("input", (event) => {
  const target = event.target as HTMLInputElement | null;
  if (!target) return;
  if (target.getAttribute("data-field") === "graphSearch") {
    graphListQuery = target.value;
    graphListLimit = 50;
    render();
    return;
  }
  if (!formOpen) return;
  const field = target.getAttribute("data-field");
  if (field === "name" || field === "localPath" || field === "githubHost" || field === "repository") {
    formDraft = { ...formDraft, [field]: target.value };
    if (field === "name") autoFilledProjectName = "";
    if (field === "localPath") target.title = target.value;
  }
});

app.addEventListener("change", async (event) => {
  const target = event.target as HTMLElement | null;
  if (!target || !snapshot) return;
  if (target.getAttribute("data-field") === "localPath" && "value" in target) {
    applyLocalPath((target as HTMLInputElement).value, true);
    return;
  }
  if (target.getAttribute("data-field") === "startAtLogin" && "checked" in target) {
    await setStartAtLogin((target as HTMLInputElement).checked);
    render();
  }
  if (target.getAttribute("data-field") === "refreshInterval" && "value" in target) {
    const seconds = Number((target as HTMLInputElement).value);
    if (!Number.isFinite(seconds)) return;
    await rpc("setRefreshInterval", { intervalMs: Math.max(0, seconds) * 1000 });
    render();
  }
  if (target.getAttribute("data-field") === "recentLimit" && "value" in target) {
    const limit = Number((target as HTMLInputElement).value);
    if (!Number.isFinite(limit)) return;
    await rpc("setRecentCompletedLimit", { limit });
    render();
  }
  if (target.getAttribute("data-field") === "showEndedRuns" && "checked" in target) {
    overviewShowEnded = (target as HTMLInputElement).checked;
    render();
  }
  if (target.getAttribute("data-field") === "closedContext" && "checked" in target) {
    await rpc("setShowClosedGraphContext", {
      show: (target as HTMLInputElement).checked,
    });
    render();
  }
  if (target.getAttribute("data-field") === "commandPreview" && "checked" in target) {
    await rpc("setShowCommandPreview", {
      show: (target as HTMLInputElement).checked,
    });
    render();
  }
  if (
    (target.getAttribute("data-field") === "notifyDesktop" ||
      target.getAttribute("data-field") === "notifySound") &&
    "checked" in target
  ) {
    const desktop =
      target.getAttribute("data-field") === "notifyDesktop"
        ? (target as HTMLInputElement).checked
        : Boolean(snapshot.notifyDesktop);
    const sound =
      target.getAttribute("data-field") === "notifySound"
        ? (target as HTMLInputElement).checked
        : Boolean(snapshot.notifySound);
    if (desktop && typeof Notification !== "undefined" && Notification.permission === "default") {
      await Notification.requestPermission();
    }
    await rpc("setNotificationPrefs", { desktop, sound });
    render();
  }
  if (target.getAttribute("data-field") === "hostAutoAdvance" && "checked" in target) {
    await rpc("setHostAutoAdvance", {
      enabled: (target as HTMLInputElement).checked,
    });
    render();
  }
  if (target.getAttribute("data-field") === "projectAutoAdvance" && "checked" in target) {
    const projectId = snapshot.focusedProjectId;
    if (projectId) {
      await rpc("setProjectAutoAdvance", {
        projectId,
        enabled: (target as HTMLInputElement).checked,
      });
      render();
    }
  }
  if (target.getAttribute("data-field") === "restoreAutoAdvance" && "checked" in target) {
    const projectId = snapshot.focusedProjectId;
    if (projectId) {
      await rpc("setProjectRestoreAutoAdvance", {
        projectId,
        enabled: (target as HTMLInputElement).checked,
      });
      render();
    }
  }
  if (target.getAttribute("data-field") === "restoreDelay" && "value" in target) {
    const projectId = snapshot.focusedProjectId;
    const seconds = Number((target as HTMLInputElement).value);
    if (projectId && Number.isFinite(seconds)) {
      await rpc("setProjectRestoreDelay", {
        projectId,
        delayMs: Math.max(0, seconds) * 1000,
      });
      render();
    }
  }
  const launchId = target.getAttribute("data-launch");
  if (launchId && launchDraft) {
    if (target instanceof HTMLInputElement && target.type === "checkbox") {
      launchDraft.values[launchId] = target.checked ? "true" : "false";
    } else if ("value" in target) {
      launchDraft.values[launchId] = (target as HTMLInputElement | HTMLSelectElement).value;
    }
    refreshLaunchWarnings();
  }
});

app.addEventListener("input", (event) => {
  const target = event.target as HTMLElement | null;
  if (!target) return;
  const field = target.getAttribute("data-field");
  if (field === "address" && "value" in target) {
    pairingAddress = (target as HTMLInputElement).value;
  }
  if (field === "paste" && "value" in target) {
    pairingPaste = (target as HTMLTextAreaElement).value;
  }
  if (target.closest(".note-form") && "value" in target) {
    noteDraft = (target as HTMLInputElement).value;
  }
  const injectForm = target.closest<HTMLFormElement>("form[data-act='inject-run']");
  if (injectForm?.dataset.id && "value" in target) {
    injectDrafts.set(injectForm.dataset.id, (target as HTMLInputElement).value);
  }
  if (target.closest("form[data-act='issue-search']") && "value" in target) {
    const name = (target as HTMLInputElement).name;
    if (name === "title") editableIssueSearchDraft().title = (target as HTMLInputElement).value;
  }
  if (target.closest("form[data-act='usage-custom']") && "value" in target) {
    const input = target as HTMLInputElement;
    const usage = snapshot?.usage;
    if (usage && !usageCustomDraft) {
      usageCustomDraft = { from: toLocalInput(usage.fromMs), to: toLocalInput(usage.toMs) };
    }
    if (usageCustomDraft && (input.name === "from" || input.name === "to")) {
      usageCustomDraft[input.name] = input.value;
    }
  }
  if (
    (field === "name" || field === "githubHost" || field === "repository") &&
    "value" in target
  ) {
    formDraft = { ...formDraft, [field]: (target as HTMLInputElement).value };
  }
  if (field === "openingText" && launchDraft && "value" in target) {
    launchDraft.openingText = (target as HTMLTextAreaElement).value;
    if (!launchDraft.intentId) {
      launchDraft.values["initial-instruction"] = launchDraft.openingText;
      launchDraft.custom = false;
    } else if (snapshot?.launchForm) {
      launchDraft.custom =
        launchDraft.openingText.trim() !== expectedOpening(snapshot.launchForm, launchDraft).trim();
      refreshIntentChoices();
    }
  }
  const launchId = target.getAttribute("data-launch");
  if (launchId && launchDraft && "value" in target && !(target instanceof HTMLInputElement && target.type === "checkbox")) {
    launchDraft.values[launchId] = (target as HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement).value;
    refreshLaunchWarnings();
  }
});

app.addEventListener("change", async (event) => {
  const target = event.target as HTMLElement | null;
  if (target instanceof HTMLSelectElement && target.closest("form[data-act='issue-search']")) {
    const draft = editableIssueSearchDraft();
    if (target.name === "triageRole") draft.triageRole = target.value;
    if (target.name === "state") draft.state = target.value;
  }
  if (target?.getAttribute("data-overview-filter") === "project" && target instanceof HTMLSelectElement) {
    overviewProjectId = target.value;
    render();
    return;
  }
  const filter = target?.getAttribute("data-usage-filter");
  if (!filter || !snapshot?.usage || !(target instanceof HTMLSelectElement)) return;
  const next = {
    projectId: snapshot.usage.filter.projectId ?? "",
    agentId: snapshot.usage.filter.agentId ?? "",
    model: snapshot.usage.filter.model ?? "",
  };
  if (filter === "projectId") next.projectId = target.value;
  if (filter === "agentId") next.agentId = target.value;
  if (filter === "model") next.model = target.value;
  await navigateClient({
    usageQuery: {
      ...clientView.usageQuery,
      filter: {
        projectId: next.projectId || null,
        agentId: next.agentId || null,
        model: next.model || null,
      },
    },
  });
  render();
});

app.addEventListener("submit", async (event) => {
  const custom = (event.target as HTMLElement | null)?.closest<HTMLFormElement>("[data-act='usage-custom']");
  if (custom) {
    event.preventDefault();
    const data = new FormData(custom);
    const draft = {
      from: String(data.get("from") ?? ""),
      to: String(data.get("to") ?? ""),
    };
    const from = Date.parse(draft.from);
    const to = Date.parse(draft.to);
    if (Number.isNaN(from) || Number.isNaN(to)) return;
    usageCustomDraft = draft;
    const key = usageCustomFormKey(snapshot?.focusedHostId ?? "");
    const success = await runFormOperation(key, async () => {
      await navigateClient({
        usageQuery: {
          ...clientView.usageQuery,
          range: "custom",
          customFromMs: from,
          customToMs: to,
        },
      }, { clientAction: "setUsageRange" });
    });
    if (success) {
      usageCustomDraft = null;
      render();
    }
    return;
  }
  const launch = (event.target as HTMLElement | null)?.closest<HTMLFormElement>("[data-form='launch']");
  if (launch && snapshot && launchDraft) {
    event.preventDefault();
    const draft = launchDraft;
    const existingRunIds = new Set(snapshot.runs.map((run) => run.id));
    await runFormOperation(launchFormKey(draft.projectId), async () => {
      const result = await rpc("startUnboundRun", {
        projectId: draft.projectId,
        issueId: draft.issueId,
        agentId: draft.agentId,
        values: draft.values,
        openingText: draft.openingText,
      });
      const created = result.snapshot.runs.find((run) => !existingRunIds.has(run.id));
      if (created && !result.snapshot.launchForm) {
        await navigateClient({
          focusedProjectId: created.projectId,
          selectedIssueId: created.issueId ?? null,
          focusedRunId: created.id,
          workspaceView: "project",
        });
      }
    });
    return;
  }
  const form = (event.target as HTMLElement | null)?.closest<HTMLFormElement>("[data-form='project']");
  if (!form || !snapshot) return;
  event.preventDefault();
  if (projectOperation) return;
  supersedeProjectInference();
  formError = "";
  projectOperation = "save";
  render();
  try {
    if (formOpen === "edit") {
      await rpc("editProject", { projectId: formProjectId, ...formDraft });
    } else {
      const existingProjectIds = new Set(snapshot.projects.map((project) => project.id));
      const result = await rpc("registerProject", formDraft);
      const created = result.snapshot.projects.find((project) => !existingProjectIds.has(project.id));
      if (created) {
        await navigateClient({
          focusedProjectId: created.id,
          selectedIssueId: null,
          focusedRunId: "",
          workspaceView: "project",
          parentFilterId: null,
          search: { title: "", triageRole: null, state: "all" },
          graphMode: "overview",
          graphCenterIssueId: null,
          completeDependencyGraph: false,
        });
      }
    }
    formOpen = null;
  } catch (error) {
    formError = error instanceof Error ? error.message : String(error);
  } finally {
    projectOperation = null;
  }
  render();
});

function parsePairingPayload(raw: string): { address: string; code: string } | null {
  const parts = raw
    .trim()
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  if (parts.length < 2) return null;
  const address = parts[0];
  const code = parts[parts.length - 1];
  if (!address.includes("://") || !code) return null;
  return { address, code };
}

function shouldReportClientView(): boolean {
  return !desktopShellAvailable() || !focusedHostIsLocal();
}

let hostWindowVisible = true;
let lastReportedView = { projectId: "", visible: false };

function clientIsVisible(): boolean {
  return hostWindowVisible && document.visibilityState === "visible";
}

async function reportClientView(visible = clientIsVisible()): Promise<boolean> {
  if (!shouldReportClientView()) return false;
  const projectId = visible ? snapshot?.focusedProjectId ?? "" : "";
  const changed = visible !== lastReportedView.visible || projectId !== lastReportedView.projectId;
  lastReportedView = { projectId, visible };
  await rpc("setClientView", { clientId, projectId, visible });
  return changed;
}

let foregroundRefresh: Promise<void> | null = null;

function onClientForegroundOrHidden(): void {
  if (!shouldReportClientView()) return;
  if (!clientIsVisible()) {
    void reportClientView(false).then(render).catch(() => {});
    return;
  }
  if (foregroundRefresh) return;
  foregroundRefresh = (async () => {
    try {
      const changed = await reportClientView(true);
      if (!clientIsVisible()) {
        await reportClientView(false);
        return;
      }
      if (changed || clientView.focusedProjectId) await refreshProject();
      render();
    } finally {
      foregroundRefresh = null;
    }
  })();
  void foregroundRefresh.catch(() => {});
}

function ensureTick(): void {
  if (tickTimer != null) return;
  tickTimer = window.setInterval(() => {
    const extra = shouldReportClientView()
      ? {
          clientId,
          projectId: snapshot?.focusedProjectId ?? "",
          visible: clientIsVisible(),
        }
      : {};
    rpc("tick", extra).then(renderAfterTick).catch(() => {});
  }, 1000);
}

function renderAfterTick(): void {
  if (!snapshot || snapshotRenderKey(snapshot) === renderedSnapshotKey) {
    tickRenderPending = false;
    return;
  }
  if (activePointers.size > 0) {
    tickRenderPending = true;
    return;
  }
  render();
}

function finishPointerInteraction(pointerId: number): void {
  activePointers.delete(pointerId);
  if (activePointers.size > 0 || !tickRenderPending) return;
  window.setTimeout(() => {
    if (activePointers.size > 0 || !tickRenderPending) return;
    tickRenderPending = false;
    render();
  }, 0);
}

document.addEventListener("pointerdown", (event) => activePointers.add(event.pointerId), true);
document.addEventListener("pointerup", (event) => finishPointerInteraction(event.pointerId), true);
document.addEventListener("pointercancel", (event) => finishPointerInteraction(event.pointerId), true);
window.addEventListener("blur", () => {
  for (const pointerId of activePointers) finishPointerInteraction(pointerId);
});

document.addEventListener("visibilitychange", onClientForegroundOrHidden);

function terminalHasFocus(): boolean {
  const active = document.activeElement as HTMLElement | null;
  return Boolean(active?.closest(".pty-host"));
}

function typingTarget(target: EventTarget | null): boolean {
  const element = target as HTMLElement | null;
  return Boolean(element?.closest("input, textarea, select, [contenteditable='true']"));
}

document.addEventListener("keydown", (event) => {
  if (!snapshot || terminalHasFocus()) return;
  if (event.key === "?" && !typingTarget(event.target)) {
    event.preventDefault();
    keyboardHelpOpen = !keyboardHelpOpen;
    render();
    return;
  }
  if (event.key === "Escape") {
    if (keyboardHelpOpen) {
      keyboardHelpOpen = false;
      render();
    }
    return;
  }
  if (event.key === "/" && !typingTarget(event.target)) {
    event.preventDefault();
    app.querySelector<HTMLInputElement>("#issue-title-search")?.focus();
    return;
  }
  if (
    typingTarget(event.target)
    || keyboardHelpOpen
    || settingsOpen
    || pairingOpen
    || formOpen
    || Boolean(removeProject)
    || Boolean(snapshot.launchForm)
    || Boolean(snapshot.quitOffer)
    || changesOpen
  ) return;
  const cards = [...app.querySelectorAll<HTMLButtonElement>(".issue-card-main")];
  if (!cards.length) return;
  if (["j", "J", "ArrowDown", "k", "K", "ArrowUp"].includes(event.key)) {
    event.preventDefault();
    const direction = ["k", "K", "ArrowUp"].includes(event.key) ? -1 : 1;
    const focusedIndex = cards.findIndex((card) => card === document.activeElement);
    const rememberedIndex = cards.findIndex((card) => card.dataset.issueId === keyboardCursorIssueId);
    const currentIndex = focusedIndex >= 0 ? focusedIndex : rememberedIndex;
    const nextIndex = (currentIndex + direction + cards.length) % cards.length;
    const nextCard = cards[nextIndex];
    keyboardCursorIssueId = nextCard?.dataset.issueId ?? "";
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

let wasMobileClient = mobileClient();

window.addEventListener("resize", () => {
  if (!snapshot) return;
  const isMobile = mobileClient();
  if (isMobile !== wasMobileClient) {
    wasMobileClient = isMobile;
    mobileLiveTerminal = false;
    render();
  }
  fitAddon?.fit();
  const runId = snapshot.focusedRunId;
  if (runId && (!isMobile || mobileLiveTerminal)) void sendPtyResize(runId);
});

rpc("snapshot")
  .then(async () => {
    render();
    ensureTick();
    await reportClientView();
    if (shouldReportClientView() && clientView.focusedProjectId) await refreshProject();
    render();
    if (desktopShellAvailable() && !startupUpdateChecked && snapshot?.windowVisible) {
      startupUpdateChecked = true;
      window.setTimeout(() => {
        if (updateState.kind === "idle") void checkForUpdates(false);
      }, 250);
    }
  })
  .catch((error: unknown) => {
    if (app) {
      app.textContent = error instanceof Error ? error.message : String(error);
    }
  });
