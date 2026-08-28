import { chromium } from "playwright";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import {
  assertShellRegionsDoNotOverlap,
  createVisualAssert,
  installDeterministicHostProtocol,
} from "./visual-regression.mjs";
import { hostSnapshot } from "./issue-100-harness.mjs";

const url = process.env.BOARD_URL;
if (!url) {
  console.error("missing BOARD_URL");
  process.exit(1);
}
const screenshotDir = process.env.ISSUE_DOCUMENT_SCREENSHOT_DIR;
if (screenshotDir) await mkdir(screenshotDir, { recursive: true });
const capture = async (name) => {
  if (screenshotDir) await page.screenshot({ path: join(screenshotDir, name), fullPage: false });
};

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ locale: "zh-CN", viewport: { width: 1280, height: 840 } });
const page = await context.newPage();
const assertVisual = createVisualAssert(page);
page.on("pageerror", (error) => {
  console.error("pageerror", error);
});
page.on("console", (msg) => {
  if (msg.type() === "error") {
    console.error("console", msg.text());
  }
});
await installDeterministicHostProtocol(page, url);
await page.addInitScript(() => {
  window.__OPENED_URLS__ = [];
  window.open = (target) => {
    window.__OPENED_URLS__.push(String(target));
    return null;
  };
});
await page.goto(url, { waitUntil: "domcontentloaded" });
try {
  await page.waitForSelector(".lanes");
} catch (error) {
  const html = await page.content();
  console.error("page html", html.slice(0, 4000));
  throw error;
}
await page.click("button[data-act='toggle-hosts']");
const visibleHosts = await page.$$eval(".host-picker button[data-act='focus-host']", (nodes) =>
  nodes.map((node) => node.textContent?.replace(/\s+/g, " ").trim()),
);
if (visibleHosts.length < 2) {
  throw new Error(`daily shell fixture should expose multiple Hosts, got ${JSON.stringify(visibleHosts)}`);
}
await page.click("button[data-act='toggle-hosts']");
if (await page.$(".board-shell > .issue-detail")) {
  throw new Error("issue inspector should not occupy the board before an Issue is selected");
}
if (await page.$("button[data-act='toggle-issue']")) {
  throw new Error("issue inspector toggle should not appear before an Issue is selected");
}
await page.waitForSelector(".refresh-bar");
if (await page.$('.refresh-bar[data-kind="incomplete"]')) {
  const incompleteText = await page.$eval(".refresh-bar", (node) => node.textContent.replace(/\s+/g, " ").trim());
  if (!incompleteText.includes("数据不完整") || !incompleteText.includes("pagination stopped early")) {
    throw new Error(`incomplete refresh detail missing: ${incompleteText}`);
  }
  if (await page.$(".lanes") || await page.$(".dep-graph")) {
    throw new Error("incomplete tracker data must hide Frontier lanes and the dependency graph");
  }
  await page.waitForSelector('[data-empty="incomplete-read"]');
  await page.click('.refresh-bar button[data-act="refresh"]');
  await page.waitForSelector(".lanes");
}

await page.keyboard.press("?");
await page.waitForSelector(".keyboard-help");
await page.keyboard.press("?");
await page.waitForFunction(() => !document.querySelector(".keyboard-help"));
await page.keyboard.press("j");
await page.waitForFunction(() => document.activeElement?.classList.contains("issue-card-main"));
const keyboardFocusedCard = await page.evaluate(() => document.activeElement?.classList.contains("issue-card-main"));
if (!keyboardFocusedCard) {
  throw new Error("j should focus a board card");
}
await page.keyboard.press("Enter");
await page.waitForSelector(".issue-detail .detail-hd");
await page.waitForSelector("button[data-act='toggle-issue']");
const lanesWithInspector = await page.$eval(".lanes", (node) => node.getBoundingClientRect().width);
await page.click("button[data-act='toggle-issue']");
await page.waitForFunction(() => !document.querySelector(".board-shell > .issue-detail"));
const lanesWithoutInspector = await page.$eval(".lanes", (node) => node.getBoundingClientRect().width);
if (lanesWithoutInspector <= lanesWithInspector) {
  throw new Error("hiding the inspector should give the lanes more width");
}
if (!(await page.$("button[data-act='toggle-issue']"))) {
  throw new Error("hiding the inspector should keep the restore control in the chrome");
}
await page.click("button[data-act='toggle-issue']");
await page.waitForSelector(".issue-detail .detail-hd");

const beforeTypingSearch = await page.$$eval(".issue-card .issue-title", (nodes) => nodes.map((node) => node.textContent));
await page.fill("#issue-title-search", "child ready");
const whileTypingSearch = await page.$$eval(".issue-card .issue-title", (nodes) => nodes.map((node) => node.textContent));
if (whileTypingSearch.join(",") !== beforeTypingSearch.join(",")) {
  throw new Error("title search should not run before Enter");
}
await page.press("#issue-title-search", "Enter");
await page.waitForFunction(() => document.querySelectorAll(".issue-card").length === 1);
const searchResult = await page.$eval(".issue-card .issue-title", (node) => node.textContent);
if (searchResult !== "child ready") {
  throw new Error(`unexpected title search result: ${searchResult}`);
}
await page.selectOption(".issue-search select[name='state']", "closed");
await page.press("#issue-title-search", "Enter");
await page.waitForFunction(() => document.querySelectorAll(".issue-card").length === 0);
await page.fill("#issue-title-search", "");
await page.selectOption(".issue-search select[name='state']", "all");
await page.press("#issue-title-search", "Enter");
await page.waitForFunction(() => document.querySelectorAll(".issue-card").length > 1);

const refreshText = await page.$eval(".refresh-bar", (node) => node.textContent.replace(/\s+/g, " ").trim());
if (!refreshText.includes("数据截至") && !refreshText.includes("Data as of")) {
  throw new Error(`refresh bar missing as-of time: ${refreshText}`);
}
if (!refreshText.includes("下次刷新") && !refreshText.includes("Next refresh")) {
  throw new Error(`refresh bar missing auto-refresh countdown: ${refreshText}`);
}

function isRefreshRpc(req) {
  if (req.method() !== "POST" || !req.url().includes("/rpc")) return false;
  try {
    return req.postDataJSON()?.op === "refresh";
  } catch {
    return false;
  }
}

const focusRefresh = page.waitForRequest(isRefreshRpc, { timeout: 3000 });
await page.evaluate(() => window.dispatchEvent(new Event("focus")));
await focusRefresh;
await page.waitForFunction(() => {
  const bar = document.querySelector(".refresh-bar");
  return Boolean(bar) && bar.getAttribute("data-kind") !== "refreshing";
});
await new Promise((resolve) => setTimeout(resolve, 300));

let coalescedRefreshCount = 0;
const countCoalescedRefresh = (req) => {
  if (isRefreshRpc(req)) coalescedRefreshCount += 1;
};
page.on("request", countCoalescedRefresh);
await page.evaluate(() => {
  document.dispatchEvent(new Event("visibilitychange"));
  window.dispatchEvent(new Event("focus"));
});
const coalesceDeadline = Date.now() + 1500;
while (coalescedRefreshCount < 1 && Date.now() < coalesceDeadline) {
  await new Promise((resolve) => setTimeout(resolve, 50));
}
await new Promise((resolve) => setTimeout(resolve, 250));
page.off("request", countCoalescedRefresh);
if (coalescedRefreshCount !== 1) {
  throw new Error(`same foreground event should refresh once, got ${coalescedRefreshCount}`);
}

await page.click(".refresh-bar button[data-act='refresh']");
await page.waitForSelector(".lanes");

const headers = await page.$$eval(".lane-hd", (nodes) =>
  nodes.map((node) => node.textContent.replace(/\s+/g, " ").trim()),
);
if (headers.length !== 4) {
  throw new Error(`expected 4 columns, got ${JSON.stringify(headers)}`);
}
if (!headers[0].startsWith("阻塞中") || !headers[1].startsWith("Frontier") || !headers[2].startsWith("进行中") || !headers[3].startsWith("最近完成")) {
  throw new Error(`unexpected column order: ${JSON.stringify(headers)}`);
}

const dailyShellGeometry = await page.evaluate(() => {
  const rect = (selector) => document.querySelector(selector)?.getBoundingClientRect();
  const chrome = rect(".chrome");
  const side = rect(".side");
  const lanes = [...document.querySelectorAll(".lane")].map((node) => node.getBoundingClientRect());
  const boardTabs = [...document.querySelectorAll('.chrome [data-act="center-view"]')];
  return {
    chromeHeight: chrome?.height ?? 0,
    sideWidth: side?.width ?? 0,
    boardTabs: boardTabs.map((node) => node.textContent?.trim()),
    laneLefts: lanes.map((lane) => lane.left),
    laneWidths: lanes.map((lane) => lane.width),
    laneBorderWidths: lanes.map((_, index) =>
      getComputedStyle(document.querySelectorAll(".lane")[index]).borderLeftWidth,
    ),
    horizontalOverflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
  };
});
if (dailyShellGeometry.chromeHeight > 40) {
  throw new Error(`desktop chrome should stay native and compact, got ${dailyShellGeometry.chromeHeight}px`);
}
if (dailyShellGeometry.sideWidth < 220 || dailyShellGeometry.sideWidth > 250) {
  throw new Error(`desktop Host / Project hierarchy should keep a stable native rail, got ${dailyShellGeometry.sideWidth}px`);
}
if (dailyShellGeometry.boardTabs.join("|") !== "看板|依赖图") {
  throw new Error(`board and graph controls should live in the stable middle chrome, got ${JSON.stringify(dailyShellGeometry.boardTabs)}`);
}
if (dailyShellGeometry.laneLefts.some((left, index, all) => index > 0 && left <= all[index - 1])) {
  throw new Error(`four desktop lanes should remain ordered left to right: ${JSON.stringify(dailyShellGeometry.laneLefts)}`);
}
if (dailyShellGeometry.laneWidths.some((width) => width < 140)) {
  throw new Error(`four desktop lanes should remain scannable without horizontal paging: ${JSON.stringify(dailyShellGeometry.laneWidths)}`);
}
if (dailyShellGeometry.laneBorderWidths.some((width) => width !== "0px")) {
  throw new Error(`main board lanes should be calm surfaces instead of bordered dashboard cards: ${JSON.stringify(dailyShellGeometry.laneBorderWidths)}`);
}
if (dailyShellGeometry.horizontalOverflow > 0) {
  throw new Error(`daily desktop shell should not create page-level horizontal scrolling: ${dailyShellGeometry.horizontalOverflow}px`);
}
await assertVisual("issue-99-desktop-1280x840.png");
await assertShellRegionsDoNotOverlap(page);

const shellStructure = async () => page.evaluate(() => ({
  regions: [".chrome", ".side", ".board-main", ".issue-detail"]
    .map((selector) => Boolean(document.querySelector(selector))),
  lanes: [...document.querySelectorAll(".lane")].map((node) => ({
    lane: node.getAttribute("data-lane"),
    left: Math.round(node.getBoundingClientRect().left),
    width: Math.round(node.getBoundingClientRect().width),
  })),
}));
const warmStructure = await shellStructure();
for (const theme of ["plain-paper", "plain-night", "warm-paper"]) {
  await page.click("button[data-act='settings']");
  await page.click(`button[data-act='theme'][data-id='${theme}']`);
  await page.click(".overlay[data-act='close-settings']", { position: { x: 2, y: 2 } });
  const themedStructure = await shellStructure();
  if (JSON.stringify(themedStructure) !== JSON.stringify(warmStructure)) {
    throw new Error(`theme ${theme} changed the shell information architecture: ${JSON.stringify(themedStructure)}`);
  }
}

await page.setViewportSize({ width: 1440, height: 900 });
let releaseDocumentRefresh;
const documentRefreshGate = new Promise((resolve) => {
  releaseDocumentRefresh = resolve;
});
let cachedDocumentInjected = false;
const preserveCachedDocumentDuringRefresh = async (route) => {
  if (!route.request().url().endsWith("/rpc")) {
    await route.continue();
    return;
  }
  let request;
  try {
    request = route.request().postDataJSON();
  } catch {
    await route.continue();
    return;
  }
  if (
    !cachedDocumentInjected
    && request?.op === "snapshot"
    && request.clientView?.selectedIssueId === "you/garden#2"
  ) {
    cachedDocumentInjected = true;
    const response = await route.fetch();
    const result = await response.json();
    result.snapshot.board.selected.document = {
      kind: "loading",
      body: "# Cached issue body\n\nVisible while Tracker refreshes.",
      fetchedAtMs: Date.now() - 60_000,
    };
    await route.fulfill({ response, json: result });
    return;
  }
  if (request?.op === "loadIssueDocument" && request.issueId === "you/garden#2") {
    await documentRefreshGate;
  }
  await route.continue();
};
await page.route("**/*", preserveCachedDocumentDuringRefresh);
await page.click(".issue-card:has-text('child ready') .issue-card-main");
await page.waitForSelector(".detail-hd:has-text('child ready')");
await page.waitForSelector('[data-document-state="loading"] .issue-markdown:has-text("Cached issue body")');
const loadingStatus = await page.$eval('[data-document-state="loading"] .document-status', (node) => node.textContent);
if (!loadingStatus?.includes("数据截至") && !loadingStatus?.includes("Data as of")) {
  throw new Error(`cached Issue document should keep its as-of time while refreshing, got ${loadingStatus}`);
}
releaseDocumentRefresh();
await page.waitForSelector('[data-document-state="ready"]');
await page.unroute("**/*", preserveCachedDocumentDuringRefresh);
const documentText = await page.$eval(".issue-markdown", (node) => node.textContent?.replace(/\s+/g, " ").trim());
if (!documentText?.includes("Can the operator read every constraint") || !documentText.includes("Paragraph six")) {
  throw new Error(`Issue document should render the complete long body, got ${documentText}`);
}
for (const selector of [".issue-markdown h1", ".issue-markdown h2", ".issue-markdown strong", ".issue-markdown code", ".issue-markdown ul"]) {
  if (!(await page.$(selector))) throw new Error(`Markdown rendering missing ${selector}`);
}
if (await page.$(".issue-markdown script") || await page.evaluate(() => window.__ISSUE_HTML_EXECUTED__ === true)) {
  throw new Error("raw Issue HTML must stay escaped and inert");
}
if (!(await page.$('.issue-markdown a[data-url="https://github.com/you/garden/issues/2"]'))) {
  throw new Error("safe HTTPS markdown link should remain available");
}
if (await page.$('.issue-markdown [data-url^="javascript:"]')) {
  throw new Error("dangerous markdown URLs must not become actions");
}
if (!(await page.$(".issue-markdown .unsafe-link"))) {
  throw new Error("dangerous markdown link should be rendered as inert text");
}
const sectionOrder = await page.evaluate(() => {
  const documentTop = document.querySelector(".issue-document")?.getBoundingClientRect().top ?? 0;
  const family = document.querySelector(".detail-block")?.getBoundingClientRect().top ?? 0;
  return { document: documentTop, family };
});
if (sectionOrder.family <= sectionOrder.document) {
  throw new Error(`family and Dependency sections should follow the document: ${JSON.stringify(sectionOrder)}`);
}
const headerTopBeforeScroll = await page.$eval(".detail-sticky", (node) => node.getBoundingClientRect().top);
await page.$eval(".detail-scroll", (node) => { node.scrollTop = node.scrollHeight; });
const headerTopAfterScroll = await page.$eval(".detail-sticky", (node) => node.getBoundingClientRect().top);
if (Math.abs(headerTopAfterScroll - headerTopBeforeScroll) > 1) {
  throw new Error("Issue title and actions should stay pinned while document content scrolls");
}
const detailScroll = await page.$(".detail-scroll");
const detailScrollBox = await detailScroll?.boundingBox();
const detailScrollBeforeTicks = await page.$eval(".detail-scroll", (node) => {
  node.scrollTop = Math.min(240, node.scrollHeight - node.clientHeight);
  return node.scrollTop;
});
if (!detailScroll || !detailScrollBox || detailScrollBeforeTicks <= 0) {
  throw new Error("detail scroll regression needs a scrollable Issue document");
}
await page.mouse.move(detailScrollBox.x + detailScrollBox.width / 2, detailScrollBox.y + detailScrollBox.height / 2);
await page.mouse.down();
for (let tickIndex = 0; tickIndex < 2; tickIndex += 1) {
  const response = page.waitForResponse((candidate) =>
    candidate.url().endsWith("/rpc") && candidate.request().postData()?.includes('"op":"tick"'),
  );
  await page.evaluate(() => window.__RUN_INTERVAL_CALLBACKS__());
  await response;
}
await page.mouse.up();
await page.waitForTimeout(50);
const detailScrollAfterTicks = await page.$eval(".detail-scroll", (node) => node.scrollTop);
if (Math.abs(detailScrollAfterTicks - detailScrollBeforeTicks) > 1) {
  throw new Error(`releasing the pointer after two Host ticks must preserve Issue scroll: ${detailScrollBeforeTicks} -> ${detailScrollAfterTicks}`);
}
await page.$eval(".detail-scroll", (node) => { node.scrollTop = 0; });
await capture("issue-98-desktop-detail-1440x900.png");
await assertVisual("issue-99-desktop-1440x900.png");
await assertShellRegionsDoNotOverlap(page);
const normalDetailWidth = await page.$eval(".board-shell > .issue-detail", (node) => node.getBoundingClientRect().width);
if (normalDetailWidth < 340) {
  throw new Error(`Issue document should be readable in the default desktop shell, got ${normalDetailWidth}px`);
}
if (await page.$('button[data-act="toggle-issue-width"]')) {
  throw new Error("Issue details should not expose a widen/narrow action");
}
const detailHide = page.locator('.issue-detail .detail-title-row button[data-act="toggle-issue"]');
if ((await detailHide.count()) !== 1) {
  throw new Error("Issue details should expose one local hide control");
}
if ((await detailHide.getAttribute("aria-label")) !== "收起详情" || (await detailHide.textContent())?.trim()) {
  throw new Error("Issue detail hide control should be icon-only with an accessible label");
}
if ((await detailHide.locator("svg").count()) !== 1) {
  throw new Error("Issue detail hide control should use a meaningful panel icon");
}
await detailHide.click();
await page.waitForFunction(() => !document.querySelector(".board-shell > .issue-detail"));
const restoreDetail = page.locator('button[data-act="toggle-issue"][aria-label="显示详情"]');
if ((await restoreDetail.count()) !== 1 || (await restoreDetail.locator("svg").count()) !== 1) {
  throw new Error("collapsed Issue details should keep an icon-only restore control in chrome");
}
await restoreDetail.click();
await page.waitForSelector(".board-shell > .issue-detail");
await page.click(".issue-detail button[data-act='open-issue']");
const openedDetailUrl = await page.evaluate(() => window.__OPENED_URLS__.at(-1));
if (openedDetailUrl !== "https://github.com/you/garden/issues/2") {
  throw new Error(`details should open the GitHub Issue, got ${openedDetailUrl}`);
}
const beforeFrontier = await page.$$eval('[data-lane="frontier"] .issue-card', (nodes) => nodes.length);

await page.click(".name-btn:has-text('#1 parent')");
await page.waitForSelector(".detail-hd:has-text('parent')");
const stillFrontier = await page.$$eval('[data-lane="frontier"] .issue-card', (nodes) => nodes.length);
if (stillFrontier !== beforeFrontier) {
  throw new Error("clicking a parent link filtered the board");
}

await page.click("button:has-text('只看这些子票')");
try {
  await page.waitForSelector("button:has-text('清除过滤')", { timeout: 2000 });
} catch {
  const diagnostic = await page.evaluate(() => ({
    clientView: Object.entries(localStorage).filter(([key]) => key.includes("client-view")),
    boardHint: document.querySelector(".board-hint")?.textContent,
    detail: document.querySelector(".detail-hd")?.textContent,
  }));
  throw new Error(`parent filter did not render: ${JSON.stringify(diagnostic)}`);
}
const filtered = await page.$$eval('[data-lane="frontier"] .issue-card .issue-title', (nodes) =>
  nodes.map((node) => node.textContent),
);
if (filtered.join(",") !== "child ready") {
  throw new Error(`parent filter should show only children, got ${JSON.stringify(filtered)}`);
}

await page.click("button:has-text('清除过滤')");
await page.waitForFunction(() => !document.querySelector("button[data-act='clear-filter']"));

const boardActive = await page.$eval("button[data-act='center-view'][data-id='board']", (node) =>
  node.classList.contains("active"),
);
if (!boardActive) {
  throw new Error("factory default should be the board view");
}

const graphTabBeforeTick = await page.$("button[data-act='center-view'][data-id='graph']");
const graphTabBox = await graphTabBeforeTick?.boundingBox();
if (!graphTabBeforeTick || !graphTabBox) {
  throw new Error("single-click navigation regression needs the graph tab");
}
await page.mouse.move(graphTabBox.x + graphTabBox.width / 2, graphTabBox.y + graphTabBox.height / 2);
await page.mouse.down();
const navigationTickResponse = page.waitForResponse((response) =>
  response.url().endsWith("/rpc") && response.request().postData()?.includes('"op":"tick"'),
);
await page.evaluate(() => window.__RUN_INTERVAL_CALLBACKS__());
await navigationTickResponse;
await page.waitForTimeout(50);
if (!(await graphTabBeforeTick.evaluate((node) => node.isConnected))) {
  throw new Error("Host tick should not replace a navigation target while its pointer is pressed");
}
await page.mouse.up();
await page.waitForSelector(".dep-graph", { timeout: 1000 });
const overviewTitles = await page.$$eval(".graph-node .issue-title", (nodes) =>
  nodes.map((node) => node.textContent),
);
for (const title of ["parent", "child ready", "child blocked", "unparented ready", "waiting on history", "blocker", "active work"]) {
  if (!overviewTitles.includes(title)) {
    throw new Error(`dependency overview should include open Issue ${title}, got ${JSON.stringify(overviewTitles)}`);
  }
}
if (overviewTitles.includes("older closed") || (await page.$('[data-graph-mode="overview"]')) == null) {
  throw new Error(`dependency overview should contain only open Issues: ${JSON.stringify(overviewTitles)}`);
}
await page.click(".graph-node:has-text('child blocked') .graph-node-main");
await page.waitForFunction(() => document.querySelector(".graph-center-label")?.textContent?.includes("#3 child blocked"));
await page.waitForSelector("button[data-act='graph-overview']");
await page.click("button[data-act='graph-overview']");
await page.waitForSelector('[data-graph-mode="overview"]');
await page.click("button[data-act='center-view'][data-id='board']");
await page.waitForSelector(".lanes");

await page.click(".issue-card:has-text('child blocked') button[data-act='view-dependencies']");
await page.waitForSelector(".dep-graph");
if (await page.$(".lanes")) {
  throw new Error("graph view should replace the four columns");
}
const graphTitles = await page.$$eval(".graph-node .issue-title", (nodes) =>
  nodes.map((node) => node.textContent),
);
for (const title of ["blocker", "child blocked", "waiting on history", "active work"]) {
  if (!graphTitles.includes(title)) {
    throw new Error(`one-hop graph should include ${title}, got ${JSON.stringify(graphTitles)}`);
  }
}
for (const unrelated of ["parent", "unparented ready", "older closed"]) {
  if (graphTitles.includes(unrelated)) {
    throw new Error(`centered graph should exclude unrelated Issue ${unrelated}`);
  }
}
const graphCenterText = await page.$eval(".graph-center-label", (node) => node.textContent?.trim());
if (graphCenterText !== "中心 Issue：#3 child blocked") {
  throw new Error(`graph should name its stable center Issue, got ${graphCenterText}`);
}
const completeGraphAction = page.getByRole("button", { name: "查看完整上下游（61 个 Issue）" });
if ((await completeGraphAction.count()) !== 1) {
  throw new Error("graph should offer the complete connected upstream/downstream closure with a count");
}
const edge = await page.$('path[data-from="you/garden#9"][data-to="you/garden#3"]');
if (!edge) {
  throw new Error("graph should draw the blocker edge from left to right");
}
if (!(await page.$('path[data-from="you/garden#3"][data-to="you/garden#5"]'))) {
  throw new Error("graph should draw downstream dependencies from the center Issue");
}
if (await page.$('path[data-from="you/garden#1"][data-to="you/garden#2"]')) {
  throw new Error("graph should not draw parent/child as an edge");
}

const stableGraphCanvas = await page.$(".graph-canvas");
await page.click(".graph-node:has-text('blocker') .graph-node-main");
await page.waitForSelector(".detail-hd:has-text('blocker')");
if (await page.$("button[data-act='clear-filter']")) {
  throw new Error("clicking a graph node should not filter the board");
}
if ((await page.$eval(".graph-center-label", (node) => node.textContent?.trim())) !== "中心 Issue：#3 child blocked") {
  throw new Error("clicking a graph node should only change details, not the graph center");
}
if (!stableGraphCanvas || !(await stableGraphCanvas.evaluate((node) => node.isConnected))) {
  throw new Error("changing graph node details should preserve the graph canvas");
}

const expandFromWaiting = page.getByRole("button", { name: "从此处展开 #5" });
if ((await expandFromWaiting.count()) !== 1 || !(await expandFromWaiting.textContent())?.includes("从此处展开")) {
  throw new Error("graph nodes should name the re-centering action instead of relying on an unexplained target icon");
}
await expandFromWaiting.click();
await page.waitForFunction(() => document.querySelector(".graph-center-label")?.textContent?.includes("#5 waiting on history"));
await page.waitForSelector(".detail-hd:has-text('waiting on history')");

await page.getByRole("button", { name: "从此处展开 #3" }).click();
await page.waitForFunction(() => document.querySelector(".graph-center-label")?.textContent?.includes("#3 child blocked"));
await page.waitForSelector(".detail-hd:has-text('child blocked')");
await page.getByRole("button", { name: "查看完整上下游（61 个 Issue）" }).click();
await page.waitForSelector(".graph-index");
await page.waitForFunction(() => document.querySelectorAll(".graph-node").length === 61);
if (await page.$("button[data-act='graph-more']")) {
  throw new Error("large dependency graphs should fill remaining node batches automatically");
}
if ((await page.$$(".graph-index-row")).length !== 50 || !(await page.$("button[data-act='graph-list-more']"))) {
  throw new Error("complete relationship list should paginate instead of mounting every Issue row");
}
await page.fill('[data-field="graphSearch"]', "just closed");
await page.waitForFunction(() => document.querySelectorAll(".graph-index-row").length === 1);
const searchedRelationship = await page.$eval(".graph-index-row", (node) => node.textContent?.replace(/\s+/g, " ").trim());
if (!searchedRelationship?.includes("just closed")) {
  throw new Error(`complete relationship search should find closed downstream Issues, got ${searchedRelationship}`);
}
await page.fill('[data-field="graphSearch"]', "waiting on history");
await page.waitForFunction(() => document.querySelectorAll(".graph-index-row").length === 1);
await page.getByRole("button", { name: "从此处展开 #5" }).first().click();
await page.waitForFunction(() => document.querySelector(".graph-center-label")?.textContent?.includes("#5 waiting on history"));
await page.waitForSelector(".detail-hd:has-text('waiting on history')");
if (!(await page.$(".graph-index")) || !(await page.getByRole("button", { name: "收起到一跳上下游" }).count())) {
  throw new Error("re-centering a complete upstream/downstream view should preserve its range");
}
if ((await page.inputValue('[data-field="graphSearch"]')) !== "waiting on history") {
  throw new Error("re-centering a complete upstream/downstream view should preserve its search context");
}
await page.fill('[data-field="graphSearch"]', "");
await page.waitForFunction(() => document.querySelectorAll(".graph-index-row").length === 50);
await page.getByRole("button", { name: "从此处展开 #3" }).first().click();
await page.waitForFunction(() => document.querySelector(".graph-center-label")?.textContent?.includes("#3 child blocked"));
await page.waitForSelector(".graph-index");

await page.click(".graph-node:has-text('active work') .graph-node-main");
await page.waitForSelector(".detail-hd:has-text('active work')");
await page.waitForSelector('.issue-markdown:has-text("Active Run Question")');
if (await page.$(".lifted-run")) {
  throw new Error("dependency graph nodes should only change Issue details");
}
await assertVisual("issue-99-graph-1440x900.png");
await assertShellRegionsDoNotOverlap(page);

const graphCanvas = await page.$(".graph-canvas");
const graphEdgePath = await page.$(".graph-edges path");
const graphScrollLeft = await page.$eval(".graph-canvas", (node) => {
  const flow = node.querySelector(".graph-flow");
  flow.style.width = "1600px";
  node.scrollLeft = Math.floor((node.scrollWidth - node.clientWidth) / 2);
  return node.scrollLeft;
});
if (!graphCanvas || graphScrollLeft <= 0) {
  throw new Error(`graph scroll regression needs horizontal overflow, got ${graphScrollLeft}`);
}
const tickResponse = page.waitForResponse((response) =>
  response.url().endsWith("/rpc") && response.request().postData()?.includes('"op":"tick"'),
);
await page.evaluate(() => window.__RUN_INTERVAL_CALLBACKS__());
await tickResponse;
await page.waitForTimeout(50);
const graphCanvasConnected = await graphCanvas.evaluate((node) => node.isConnected);
const graphEdgeConnected = graphEdgePath ? await graphEdgePath.evaluate((node) => node.isConnected) : false;
const graphScrollAfterTick = await page.$eval(".graph-canvas", (node) => node.scrollLeft);
if (!graphCanvasConnected || !graphEdgeConnected || graphScrollAfterTick !== graphScrollLeft) {
  throw new Error(
    `Host tick should preserve the dependency graph DOM and viewport, got canvas=${graphCanvasConnected} edge=${graphEdgeConnected} scroll=${graphScrollLeft}->${graphScrollAfterTick}`,
  );
}

await page.click("button[data-act='center-view'][data-id='board']");
await page.waitForSelector(".lanes");

await page.click("button[data-act='open-overview']");
await page.waitForSelector(".overview-page");
if (await page.$(".lanes")) {
  throw new Error("Host overview should replace the Project board");
}
for (const group of ["running", "stopped"]) {
  if (!(await page.$(`[data-run-group="${group}"]`))) {
    throw new Error(`Host overview missing ${group} group`);
  }
}
if (await page.$('[data-run-group="ended"]')) {
  throw new Error("ended Runs should be hidden by default");
}
const overviewProjects = await page.$$eval(".run-thumbnail .run-project", (nodes) => nodes.map((node) => node.textContent));
if (!overviewProjects.includes("garden") || !overviewProjects.includes("tools")) {
  throw new Error(`Host overview should include Runs from all Projects, got ${JSON.stringify(overviewProjects)}`);
}
await page.selectOption('[data-overview-filter="project"]', { label: "garden" });
const filteredProjects = await page.$$eval(".run-thumbnail .run-project", (nodes) => nodes.map((node) => node.textContent));
if (filteredProjects.some((name) => name !== "garden")) {
  throw new Error(`Host overview Project filter leaked: ${JSON.stringify(filteredProjects)}`);
}
await assertVisual("issue-99-overview-1440x900.png");
await assertShellRegionsDoNotOverlap(page);
await page.click("button[data-act='return-board']");
await page.waitForSelector(".lanes");

const emptyRunsOverviewResponse = async (route) => {
  if (!route.request().url().endsWith("/rpc")) {
    await route.continue();
    return;
  }
  let request;
  try {
    request = route.request().postDataJSON();
  } catch {
    await route.continue();
    return;
  }
  if (request?.op !== "snapshot" || request.clientView?.workspaceView !== "host-overview") {
    await route.continue();
    return;
  }
  const response = await route.fetch();
  const result = await response.json();
  result.snapshot.runs = [];
  result.snapshot.projects = result.snapshot.projects
    .filter((project) => project.name === "tools")
    .map((project) => ({
      ...project,
      hasActiveRun: false,
      hasExecutionStopped: false,
    }));
  await route.fulfill({ response, json: result });
};
await page.route("**/*", emptyRunsOverviewResponse);
await page.click("button[data-act='open-overview']");
await page.waitForSelector(".overview-page");
const toolsOverview = await page.$$eval(".overview-project:has-text('tools') .overview-project-metrics > span", (nodes) =>
  Object.fromEntries(nodes.map((node) => [node.querySelector("i")?.textContent, node.querySelector("b")?.textContent])),
);
if (toolsOverview.Open !== "1" || toolsOverview.Frontier !== "1") {
  throw new Error(`Host overview should discard a stale cross-Host Project filter and keep Issue data without Runs, got ${JSON.stringify(toolsOverview)}`);
}
if (!(await page.$(".overview-runs-empty"))) {
  throw new Error("a Host with no Runs should keep a compact Run empty state below Project data");
}
await page.unroute("**/*", emptyRunsOverviewResponse);
await page.click("button[data-act='return-board']");
await page.waitForSelector(".lanes");

await page.click('[data-lane="inProgress"] .issue-card:has-text("active work") .issue-card-main');
await page.waitForSelector(".lifted-run");
if (await page.$(".lanes")) {
  throw new Error("lifting a Run should replace the board");
}
if (await page.$(".side")) {
  throw new Error("lifting a Run should remove the sidebar from layout");
}
await page.waitForSelector(".lifted-run .issue-detail .detail-hd:has-text('active work')");
await page.waitForSelector('.lifted-run [data-document-state="ready"]');
const liftedDocument = await page.$eval(".lifted-run .issue-markdown", (node) => node.textContent?.replace(/\s+/g, " ").trim());
if (!liftedDocument?.includes("Active Run Question") || !liftedDocument.includes("same complete Issue")) {
  throw new Error(`entering a Run should retain the complete Issue document, got ${liftedDocument}`);
}
await capture("issue-98-existing-run-1440x900.png");
await assertVisual("issue-99-run-1440x900.png");
await assertShellRegionsDoNotOverlap(page);
const liftedWidths = await page.evaluate(() => {
  const terminal = document.querySelector(".lifted-terminal")?.getBoundingClientRect().width ?? 0;
  const detail = document.querySelector(".lifted-run .issue-detail")?.getBoundingClientRect().width ?? 0;
  const lifted = document.querySelector(".lifted-run");
  const style = lifted ? getComputedStyle(lifted) : null;
  return {
    terminal,
    detail,
    gap: style?.columnGap ?? "",
    padding: style?.padding ?? "",
    horizontalOverflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
  };
});
if (liftedWidths.terminal < liftedWidths.detail * 1.8 || liftedWidths.terminal > liftedWidths.detail * 2.2) {
  throw new Error(`lifted Run should use about a 2:1 split, got ${JSON.stringify(liftedWidths)}`);
}
if (liftedWidths.gap !== "0px" || liftedWidths.padding !== "0px") {
  throw new Error(`lifted Run and Issue should share one continuous workspace seam, got ${JSON.stringify(liftedWidths)}`);
}
if (liftedWidths.horizontalOverflow > 0) {
  throw new Error(`lifted Run should not create page-level horizontal scrolling: ${liftedWidths.horizontalOverflow}px`);
}
const telemetryCapsule = page.locator(".lifted-terminal .telemetry-desktop .capsule").first();
if (!(await telemetryCapsule.count())) {
  throw new Error("Run telemetry should expose a model/lane capsule beside the Terminal");
}
await telemetryCapsule.click();
await page.waitForSelector(".lifted-terminal .telemetry-cards .telemetry-card");
const telemetryText = (await page.locator(".lifted-terminal .telemetry-cards").textContent())?.replace(/\s+/g, " ").trim() ?? "";
if (!telemetryText.includes("grok-4.6") || (!telemetryText.includes("不管理") && !telemetryText.includes("does not manage"))) {
  throw new Error(`expanded telemetry should keep model identity and the network boundary: ${telemetryText}`);
}
await telemetryCapsule.click();
await page.click("button[data-act='return-board']");
await page.waitForSelector(".lanes");
await page.waitForSelector(".side");
if (!(await page.$(".run-dock"))) {
  throw new Error("returning to the board should restore the active Issue terminal dock");
}
await page.click(".issue-card:has-text('child ready') .issue-card-main");
await page.waitForSelector(".detail-hd:has-text('child ready')");
if (await page.$(".run-dock")) {
  throw new Error("selecting an Issue without an active Run should remove the terminal dock");
}

const issueToggleLeftBeforeSidebarFold = await page.$eval("button[data-act='toggle-issue']", (node) =>
  node.getBoundingClientRect().left,
);
await page.click("button[data-act='toggle-sidebar']");
if (await page.$(".side")) {
  throw new Error("the sidebar toggle should remove the sidebar from layout");
}
const issueToggleLeftAfterSidebarFold = await page.$eval("button[data-act='toggle-issue']", (node) =>
  node.getBoundingClientRect().left,
);
if (Math.abs(issueToggleLeftAfterSidebarFold - issueToggleLeftBeforeSidebarFold) > 1) {
  throw new Error(`Issue detail toggle should keep its chrome coordinate when the sidebar folds: ${issueToggleLeftBeforeSidebarFold} -> ${issueToggleLeftAfterSidebarFold}`);
}
await page.click('[data-lane="inProgress"] .issue-card:has-text("active work") .issue-card-main');
await page.waitForSelector(".lifted-run");
await page.click("button[data-act='return-board']");
await page.waitForSelector(".lanes");
if (await page.$(".side")) {
  throw new Error("returning should preserve a sidebar that was already collapsed");
}
await page.click("button[data-act='toggle-sidebar']");
await page.waitForSelector(".side");

await page.click("button[data-act='open-usage']");
await page.waitForSelector(".usage-page");
if (await page.$(".lanes")) {
  throw new Error("usage page should replace the board, not sit as a tab or overlay");
}
const usageTitle = await page.$eval(".usage-page h1", (node) => node.textContent ?? "");
if (usageTitle !== "用量" && usageTitle !== "Usage") {
  throw new Error(`usage page title, got ${usageTitle}`);
}
if (!(await page.$("button.active[data-act='usage-range'][data-id='today']"))) {
  throw new Error("usage range should default to today");
}
if ((await page.locator(".usage-trend-block").count()) !== 2) {
  throw new Error("usage should render separate TTFT and generation-rate trends");
}
const usageDisclaimer = (await page.locator(".usage-page > .tiny").textContent())?.replace(/\s+/g, " ").trim() ?? "";
if (!usageDisclaimer.includes("不管理") && !usageDisclaimer.includes("does not manage")) {
  throw new Error(`usage should state the proxy/network boundary: ${usageDisclaimer}`);
}
const trendColors = await page.evaluate(() => {
  const bars = [...document.querySelectorAll(".usage-trend i")];
  if (bars.length < 2) return [];
  const normal = getComputedStyle(bars[0]).backgroundColor;
  bars[1].classList.add("slow");
  const slow = getComputedStyle(bars[1]).backgroundColor;
  return [normal, slow];
});
if (trendColors.length !== 2 || trendColors[0] === trendColors[1]) {
  throw new Error(`slow usage samples should be visibly distinguished: ${JSON.stringify(trendColors)}`);
}
await page.click("button[data-act='close-usage']");
await page.waitForSelector(".lanes");

const afterGraphFrontier = await page.$$eval('[data-lane="frontier"] .issue-card .issue-title', (nodes) =>
  nodes.map((node) => node.textContent),
);
if (!afterGraphFrontier.includes("unparented ready")) {
  throw new Error("returning to the board should keep the unfiltered Frontier");
}

const inProgress = page.locator('[data-lane="inProgress"] .issue-card:has-text("active work")');
for (const action of ["focus-run", "stop-run", "view-changes"]) {
  if (!(await inProgress.locator(`button[data-act="${action}"]`).count())) {
    throw new Error(`in-progress row should expose ${action}`);
  }
}

const recentOpen = page.locator('[data-lane="recentlyCompleted"] button[data-act="open-issue"]').first();
await recentOpen.click();
const openedRecentUrl = await page.evaluate(() => window.__OPENED_URLS__.at(-1));
if (!openedRecentUrl?.startsWith("https://github.com/you/garden/issues/")) {
  throw new Error(`recently completed should open its GitHub Issue, got ${openedRecentUrl}`);
}
if (!(await page.locator('[data-lane="recentlyCompleted"] button[data-act="view-changes"]').count())) {
  throw new Error("recently completed row with a Run should expose view changes");
}

const frontierRun = page.locator('[data-lane="frontier"] button[data-act="execute-run"]').first();
if (!(await frontierRun.count())) {
  throw new Error("Frontier row should expose Run");
}

const newLabel = await page.$eval("button[data-act='new-run']", (node) => node.getAttribute("aria-label"));
if (newLabel !== "新建" && newLabel !== "New") {
  throw new Error(`project row plus should be New, got ${newLabel}`);
}
await page.click("button[data-act='register']");
await page.waitForSelector("form[data-form='project']");
if (!(await page.$("button[data-act='choose-project-directory']"))) {
  throw new Error("local Host registration should keep a folder-picker action next to the path field");
}
if (await page.$("button[data-act='infer']")) {
  throw new Error("registration should infer automatically instead of asking for a manual infer click");
}
await page.click(".overlay.modal[data-act='close-form']", { position: { x: 2, y: 2 } });
if (!(await page.$("form[data-form='project']"))) {
  throw new Error("clicking outside the registration sheet should keep the form open");
}
await page.click("button[data-act='choose-project-directory']");
await page.waitForSelector("form[data-form='project'] .notice.bad");
const pickerNotice = await page.$eval("form[data-form='project'] .notice.bad", (node) => node.textContent?.trim());
if (!pickerNotice?.includes("系统目录选择只在本机桌面窗口可用")) {
  throw new Error(`browser Client should explain the desktop-only folder picker, got ${pickerNotice}`);
}
await page.click("form[data-form='project'] button[data-act='close-form']");
await page.waitForFunction(() => !document.querySelector("form[data-form='project']"));
const claimsBeforeUnbound = Object.values((await hostSnapshot(page, url)).board.columns)
  .flat()
  .map((issue) => [issue.id, issue.claimedBy])
  .sort(([left], [right]) => left.localeCompare(right));
await page.click("button[data-act='new-run']");
await page.waitForSelector(".launch-sheet");
const claimsWithUnboundFormOpen = Object.values((await hostSnapshot(page, url)).board.columns)
  .flat()
  .map((issue) => [issue.id, issue.claimedBy])
  .sort(([left], [right]) => left.localeCompare(right));
if (JSON.stringify(claimsWithUnboundFormOpen) !== JSON.stringify(claimsBeforeUnbound)) {
  throw new Error("opening an unbound Run form must not claim any Issue");
}
const pick = page.locator("button[data-act='pick-agent']:not([disabled])").first();
if (await pick.count()) {
  await pick.click();
  await page.waitForSelector("textarea[data-field='openingText']");
}
await page.click(".launch-sheet button[data-act='intent'][data-id='modify']");
const openingText = page.locator("textarea[data-field='openingText']");
await openingText.fill("");
await openingText.pressSequentially("e2e unbound run");
const customIntent = await page.$eval(".launch-sheet button[data-act='intent-custom']", (node) => ({
  text: node.textContent?.trim(),
  active: node.classList.contains("active"),
  hidden: node.hidden,
}));
if (customIntent.hidden || !customIntent.active || (customIntent.text !== "自定义" && customIntent.text !== "Custom")) {
  throw new Error(`editing an intent prefix should show Custom, got ${JSON.stringify(customIntent)}`);
}
if ((await openingText.inputValue()) !== "e2e unbound run" || !(await openingText.evaluate((node) => node === document.activeElement))) {
  throw new Error("editing an intent prefix should preserve the textarea and its focus");
}
await page.click(".launch-sheet button[type='submit']");
await page.waitForSelector(".run-dock");
await page.waitForFunction(() => !document.querySelector(".launch-sheet"));
const afterUnbound = await hostSnapshot(page, url);
const claimsAfterUnbound = Object.values(afterUnbound.board.columns)
  .flat()
  .map((issue) => [issue.id, issue.claimedBy])
  .sort(([left], [right]) => left.localeCompare(right));
if (JSON.stringify(claimsAfterUnbound) !== JSON.stringify(claimsBeforeUnbound)) {
  throw new Error("starting an unbound Run must not claim any Issue");
}
if (!afterUnbound.runs.some((run) => run.unbound && run.status === "running")) {
  throw new Error("starting an unbound Run must create a running Run without an Issue binding");
}
const dockText = await page.$eval(".run-dock", (node) => node.textContent.replace(/\s+/g, " ").trim());
if (!dockText.includes("Grok Build") || (!dockText.includes("未绑定 Issue") && !dockText.includes("Unbound Issue"))) {
  throw new Error(`unbound Run dock missing identity, got ${dockText}`);
}
if (!(await page.$(".pty-slot"))) {
  throw new Error("Embedded Terminal slot missing");
}
await page.click(".xterm-helper-textarea");
await page.keyboard.press("?");
if (await page.$(".keyboard-help")) {
  throw new Error("terminal focus should keep ? in the official TUI");
}
await page.click(".run-dock button[data-act='stop-run']");
await page.waitForFunction(() => !document.querySelector(".run-dock"));

await page.click("button:has-text('设置')");
await page.waitForSelector("#recent-limit");
const browserUpdateText = await page.$eval(".update-settings", (node) => node.textContent?.replace(/\s+/g, " ").trim());
if (!browserUpdateText?.includes("浏览器 Client 不能给 Host 换包")) {
  throw new Error(`browser Client should not expose update installation: ${browserUpdateText}`);
}
if (await page.$("button[data-act='check-updates']") || await page.$("button[data-act='install-update']")) {
  throw new Error("browser Client must not expose updater actions");
}
const initialPreviewSetting = await page.$eval("input[data-field='commandPreview']", (node) => node.checked);
if (!initialPreviewSetting) {
  throw new Error("command preview should be enabled by default");
}
await page.locator("input[data-field='commandPreview']").uncheck();
await page.waitForFunction(async (protocol) => {
  const response = await fetch(`${protocol}/rpc`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ op: "snapshot" }),
  });
  return (await response.json()).snapshot.showCommandPreview === false;
}, url);
const browserStartupText = await page.$eval(".startup-settings", (node) => node.textContent?.replace(/\s+/g, " ").trim());
if (!browserStartupText?.includes("只能在桌面应用中修改")) {
  throw new Error(`browser Client should explain the desktop startup boundary: ${browserStartupText}`);
}
if (await page.$("button[data-act='host-mode']") || await page.$("input[data-field='startAtLogin']")) {
  throw new Error("browser Client must not expose Host mode or system autostart controls");
}
await page.click("button[data-act='refresh-launch-environment']");
await page.waitForSelector('[data-launch-environment-status="ready"]');
const launchEnvironmentText = await page.$eval("[data-launch-environment-status]", (node) => node.textContent?.replace(/\s+/g, " ").trim());
if (!launchEnvironmentText?.includes("启动环境已更新")) {
  throw new Error(`launch environment refresh should report success: ${launchEnvironmentText}`);
}
await page.fill("#recent-limit", "1");
await page.locator("#recent-limit").dispatchEvent("change");
await page.waitForFunction(() => document.querySelectorAll('[data-lane="recentlyCompleted"] .issue-card').length === 1);
await page.click(".overlay[data-act='close-settings']", { position: { x: 2, y: 2 } });
await page.waitForFunction(() => !document.querySelector(".overlay[data-act='close-settings']"));

await page.setViewportSize({ width: 390, height: 844 });
await page.waitForSelector(".mobile-nav");
const mobileNavLabels = await page.$$eval(".mobile-nav button", (nodes) => nodes.map((node) => node.textContent?.trim()));
if (mobileNavLabels.join("|") !== "看板|票|Run") {
  throw new Error(`mobile bottom navigation should be 看板 | 票 | Run, got ${JSON.stringify(mobileNavLabels)}`);
}
if (await page.$(".side")) {
  throw new Error("mobile should move Host and Project lists out of the main layout");
}
const mobileProjectOrder = await page.$$eval(".project-board > *", (nodes) =>
  nodes.map((node) => node.className).filter(Boolean),
);
const refreshIndex = mobileProjectOrder.findIndex((name) => name.includes("refresh-bar"));
const lanesIndex = mobileProjectOrder.findIndex((name) => name.includes("board-shell"));
if (refreshIndex < 0 || lanesIndex < 0 || refreshIndex > lanesIndex) {
  throw new Error(`mobile refresh status should precede work lanes, got ${JSON.stringify(mobileProjectOrder)}`);
}
const visibleMobileLanes = await page.$$eval(".lane", (nodes) =>
  nodes.filter((node) => getComputedStyle(node).display !== "none").map((node) => node.getAttribute("data-lane")),
);
if (visibleMobileLanes.join("|") !== "inProgress|frontier") {
  throw new Error(`mobile board should prioritize in progress then Frontier, got ${JSON.stringify(visibleMobileLanes)}`);
}
const mobileChangesButtons = await page.$$eval('button[data-act="view-changes"]', (nodes) =>
  nodes.filter((node) => getComputedStyle(node).display !== "none").length,
);
if (mobileChangesButtons !== 0) {
  throw new Error(`mobile should not expose full view changes, got ${mobileChangesButtons} buttons`);
}
if (await page.$('button[data-act="view-changes"]')) {
  throw new Error("mobile should not render full view changes actions");
}

const mobileFrontierCard = page.locator('[data-lane="frontier"] .issue-card').first();
const mobileFrontierIssueId = await mobileFrontierCard.getAttribute("data-issue-id");
await mobileFrontierCard.locator('button[data-act="execute-run"]').click();
await page.waitForSelector(".launch-sheet");
const mobileAgentPick = page.locator("button[data-act='pick-agent']:not([disabled])").first();
if (await mobileAgentPick.count()) {
  await mobileAgentPick.click();
  await page.waitForSelector("form[data-form='launch']");
}
await page.click("form[data-form='launch'] button[type='submit']");
await page.waitForFunction(() => !document.querySelector(".launch-sheet"));
await page.waitForTimeout(100);
const startedIssueRun = await page.evaluate(async ({ protocol, issueId }) => {
  const response = await fetch(`${protocol}/rpc`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ op: "snapshot" }),
  });
  const snapshot = (await response.json()).snapshot;
  return snapshot.runs.find((run) => run.issueId === issueId) ?? { debug: snapshot.runs };
}, { protocol: url, issueId: mobileFrontierIssueId });
if (!startedIssueRun?.id || startedIssueRun.status !== "running") {
  throw new Error(`mobile should start a Frontier Run through the normal launch form: ${JSON.stringify(startedIssueRun)}`);
}
await page.evaluate((runId) => fetch(`${window.__HOST_PROTOCOL__}/rpc`, {
  method: "POST",
  headers: { "content-type": "application/json" },
  body: JSON.stringify({ op: "focusRun", runId }),
}), startedIssueRun.id);
await page.click("button[data-act='mobile-run']");
await page.waitForSelector(".mobile-run-view");
await page.click(".mobile-run-view button[data-act='stop-run']");
await page.waitForSelector(".mobile-board-view");

await page.click("button[data-act='mobile-scope']");
await page.waitForSelector(".mobile-scope-sheet");
if (!(await page.$(".mobile-scope-hosts button[data-act='focus-host']"))) {
  throw new Error("mobile scope switcher should expose the Host list");
}
for (const action of ["register", "edit-project", "remove-project"]) {
  if (!(await page.$(`.mobile-scope-sheet [data-act="${action}"]`))) {
    throw new Error(`mobile scope switcher should expose ${action}`);
  }
}
await page.click(".mobile-scope-sheet button[data-act='edit-project']");
await page.waitForSelector("form[data-form='project']");
await page.click("form[data-form='project'] button[data-act='close-form']");
await page.click("button[data-act='mobile-scope']");
await page.click(".mobile-scope-sheet button[data-act='remove-project']");
await page.waitForSelector(".overlay[data-act='close-remove']");
await page.click("button[data-act='close-remove']");

await page.click(".issue-card:has-text('child ready') .issue-card-main");
await page.waitForSelector(".mobile-issue-view .issue-detail");
await page.waitForSelector('.mobile-issue-view [data-document-state="ready"]');
const mobileDocument = await page.$eval(".mobile-issue-view .issue-markdown", (node) => node.textContent?.replace(/\s+/g, " ").trim());
if (!mobileDocument?.includes("Can the operator read every constraint") || !mobileDocument.includes("Paragraph six")) {
  throw new Error(`390px Issue view should expose the complete document, got ${mobileDocument}`);
}
if (await page.$('.mobile-issue-view button[data-act="view-changes"]')) {
  throw new Error("mobile Issue view should still omit full view changes");
}
if (await page.$('.mobile-issue-view button[data-act="toggle-issue"]')) {
  throw new Error("mobile Issue view should not expose a desktop panel-collapse control");
}
await capture("issue-98-mobile-390x844.png");
await assertVisual("issue-99-mobile-390x844.png");
await assertShellRegionsDoNotOverlap(page);
await page.click("button[data-act='mobile-board']");
await page.waitForSelector(".mobile-board-view");

await page.click('[data-lane="inProgress"] .issue-card:has-text("active work") button[data-act="focus-run"]');
await page.waitForSelector(".mobile-run-view");
if (await page.$(".mobile-run-view .pty-slot")) {
  throw new Error("mobile Run should show recent output before the live terminal escape hatch");
}
await page.waitForFunction(() => document.querySelector(".mobile-run-output")?.textContent?.includes("mobile recent output"));
const recentOutput = await page.$eval(".mobile-run-output", (node) => node.textContent ?? "");
if (!recentOutput.includes("mobile recent output")) {
  throw new Error(`mobile Run should expose recent read-only output, got ${recentOutput}`);
}
await page.fill(".mobile-run-view .inject-row input", "mobile answer");
await page.click(".mobile-run-view .inject-row button[type='submit']");
await page.waitForFunction(() => document.querySelector(".mobile-run-view .inject-row input")?.value === "");
const injectedRunId = await page.$eval(".mobile-run-output", (node) => node.getAttribute("data-run"));
await page.waitForFunction(async ({ protocol, runId }) => {
  const response = await fetch(`${protocol}/runs/${encodeURIComponent(runId)}/output?after=0`);
  if (!response.ok) return false;
  const json = await response.json();
  const output = new TextDecoder().decode(Uint8Array.from(atob(json.data), (byte) => byte.charCodeAt(0)));
  return output.includes("mobile answer");
}, { protocol: url, runId: injectedRunId });
if (!(await page.$(".telemetry-mobile .capsule")) || !(await page.$(".telemetry-mobile .telemetry-simple"))) {
  throw new Error("mobile telemetry should keep the main model capsule and simple multi-model list");
}
await page.click("button[data-act='mobile-live-terminal']");
await page.waitForSelector(".mobile-run-view .pty-slot");
const endedRunId = await page.$eval(".mobile-run-view .pty-slot", (node) => node.getAttribute("data-run"));
await page.click(".mobile-run-view button[data-act='stop-run']");
await page.waitForSelector(".mobile-board-view");
await page.evaluate((runId) => fetch(`${window.__HOST_PROTOCOL__}/rpc`, {
  method: "POST",
  headers: { "content-type": "application/json" },
  body: JSON.stringify({ op: "focusRun", runId }),
}), endedRunId);
await page.click("button[data-act='mobile-run']");
await page.waitForSelector(".mobile-run-view");
const endedRecentOutput = await page.$eval(".mobile-run-output", (node) => node.textContent ?? "");
if (!endedRecentOutput.includes("mobile recent output") || !endedRecentOutput.includes("mobile answer")) {
  throw new Error(`mobile should retain recent output after a Run ends, got ${endedRecentOutput}`);
}
await page.click("button[data-act='mobile-board']");
await page.waitForSelector(".mobile-board-view");

await page.click("button[data-act='mobile-scope']");
if (!(await page.$(".mobile-scope-sheet button[data-act='open-usage']"))) {
  throw new Error("mobile usage should stay reachable from the scope switcher");
}
await page.click(".mobile-scope-sheet button[data-act='open-usage']");
await page.waitForSelector(".usage-page");
for (const selector of [".usage-ranges", ".usage-filters", ".usage-trend-block", ".usage-full"]) {
  const visible = await page.$eval(selector, (node) => getComputedStyle(node).display !== "none");
  if (visible) throw new Error(`mobile usage should hide ${selector}`);
}
if (!(await page.$(".token-row.totals"))) {
  throw new Error("mobile usage should retain current totals");
}
const compactProjects = await page.$$eval(".usage-compact .usage-row", (nodes) => nodes.length);
if (compactProjects < 1 || compactProjects > 3) {
  throw new Error(`mobile usage should show one to three Project rows, got ${compactProjects}`);
}
await page.click("button[data-act='close-usage']");

await page.click("button[data-act='settings']");
for (const forbidden of ["notifyDesktop", "notifySound", "hostAutoAdvance"]) {
  if (await page.$(`[data-field="${forbidden}"]`)) {
    throw new Error(`mobile settings should not expose ${forbidden}`);
  }
}
if (await page.$("button[data-act='quit']")) {
  throw new Error("mobile settings should not expose Host quit");
}
await page.click("button[data-act='language'][data-id='en']");
await page.click("button[data-act='theme'][data-id='plain-night']");
const storedMobileAppearance = await page.evaluate(() => localStorage.getItem("agent-taskboard-mobile-appearance"));
if (!storedMobileAppearance?.includes('"language":"en"') || !storedMobileAppearance.includes('"theme":"plain-night"')) {
  throw new Error(`mobile appearance should persist in this browser Client, got ${storedMobileAppearance}`);
}
const hostAppearance = await page.evaluate(async (protocol) => {
  const response = await fetch(`${protocol}/rpc`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ op: "snapshot" }),
  });
  return (await response.json()).snapshot.appearance;
}, url);
if (hostAppearance.language !== "zh-CN" || hostAppearance.theme === "plain-night") {
  throw new Error(`mobile appearance must not overwrite the Host Client, got ${JSON.stringify(hostAppearance)}`);
}
const mobileNotificationPermission = await page.evaluate(() =>
  typeof Notification === "undefined" ? "unavailable" : Notification.permission,
);
if (mobileNotificationPermission === "granted") {
  throw new Error("mobile browser must not request lock-screen notification permission");
}

await browser.close();
console.log("board e2e ok");
