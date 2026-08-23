import { FitAddon } from "@xterm/addon-fit";
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
  emptyActions: Array<"register-first-project" | "pair-another-host">;
  loopbackPage: LoopbackPage;
  pairingOffer: PairingOffer | null;
  pairedClients: PairedClient[];
  board: BoardSnapshot | null;
  recentCompletedLimit: number;
  centerView: CenterView;
  runs: RunSummary[];
  focusedRunId: string;
  quitOffer: QuitOffer | null;
  launchForm?: RunLaunchForm | null;
  showCommandPreview?: boolean;
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
  commandPreview: string;
  intents: IntentOption[];
  warnings?: string[];
  error?: string | null;
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
  recentAction?: string | null;
  failure?: string | null;
  previousRunId?: string | null;
  nativeSessionId?: string | null;
  endedReason?: "exited" | "stopped" | "abnormal" | "crash" | null;
};

type QuitOffer = {
  activeRunCount: number;
};

type RpcResult = {
  snapshot: Snapshot;
  process: "keep-running" | "exit";
  inference?: ProjectDraft;
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
  const warnings = liveEnumWarnings(
    snapshot.launchForm,
    launchDraft,
    snapshot.appearance.language,
  );
  node.textContent = warnings.join(" ");
  node.hidden = warnings.length === 0;
}

function expectedOpening(form: RunLaunchForm, draft: LaunchDraft): string {
  const prefix = form.intents.find((intent) => intent.id === draft.intentId)?.prefix ?? "";
  const body = (draft.values["initial-instruction"] ?? "").trim();
  if (prefix && body) return `${prefix}\n${body}`;
  return prefix || body;
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

let rpcGeneration = 0;

async function rpc(op: string, extra: Record<string, unknown> = {}): Promise<RpcResult> {
  const generation = ++rpcGeneration;
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
  result.snapshot.showCommandPreview = result.snapshot.showCommandPreview ?? true;
  syncLaunchDraft(result.snapshot);
  if (generation !== rpcGeneration && snapshot) {
    return { snapshot, process: "keep-running", inference: result.inference };
  }
  snapshot = result.snapshot;
  return result;
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

function languageLabel(copy: ShellCopy, language: Language): string {
  return language === "zh-CN" ? copy.languageZh : copy.languageEn;
}

function render(): void {
  if (!snapshot || !app) return;
  const snap = snapshot;
  const { copy, appearance, hosts, projects } = snap;
  document.documentElement.lang = appearance.language === "zh-CN" ? "zh-CN" : "en";
  document.documentElement.dataset.theme = appearance.theme;
  document.title = copy.appName;

  const host = hosts.find((item) => item.id === snapshot?.focusedHostId) ?? hosts[0];
  const empty = snapshot.emptyActions.length > 0;
  if (!pairingAddress) {
    pairingAddress = (snapshot.loopbackPage.url || "http://127.0.0.1:10529/").replace(/\/$/, "");
  }

  app.innerHTML = `
    <div class="frame">
      <header class="chrome">
        <button type="button" class="ghost" data-act="settings">${escapeHtml(copy.settings)}</button>
        <span class="chrome-trail">
          <button type="button" class="ghost ${appearance.theme !== "plain-night" ? "active" : ""}" data-act="shade" data-id="light">${escapeHtml(copy.shadeLight)}</button>
          <button type="button" class="ghost ${appearance.theme === "plain-night" ? "active" : ""}" data-act="shade" data-id="dark">${escapeHtml(copy.shadeDark)}</button>
        </span>
      </header>
      <div class="body">
        <aside class="side">
          <div>
            <div class="group-name">${escapeHtml(copy.hosts)}</div>
            ${
              host
                ? `<div class="host-line">
                    <button type="button" class="item active" data-act="toggle-hosts"><span class="dot"></span>${escapeHtml(host.displayName)}${host.local ? `<span class="tag">${escapeHtml(copy.thisMachine)}</span>` : ""}</button>
                    <button type="button" class="title-icon" data-act="pair" aria-label="${escapeHtml(copy.pairAnotherHost)}">⊕</button>
                  </div>`
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
        </aside>
        <main class="workspace ${empty ? "" : "board-open"}${focusedRun(snap) ? " has-run" : ""}">
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
              : `${projectMain(copy, snap)}${runDock(copy, snap)}`
          }
        </main>
      </div>
    </div>
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
              <button type="button" data-act="quit">${escapeHtml(copy.quitHost)}</button>
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
  `;
  paintGraphEdges();
  attachTerminal(snap);
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

function focusedRun(snap: Snapshot): RunSummary | undefined {
  const runs = snap.runs ?? [];
  return runs.find((run) => run.id === snap.focusedRunId) ?? runs.find((run) => run.status !== "ended");
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
  return `<button type="button" class="run-row ${run.id === focusedId ? "active" : ""} ${escapeHtml(run.status)}" data-act="focus-run" data-id="${escapeHtml(run.id)}">
    <b>${escapeHtml(run.agentName)}</b>
    <span>${escapeHtml(identity)}</span>
    ${action ? `<span class="run-action">${action}</span>` : ""}
    ${run.failure ? `<span class="run-fail">${escapeHtml(run.failure)}</span>` : ""}
  </button>`;
}

function runDock(copy: ShellCopy, snap: Snapshot): string {
  const run = focusedRun(snap);
  if (!run) return "";
  const identity = runIdentity(copy, run);
  return `<section class="run-dock">
    <header class="run-dock-hd">
      <div>
        <b>${escapeHtml(run.agentName)}</b>
        <span>${escapeHtml(identity)}</span>
      </div>
      <button type="button" data-act="stop-run" data-id="${escapeHtml(run.id)}" ${run.status === "ended" ? "disabled" : ""}>${escapeHtml(copy.stopRun)}</button>
    </header>
    ${run.failure ? `<p class="notice bad">${escapeHtml(run.failure)}</p>` : ""}
    <div class="pty-slot" data-run="${escapeHtml(run.id)}"></div>
  </section>`;
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
    ${connectionPanel(copy, project)}
    ${boardView(copy, snap)}
  </div>`;
}

function boardView(copy: ShellCopy, snap: Snapshot): string {
  const board = snap.board;
  if (!board || board.empty === "no-data" || !board.columns) {
    return `<div class="board-empty">${escapeHtml(copy.emptyNoData)}</div>`;
  }
  const onGraph = snap.centerView === "graph";
  const hint = onGraph ? copy.graphHint : board.parentFilter ? copy.childHint : copy.boardHint;
  return `<div class="board-shell" data-center-view="${onGraph ? "graph" : "board"}">
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
    <aside class="issue-detail">${issueDetail(copy, board)}</aside>
  </div>`;
}

function boardLanes(copy: ShellCopy, board: BoardSnapshot): string {
  const cols: Array<["blocked" | "frontier" | "inProgress" | "recentlyCompleted", string, IssueCard[]]> = [
    ["blocked", copy.colBlocked, board.columns?.blocked ?? []],
    ["frontier", copy.colFrontier, board.columns?.frontier ?? []],
    ["inProgress", copy.colInProgress, board.columns?.inProgress ?? []],
    ["recentlyCompleted", copy.colRecent, board.columns?.recentlyCompleted ?? []],
  ];
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
          ${items.map((issue) => issueCard(copy, issue, board.selected?.id)).join("")}
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

function issueCard(copy: ShellCopy, issue: IssueCard, selectedId: string | undefined): string {
  const tags = [
    issue.triageRole ? `<span class="tag">${escapeHtml(issue.triageRole)}</span>` : "",
    issue.claimedBy.length
      ? `<span class="tag">${escapeHtml(copy.claimed)} ${escapeHtml(issue.claimedBy.join(", "))}</span>`
      : "",
  ]
    .filter(Boolean)
    .join("");
  return `<button type="button" class="issue-card ${issue.id === selectedId ? "sel" : ""}" data-act="focus-issue" data-id="${escapeHtml(issue.id)}">
    <div class="issue-id">#${issue.number}</div>
    <div class="issue-title">${escapeHtml(issue.title)}</div>
    ${tags ? `<div class="issue-tags">${tags}</div>` : ""}
  </button>`;
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
      ${issue.executionStopped ? `<span class="tag">${escapeHtml(copy.executionStopped)}</span>` : ""}
      ${actions}
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
      <label class="graph-opt isolation-off">
        <input type="checkbox" disabled />
        ${escapeHtml(copy.isolation)}
      </label>
      <p class="hint">${escapeHtml(form.isolationReason)} ${escapeHtml(copy.isolationHint)}</p>
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
  if (act === "close-settings" && event.target === target) {
    settingsOpen = false;
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
    await rpc("focusRun", { runId: target.dataset.id });
    render();
    return;
  }
  if (act === "stop-run" && target.dataset.id) {
    await rpc("stopRun", { runId: target.dataset.id });
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
    await rpc("setLanguage", { language: target.dataset.id });
    render();
    return;
  }
  if (act === "theme" && target.dataset.id) {
    await rpc("setTheme", { theme: target.dataset.id });
    render();
    return;
  }
  if (act === "shade") {
    const next =
      target.dataset.id === "dark" ? "plain-night" : snapshot.appearance.lastLightTheme;
    await rpc("setTheme", { theme: next });
    render();
    return;
  }
  if (act === "quit") {
    settingsOpen = false;
    await rpc("quitHost");
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

app.addEventListener("change", async (event) => {
  const target = event.target as HTMLElement | null;
  if (!target || !snapshot) return;
  if (target.getAttribute("data-field") === "recentLimit" && "value" in target) {
    const limit = Number((target as HTMLInputElement).value);
    if (!Number.isFinite(limit)) return;
    await rpc("setRecentCompletedLimit", { limit });
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

app.addEventListener("submit", async (event) => {
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

window.addEventListener("focus", () => {
  if (document.visibilityState !== "visible") return;
  rpc("refresh").then(render).catch(() => {});
});

window.addEventListener("resize", () => {
  fitAddon?.fit();
  const runId = snapshot?.focusedRunId;
  if (runId) void sendPtyResize(runId);
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
