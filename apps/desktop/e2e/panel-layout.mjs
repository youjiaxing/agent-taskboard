import { chromium } from "playwright";

const url = process.env.BOARD_URL;
if (!url) throw new Error("missing Issue #116 E2E environment");

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ locale: "zh-CN", viewport: { width: 1280, height: 840 } });

const openClient = async ({ tauri = false } = {}) => {
  const page = await context.newPage();
  await page.addInitScript(({ protocol, desktop }) => {
    window.__HOST_PROTOCOL__ = protocol;
    if (desktop) window.__TAURI_INTERNALS__ = {};
  }, { protocol: url, desktop: tauri });
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.waitForSelector(".lanes");
  return page;
};

const dragBy = async (page, selector, dx, dy) => {
  const box = await page.locator(selector).boundingBox();
  if (!box) throw new Error(`missing drag target ${selector}`);
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width / 2 + dx, box.y + box.height / 2 + dy, { steps: 6 });
  await page.mouse.up();
};

const browserPage = await openClient();
const staleLayoutKey = "agent-taskboard-panel-layout:v1:browser:stale-history-instance";
await browserPage.evaluate(({ staleKey }) => {
  localStorage.setItem(staleKey, JSON.stringify({ inspector: { width: 999 } }));
  localStorage.setItem(
    "agent-taskboard-panel-layout-registry:v1",
    JSON.stringify({ [staleKey]: Date.now() - 8 * 86_400_000 }),
  );
}, { staleKey: staleLayoutKey });
await browserPage.reload({ waitUntil: "domcontentloaded" });
if (await browserPage.evaluate(({ staleKey }) => localStorage.getItem(staleKey), { staleKey: staleLayoutKey }) !== null) {
  throw new Error("unused historical panel Client layout instances must be pruned");
}
await browserPage.locator(".issue-card-main", { hasText: "panel layout issue" }).click();
await browserPage.waitForSelector(".lifted-run");
await browserPage.waitForSelector('[data-workbench-panel="inspector"]');
await browserPage.waitForSelector('[data-document-state="ready"]');
if (await browserPage.locator('[data-workbench-panel="inspector"]').getAttribute("data-floating") === "false") {
  await browserPage.click('[data-panel-mode="inspector"]');
  await browserPage.waitForSelector('[data-workbench-panel="inspector"][data-floating="true"]');
}
await browserPage.waitForFunction(() => getComputedStyle(document.querySelector('[data-workbench-panel="inspector"]')).position === "absolute");

for (const selector of [
  '[data-panel-drag="inspector"]',
  '[data-panel-resize="inspector"]',
  '[data-panel-size="inspector"]',
  '[data-panel-mode="inspector"]',
]) {
  await browserPage.waitForSelector(selector);
}

const inspector = browserPage.locator('[data-workbench-panel="inspector"]');
const inspectorBefore = await inspector.boundingBox();
await dragBy(browserPage, '[data-panel-drag="inspector"]', -180, 70);
const inspectorPositionAfter = await inspector.boundingBox();
if (!inspectorBefore || !inspectorPositionAfter || inspectorPositionAfter.x > inspectorBefore.x - 120) {
  throw new Error(`Inspector drag did not move the panel: ${JSON.stringify({ inspectorBefore, inspectorPositionAfter })}`);
}

await dragBy(browserPage, '[data-panel-resize="inspector"]', 96, 44);
let inspectorAfterResize = await inspector.boundingBox();
if (!inspectorAfterResize || inspectorAfterResize.width < inspectorPositionAfter.width + 60) {
  throw new Error(`Inspector resize did not produce clear width feedback: ${JSON.stringify({ inspectorPositionAfter, inspectorAfterResize })}`);
}
const inspectorSizeText = await browserPage.locator('[data-panel-size="inspector"]').textContent();
if (!inspectorSizeText?.includes(`${Math.round(inspectorAfterResize.width)}`)) {
  throw new Error(`Inspector size feedback should include its current width: ${inspectorSizeText}`);
}
const inspectorModeAfterResize = await inspector.getAttribute("data-floating");
await browserPage.click('[data-panel-mode="inspector"]');
await browserPage.waitForSelector('[data-workbench-panel="inspector"][data-floating="false"]');
const dockedInspectorBefore = await browserPage.locator('[data-workbench-panel="inspector"]').boundingBox();
await dragBy(browserPage, '[data-panel-resize="inspector"]', 80, 0);
const dockedInspectorAfter = await browserPage.locator('[data-workbench-panel="inspector"]').boundingBox();
if (!dockedInspectorBefore || !dockedInspectorAfter || dockedInspectorAfter.width > dockedInspectorBefore.width - 40) {
  throw new Error(`docked Inspector resize did not update its grid column: ${JSON.stringify({ dockedInspectorBefore, dockedInspectorAfter })}`);
}
await browserPage.click('[data-panel-mode="inspector"]');
await browserPage.waitForSelector('[data-workbench-panel="inspector"][data-floating="true"]');
inspectorAfterResize = await browserPage.locator('[data-workbench-panel="inspector"]').boundingBox();

await browserPage.reload({ waitUntil: "domcontentloaded" });
await browserPage.waitForSelector(".lifted-run");
await browserPage.waitForSelector('[data-workbench-panel="inspector"]');
await browserPage.waitForSelector('[data-document-state="ready"]');
const inspectorAfterReload = await browserPage.waitForFunction(() => {
  const panel = document.querySelector('[data-workbench-panel="inspector"]');
  if (!panel || getComputedStyle(panel).position !== "absolute") return false;
  const rect = panel.getBoundingClientRect();
  return rect.width > 0 ? { x: rect.x, y: rect.y, width: rect.width, height: rect.height } : false;
}).then((handle) => handle.jsonValue());
const inspectorModeAfterReload = await browserPage.locator('[data-workbench-panel="inspector"]').getAttribute("data-floating");
const geometryTolerance = 8;
if (
  !inspectorAfterReload
  || !inspectorAfterResize
  || Math.abs(inspectorAfterReload.width - inspectorAfterResize.width) > 2
  || Math.abs(inspectorAfterReload.height - inspectorAfterResize.height) > 2
  || Math.abs(inspectorAfterReload.x - inspectorAfterResize.x) > geometryTolerance
  || Math.abs(inspectorAfterReload.y - inspectorAfterResize.y) > geometryTolerance
  || inspectorModeAfterReload !== inspectorModeAfterResize
) {
  throw new Error(`browser Client layout did not survive reload: ${JSON.stringify({ inspectorAfterResize, inspectorAfterReload })}`);
}

const desktopPage = await openClient({ tauri: true });
await desktopPage.locator(".issue-card-main", { hasText: "panel layout issue" }).click();
await desktopPage.waitForSelector('[data-workbench-panel="inspector"]');
await desktopPage.waitForSelector('[data-document-state="ready"]');
const desktopInspector = await desktopPage.locator('[data-workbench-panel="inspector"]').boundingBox();
const desktopInspectorFloating = await desktopPage.locator('[data-workbench-panel="inspector"]').getAttribute("data-floating");
if (!desktopInspector || desktopInspectorFloating !== "false") {
  throw new Error(`Tauri and browser Clients must not overwrite each other's layout: ${JSON.stringify({ desktopInspector, inspectorAfterReload })}`);
}

const secondBrowserPage = await openClient();
await secondBrowserPage.locator(".issue-card-main", { hasText: "panel layout issue" }).click();
await secondBrowserPage.waitForSelector('[data-workbench-panel="inspector"]');
await secondBrowserPage.waitForSelector('[data-document-state="ready"]');
const secondBrowserInspector = await secondBrowserPage.locator('[data-workbench-panel="inspector"]').boundingBox();
const secondBrowserInspectorFloating = await secondBrowserPage.locator('[data-workbench-panel="inspector"]').getAttribute("data-floating");
if (!secondBrowserInspector || secondBrowserInspectorFloating !== "false" || secondBrowserInspector.width > 500) {
  throw new Error(`two Browser Clients must keep independent panel layouts: ${JSON.stringify({ secondBrowserInspector, secondBrowserInspectorFloating, inspectorAfterReload })}`);
}
await secondBrowserPage.close();

await browserPage.bringToFront();
await browserPage.waitForSelector('[data-workbench-panel="terminal"]');
for (const selector of [
  '[data-panel-drag="terminal"]',
  '[data-panel-size="terminal"]',
  '[data-panel-mode="terminal"]',
]) {
  await browserPage.waitForSelector(selector);
}
const terminalBefore = await browserPage.locator('[data-workbench-panel="terminal"]').boundingBox();
const terminalWasLifted = await browserPage.locator('[data-workbench-panel="terminal"]').evaluate((node) => node.classList.contains("lifted-terminal"));
if (terminalWasLifted) {
  await browserPage.click('[data-panel-mode="terminal"]');
  await browserPage.waitForSelector('[data-workbench-panel="terminal"][data-floating="true"]');
}
await browserPage.waitForSelector('[data-panel-resize="terminal"]');
const terminalResizeStart = await browserPage.locator('[data-workbench-panel="terminal"]').boundingBox();
await dragBy(browserPage, '[data-panel-resize="terminal"]', 0, terminalWasLifted ? 72 : -72);
const terminalAfter = await browserPage.locator('[data-workbench-panel="terminal"]').boundingBox();
if (!terminalBefore || !terminalResizeStart || !terminalAfter || terminalAfter.height < terminalResizeStart.height + 40) {
  throw new Error(`Terminal resize did not increase its height: ${JSON.stringify({ terminalBefore, terminalResizeStart, terminalAfter })}`);
}

await browserPage.setViewportSize({ width: 760, height: 620 });
await browserPage.click('[data-panel-mode="terminal"]');
await browserPage.waitForSelector('[data-workbench-panel="terminal"][data-floating="false"]');
const dockedTerminalBefore = await browserPage.locator('[data-workbench-panel="terminal"]').boundingBox();
await dragBy(browserPage, '[data-panel-resize="terminal"]', 0, 60);
const dockedTerminalAfter = await browserPage.locator('[data-workbench-panel="terminal"]').boundingBox();
if (!dockedTerminalBefore || !dockedTerminalAfter || dockedTerminalAfter.height < dockedTerminalBefore.height + 30) {
  throw new Error(`narrow docked Terminal resize should grow when dragging its bottom edge: ${JSON.stringify({ dockedTerminalBefore, dockedTerminalAfter })}`);
}
await browserPage.click('[data-panel-mode="terminal"]');
await browserPage.waitForSelector('[data-workbench-panel="terminal"][data-floating="true"]');
const visibleRunPanels = await browserPage.evaluate(() => [...document.querySelectorAll(".lifted-run > [data-workbench-panel]")]
  .filter((node) => getComputedStyle(node).display !== "none" && node.getBoundingClientRect().width > 0)
  .map((node) => node.getAttribute("data-workbench-panel")));
if (visibleRunPanels.length !== 1 || visibleRunPanels[0] !== "terminal") {
  throw new Error(`narrow Run view should expose one reachable front panel: ${JSON.stringify(visibleRunPanels)}`);
}
await browserPage.click('header.chrome button[data-act="toggle-issue"]');
await browserPage.click('header.chrome button[data-act="toggle-issue"]');
await browserPage.waitForFunction(() => {
  const panels = [...document.querySelectorAll(".lifted-run > [data-workbench-panel]")]
    .filter((node) => getComputedStyle(node).display !== "none" && node.getBoundingClientRect().width > 0)
    .map((node) => node.getAttribute("data-workbench-panel"));
  return panels.length === 1 && panels[0] === "inspector";
});
await browserPage.click('header.chrome button[data-act="toggle-issue"]');

await browserPage.setViewportSize({ width: 1280, height: 840 });

const terminalDraft = "draft survives panel switches";
const activeRunId = await browserPage.locator('[data-workbench-panel="terminal"] .pty-slot').getAttribute("data-run");
await browserPage.fill('[data-workbench-panel="terminal"] form[data-act="inject-run"] input', terminalDraft);
await browserPage.click('[data-workbench-panel="terminal"] button[data-act="hide-terminal"]');
await browserPage.waitForSelector(".lanes");
await browserPage.waitForFunction(() => !document.querySelector('[data-workbench-panel="terminal"]'));
await browserPage.click('header.chrome button[data-act="toggle-issue"]');

await browserPage.setViewportSize({ width: 760, height: 620 });
await browserPage.click(".detail-maintenance > summary");
await browserPage.fill("form[data-form='issue-comment'] textarea[name='body']", "Issue draft survives panel switches");
await browserPage.addStyleTag({ content: '[data-lane="frontier"] { max-height: 120px; }' });
const frontierScrollBefore = await browserPage.$eval('[data-lane="frontier"]', (node) => {
  node.scrollTop = node.scrollHeight;
  return node.scrollTop;
});
if (frontierScrollBefore <= 0) throw new Error("panel layout fixture needs a scrollable Frontier lane");

await browserPage.click("button[data-act='show-terminal']");
await browserPage.waitForSelector('[data-workbench-panel="terminal"]');
if (await browserPage.inputValue('[data-workbench-panel="terminal"] form[data-act="inject-run"] input') !== terminalDraft) {
  throw new Error("hiding and restoring Terminal must retain its input draft");
}
if (await browserPage.locator('[data-workbench-panel="terminal"] .pty-slot').getAttribute("data-run") !== activeRunId) {
  throw new Error("hiding and restoring Terminal must retain the current Run");
}
await browserPage.click('[data-workbench-panel="terminal"] button[data-act="hide-terminal"]');
await browserPage.waitForFunction(() => !document.querySelector('[data-workbench-panel="terminal"]'));

await browserPage.click("button[data-act='open-usage']");
await browserPage.waitForSelector('[data-workbench-panel="usage"]');
for (const selector of [
  '[data-panel-drag="usage"]',
  '[data-panel-resize="usage"]',
  '[data-panel-size="usage"]',
  "button[data-act='close-usage']",
]) {
  await browserPage.waitForSelector(selector);
}
const dockedUsageBefore = await browserPage.locator('[data-workbench-panel="usage"]').boundingBox();
await dragBy(browserPage, '[data-panel-resize="usage"]', 0, -80);
const dockedUsageAfter = await browserPage.locator('[data-workbench-panel="usage"]').boundingBox();
if (!dockedUsageBefore || !dockedUsageAfter || dockedUsageAfter.height > dockedUsageBefore.height - 40) {
  throw new Error(`docked Usage resize did not update its size: ${JSON.stringify({ dockedUsageBefore, dockedUsageAfter })}`);
}
await dragBy(browserPage, '[data-panel-drag="usage"]', 70, 45);
const usageBeforeResize = await browserPage.locator('[data-workbench-panel="usage"]').boundingBox();
await dragBy(browserPage, '[data-panel-resize="usage"]', 0, -80);
const usageAfterResize = await browserPage.locator('[data-workbench-panel="usage"]').boundingBox();
if (!usageBeforeResize || !usageAfterResize || usageAfterResize.height > usageBeforeResize.height - 40) {
  throw new Error(`Usage resize did not produce clear height feedback: ${JSON.stringify({ usageBeforeResize, usageAfterResize })}`);
}
const usageOverflow = await browserPage.locator('[data-workbench-panel="usage"]').evaluate((node) => ({
  overflowY: getComputedStyle(node).overflowY,
  scrollHeight: node.scrollHeight,
  clientHeight: node.clientHeight,
}));
if (usageOverflow.overflowY === "hidden" || usageOverflow.scrollHeight <= usageOverflow.clientHeight) {
  throw new Error(`Usage content must remain vertically reachable in a narrow window: ${JSON.stringify(usageOverflow)}`);
}

const overflow = await browserPage.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
if (overflow > 1) throw new Error(`insufficient-space layout must not force horizontal scrolling: ${overflow}`);
await browserPage.click("button[data-act='close-usage']");
await browserPage.waitForSelector(".lanes");
await browserPage.waitForSelector('.detail-hd:has-text("panel layout issue")');
await browserPage.click(".detail-maintenance > summary");
if (await browserPage.inputValue("form[data-form='issue-comment'] textarea[name='body']") !== "Issue draft survives panel switches") {
  throw new Error("switching through Usage must retain the current Issue draft");
}
const frontierScrollAfter = await browserPage.$eval('[data-lane="frontier"]', (node) => node.scrollTop);
if (Math.abs(frontierScrollAfter - frontierScrollBefore) > 1) {
  throw new Error(`switching panels must retain board scroll: ${frontierScrollBefore} -> ${frontierScrollAfter}`);
}

await browser.close();
console.log("Issue #116 panel layout e2e ok");
