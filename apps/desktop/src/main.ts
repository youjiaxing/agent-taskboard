import { FitAddon } from "@xterm/addon-fit";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import "./shell.css";

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
  showClosedContext: string;
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
  unclearIssue: string;
  refreshNow: string;
  refreshRefreshing: string;
  refreshAsOf: string;
  refreshNext: string;
  refreshOffline: string;
  refreshNever: string;
  refreshRateLimited: string;
  refreshRetry: string;
  refreshPaused: string;
  refreshAuth: string;
  newRun: string;
  executeRun: string;
  startRun: string;
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

type CredentialSource = "app-env" | "secrets-file" | "cli" | "generic-env";

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
  tracker: "github";
  githubHost: string;
  repository: string;
  connection: ProjectConnection;
  hasActiveRun: boolean;
  hasExecutionStopped?: boolean;
  trackerSynced: boolean;
  autoAdvance?: boolean;
  restoreAutoAdvance?: boolean;
  restoreDelayMs?: number;
};

type ProjectDraft = {
  name: string;
  localPath: string;
  githubHost: string;
  repository: string;
};

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
  executionStopped?: boolean;
  waitingForUser?: boolean;
  activeRunId?: string | null;
};

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
  | { kind: "auth-failed"; fetchedAtMs?: number | null };

type GraphNode = {
  id: string;
  repository: string;
  number: number;
  title: string;
  open: boolean;
  rank: number;
};

type GraphEdge = {
  from: string;
  to: string;
};

type DependencyGraph = {
  nodes: GraphNode[];
  edges: GraphEdge[];
};

type CenterView = "board" | "graph";
type WorkspaceView = "project" | "host-overview" | "run";

type BoardSnapshot = {
  projectId: string;
  columns: BoardColumns | null;
  empty: "no-data" | null;
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

type RpcResult = {
  snapshot: Snapshot;
  process: "keep-running" | "exit";
  inference?: ProjectDraft;
  events?: HostEvent[];
  viewChanges?: ViewChanges;
};

const app = document.querySelector<HTMLDivElement>("#app");
if (!app) {
  throw new Error("missing #app");
}

let snapshot: Snapshot | null = null;
let settingsOpen = false;
let pairingOpen = false;
let hostPickerOpen = false;
let pairingAddress = "";
let pairingPaste = "";
let pairingError = "";
let projectMenuId = "";
let formOpen: "register" | "edit" | null = null;
let formProjectId = "";
let formDraft: ProjectDraft = emptyDraft();
let inferred: ProjectDraft | null = null;
let formError = "";
let removeProject: Project | null = null;
let refreshing = false;
let tickTimer: number | undefined;
let term: Terminal | null = null;
let fitAddon: FitAddon | null = null;
let termHost: HTMLDivElement | null = null;
let ptyOffset = 0;
let ptyRunId = "";
let ptyPumping = false;
let launchDraft: LaunchDraft | null = null;
let launchFolded = false;
let changesOpen = false;
let changesScope: ChangeScope = "this-round";
let changesView: ViewChanges | null = null;
let noteDraft = "";
let noteTarget: { repo: string; path: string; line: number } | null = null;
let telemetryExpanded = false;
let keyboardHelpOpen = false;
let keyboardCursorIssueId = "";
let sidebarVisible = true;
let issueDetailVisible = true;
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

function emptyDraft(): ProjectDraft {
  return { name: "", localPath: "", githubHost: "github.com", repository: "" };
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

function expectedOpening(form: RunLaunchForm, draft: LaunchDraft): string {
  const prefix = form.intents.find((intent) => intent.id === draft.intentId)?.prefix ?? "";
  const body = (draft.values["initial-instruction"] ?? "").trim();
  const notes = (form.changeNotesText ?? "").trim();
  const core = prefix && body ? `${prefix}\n${body}` : prefix || body;
  if (core && notes) return `${core}\n\n${notes}`;
  return core || notes;
}

async function openExternalUrl(url: string): Promise<void> {
  if ("__TAURI_INTERNALS__" in window) {
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
  const request = rpcQueue.then(async () => {
    const response = await fetch(`${await protocolBase()}/rpc`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ op, ...extra }),
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
    result.events = result.events ?? [];
    syncLaunchDraft(result.snapshot);
    deliverHostEvents(result.events, result.snapshot);
    snapshot = result.snapshot;
    if (result.viewChanges) {
      changesView = result.viewChanges;
      changesOpen = true;
    }
    return result;
  });
  rpcQueue = request.then(() => undefined, () => undefined);
  return request;
}

async function loadViewChanges(runId: string, scope: ChangeScope): Promise<void> {
  const result = await rpc("viewChanges", { runId, scope });
  changesView = result.viewChanges ?? null;
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
  if (event.projectId) {
    await rpc("focusProject", { projectId: event.projectId });
  }
  if (event.issueId) {
    await rpc("focusIssue", { issueId: event.issueId });
  }
  if (event.runId) {
    await rpc("focusRun", { runId: event.runId });
  }
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

function render(): void {
  if (!snapshot || !app) return;
  const snap = snapshot;
  const isMobile = mobileClient();
  const appearance = isMobile
    ? { ...snap.appearance, ...ensureMobileAppearance() }
    : snap.appearance;
  const copy = isMobile && appearance.language !== snap.appearance.language
    ? clientCopy(appearance.language, snap.copy)
    : snap.copy;
  const { hosts, projects } = snap;
  document.documentElement.lang = appearance.language === "zh-CN" ? "zh-CN" : "en";
  document.documentElement.dataset.theme = appearance.theme;
  document.documentElement.dataset.mobile = isMobile ? "true" : "false";
  document.title = copy.appName;

  const host = hosts.find((item) => item.id === snapshot?.focusedHostId) ?? hosts[0];
  const empty = snapshot.emptyActions.length > 0;
  const runLifted = !isMobile && snap.workspaceView === "run" && Boolean(focusedRun(snap));
  const showSidebar = !isMobile && sidebarVisible && !runLifted;
  if (!pairingAddress) {
    pairingAddress = (snapshot.loopbackPage.url || "http://127.0.0.1:10529/").replace(/\/$/, "");
  }

  app.innerHTML = `
    <div class="frame">
      <header class="chrome">
        ${isMobile
          ? `<button type="button" class="ghost" data-act="mobile-scope">${escapeHtml(copy.mobileSwitchScope)}</button>`
          : `<button type="button" class="ghost" data-act="toggle-sidebar" aria-label="${escapeHtml(showSidebar ? copy.hideSidebar : copy.showSidebar)}">☰</button>
             <button type="button" class="ghost" data-act="toggle-issue" aria-label="${escapeHtml(issueDetailVisible ? copy.hideIssueDetail : copy.showIssueDetail)}">▱</button>`}
        ${!isMobile && !showSidebar ? `<button type="button" class="ghost ${snap.workspaceView === "host-overview" ? "active" : ""}" data-act="open-overview">${escapeHtml(copy.hostOverview)}</button>` : ""}
        ${runLifted ? `<button type="button" class="ghost" data-act="return-board">← ${escapeHtml(copy.returnToBoard)}</button>` : ""}
        <button type="button" class="ghost" data-act="settings">${escapeHtml(copy.settings)}</button>
        <span class="chrome-trail">
          <button type="button" class="ghost ${appearance.theme !== "plain-night" ? "active" : ""}" data-act="shade" data-id="light">${escapeHtml(copy.shadeLight)}</button>
          <button type="button" class="ghost ${appearance.theme === "plain-night" ? "active" : ""}" data-act="shade" data-id="dark">${escapeHtml(copy.shadeDark)}</button>
        </span>
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
                        `<button type="button" class="item ${item.id === host?.id ? "active" : ""}" data-act="focus-host" data-id="${escapeHtml(item.id)}">${escapeHtml(item.displayName)}${item.local ? `<span class="tag">${escapeHtml(copy.thisMachine)}</span>` : ""}</button>`,
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
                      : `${projectMain(copy, snap)}${runDock(copy, snap)}`
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
                  <button type="button" class="primary" data-act="show-offer">${escapeHtml(copy.pairingShow)}</button>
                </div>
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
                            `<div class="client-row"><span>${escapeHtml(client.name)}</span><button type="button" data-act="revoke" data-id="${escapeHtml(client.id)}">${escapeHtml(copy.revokeClient)}</button></div>`,
                        )
                        .join("")
                    : `<div class="nested">${escapeHtml(copy.noPairedClients)}</div>`
                }
              </div>
              <div class="field">
                <div class="label">${escapeHtml(copy.pairingToAnother)}</div>
                <textarea data-field="paste" rows="4" placeholder="${escapeHtml(copy.pairingPaste)}">${escapeHtml(pairingPaste)}</textarea>
                <div class="actions">
                  <button type="button" class="primary" data-act="connect-host">${escapeHtml(copy.pairingConnect)}</button>
                </div>
              </div>
              ${pairingError ? `<p class="notice">${escapeHtml(pairingError)}</p>` : ""}
            </div>
          </div>`
        : ""
    }
    ${formOpen ? projectForm(copy) : ""}
    ${snap.launchForm ? launchForm(copy, snap) : ""}
    ${removeProject ? removeDialog(copy, removeProject) : ""}
    ${snap.quitOffer ? quitOfferDialog(copy) : ""}
    ${changesOpen ? viewChangesPanel(copy) : ""}
    ${keyboardHelpOpen ? keyboardHelpDialog(copy) : ""}
  `;
  paintGraphEdges();
  if (isMobile && !mobileLiveTerminal) {
    ptyPumping = false;
    void pumpMobileOutput(snap);
  } else {
    mobilePtyPumping = false;
    attachTerminal(snap);
  }
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

function currentProject(snap: Snapshot): Project | undefined {
  return (
    snap.projects.find((project) => project.id === snap.focusedProjectId) ?? snap.projects[0]
  );
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
      <button type="button" class="project-main" data-act="focus-project" data-id="${escapeHtml(project.id)}"><b>${escapeHtml(project.name)}</b><span>${escapeHtml(project.repository)}</span></button>
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
    return `<section class="mobile-issue-view"><aside class="issue-detail">${snap.board ? issueDetail(copy, snap.board) : ""}</aside></section>`;
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
        : `<pre class="mobile-run-output" data-run="${escapeHtml(run.id)}">${escapeHtml(mobilePtyText.get(run.id) ?? run.recentOutput ?? "")}</pre>`}
    </section>
    ${run.status === "ended" ? "" : `<form class="inject-row" data-act="inject-run" data-id="${escapeHtml(run.id)}"><input name="text" maxlength="4000" placeholder="${escapeHtml(copy.injectPlaceholder)}" /><button type="submit">${escapeHtml(copy.injectLine)}</button></form>`}
    ${mobileLiveTerminal ? "" : `<button type="button" class="ghost mobile-terminal-escape" data-act="mobile-live-terminal">${escapeHtml(copy.mobileLiveTerminal)}</button>`}
  </section>`;
}

function focusedRun(snap: Snapshot): RunSummary | undefined {
  return (snap.runs ?? []).find((run) => run.id === snap.focusedRunId);
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
      <span>${escapeHtml(project.githubHost)}/${escapeHtml(project.repository)}</span>
    </button>
    ${degraded ? `<span class="dot warn" title="${escapeHtml(copy.authFailed)}"></span>` : ""}
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
        ? `<form class="usage-custom" data-act="usage-custom">
            <input type="datetime-local" name="from" value="${escapeHtml(toLocalInput(usage.fromMs))}" />
            <input type="datetime-local" name="to" value="${escapeHtml(toLocalInput(usage.toMs))}" />
            <button type="submit">${escapeHtml(copy.rangeCustom)}</button>
          </form>`
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
    <div class="overview-groups">
      ${groups
        .filter(([id]) => id !== "ended" || overviewShowEnded)
        .map(
          ([id, title, runs]) => `<section class="overview-group" data-run-group="${id}">
            <div class="lane-hd">${escapeHtml(title)} <span>${runs.length}</span></div>
            <div class="run-thumbnails">${runs.length ? runs.map((run) => runThumbnail(copy, run, snap)).join("") : `<p class="lane-empty">${escapeHtml(copy.noItems)}</p>`}</div>
          </section>`,
        )
        .join("")}
    </div>
  </div>`;
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
    ${run.status === "ended" ? "" : `<form class="inject-row" data-act="inject-run" data-id="${escapeHtml(run.id)}"><input name="text" maxlength="4000" placeholder="${escapeHtml(copy.injectPlaceholder)}" /><button type="submit">${escapeHtml(copy.injectLine)}</button></form>`}
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
  return `<section class="lifted-run ${issueDetailVisible ? "" : "issue-collapsed"}">
    ${terminalPanel(copy, run, "lifted-terminal")}
    ${issueDetailVisible ? `<aside class="issue-detail">${snap.board ? issueDetail(copy, snap.board) : ""}</aside>` : ""}
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
  const noteForm = active
    ? `<form class="note-form" data-act="write-note">
        <input name="text" maxlength="400" value="${escapeHtml(noteDraft)}" placeholder="${escapeHtml(copy.changeNotePlaceholder)}" />
        <button type="submit">${escapeHtml(copy.addChangeNote)}</button>
      </form>`
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

function keyboardHelpDialog(copy: ShellCopy): string {
  return `<div class="overlay modal keyboard-help" data-act="close-keyboard-help">
    <div class="sheet" data-act="form-noop" role="dialog" aria-modal="true" aria-label="${escapeHtml(copy.keyboardHelp)}">
      <h2>${escapeHtml(copy.keyboardHelp)}</h2>
      <p class="hint">${escapeHtml(copy.keyboardHelpBody)}</p>
      <div class="actions"><button type="button" data-act="close-keyboard-help">${escapeHtml(copy.gotIt)}</button></div>
    </div>
  </div>`;
}

function projectMain(copy: ShellCopy, snap: Snapshot): string {
  const project = currentProject(snap);
  if (!project) return loopbackNotice(snap.loopbackPage);
  return `<div class="project-board">
    ${loopbackNotice(snap.loopbackPage)}
    <div class="board-head">
      <div class="board-head-row">
        <div>
          <h1>${escapeHtml(project.name)}</h1>
          <p>${escapeHtml(project.localPath)}</p>
          <p>${escapeHtml(project.githubHost)}/${escapeHtml(project.repository)}</p>
        </div>
        <div class="view-switch" role="tablist">
          <button type="button" class="${snap.centerView === "board" ? "active" : ""}" data-act="center-view" data-id="board">${escapeHtml(copy.viewBoard)}</button>
          <button type="button" class="${snap.centerView === "graph" ? "active" : ""}" data-act="center-view" data-id="graph">${escapeHtml(copy.viewGraph)}</button>
        </div>
      </div>
    </div>
    ${refreshBar(copy, snap.board)}
    ${issueSearch(copy, snap)}
    ${pendingBar(copy, snap)}
    ${connectionPanel(copy, project)}
    ${boardView(copy, snap)}
  </div>`;
}

function issueSearch(copy: ShellCopy, snap: Snapshot): string {
  const search = snap.board?.search ?? { title: "", triageRole: null, state: "all" as const };
  const triageRoles: TriageRole[] = [
    "needs-triage",
    "needs-info",
    "ready-for-agent",
    "ready-for-human",
    "wontfix",
  ];
  return `<form class="issue-search" data-act="issue-search">
    <label class="sr-only" for="issue-title-search">${escapeHtml(copy.searchTitle)}</label>
    <input id="issue-title-search" name="title" type="search" value="${escapeHtml(search.title)}" placeholder="${escapeHtml(copy.searchPlaceholder)}" />
    <select name="triageRole" aria-label="${escapeHtml(copy.searchAllTriage)}">
      <option value="">${escapeHtml(copy.searchAllTriage)}</option>
      ${triageRoles.map((role) => `<option value="${role}" ${search.triageRole === role ? "selected" : ""}>${role}</option>`).join("")}
    </select>
    <select name="state" aria-label="${escapeHtml(copy.searchAllStates)}">
      <option value="all" ${search.state === "all" ? "selected" : ""}>${escapeHtml(copy.searchAllStates)}</option>
      <option value="open" ${search.state === "open" ? "selected" : ""}>${escapeHtml(copy.searchOpen)}</option>
      <option value="closed" ${search.state === "closed" ? "selected" : ""}>${escapeHtml(copy.searchClosed)}</option>
    </select>
    <button type="submit">${escapeHtml(copy.searchSubmit)}</button>
    <button type="button" data-act="keyboard-help" aria-label="${escapeHtml(copy.keyboardHelp)}">?</button>
  </form>`;
}

function boardView(copy: ShellCopy, snap: Snapshot): string {
  const board = snap.board;
  if (!board || board.empty === "no-data" || !board.columns) {
    return `<div class="board-empty">${escapeHtml(copy.emptyNoData)}</div>`;
  }
  const onGraph = snap.centerView === "graph";
  const hint = onGraph ? copy.graphHint : board.parentFilter ? copy.childHint : copy.boardHint;
  return `<div class="board-shell ${issueDetailVisible ? "" : "issue-collapsed"}" data-center-view="${onGraph ? "graph" : "board"}">
    <div class="board-main">
      <div class="board-hint">
        ${escapeHtml(hint)}
        ${
          board.parentFilter
            ? `<button type="button" data-act="clear-filter">${escapeHtml(copy.clearFilter)}</button>`
            : ""
        }
      </div>
      ${onGraph ? dependencyGraphView(copy, board) : boardLanes(copy, board)}
    </div>
    ${issueDetailVisible ? `<aside class="issue-detail">${issueDetail(copy, board)}</aside>` : ""}
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

function dependencyGraphView(copy: ShellCopy, board: BoardSnapshot): string {
  const graph = board.graph;
  if (!graph) {
    return `<div class="board-empty">${escapeHtml(copy.emptyNoData)}</div>`;
  }
  const columns = new Map<number, GraphNode[]>();
  for (const node of graph.nodes) {
    const list = columns.get(node.rank) ?? [];
    list.push(node);
    columns.set(node.rank, list);
  }
  const ranks = [...columns.keys()].sort((a, b) => a - b);
  return `<div class="dep-graph">
    <label class="graph-opt">
      <input type="checkbox" data-field="closedContext" ${board.showClosedGraphContext ? "checked" : ""} />
      ${escapeHtml(copy.showClosedContext)}
    </label>
    <div class="graph-canvas">
      <svg class="graph-edges" aria-hidden="true"></svg>
      <div class="graph-flow">
        ${ranks
          .map(
            (rank) =>
              `<div class="graph-col" data-rank="${rank}">${(columns.get(rank) ?? [])
                .map((node) => graphNode(node, board.selected?.id))
                .join("")}</div>`,
          )
          .join("")}
      </div>
    </div>
  </div>`;
}

function graphNode(node: GraphNode, selectedId: string | undefined): string {
  const selected = node.id === selectedId ? "sel" : "";
  const closed = node.open ? "" : "closed";
  return `<button type="button" class="graph-node ${selected} ${closed}" data-act="focus-issue" data-id="${escapeHtml(node.id)}">
    <div class="issue-id">#${node.number}</div>
    <div class="issue-title">${escapeHtml(node.title)}</div>
  </button>`;
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
  const actions = lane === "frontier"
    ? `<button type="button" class="primary" data-act="execute-run" data-id="${escapeHtml(issue.id)}">${escapeHtml(copy.executeRun)}</button>`
    : lane === "inProgress" && issue.runId
      ? `<button type="button" data-act="focus-run" data-id="${escapeHtml(issue.runId)}">${escapeHtml(copy.focusRun)}</button>
         <button type="button" data-act="stop-run" data-id="${escapeHtml(issue.runId)}">${escapeHtml(copy.stopRun)}</button>
         ${mobileClient() ? "" : `<button type="button" data-act="view-changes" data-id="${escapeHtml(issue.runId)}">${escapeHtml(copy.viewChanges)}</button>`}`
      : lane === "recentlyCompleted"
        ? `${!mobileClient() && issue.runId ? `<button type="button" data-act="view-changes" data-id="${escapeHtml(issue.runId)}">${escapeHtml(copy.viewChanges)}</button>` : ""}
           <button type="button" data-act="open-issue" data-url="${escapeHtml(issue.url)}">${escapeHtml(copy.openIssue)}</button>`
        : "";
  return `<article class="issue-card ${issue.id === selectedId ? "sel" : ""} ${issue.activity ? escapeHtml(issue.activity) : ""}" data-issue-id="${escapeHtml(issue.id)}">
    <button type="button" class="issue-card-main" data-act="${cardAction}" data-id="${escapeHtml(actionTargetId)}" data-issue-id="${escapeHtml(issue.id)}">
      <div class="issue-id">#${issue.number}</div>
      <div class="issue-title">${escapeHtml(issue.title)}</div>
      ${tags ? `<div class="issue-tags">${tags}</div>` : ""}
    </button>
    ${actions ? `<div class="issue-card-actions">${actions}</div>` : ""}
  </article>`;
}

function issueDetail(copy: ShellCopy, board: BoardSnapshot): string {
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
    <div class="detail-hd">#${issue.number} ${escapeHtml(issue.title)}</div>
    <div class="detail-meta">
      ${issue.triageRole ? `<span class="tag">${escapeHtml(issue.triageRole)}</span>` : ""}
      <span class="tag">${escapeHtml(claim)}</span>
      ${issue.waitingForUser ? `<span class="tag">${escapeHtml(copy.waiting)}</span>` : ""}
      ${issue.executionStopped ? `<span class="tag">${escapeHtml(copy.executionStopped)}</span>` : ""}
      ${actions}
      <button type="button" data-act="open-issue" data-url="${escapeHtml(issue.url)}">${escapeHtml(copy.openIssue)}</button>
    </div>
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
    </section>`;
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
    if (status.fetchedAtMs) {
      parts.push(`${copy.refreshAsOf} ${formatTime(status.fetchedAtMs)}`);
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
  if (project.connection.status !== "auth-failed") {
    return "";
  }
  const repair = project.connection.repair;
  return `<div class="notice bad">
    <b>${escapeHtml(copy.authFailed)}</b>
    <p>${escapeHtml(project.connection.message)}</p>
    <ul class="repair">
      <li>${escapeHtml(copy.repairCli)}${repair.cliDetected ? "" : ` — ${escapeHtml(copy.noGhDetected)}`}</li>
      <li>${escapeHtml(copy.repairSecrets)}：<code>${escapeHtml(repair.secretsPath)}</code></li>
      <li>${escapeHtml(copy.repairEnv)}：<code>${escapeHtml(repair.appEnv)}</code> / <code>${escapeHtml(repair.genericEnv)}</code></li>
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
                `<button type="button" class="${agent.id === form.selectedAgentId ? "active" : ""}" data-act="pick-agent" data-id="${escapeHtml(agent.id)}" ${agent.installed ? "" : "disabled"}>${escapeHtml(agent.name)}</button>`,
            )
            .join("")}
        </div>
        <div class="actions">
          <button type="button" data-act="close-launch">${escapeHtml(copy.cancel)}</button>
        </div>
      </div>
    </div>`;
  }
  const first = form.fields.filter((field) => !field.folded && field.id !== "initial-instruction");
  const folded = form.fields.filter((field) => field.folded);
  const intentActive = draft.custom ? "" : draft.intentId;
  return `<div class="overlay modal" data-act="close-launch">
    <form class="sheet form-sheet launch-sheet" data-act="form-noop" data-form="launch">
      <h2>${escapeHtml(copy.launchTitle)}</h2>
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
          ${draft.custom ? `<button type="button" class="active" data-act="intent-custom">${escapeHtml(copy.intentCustom)}</button>` : ""}
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
      ${form.error ? `<p class="notice bad">${escapeHtml(form.error)}</p>` : ""}
      <div class="actions">
        <button type="button" data-act="close-launch">${escapeHtml(copy.cancel)}</button>
        <button type="submit" class="primary">${escapeHtml(copy.startRun)}</button>
      </div>
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
  return `<div class="overlay modal" data-act="close-form">
    <form class="sheet form-sheet" data-act="form-noop" data-form="project">
      <h2>${escapeHtml(editing ? copy.editProjectTitle : copy.registerProjectTitle)}</h2>
      <p class="hint">${escapeHtml(copy.inferenceHint)}</p>
      <div class="field">
        <label class="label" for="project-name">${escapeHtml(copy.displayName)}</label>
        <input id="project-name" data-field="name" required value="${escapeHtml(formDraft.name)}" />
      </div>
      <div class="field">
        <label class="label" for="project-path">${escapeHtml(copy.localDirectory)}</label>
        <input id="project-path" data-field="localPath" required value="${escapeHtml(formDraft.localPath)}" />
      </div>
      <div class="field">
        <label class="label" for="project-host">${escapeHtml(copy.githubHost)}</label>
        <input id="project-host" data-field="githubHost" value="${escapeHtml(formDraft.githubHost)}" />
      </div>
      <div class="field">
        <label class="label" for="project-repo">${escapeHtml(copy.repository)}</label>
        <input id="project-repo" data-field="repository" required placeholder="owner/repo" value="${escapeHtml(formDraft.repository)}" />
      </div>
      <div class="inference">
        <button type="button" data-act="infer">${escapeHtml(copy.inferFromDirectory)}</button>
        ${
          inferred
            ? `<div class="notice" style="margin-top:8px">
                <b>${escapeHtml(inferred.githubHost)}/${escapeHtml(inferred.repository)}</b>
                <div class="actions">
                  <button type="button" data-act="apply-infer">${escapeHtml(copy.useInference)}</button>
                </div>
              </div>`
            : ""
        }
      </div>
      ${formError ? `<p class="notice bad">${escapeHtml(formError)}</p>` : ""}
      <div class="actions">
        <button type="button" data-act="close-form">${escapeHtml(copy.cancel)}</button>
        <button type="submit" class="primary">${escapeHtml(editing ? copy.saveRegistration : copy.addProject)}</button>
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
      <div class="actions">
        <button type="button" data-act="close-remove">${escapeHtml(copy.cancel)}</button>
        <button type="button" class="danger primary" data-act="confirm-remove">${escapeHtml(copy.removeConfirm)}</button>
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
    await rpc("openHostOverview");
    render();
    return;
  }
  if (act === "return-board") {
    sidebarVisible = sidebarBeforeLift;
    await rpc("returnToBoard");
    render();
    return;
  }
  if (act === "settings") {
    settingsOpen = true;
    pairingOpen = false;
    formOpen = null;
    removeProject = null;
    projectMenuId = "";
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
    render();
    return;
  }
  if (act === "register") {
    mobileScopeOpen = false;
    formOpen = "register";
    formProjectId = "";
    formDraft = emptyDraft();
    inferred = null;
    formError = "";
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
  if (act === "close-form" && (event.target === target || target.tagName === "BUTTON")) {
    formOpen = null;
    inferred = null;
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
    await rpc("focusProject", { projectId: target.dataset.id });
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
    await rpc("prepareRunLaunch", { projectId: target.dataset.id });
    render();
    return;
  }
  if (act === "execute-run" && target.dataset.id && snapshot.focusedProjectId) {
    settingsOpen = false;
    pairingOpen = false;
    formOpen = null;
    launchDraft = null;
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
    await rpc("cancelRunLaunch");
    launchDraft = null;
    render();
    return;
  }
  if (act === "switch-agent") {
    const form = snapshot.launchForm;
    if (!form) return;
    launchDraft = null;
    await rpc("prepareRunLaunch", {
      projectId: form.projectId,
      issueId: form.issueId,
      pickAgent: true,
    });
    render();
    return;
  }
  if (act === "pick-agent" && target.dataset.id) {
    const form = snapshot.launchForm;
    if (!form) return;
    launchDraft = null;
    await rpc("prepareRunLaunch", {
      projectId: form.projectId,
      issueId: form.issueId,
      agentId: target.dataset.id,
    });
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
    await rpc("focusRun", { runId: target.dataset.id });
    if (mobileClient()) {
      mobileView = "run";
      mobileLiveTerminal = false;
    } else {
      sidebarVisible = false;
    }
    render();
    return;
  }
  if (act === "open-usage") {
    mobileScopeOpen = false;
    settingsOpen = false;
    pairingOpen = false;
    formOpen = null;
    await rpc("openUsage");
    render();
    return;
  }
  if (act === "close-usage") {
    await rpc("closeUsage");
    render();
    return;
  }
  if (act === "usage-range" && target.dataset.id) {
    await rpc("setUsageRange", { range: target.dataset.id });
    render();
    return;
  }
  if (act === "open-usage-run" && target.dataset.id) {
    await rpc("openUsageForRun", { runId: target.dataset.id });
    render();
    return;
  }
  if (act === "open-run-usage" && target.dataset.id) {
    await rpc("openRunFromUsage", { runId: target.dataset.id });
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
    await loadViewChanges(target.dataset.id, changesScope);
    render();
    return;
  }
  if (act === "close-changes") {
    changesOpen = false;
    changesView = null;
    noteTarget = null;
    noteDraft = "";
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
      await rpc("refresh");
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
    inferred = null;
    formError = "";
    projectMenuId = "";
    render();
    return;
  }
  if (act === "remove-project" && target.dataset.id) {
    mobileScopeOpen = false;
    removeProject = snapshot.projects.find((item) => item.id === target.dataset.id) ?? null;
    projectMenuId = "";
    render();
    return;
  }
  if (act === "close-remove" && (event.target === target || target.tagName === "BUTTON")) {
    removeProject = null;
    render();
    return;
  }
  if (act === "confirm-remove" && removeProject) {
    try {
      await rpc("removeProject", { projectId: removeProject.id });
      removeProject = null;
    } catch (error) {
      formError = error instanceof Error ? error.message : String(error);
    }
    render();
    return;
  }
  if (act === "infer") {
    formError = "";
    try {
      const result = await rpc("inferProject", { localPath: formDraft.localPath });
      inferred = result.inference ?? null;
      if (!inferred) {
        formError = snapshot.copy.inferenceHint;
      }
    } catch (error) {
      inferred = null;
      formError = error instanceof Error ? error.message : String(error);
    }
    render();
    return;
  }
  if (act === "apply-infer" && inferred) {
    formDraft = {
      name: formDraft.name || inferred.name,
      localPath: formDraft.localPath || inferred.localPath,
      githubHost: inferred.githubHost,
      repository: inferred.repository,
    };
    inferred = null;
    render();
    return;
  }
  if (act === "toggle-hosts") {
    hostPickerOpen = snapshot.hosts.length > 1 ? !hostPickerOpen : false;
    render();
    return;
  }
  if (act === "focus-host" && target.dataset.id) {
    await rpc("focusHost", { hostId: target.dataset.id });
    hostPickerOpen = false;
    render();
    return;
  }
  if (act === "show-offer") {
    pairingError = "";
    const addressInput = app.querySelector<HTMLInputElement>("[data-field='address']");
    pairingAddress = addressInput?.value ?? pairingAddress;
    try {
      await rpc("beginPairingOffer", { address: pairingAddress });
    } catch (error) {
      pairingError = error instanceof Error ? error.message : String(error);
    }
    render();
    return;
  }
  if (act === "copy-offer" && snapshot.pairingOffer) {
    await navigator.clipboard.writeText(snapshot.pairingOffer.text);
    return;
  }
  if (act === "revoke" && target.dataset.id) {
    pairingError = "";
    try {
      await rpc("revokeClient", { clientId: target.dataset.id });
    } catch (error) {
      pairingError = error instanceof Error ? error.message : String(error);
    }
    render();
    return;
  }
  if (act === "connect-host") {
    pairingError = "";
    const pasteInput = app.querySelector<HTMLTextAreaElement>("[data-field='paste']");
    pairingPaste = pasteInput?.value ?? pairingPaste;
    const parsed = parsePairingPayload(pairingPaste);
    if (!parsed) {
      pairingError = snapshot.copy.pairingPaste;
      render();
      return;
    }
    try {
      await rpc("pairRemoteHost", parsed);
      pairingPaste = "";
      pairingOpen = false;
    } catch (error) {
      pairingError = error instanceof Error ? error.message : String(error);
    }
    render();
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
    await rpc("setCenterView", { view: target.dataset.id });
    render();
    return;
  }
  if (act === "focus-issue" && target.dataset.id) {
    await rpc("focusIssue", { issueId: target.dataset.id });
    if (mobileClient()) {
      mobileView = "issue";
      mobileLiveTerminal = false;
    } else if (target.closest(".issue-card") && snapshot.focusedRunId) {
      sidebarBeforeLift = sidebarVisible;
      await rpc("focusRun", { runId: snapshot.focusedRunId });
      sidebarVisible = false;
    }
    render();
    return;
  }
  if (act === "filter-parent" && target.dataset.id) {
    await rpc("filterParent", { issueId: target.dataset.id });
    render();
    return;
  }
  if (act === "clear-filter") {
    await rpc("clearParentFilter");
    render();
  }
});

app.addEventListener("submit", async (event) => {
  const search = (event.target as HTMLElement | null)?.closest<HTMLFormElement>("form[data-act='issue-search']");
  if (search && snapshot) {
    event.preventDefault();
    const data = new FormData(search);
    await rpc("searchIssues", {
      projectId: snapshot.focusedProjectId,
      title: String(data.get("title") ?? ""),
      triageRole: String(data.get("triageRole") ?? ""),
      state: String(data.get("state") ?? "all"),
    });
    keyboardCursorIssueId = "";
    render();
    return;
  }
  const inject = (event.target as HTMLElement | null)?.closest<HTMLFormElement>("form[data-act='inject-run']");
  if (inject && snapshot) {
    event.preventDefault();
    const runId = inject.dataset.id;
    const input = inject.querySelector<HTMLInputElement>("input[name='text']");
    const text = input?.value ?? "";
    if (!runId || !text.trim()) return;
    await rpc("injectRunInput", { runId, text });
    if (input) input.value = "";
    render();
    return;
  }
  const noteForm = (event.target as HTMLElement | null)?.closest<HTMLFormElement>("form[data-act='write-note']");
  if (!noteForm || !snapshot || !noteTarget || !changesView) return;
  event.preventDefault();
  const input = noteForm.querySelector<HTMLInputElement>("input[name='text']");
  const text = input?.value ?? noteDraft;
  if (!text.trim()) return;
  await rpc("writeChangeNote", {
    runId: changesView.runId,
    repo: noteTarget.repo,
    path: noteTarget.path,
    line: noteTarget.line,
    text,
  });
  noteDraft = "";
  noteTarget = null;
  await loadViewChanges(changesView.runId, changesScope);
  render();
});

app.addEventListener("change", async (event) => {
  const target = event.target as HTMLElement | null;
  if (!target || !snapshot) return;
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
  if (
    (field === "name" || field === "localPath" || field === "githubHost" || field === "repository") &&
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
  await rpc("setUsageFilter", next);
  render();
});

app.addEventListener("submit", async (event) => {
  const custom = (event.target as HTMLElement | null)?.closest<HTMLFormElement>("[data-act='usage-custom']");
  if (custom) {
    event.preventDefault();
    const data = new FormData(custom);
    const from = Date.parse(String(data.get("from") ?? ""));
    const to = Date.parse(String(data.get("to") ?? ""));
    if (Number.isNaN(from) || Number.isNaN(to)) return;
    await rpc("setUsageRange", { range: "custom", fromMs: from, toMs: to });
    render();
    return;
  }
  const launch = (event.target as HTMLElement | null)?.closest<HTMLFormElement>("[data-form='launch']");
  if (launch && snapshot && launchDraft) {
    event.preventDefault();
    await rpc("startUnboundRun", {
      projectId: launchDraft.projectId,
      issueId: launchDraft.issueId,
      agentId: launchDraft.agentId,
      values: launchDraft.values,
      openingText: launchDraft.openingText,
    });
    render();
    return;
  }
  const form = (event.target as HTMLElement | null)?.closest<HTMLFormElement>("[data-form='project']");
  if (!form || !snapshot) return;
  event.preventDefault();
  formError = "";
  try {
    if (formOpen === "edit") {
      await rpc("editProject", { projectId: formProjectId, ...formDraft });
    } else {
      await rpc("registerProject", formDraft);
    }
    formOpen = null;
    inferred = null;
  } catch (error) {
    formError = error instanceof Error ? error.message : String(error);
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

async function reportClientView(): Promise<void> {
  const projectId = snapshot?.focusedProjectId ?? "";
  const visible = document.visibilityState === "visible";
  await rpc("setClientView", { clientId, projectId, visible });
}

function ensureTick(): void {
  if (tickTimer != null) return;
  tickTimer = window.setInterval(() => {
    rpc("tick").then(render).catch(() => {});
  }, 1000);
}

document.addEventListener("visibilitychange", () => {
  const visible = document.visibilityState === "visible";
  const work = visible ? rpc("refresh").then(() => reportClientView()) : reportClientView();
  work.then(render).catch(() => {});
});

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
  if (document.visibilityState !== "visible") return;
  rpc("refresh").then(render).catch(() => {});
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
    render();
  })
  .catch((error: unknown) => {
    if (app) {
      app.textContent = error instanceof Error ? error.message : String(error);
    }
  });
