import { chromium } from "playwright";

const url = process.env.BOARD_URL;
if (!url) throw new Error("missing Issue #147 E2E environment");

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ locale: "zh-CN", viewport: { width: 1280, height: 840 } });
const panelKey = (clientId) => `agent-taskboard-client-panels:v1:browser:${clientId}`;

const openClient = async ({ clientId, panelState, legacy = false, tauriLabel = "" }) => {
  const page = await context.newPage();
  await page.addInitScript(({ protocol, id, state, legacyState, desktopLabel }) => {
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
    if (legacyState) {
      localStorage.setItem("agent-taskboard-panel-layout:v2:browser:legacy", JSON.stringify({
        inspector: { width: 999, height: 500, x: 24, y: 40, floating: true },
      }));
      localStorage.setItem("agent-taskboard-panel-layout-registry:v2", JSON.stringify({ legacy: Date.now() }));
    }
  }, { protocol: url, id: clientId, state: panelState, legacyState: legacy, desktopLabel: tauriLabel });
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.waitForSelector(".lanes");
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

const browserPage = await openClient({ clientId: "panel-browser-a", legacy: true });
const browserKey = panelKey("panel-browser-a");
const legacyKeys = await browserPage.evaluate(() => Object.keys(localStorage).filter((key) => key.startsWith("agent-taskboard-panel-layout")));
if (legacyKeys.length) throw new Error(`legacy free-layout keys must be removed without migration: ${JSON.stringify(legacyKeys)}`);
assertNear(await widthOf(browserPage, ".side"), 248, "default sidebar width");

await dragBy(browserPage, '[data-panel-resize="sidebar"]', 60);
assertNear(await widthOf(browserPage, ".side"), 308, "resized sidebar width");
await browserPage.locator(".issue-card-main", { hasText: "panel layout issue" }).click();
await browserPage.waitForSelector(".lifted-run");
await browserPage.waitForSelector('[data-fixed-panel="right-rail"]');
await browserPage.waitForSelector('[data-document-state="ready"]');
assertNear(await widthOf(browserPage, '[data-fixed-panel="right-rail"]'), 320, "default right rail width");
await dragBy(browserPage, '[data-panel-resize="right-rail"]', -80);
assertNear(await widthOf(browserPage, '[data-fixed-panel="right-rail"]'), 400, "resized right rail width");
await browserPage.click('.chrome button[data-act="view-changes"]');
await browserPage.waitForSelector('[data-fixed-panel="changes-panel"] .changes-sheet[data-view-state="loaded"]');
assertNear(await widthOf(browserPage, '[data-fixed-panel="changes-panel"]'), 520, "default changes panel width");
await browserPage.setViewportSize({ width: 899, height: 840 });
await browserPage.waitForFunction(() => document.documentElement.dataset.viewport === "compact-desktop");
const compactChangesLayout = await browserPage.evaluate(() => ({
  workspaceDisplay: getComputedStyle(document.querySelector(".workspace")).display,
  panelWidth: document.querySelector('[data-fixed-panel="changes-panel"]')?.getBoundingClientRect().width ?? 0,
  sideWidth: document.querySelector(".side")?.getBoundingClientRect().width ?? 0,
  overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
}));
assertNear(compactChangesLayout.panelWidth, 899 - compactChangesLayout.sideWidth, "compact changes region", 4);
if (compactChangesLayout.workspaceDisplay !== "none" || compactChangesLayout.overflow > 0) {
  throw new Error(`compact desktop should show one fixed content region without overlap: ${JSON.stringify(compactChangesLayout)}`);
}
await browserPage.setViewportSize({ width: 900, height: 840 });
await browserPage.waitForFunction(() => document.documentElement.dataset.viewport === "full-desktop");
assertNear(await widthOf(browserPage, '[data-fixed-panel="changes-panel"]'), 520, "900px changes panel boundary");
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
await browserPage.waitForSelector('[data-fixed-panel="changes-panel"] .changes-sheet[data-view-state="loaded"]');
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

await browserPage.close();
await browser.close();
console.log("Issue #147 fixed panel state e2e ok");
