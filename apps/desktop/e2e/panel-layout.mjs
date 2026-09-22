import { chromium } from "playwright";

const url = process.env.BOARD_URL;
if (!url) throw new Error("missing Issue #147 E2E environment");

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ locale: "zh-CN", viewport: { width: 1280, height: 840 } });
const panelKey = (clientId) => `agent-taskboard-client-panels:v1:browser:${clientId}`;

const openClient = async ({ clientId, panelState, tauriLabel = "", runId = "", hostId = "", projectId = "" }) => {
  const page = await context.newPage();
  await page.addInitScript(({ protocol, id, state, desktopLabel }) => {
    window.__HOST_PROTOCOL__ = protocol;
    sessionStorage.setItem("agent-taskboard-client-id", id);
    window.name = `agent-taskboard-client-window:${id}`;
    if (desktopLabel) {
      window.__TAURI_INTERNALS__ = { metadata: { currentWindow: { label: desktopLabel } } };
    }
    if (state) {
      const kind = desktopLabel ? "tauri" : "browser";
      const identity = desktopLabel || id;
      localStorage.setItem(`agent-taskboard-client-panels:v1:${kind}:${identity}`, JSON.stringify(state));
    }
  }, { protocol: url, id: clientId, state: panelState, desktopLabel: tauriLabel });
  const query = new URLSearchParams({ runId, hostId, projectId }).toString();
  const pageUrl = runId ? `${url}${url.includes("?") ? "&" : "?"}${query}` : url;
  await page.goto(pageUrl, { waitUntil: "domcontentloaded" });
  await page.waitForSelector(runId ? '[data-terminal-surface="live"]' : ".lanes");
  await page.waitForTimeout(100);
  await page.bringToFront();
  return page;
};

let pointerId = 100;
const dragBy = async (page, selector, dx) => {
  const target = page.locator(selector);
  const box = await target.boundingBox();
  if (!box) throw new Error(`missing resize target ${selector}`);
  pointerId += 1;
  const start = { x: box.x + box.width / 2, y: box.y + Math.min(box.height / 2, 80) };
  await target.dispatchEvent("pointerdown", { pointerId, clientX: start.x, clientY: start.y, bubbles: true });
  await page.evaluate(({ id, x, y }) => {
    document.dispatchEvent(new PointerEvent("pointermove", { pointerId: id, clientX: x, clientY: y, bubbles: true }));
    window.dispatchEvent(new PointerEvent("pointerup", { pointerId: id, clientX: x, clientY: y, bubbles: true }));
  }, { id: pointerId, x: start.x + dx, y: start.y });
};

const widthOf = async (page, selector) => {
  const box = await page.locator(selector).boundingBox();
  if (!box) {
    const visibleDomWidth = await page.$eval(selector, (node) => {
      const rect = node.getBoundingClientRect();
      const style = getComputedStyle(node);
      const hit = document.elementFromPoint(
        Math.min(innerWidth - 1, Math.max(0, rect.left + rect.width / 2)),
        Math.min(innerHeight - 1, Math.max(0, rect.top + rect.height / 2)),
      );
      return style.display !== "none"
        && style.visibility !== "hidden"
        && rect.width > 0
        && rect.height > 0
        && Boolean(hit && node.contains(hit))
        ? rect.width
        : 0;
    });
    if (visibleDomWidth > 0) return visibleDomWidth;
    const diagnostic = await page.evaluate((target) => ({
      targetExists: Boolean(document.querySelector(target)),
      targetDisplay: document.querySelector(target) ? getComputedStyle(document.querySelector(target)).display : "missing",
      mobile: document.documentElement.dataset.mobile,
      viewport: document.documentElement.dataset.viewport,
      innerWidth: window.innerWidth,
      clientId: sessionStorage.getItem("agent-taskboard-client-id"),
      windowName: window.name,
      storage: Object.fromEntries(Object.entries(localStorage).filter(([key]) => key.includes("client-panels"))),
      frame: document.querySelector(".frame")?.className,
      frameRect: document.querySelector(".frame")?.getBoundingClientRect().toJSON(),
      bodyRect: document.querySelector(".body")?.getBoundingClientRect().toJSON(),
      targetRect: document.querySelector(target)?.getBoundingClientRect().toJSON(),
    }), selector);
    throw new Error(`missing region ${selector}: ${JSON.stringify(diagnostic)}`);
  }
  return box.width;
};

const cssPanelWidth = (page, name) => page.$eval(".frame", (node, property) =>
  Number.parseFloat(getComputedStyle(node).getPropertyValue(property)), name);

const assertNear = (actual, expected, label, tolerance = 3) => {
  if (Math.abs(actual - expected) > tolerance) {
    throw new Error(`${label}: expected ${expected}px, got ${actual}px`);
  }
};

const assertStoredContract = async (page, storageKey, expected = {}) => {
  const state = await page.evaluate((key) => JSON.parse(localStorage.getItem(key) ?? "null"), storageKey);
  const keys = Object.keys(state ?? {}).sort();
  const allowed = ["changesPanelWidth", "rightRailWidth", "rightSide", "sidebarVisible", "sidebarWidth"].sort();
  if (JSON.stringify(keys) !== JSON.stringify(allowed)) {
    throw new Error(`panel persistence contains unsupported fields: ${JSON.stringify(state)}`);
  }
  const serialized = JSON.stringify(state);
  for (const forbidden of ["\"x\"", "\"y\"", "height", "floating", "overlap"]) {
    if (serialized.includes(forbidden)) throw new Error(`panel persistence contains ${forbidden}: ${serialized}`);
  }
  for (const [key, value] of Object.entries(expected)) {
    if (state[key] !== value) throw new Error(`stored ${key} should be ${value}, got ${JSON.stringify(state)}`);
  }
  return state;
};

const workspaceGeometry = (page, panelSelector, terminalMarker) => page.evaluate(({ panelSelector, terminalMarker }) => {
  const terminalMain = document.querySelector(".focus-workspace-main");
  const terminalHost = document.querySelector(".lifted-terminal .pty-host");
  const screen = terminalHost?.querySelector(".xterm-screen")?.getBoundingClientRect();
  const measureNode = terminalHost?.querySelector(".xterm-char-measure-element");
  const measure = measureNode?.getBoundingClientRect();
  const measuredCharacters = measureNode?.textContent?.length ?? 0;
  const panel = document.querySelector(panelSelector);
  const resizeHandle = panel?.querySelector("[data-panel-resize]");
  const handle = resizeHandle?.getBoundingClientRect();
  const handleHitWidth = resizeHandle && handle
    ? [...Array(8).keys()].filter((offset) => {
        const hit = document.elementFromPoint(
          handle.left + offset + 0.5,
          handle.top + Math.min(handle.height / 2, 80),
        );
        return hit?.closest("[data-panel-resize]") === resizeHandle;
      }).length
    : 0;
  const side = document.querySelector(".side");
  return {
    runId: document.querySelector('[data-terminal-surface="live"]')?.getAttribute("data-run"),
    terminalMainDisplay: terminalMain ? getComputedStyle(terminalMain).display : "missing",
    terminalStillMounted: window[terminalMarker] === terminalHost,
    terminalColumns: screen && measure?.width && measuredCharacters
      ? Math.floor(screen.width / (measure.width / measuredCharacters))
      : 0,
    terminalRows: screen && measure?.height ? Math.floor(screen.height / measure.height) : 0,
    terminalScreenSize: screen ? [screen.width, screen.height] : [],
    terminalCellSize: measure && measuredCharacters ? [measure.width / measuredCharacters, measure.height] : [],
    panelWidth: panel?.getBoundingClientRect().width ?? 0,
    panelControls: panel?.querySelectorAll("button, summary").length ?? 0,
    handleWidth: handle?.width ?? 0,
    handleHitWidth,
    handleInsideViewport: Boolean(handle && handle.left >= 0 && handle.right <= innerWidth),
    sideOverflow: side ? side.scrollWidth - side.clientWidth : 0,
    pageOverflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
  };
}, { panelSelector, terminalMarker });

const assertMinimumWorkspace = (geometry, label, runId) => {
  if (
    geometry.runId !== runId
    || geometry.terminalMainDisplay === "none"
    || !geometry.terminalStillMounted
    || geometry.terminalColumns < 40
    || geometry.terminalRows < 8
    || geometry.panelControls < 2
    || geometry.handleWidth < 8
    || geometry.handleHitWidth < 8
    || !geometry.handleInsideViewport
    || geometry.sideOverflow > 0
    || geometry.pageOverflow > 0
  ) {
    throw new Error(`${label} must keep the Run, Terminal, panel controls and resize handle operable: ${JSON.stringify(geometry)}`);
  }
};

const waitForRunOutput = (page, runId, text) => page.waitForFunction(async ({ protocol, runId, text }) => {
  const response = await fetch(`${protocol}/runs/${encodeURIComponent(runId)}/output?after=0`);
  if (!response.ok) return false;
  const json = await response.json();
  const bytes = Uint8Array.from(atob(json.data), (byte) => byte.charCodeAt(0));
  return new TextDecoder().decode(bytes).includes(text);
}, { protocol: url, runId, text });

const browserPage = await openClient({ clientId: "panel-browser-a" });
const browserKey = panelKey("panel-browser-a");
assertNear(await widthOf(browserPage, ".side"), 248, "default sidebar width");

await dragBy(browserPage, '[data-panel-resize="sidebar"]', 60);
assertNear(await widthOf(browserPage, ".side"), 308, "resized sidebar width");
await browserPage.locator(".issue-card-main", { hasText: "panel layout issue" }).click();
await browserPage.waitForSelector(".lifted-run");
await browserPage.waitForSelector('[data-fixed-panel="right-rail"]');
await browserPage.click('.workspace-rail-section[data-workspace-section="issue"] > summary');
await browserPage.waitForSelector('[data-document-state="ready"]');
assertNear(await widthOf(browserPage, '[data-fixed-panel="right-rail"]'), 320, "default right rail width");
await dragBy(browserPage, '[data-panel-resize="right-rail"]', -80);
assertNear(await widthOf(browserPage, '[data-fixed-panel="right-rail"]'), 400, "resized right rail width");
const activeWorkspaceRunId = await browserPage.$eval('[data-terminal-surface="live"]', (node) => node.getAttribute("data-run"));
await browserPage.evaluate(() => { window.__PANEL_TERMINAL_HOST__ = document.querySelector(".lifted-terminal .pty-host"); });
await browserPage.setViewportSize({ width: 880, height: 560 });
await browserPage.waitForFunction(() => document.documentElement.dataset.viewport === "compact-desktop");
const compactRailLayout = await workspaceGeometry(browserPage, '[data-fixed-panel="right-rail"]', "__PANEL_TERMINAL_HOST__");
assertMinimumWorkspace(compactRailLayout, "880x560 right rail layout", activeWorkspaceRunId);
await browserPage.click(".lifted-terminal .pty-host");
await browserPage.keyboard.type("compact draft ");
await dragBy(browserPage, '[data-panel-resize="right-rail"]', 100);
assertNear(await widthOf(browserPage, '[data-fixed-panel="right-rail"]'), 300, "compact resized right rail width");
await dragBy(browserPage, '[data-panel-resize="right-rail"]', -100);
const resizedCompactRail = await workspaceGeometry(browserPage, '[data-fixed-panel="right-rail"]', "__PANEL_TERMINAL_HOST__");
assertMinimumWorkspace(resizedCompactRail, "resized 880x560 right rail layout", activeWorkspaceRunId);
await browserPage.click('.chrome button[data-act="view-changes"]');
await browserPage.waitForSelector('[data-fixed-panel="changes-panel"] .changes-sheet[data-view-state="loaded"]');
const compactChangesLayout = await workspaceGeometry(browserPage, '[data-fixed-panel="changes-panel"]', "__PANEL_TERMINAL_HOST__");
assertMinimumWorkspace(compactChangesLayout, "880x560 changes layout", activeWorkspaceRunId);
await browserPage.click('.chrome button[data-act="view-changes"]');
await browserPage.waitForSelector('[data-fixed-panel="right-rail"]');
const compactRailAfterClose = await workspaceGeometry(browserPage, '[data-fixed-panel="right-rail"]', "__PANEL_TERMINAL_HOST__");
assertMinimumWorkspace(compactRailAfterClose, "closed 880x560 changes layout", activeWorkspaceRunId);
await browserPage.click(".lifted-terminal .pty-host");
await browserPage.keyboard.type("survives panels");
await browserPage.keyboard.press("Enter");
await waitForRunOutput(browserPage, activeWorkspaceRunId, "compact draft survives panels");
await browserPage.click('.chrome button[data-act="view-changes"]');
await browserPage.waitForSelector('[data-fixed-panel="changes-panel"] .changes-sheet[data-view-state="loaded"]');
await browserPage.setViewportSize({ width: 900, height: 640 });
await browserPage.waitForFunction(() => document.documentElement.dataset.viewport === "full-desktop");
const fullDesktopBoundary = await workspaceGeometry(browserPage, '[data-fixed-panel="changes-panel"]', "__PANEL_TERMINAL_HOST__");
assertMinimumWorkspace(fullDesktopBoundary, "900x640 desktop boundary", activeWorkspaceRunId);
await browserPage.setViewportSize({ width: 1280, height: 840 });
await dragBy(browserPage, '[data-panel-resize="changes-panel"]', -80);
assertNear(await widthOf(browserPage, '[data-fixed-panel="changes-panel"]'), 600, "resized changes panel width");
if (await browserPage.$("[data-panel-drag], [data-panel-mode], [data-floating], [data-workbench-panel]")) {
  throw new Error("fixed regions must not expose drag, float, coordinate or legacy workbench controls");
}
await assertStoredContract(browserPage, browserKey, {
  sidebarVisible: true,
  sidebarWidth: 308,
  rightSide: "changes",
  rightRailWidth: 400,
  changesPanelWidth: 600,
});

await browserPage.reload({ waitUntil: "domcontentloaded" });
try {
  await browserPage.waitForSelector('[data-fixed-panel="changes-panel"] .changes-sheet[data-view-state="loaded"]');
} catch (error) {
  const diagnostic = await browserPage.evaluate(async (protocol) => {
    const response = await fetch(`${protocol}/rpc`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ op: "snapshot", clientInstanceId: sessionStorage.getItem("agent-taskboard-client-id") }),
    });
    const result = await response.json();
    return {
      body: document.querySelector(".body")?.className,
      focus: Boolean(document.querySelector(".focus-workspace-layout")),
      board: Boolean(document.querySelector(".lanes")),
      changes: Boolean(document.querySelector('[data-fixed-panel="changes-panel"]')),
      stored: Object.fromEntries(Object.entries(localStorage).filter(([key]) => key.includes("client-panels"))),
      clientId: sessionStorage.getItem("agent-taskboard-client-id"),
      snapshot: { workspaceView: result.snapshot?.workspaceView, focusedRunId: result.snapshot?.focusedRunId },
    };
  }, url);
  throw new Error(`failed to restore changes panel: ${JSON.stringify(diagnostic)}; ${error}`);
}
assertNear(await widthOf(browserPage, '[data-fixed-panel="changes-panel"]'), 600, "restored changes panel width");
await browserPage.click('.chrome button[data-act="view-changes"]');
await browserPage.waitForFunction(() => !document.querySelector('[data-fixed-panel="changes-panel"]'));
await browserPage.waitForSelector('[data-fixed-panel="right-rail"]');
assertNear(await widthOf(browserPage, '[data-fixed-panel="right-rail"]'), 400, "restored right rail width");
await browserPage.click('button[data-act="toggle-sidebar"]');
await browserPage.waitForFunction(() => !document.querySelector(".side"));
await assertStoredContract(browserPage, browserKey, {
  sidebarVisible: false,
  sidebarWidth: 308,
  rightSide: "rail",
  rightRailWidth: 400,
  changesPanelWidth: 600,
});
await browserPage.reload({ waitUntil: "domcontentloaded" });
await browserPage.waitForSelector(".lifted-run");
await browserPage.waitForSelector('[data-fixed-panel="right-rail"]');
if (await browserPage.$(".side")) throw new Error("the same Client must restore sidebar visibility after reload");
await browserPage.click('button[data-act="toggle-sidebar"]');
await browserPage.waitForSelector(".side");
assertNear(await widthOf(browserPage, ".side"), 308, "restored sidebar width");

await browserPage.setViewportSize({ width: 390, height: 844 });
await browserPage.waitForFunction(() => document.documentElement.dataset.mobile === "true" && document.querySelector(".mobile-workspace-view"));
const mobileMainWindow = await browserPage.evaluate(() => ({
  mobileWorkspace: Boolean(document.querySelector(".mobile-workspace-view")),
  mobileNav: Boolean(document.querySelector(".mobile-nav")),
  mobileInput: Boolean(document.querySelector(".mobile-input-row")),
  desktopSidebar: Boolean(document.querySelector(".side")),
  desktopRail: Boolean(document.querySelector('[data-fixed-panel="right-rail"], [data-fixed-panel="changes-panel"]')),
  horizontalOverflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
}));
if (
  !mobileMainWindow.mobileWorkspace
  || !mobileMainWindow.mobileNav
  || !mobileMainWindow.mobileInput
  || mobileMainWindow.desktopSidebar
  || mobileMainWindow.desktopRail
  || mobileMainWindow.horizontalOverflow > 0
) {
  throw new Error(`390x844 main window must keep the mobile single-task workspace: ${JSON.stringify(mobileMainWindow)}`);
}
await browserPage.setViewportSize({ width: 1280, height: 840 });
await browserPage.waitForFunction(() => document.documentElement.dataset.mobile === "false" && document.querySelector('.lifted-terminal .pty-host'));

const secondBrowserPage = await openClient({
  clientId: "panel-browser-b",
  panelState: { sidebarVisible: true, sidebarWidth: 248, rightSide: "rail", rightRailWidth: 320, changesPanelWidth: 520 },
});
assertNear(await cssPanelWidth(secondBrowserPage, "--client-sidebar-width"), 248, "independent Browser Client sidebar width");
assertNear(await cssPanelWidth(secondBrowserPage, "--client-right-rail-width"), 320, "independent Browser Client right rail width");
await assertStoredContract(secondBrowserPage, panelKey("panel-browser-b"), {
  sidebarVisible: true,
  sidebarWidth: 248,
  rightSide: "rail",
  rightRailWidth: 320,
  changesPanelWidth: 520,
});
await secondBrowserPage.close();

const clampedPage = await openClient({
  clientId: "panel-browser-clamped",
  panelState: {
    sidebarVisible: true,
    sidebarWidth: 9999,
    rightSide: "rail",
    rightRailWidth: -50,
    changesPanelWidth: 9999,
  },
});
assertNear(await cssPanelWidth(clampedPage, "--client-sidebar-width"), 320, "clamped sidebar width");
assertNear(await cssPanelWidth(clampedPage, "--client-right-rail-width"), 280, "clamped right rail width");
await assertStoredContract(clampedPage, panelKey("panel-browser-clamped"), {
  sidebarWidth: 320,
  rightRailWidth: 280,
  changesPanelWidth: 640,
});
await clampedPage.close();

const corruptPage = await openClient({
  clientId: "panel-browser-corrupt",
  panelState: { sidebarVisible: "yes", sidebarWidth: 300, rightSide: "floating", x: 20, height: 500 },
});
assertNear(await cssPanelWidth(corruptPage, "--client-sidebar-width"), 248, "corrupt state fallback sidebar width");
await assertStoredContract(corruptPage, panelKey("panel-browser-corrupt"), {
  sidebarVisible: true,
  sidebarWidth: 248,
  rightSide: "rail",
  rightRailWidth: 320,
  changesPanelWidth: 520,
});
await corruptPage.close();

const desktopPage = await openClient({
  clientId: "ignored-for-tauri",
  tauriLabel: "main",
  panelState: { sidebarVisible: true, sidebarWidth: 248, rightSide: "rail", rightRailWidth: 320, changesPanelWidth: 520 },
});
if (!(await desktopPage.$(".side"))) throw new Error("Tauri Client should keep its own visible sidebar state");
await assertStoredContract(desktopPage, "agent-taskboard-client-panels:v1:tauri:main", {
  sidebarVisible: true,
  sidebarWidth: 248,
});
await desktopPage.close();

const activeRunId = await browserPage.$eval('.lifted-terminal .pty-slot', (node) => node.dataset.run);
const snapshotFor = async (page, clientId) => page.evaluate(async ({ protocol, id }) => {
  const response = await fetch(`${protocol}/rpc`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ op: "snapshot", clientInstanceId: id }),
  });
  return (await response.json()).snapshot;
}, { protocol: url, id: clientId });
const beforeRunWindow = await snapshotFor(browserPage, "run-window-check");
await browserPage.evaluate(() => { window.__MAIN_TERMINAL_HOST__ = document.querySelector(".lifted-terminal .pty-host"); });
const runWindowPage = await openClient({
  clientId: "run-window-client",
  tauriLabel: "run-e2e",
  runId: activeRunId,
  hostId: beforeRunWindow.focusedHostId,
  projectId: beforeRunWindow.focusedProjectId,
  panelState: { sidebarVisible: true, sidebarWidth: 300, rightSide: "rail", rightRailWidth: 400, changesPanelWidth: 600 },
});
await runWindowPage.setViewportSize({ width: 390, height: 640 });
await runWindowPage.waitForTimeout(100);
const narrowRunWindow = await runWindowPage.evaluate(() => ({
  mobile: document.documentElement.dataset.mobile,
  liveTerminal: Boolean(document.querySelector('[data-terminal-surface="live"]')),
  mobileNav: Boolean(document.querySelector(".mobile-nav")),
  mobileInput: Boolean(document.querySelector(".mobile-input-row")),
}));
if (
  narrowRunWindow.mobile !== "false"
  || !narrowRunWindow.liveTerminal
  || narrowRunWindow.mobileNav
  || narrowRunWindow.mobileInput
) {
  throw new Error(`native Run window must keep its operable desktop terminal when narrow: ${JSON.stringify(narrowRunWindow)}`);
}
await runWindowPage.setViewportSize({ width: 900, height: 640 });
await runWindowPage.waitForTimeout(100);
const minimumRunWindow = await runWindowPage.evaluate(() => ({
  liveTerminal: Boolean(document.querySelector('[data-terminal-surface="live"]')),
  horizontalOverflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
  verticalOverflow: document.documentElement.scrollHeight - document.documentElement.clientHeight,
}));
if (
  !minimumRunWindow.liveTerminal
  || minimumRunWindow.horizontalOverflow > 0
  || minimumRunWindow.verticalOverflow > 0
) {
  throw new Error(`native Run window minimum size must remain operable without page overflow: ${JSON.stringify(minimumRunWindow)}`);
}
await runWindowPage.click(".lifted-terminal .pty-host");
await runWindowPage.keyboard.type("minimum run window input");
await runWindowPage.keyboard.press("Enter");
await waitForRunOutput(runWindowPage, activeRunId, "minimum run window input");
const runWindowContract = await runWindowPage.evaluate((runId) => ({
  runId: document.querySelector('[data-terminal-surface="live"]')?.getAttribute("data-run"),
  sidebar: Boolean(document.querySelector(".side")),
  rail: Boolean(document.querySelector('[data-fixed-panel="right-rail"]')),
  canOpenAnother: Boolean(document.querySelector('button[data-act="open-run-window"]')),
  queryRunId: new URLSearchParams(location.search).get("runId"),
  queryHostId: new URLSearchParams(location.search).get("hostId"),
  queryProjectId: new URLSearchParams(location.search).get("projectId"),
}), activeRunId);
if (
  runWindowContract.runId !== activeRunId
  || runWindowContract.queryRunId !== activeRunId
  || runWindowContract.queryHostId !== beforeRunWindow.focusedHostId
  || runWindowContract.queryProjectId !== beforeRunWindow.focusedProjectId
  || runWindowContract.sidebar
  || runWindowContract.rail
  || runWindowContract.canOpenAnother
) {
  throw new Error(`standalone Run window must bind one existing Run with a minimal shell: ${JSON.stringify(runWindowContract)}`);
}
await assertStoredContract(runWindowPage, "agent-taskboard-client-panels:v1:tauri:run-e2e", {
  sidebarVisible: true,
});
await runWindowPage.close();
const afterRunWindow = await snapshotFor(browserPage, "run-window-check");
const mainTerminalUnaffected = await browserPage.evaluate(() => window.__MAIN_TERMINAL_HOST__ === document.querySelector(".lifted-terminal .pty-host"));
if (afterRunWindow.runs.length !== beforeRunWindow.runs.length || !afterRunWindow.runs.some((run) => run.id === activeRunId && run.status !== "ended") || !mainTerminalUnaffected) {
  throw new Error("opening and closing a standalone Run window must not add, restart, stop, or detach the main Run");
}

await browserPage.close();
await browser.close();
console.log("Issue #147 fixed panel state e2e ok");
