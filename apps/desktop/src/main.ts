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
        <main class="workspace">
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
      <h1>${escapeHtml(project.name)}</h1>
      <p>${escapeHtml(project.localPath)}</p>
      <p>${escapeHtml(project.githubHost)}/${escapeHtml(project.repository)}</p>
    </div>
    ${connectionPanel(copy, project)}
  </div>`;
}

function connectionPanel(copy: ShellCopy, project: Project): string {
  if (project.connection.status === "ready") {
    return `<p class="notice ok">${escapeHtml(copy.connectionReady)}</p>`;
  }
  const repair = project.connection.repair;
  const title =
    project.connection.status === "auth-failed" ? copy.authFailed : project.connection.message;
  const body =
    project.connection.status === "auth-failed" ? project.connection.message : "";
  return `<div class="notice bad">
    <b>${escapeHtml(title)}</b>
    ${body ? `<p>${escapeHtml(body)}</p>` : ""}
    ${
      project.connection.status === "auth-failed"
        ? `<ul class="repair">
      <li>${escapeHtml(copy.repairCli)}${repair.cliDetected ? "" : ` — ${escapeHtml(copy.noGhDetected)}`}</li>
      <li>${escapeHtml(copy.repairSecrets)}：<code>${escapeHtml(repair.secretsPath)}</code></li>
      <li>${escapeHtml(copy.repairEnv)}：<code>${escapeHtml(repair.appEnv)}</code> / <code>${escapeHtml(repair.genericEnv)}</code></li>
    </ul>
    <p class="tiny">${escapeHtml(repair.suggestedScope)}</p>`
        : ""
    }
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

rpc("snapshot").then(render).catch((error: unknown) => {
  if (app) {
    app.textContent = error instanceof Error ? error.message : String(error);
  }
});
