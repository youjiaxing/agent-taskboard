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
                ? projects.map((project) => projectRow(copy, project, snap.focusedProjectId)).join("")
                : `<div class="nested">${escapeHtml(copy.noProjectTitle)}</div>`
            }
          </div>
        </aside>
        <main class="workspace ${empty ? "" : "board-open"}">
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
              : projectMain(copy, snap)
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
    ${removeProject ? removeDialog(copy, removeProject) : ""}
  `;
  paintGraphEdges();
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

function projectRow(copy: ShellCopy, project: Project, focusedId: string): string {
  const active = project.id === focusedId;
  const degraded = project.connection.status !== "ready";
  return `<div class="project-row ${active ? "active" : ""}">
    <button type="button" class="project-main" data-act="focus-project" data-id="${escapeHtml(project.id)}">
      <b>${escapeHtml(project.name)}</b>
      <span>${escapeHtml(project.githubHost)}/${escapeHtml(project.repository)}</span>
    </button>
    ${degraded ? `<span class="dot warn" title="${escapeHtml(copy.authFailed)}"></span>` : ""}
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
  return `
    <div class="detail-hd">#${issue.number} ${escapeHtml(issue.title)}</div>
    <div class="detail-meta">
      ${issue.triageRole ? `<span class="tag">${escapeHtml(issue.triageRole)}</span>` : ""}
      <span class="tag">${escapeHtml(claim)}</span>
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
    await rpc("quitHost");
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
});

app.addEventListener("submit", async (event) => {
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
