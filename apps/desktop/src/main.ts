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
};

type Snapshot = {
  running: boolean;
  windowVisible: boolean;
  focusedHostId: string;
  hosts: { id: string; displayName: string; local: boolean }[];
  projects: { id: string; name: string }[];
  appearance: {
    language: Language;
    theme: Theme;
    lastLightTheme: Theme;
    languages: Language[];
    themes: Theme[];
  };
  copy: ShellCopy;
  emptyActions: Array<"register-first-project" | "pair-another-host">;
};

type RpcResult = { snapshot: Snapshot; process: "keep-running" | "exit" };

const app = document.querySelector<HTMLDivElement>("#app");
if (!app) {
  throw new Error("missing #app");
}

let snapshot: Snapshot | null = null;
let settingsOpen = false;

async function protocolBase(): Promise<string> {
  for (let i = 0; i < 50; i += 1) {
    if (window.__HOST_PROTOCOL__) {
      return window.__HOST_PROTOCOL__;
    }
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  throw new Error("Host protocol is not available");
}

async function rpc(op: string, extra: Record<string, unknown> = {}): Promise<Snapshot> {
  const response = await fetch(`${await protocolBase()}/rpc`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ op, ...extra }),
  });
  if (!response.ok) {
    throw new Error((await response.text()) || `Host protocol ${response.status}`);
  }
  const result = (await response.json()) as RpcResult;
  snapshot = result.snapshot;
  return result.snapshot;
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
  const { copy, appearance, hosts, projects } = snapshot;
  document.documentElement.lang = appearance.language === "zh-CN" ? "zh-CN" : "en";
  document.documentElement.dataset.theme = appearance.theme;
  document.title = copy.appName;

  const host = hosts.find((item) => item.id === snapshot?.focusedHostId) ?? hosts[0];
  const empty = snapshot.emptyActions.length > 0;

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
                ? `<button type="button" class="item active"><span class="dot"></span>${escapeHtml(host.displayName)}<span class="tag">${escapeHtml(copy.thisMachine)}</span></button>`
                : ""
            }
            <button type="button" class="item" data-act="pair">+ ${escapeHtml(copy.pairAnotherHost)}</button>
          </div>
          <div>
            <div class="group-name">${escapeHtml(copy.projects)}</div>
            ${
              projects.length
                ? projects
                    .map((project) => `<button type="button" class="item">${escapeHtml(project.name)}</button>`)
                    .join("")
                : `<div class="nested">${escapeHtml(copy.noProjectTitle)}</div>`
            }
            <button type="button" class="item" data-act="register">+ ${escapeHtml(copy.registerFirstProject)}</button>
          </div>
        </aside>
        <main class="workspace">
          ${
            empty
              ? `<div class="empty">
                  <h1>${escapeHtml(copy.noProjectTitle)}</h1>
                  <p>${escapeHtml(copy.noProjectBody)}</p>
                  <div class="actions">
                    ${snapshot.emptyActions
                      .map(
                        (action, index) =>
                          `<button type="button" class="${index === 0 ? "primary" : ""}" data-act="${emptyActionAct(action)}">${escapeHtml(emptyActionLabel(copy, action))}</button>`,
                      )
                      .join("")}
                  </div>
                </div>`
              : ""
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
  `;
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

rpc("snapshot").then(render).catch((error: unknown) => {
  if (app) {
    app.textContent = error instanceof Error ? error.message : String(error);
  }
});
