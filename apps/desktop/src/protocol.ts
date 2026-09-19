export type Language = "zh-CN" | "en";
export type AppearancePreference = "system" | "light" | "dark" | "warm";
export type ResolvedTheme = "light" | "dark" | "warm";
export type SystemAppearance = "light" | "dark";
export type AppearanceState = {
  language: Language;
  appearancePreference: AppearancePreference;
  appearancePreferences: AppearancePreference[];
};
export type SetAppearancePreferenceRequest = {
  appearancePreference: AppearancePreference;
};

export type ShellCopy = {
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
  languageZh: string;
  languageEn: string;
  hosts: string;
  projects: string;
  thisMachine: string;
  editMenu: string;
  windowMenu: string;
  helpMenu: string;
  editUndo: string;
  editRedo: string;
  editCut: string;
  editCopy: string;
  editPaste: string;
  editSelectAll: string;
  usageGuide: string;
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
  noAgentSelected: string;
  nextStep: string;
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
  createIssue: string;
  editIssue: string;
  saveIssue: string;
  issueTitle: string;
  issueBody: string;
  addComment: string;
  commentPlaceholder: string;
  parentIssue: string;
  dependencyBlockers: string;
  clearDependency: string;
  saveRelations: string;
  closeIssue: string;
  reopenIssue: string;
  issueUpdates: string;
  issueConflict: string;
  issueConflictLatest: string;
  issueConflictUseLatest: string;
  issueConflictOverwrite: string;
};

export type CredentialSource = "app-env" | "secrets-file" | "cli" | "generic-env";

export type Repair = {
  cliDetected: boolean;
  secretsPath: string;
  appEnv: string;
  genericEnv: string;
  suggestedScope: string;
};

export type ProjectConnection =
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

export type Project = {
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

export type ProjectIssueCounts = {
  dataAvailable: boolean;
  total: number;
  open: number;
  closed: number;
  blocked: number;
  frontier: number;
  inProgress: number;
};

export type ProjectDraft = {
  name: string;
  localPath: string;
  githubHost: string;
  repository: string;
  tracker?: "github" | "local-markdown";
  ambiguous?: boolean;
};

export type IssueContentDraft = {
  title: string;
  body: string;
  baseTitle: string;
  baseBody: string;
};

export type IssueRelationDraft = {
  parent: string;
  blockedBy: string[];
  baseParent: string;
  baseBlockedBy: string[];
};

export type IssueConflict = {
  issueId: string;
  fields: string[];
  latest: {
    title?: string;
    body?: string;
    parent?: string;
    blockedBy?: string[];
  };
};

export class RpcHttpError extends Error {
  constructor(
    message: string,
    readonly status: number,
    readonly code = "",
    readonly conflict: IssueConflict | null = null,
  ) {
    super(message);
    this.name = "RpcHttpError";
  }
}

export type FormKey =
  | "pairing"
  | `issue-search:${string}`
  | `issue-create:${string}`
  | `issue-edit:${string}`
  | `issue-comment:${string}`
  | `issue-parent:${string}`
  | `issue-blockers:${string}`
  | `issue-open:${string}`
  | `inject-run:${string}`
  | `change-note:${string}`
  | `usage-custom:${string}`
  | `launch:${string}`;

export type IssueSearchDraft = {
  projectId: string;
  title: string;
  triageRole: string;
  state: string;
};

export type UsageCustomDraft = {
  hostId: string;
  from: string;
  to: string;
};

export type FormOperationState = {
  pending: Set<FormKey>;
  errors: Map<FormKey, string>;
  conflicts: Map<FormKey, IssueConflict>;
};

export type ProjectInferenceState =
  | { status: "idle"; requestId: number }
  | { status: "pending"; requestId: number }
  | { status: "candidate"; requestId: number; candidate: ProjectDraft }
  | { status: "failed"; requestId: number; message: string };

export type TriageRole =
  | "needs-triage"
  | "needs-info"
  | "ready-for-agent"
  | "ready-for-human"
  | "wontfix";

export type IssueCard = {
  id: string;
  repository: string;
  number: number;
  title: string;
  url: string;
  claimedBy: string[];
  labels: string[];
  triageRole: TriageRole | null;
  open: boolean;
  activity?: "running" | "waiting" | "execution-stopped" | null;
  runId?: string | null;
};

export type IssueLink = {
  id: string;
  repository: string;
  number: number | null;
  title: string;
  open: boolean | null;
  visible: boolean;
};

export type IssueDetail = {
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

export type IssueDocumentFailure = {
  kind: "offline" | "rate-limited" | "auth" | "tracker";
  message: string;
  retryAfterMs?: number | null;
};

export type IssueDocumentState =
  | { kind: "unloaded" }
  | { kind: "loading"; body?: string | null; fetchedAtMs?: number | null }
  | { kind: "ready"; body: string; editableBody?: string | null; fetchedAtMs: number }
  | { kind: "stale"; body: string; fetchedAtMs: number; failure: IssueDocumentFailure }
  | { kind: "failed"; failure: IssueDocumentFailure };

export type BoardColumns = {
  blocked: IssueCard[];
  frontier: IssueCard[];
  inProgress: IssueCard[];
  recentlyCompleted: IssueCard[];
};

export type RefreshStatus =
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

export type GraphNode = {
  id: string;
  repository: string;
  number: number;
  title: string;
  open: boolean;
  rank: number;
  distance?: number;
  relation?: "center" | "upstream" | "downstream" | "both";
};

export type GraphEdge = {
  from: string;
  to: string;
};

export type DependencyGraph = {
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

export type CenterView = "board" | "graph";
export type WorkspaceView = "project" | "host-overview" | "run";

export type BoardSnapshot = {
  projectId: string;
  columns: BoardColumns | null;
  empty: "no-data" | "incomplete-read" | "tracker-error" | null;
  frontierEmpty: "all-blocked" | "all-claimed" | "no-open" | null;
  parentFilter: IssueCard | null;
  selected: IssueDetail | null;
  issueOptions: IssueLink[];
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

export type PairingOffer = {
  address: string;
  code: string;
  text: string;
  qrText: string;
  qrSvg: string;
};

export type PairedClient = { id: string; name: string };

export type LoopbackPage =
  | { status: "serving"; url: string }
  | { status: "occupied"; url: string; reason: string }
  | { status: "host-not-running"; url: string; reason: string };

export type Snapshot = {
  running: boolean;
  windowVisible: boolean;
  hostMode: "host-and-client" | "client-only";
  focusedHostId: string;
  focusedProjectId: string;
  hosts: { id: string; displayName: string; local: boolean }[];
  projects: Project[];
  appearance: AppearanceState;
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

export type PendingConfirmation = {
  projectId: string;
  issueId: string;
  runId: string;
  agentId: string;
  deadlineMs: number;
  remainingMs: number;
};

export type AgentFieldKind = "text" | "select" | "boolean" | "multiline";

export type AgentField = {
  id: string;
  label: string;
  kind: AgentFieldKind;
  options?: string[];
  optionFilter?: {
    fieldId: string;
    optionsByValue: Record<string, string[]>;
    defaultsByValue?: Record<string, string>;
  } | null;
  required: boolean;
  folded: boolean;
};

export type AgentSummary = {
  id: string;
  name: string;
  installed: boolean;
  unavailableReason?: string | null;
  fields: AgentField[];
};

export type IntentOption = {
  id: string;
  label: string;
  prefix: string;
};

export type RunLaunchForm = {
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
  optionDiscoveryPending?: string | null;
  optionDiscoveryError?: string | null;
};

export type ChangeScope = "this-round" | "uncommitted";

export type ChangeLine = {
  kind: "context" | "add" | "delete";
  oldLine?: number | null;
  newLine?: number | null;
  text: string;
};

export type ChangeHunk = {
  header: string;
  lines: ChangeLine[];
};

export type ChangeFile = {
  path: string;
  hunks: ChangeHunk[];
};

export type ChangeRepo = {
  path: string;
  displayPath: string;
  available: boolean;
  unavailableReason?: string | null;
  startCommit?: string | null;
  files: ChangeFile[];
};

export type ChangeNote = {
  id: string;
  runId: string;
  projectId: string;
  issueId?: string | null;
  repo: string;
  path: string;
  line: number;
  text: string;
};

export type ViewChanges = {
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

export type LaunchDraft = {
  projectId: string;
  issueId?: string | null;
  agentId: string;
  values: Record<string, string>;
  openingText: string;
  intentId: string;
  custom: boolean;
};

export type RunStatus = "starting" | "running" | "ended";

export type RunSummary = {
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

export type TokenCounts = {
  input?: number | null;
  output?: number | null;
  cacheRead?: number | null;
  cacheWrite?: number | null;
  reasoning?: number | null;
  total?: number | null;
};

export type TelemetryLaneKind = "main" | "subagent" | "switched";

export type TelemetryPoint = {
  atMs: number;
  ttftMs?: number | null;
  tokensPerSec?: number | null;
  spike: boolean;
};

export type RunTelemetryLane = {
  model: string;
  lane: TelemetryLaneKind;
  tokens: TokenCounts;
  ttftMs?: number | null;
  tokensPerSec?: number | null;
  recent: TelemetryPoint[];
  spike: boolean;
};

export type UsageRange = "today" | "24-hours" | "7-days" | "30-days" | "custom";

export type UsageFilter = {
  projectId?: string | null;
  agentId?: string | null;
  model?: string | null;
};

export type UsageOption = { id: string; name: string };

export type UsageRunRow = {
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

export type UsageBucket = {
  startMs: number;
  tokens: TokenCounts;
  ttftMs?: number | null;
  tokensPerSec?: number | null;
  slow: boolean;
};

export type UsagePage = {
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

export type QuitOffer = {
  activeRunCount: number;
};

export type UpdateInstallGate = {
  allowed: boolean;
  activeRunCount: number;
};

export type UpdateState =
  | { kind: "idle" }
  | { kind: "checking"; manual: boolean }
  | { kind: "current" }
  | { kind: "available"; version: string; notes: string }
  | { kind: "blocked"; activeRunCount: number }
  | { kind: "installing"; progress: number | null }
  | { kind: "failed"; message: string };

export type NotificationKind = "waiting" | "completed" | "abnormal-stop" | "crash-recovered";

export type HostEvent =
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

export type LaunchEnvironmentState = {
  status: "idle" | "ready" | "failed";
  refreshedDirectories: number;
  message?: string | null;
};

export type RpcResult = {
  snapshot: Snapshot;
  process: "keep-running" | "exit";
  launchEnvironment?: LaunchEnvironmentState;
  inference?: ProjectDraft;
  updateInstallGate?: UpdateInstallGate;
  events?: HostEvent[];
  viewChanges?: ViewChanges;
};

export type PrimaryPage =
  | "board"
  | "dependency-graph"
  | "focus-workspace"
  | "settings"
  | "host-overview"
  | "usage";

export type RightSideMode = "hidden" | "rail" | "changes";
export type ScrollPosition = { scrollTop: number; scrollLeft: number };
export type BoardViewMemory = {
  projectId: string;
  scroll: ScrollPosition;
  lanes: Record<string, ScrollPosition>;
};
export type GraphViewportAnchor = { issueId: string; viewportX: number; viewportY: number };
export type GraphViewMemory = {
  projectId: string;
  viewportAnchor: GraphViewportAnchor | null;
};
export type ReturnPoint = {
  page: PrimaryPage;
  hostId: string;
  projectId: string | null;
  issueId: string | null;
  runId: string | null;
  board: BoardViewMemory | null;
  graph: GraphViewMemory | null;
};
export type ClientPanelState = {
  sidebarVisible: boolean;
  sidebarWidth: number;
  rightSide: RightSideMode;
  rightRailWidth: number;
  changesPanelWidth: number;
};
export type ClientLocalViewState = {
  page: PrimaryPage;
  returnPoint: ReturnPoint | null;
  panels: ClientPanelState;
};
export type FixedPanelRegion = "sidebar" | "right-rail" | "changes-panel";
export type PanelPointerInteraction = {
  pointerId: number;
  region: FixedPanelRegion;
  startClientX: number;
  startWidth: number;
};

export type BoardScrollPosition = ScrollPosition & {
  lanes: Record<string, ScrollPosition>;
};
export type BrowserAppearance = {
  language: Language;
  appearancePreference: AppearancePreference;
};
