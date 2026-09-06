import { assertShellRegionsDoNotOverlap } from "./board-harness.mjs";

export async function runDesktopBoardShell(session) {
try {
  await session.page.waitForSelector(".lanes");
} catch (error) {
  const html = await session.page.content();
  console.error("page html", html.slice(0, 4000));
  throw error;
}
await session.page.click("button[data-act='toggle-hosts']");
const visibleHosts = await session.page.$$eval(".host-picker button[data-act='focus-host']", (nodes) =>
  nodes.map((node) => node.textContent?.replace(/\s+/g, " ").trim()),
);
if (visibleHosts.length < 2) {
  throw new Error(`daily shell fixture should expose multiple Hosts, got ${JSON.stringify(visibleHosts)}`);
}
await session.page.click("button[data-act='toggle-hosts']");
if (await session.page.$(".board-shell > .issue-detail")) {
  throw new Error("issue inspector should not occupy the board before an Issue is selected");
}
if (await session.page.$("button[data-act='toggle-issue']")) {
  throw new Error("issue inspector toggle should not appear before an Issue is selected");
}
await session.page.waitForSelector(".refresh-bar");
if (await session.page.$('.refresh-bar[data-kind="incomplete"]')) {
  const incompleteText = await session.page.$eval(".refresh-bar", (node) => node.textContent.replace(/\s+/g, " ").trim());
  if (!incompleteText.includes("数据不完整") || !incompleteText.includes("pagination stopped early")) {
    throw new Error(`incomplete refresh detail missing: ${incompleteText}`);
  }
  if (await session.page.$(".lanes") || await session.page.$(".dep-graph")) {
    throw new Error("incomplete tracker data must hide Frontier lanes and the dependency graph");
  }
  await session.page.waitForSelector('[data-empty="incomplete-read"]');
  await session.page.click('.refresh-bar button[data-act="refresh"]');
  await session.page.waitForSelector(".lanes");
}

await session.page.keyboard.press("?");
await session.page.waitForSelector(".keyboard-help");
await session.page.keyboard.press("?");
await session.page.waitForFunction(() => !document.querySelector(".keyboard-help"));
await session.page.keyboard.press("j");
await session.page.waitForFunction(() => document.activeElement?.classList.contains("issue-card-main"));
const keyboardFocusedCard = await session.page.evaluate(() => document.activeElement?.classList.contains("issue-card-main"));
if (!keyboardFocusedCard) {
  throw new Error("j should focus a board card");
}
await session.page.keyboard.press("Enter");
await session.page.waitForSelector(".issue-detail .detail-hd");
await session.page.waitForSelector("button[data-act='toggle-issue']");
await session.page.waitForSelector('.issue-document[data-document-state="ready"]');
const rightmostIssueId = await session.page.$$eval(".issue-card", (nodes) => nodes
  .map((node) => ({ id: node.getAttribute("data-issue-id"), left: node.getBoundingClientRect().left }))
  .sort((left, right) => right.left - left.left)[0]?.id);
if (rightmostIssueId) {
  const leftmostIssueId = await session.page.$$eval(".issue-card", (nodes) => nodes
    .map((node) => ({ id: node.getAttribute("data-issue-id"), left: node.getBoundingClientRect().left }))
    .sort((left, right) => left.left - right.left)[0]?.id);
  if (leftmostIssueId && leftmostIssueId !== rightmostIssueId) {
    await session.clickCard(session.page.locator(`.issue-card[data-issue-id="${leftmostIssueId}"] .issue-card-main`));
    await session.page.waitForSelector('.issue-document[data-document-state="ready"]');
  }
  const rightmostBox = await session.page.locator(`.issue-card[data-issue-id="${rightmostIssueId}"] .issue-card-main`).boundingBox();
  if (!rightmostBox) throw new Error("rightmost Issue card has no clickable geometry");
  await session.page.mouse.click(rightmostBox.x + rightmostBox.width / 2, rightmostBox.y + rightmostBox.height / 2);
  await session.page.waitForFunction((id) => document.querySelector(`.issue-detail .detail-hd`)?.textContent?.includes(id.split("#").at(-1)), rightmostIssueId);
  await session.page.waitForSelector('.issue-document[data-document-state="ready"]');
  const overlap = await session.page.evaluate((id) => {
    const card = document.querySelector(`.issue-card[data-issue-id="${CSS.escape(id)}"]`);
    const detail = document.querySelector(".board-shell > .issue-detail");
    if (!card || !detail || getComputedStyle(detail).position !== "absolute") return false;
    const a = card.getBoundingClientRect();
    const b = detail.getBoundingClientRect();
    return a.left < b.right - 1 && a.right > b.left + 1 && a.top < b.bottom - 1 && a.bottom > b.top + 1;
  }, rightmostIssueId);
  if (overlap) throw new Error("floating Issue Inspector must move away from the Issue card that was clicked");
}
const scrollRegressionStyle = await session.page.addStyleTag({
  content: '[data-lane="frontier"] { max-height: 120px; } .issue-detail .detail-scroll { max-height: 180px; }',
});
const frontierScrollBeforeInspectorActions = await session.page.$eval('[data-lane="frontier"]', (node) => {
  node.scrollTop = node.scrollHeight;
  return node.scrollTop;
});
if (frontierScrollBeforeInspectorActions <= 0) {
  throw new Error("Issue Inspector regression needs a scrollable board lane");
}
const lanesWithInspector = await session.page.$eval(".lanes", (node) => node.getBoundingClientRect().width);
const detailScrollBeforeCollapse = await session.page.$eval(".detail-scroll", (node) => {
  node.scrollTop = node.scrollHeight;
  return node.scrollTop;
});
if (detailScrollBeforeCollapse <= 0) {
  throw new Error("Issue Inspector regression needs a scrollable Issue document");
}
await session.page.click("button[data-act='toggle-issue']");
await session.page.waitForFunction(() => !document.querySelector(".board-shell > .issue-detail"));
const lanesWithoutInspector = await session.page.$eval(".lanes", (node) => node.getBoundingClientRect().width);
if (Math.abs(lanesWithoutInspector - lanesWithInspector) > 1) {
  throw new Error(`floating Inspector must not change lane width: ${lanesWithInspector} -> ${lanesWithoutInspector}`);
}
const frontierScrollAfterCollapse = await session.page.$eval('[data-lane="frontier"]', (node) => node.scrollTop);
if (Math.abs(frontierScrollAfterCollapse - frontierScrollBeforeInspectorActions) > 1) {
  throw new Error(`collapsing the Inspector must preserve board scroll: ${frontierScrollBeforeInspectorActions} -> ${frontierScrollAfterCollapse}`);
}
if (!(await session.page.$("button[data-act='toggle-issue']"))) {
  throw new Error("hiding the inspector should keep the restore control in the chrome");
}
await session.page.click("button[data-act='toggle-issue']");
await session.page.waitForSelector(".issue-detail .detail-hd");
const detailScrollAfterRestore = await session.page.$eval(".detail-scroll", (node) => node.scrollTop);
if (Math.abs(detailScrollAfterRestore - detailScrollBeforeCollapse) > 1) {
  throw new Error(`restoring the same Issue must preserve Inspector scroll: ${detailScrollBeforeCollapse} -> ${detailScrollAfterRestore}`);
}
await session.page.$eval('.issue-card:has(.issue-title:text-is("unparented ready")) .issue-card-main', (node) => node.click());
await session.page.waitForSelector(".detail-hd:has-text('unparented ready')");
await session.page.waitForSelector('.issue-document[data-document-state="ready"]');
const frontierScrollAfterSwitch = await session.page.$eval('[data-lane="frontier"]', (node) => node.scrollTop);
if (Math.abs(frontierScrollAfterSwitch - frontierScrollBeforeInspectorActions) > 1) {
  throw new Error(`switching Issues must preserve board scroll: ${frontierScrollBeforeInspectorActions} -> ${frontierScrollAfterSwitch}`);
}
const inspectorHierarchy = await session.page.$eval(".issue-detail", (node) => ({
  text: node.textContent?.replace(/\s+/g, " ").trim() ?? "",
  commentVisible: Boolean(node.querySelector('form[data-act="issue-comment"]')?.getClientRects().length),
}));
if (!inspectorHierarchy.text.includes("父子关系") || !inspectorHierarchy.text.includes("依赖关系")) {
  throw new Error(`Inspector should use clear relationship headings: ${inspectorHierarchy.text}`);
}
for (const internalPhrase of ["属于 / 子票", "挡住它的 / 它挡住的", "无，可进 Frontier"]) {
  if (inspectorHierarchy.text.includes(internalPhrase)) {
    throw new Error(`Inspector should remove internal or explanatory copy: ${internalPhrase}`);
  }
}
if (inspectorHierarchy.commentVisible) {
  throw new Error("secondary Issue update forms should stay collapsed until requested");
}
await scrollRegressionStyle.evaluate((node) => node.remove());

const beforeTypingSearch = await session.page.$$eval(".issue-card .issue-title", (nodes) => nodes.map((node) => node.textContent));
await session.page.fill("#issue-title-search", "child ready");
const whileTypingSearch = await session.page.$$eval(".issue-card .issue-title", (nodes) => nodes.map((node) => node.textContent));
if (whileTypingSearch.join(",") !== beforeTypingSearch.join(",")) {
  throw new Error("title search should not run before Enter");
}
async function submitIssueSearch() {
  const responsePromise = session.page.waitForResponse((response) => {
    const request = response.request();
    if (request.method() !== "POST" || !request.url().includes("/rpc")) return false;
    try {
      return request.postDataJSON()?.op === "searchIssues";
    } catch {
      return false;
    }
  });
  await session.page.press("#issue-title-search", "Enter");
  const response = await responsePromise;
  return response.json();
}
const titleSearchResponse = await submitIssueSearch();
if (titleSearchResponse.snapshot?.board?.search?.title !== "child ready") {
  throw new Error(`search RPC should receive the entered title: ${JSON.stringify(titleSearchResponse.snapshot?.board?.search)}`);
}
await session.page.waitForFunction(() => document.querySelectorAll(".issue-card").length === 1);
const searchResult = await session.page.$eval(".issue-card .issue-title", (node) => node.textContent);
if (searchResult !== "child ready") {
  throw new Error(`unexpected title search result: ${searchResult}`);
}
await session.page.selectOption(".issue-search select[name='state']", "closed");
const closedSearchResponse = await submitIssueSearch();
if (closedSearchResponse.snapshot?.board?.search?.state !== "closed") {
  throw new Error(`search RPC should receive the selected state: ${JSON.stringify(closedSearchResponse.snapshot?.board?.search)}`);
}
await session.page.waitForFunction(() => document.querySelectorAll(".issue-card").length === 0);
await session.page.fill("#issue-title-search", "");
await session.page.selectOption(".issue-search select[name='state']", "all");
await submitIssueSearch();
await session.page.waitForFunction(() => document.querySelectorAll(".issue-card").length > 1);

const refreshText = await session.page.$eval(".refresh-bar", (node) => node.textContent.replace(/\s+/g, " ").trim());
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

const focusRefresh = session.page.waitForRequest(isRefreshRpc, { timeout: 3000 });
await session.page.evaluate(() => window.dispatchEvent(new Event("focus")));
await focusRefresh;
await session.page.waitForFunction(() => {
  const bar = document.querySelector(".refresh-bar");
  return Boolean(bar) && bar.getAttribute("data-kind") !== "refreshing";
});
await new Promise((resolve) => setTimeout(resolve, 300));

let coalescedRefreshCount = 0;
const countCoalescedRefresh = (req) => {
  if (isRefreshRpc(req)) coalescedRefreshCount += 1;
};
session.page.on("request", countCoalescedRefresh);
await session.page.evaluate(() => {
  document.dispatchEvent(new Event("visibilitychange"));
  window.dispatchEvent(new Event("focus"));
});
const coalesceDeadline = Date.now() + 1500;
while (coalescedRefreshCount < 1 && Date.now() < coalesceDeadline) {
  await new Promise((resolve) => setTimeout(resolve, 50));
}
await new Promise((resolve) => setTimeout(resolve, 250));
session.page.off("request", countCoalescedRefresh);
if (coalescedRefreshCount !== 1) {
  throw new Error(`same foreground event should refresh once, got ${coalescedRefreshCount}`);
}

await session.page.click(".refresh-bar button[data-act='refresh']");
await session.page.waitForSelector(".lanes");

const headers = await session.page.$$eval(".lane-hd", (nodes) =>
  nodes.map((node) => node.textContent.replace(/\s+/g, " ").trim()),
);
if (headers.length !== 4) {
  throw new Error(`expected 4 columns, got ${JSON.stringify(headers)}`);
}
if (!headers[0].startsWith("阻塞中") || !headers[1].startsWith("Frontier") || !headers[2].startsWith("进行中") || !headers[3].startsWith("最近完成")) {
  throw new Error(`unexpected column order: ${JSON.stringify(headers)}`);
}
if (await session.page.$(".board-hint") || await session.page.$('[data-lane="recentlyCompleted"] .lane-note')) {
  throw new Error("the default board should not repeat lane order or recently-completed rules");
}
const recentHierarchy = await session.page.$eval('[data-lane="recentlyCompleted"] .issue-card', (node) => {
  const title = node.querySelector(".issue-title");
  return {
    opacity: Number.parseFloat(getComputedStyle(node).opacity),
    decoration: title ? getComputedStyle(title).textDecorationLine : "",
    actionable: Boolean(node.querySelector("button:not([disabled])")),
  };
});
if (recentHierarchy.opacity > 0.65 || !recentHierarchy.decoration.includes("line-through") || !recentHierarchy.actionable) {
  throw new Error(`recently completed Issues should be visibly subdued but actionable: ${JSON.stringify(recentHierarchy)}`);
}

const dailyShellGeometry = await session.page.evaluate(() => {
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
await session.assertVisual("issue-99-desktop-1280x840.png");
await assertShellRegionsDoNotOverlap(session.page);

const shellStructure = async () => session.page.evaluate(() => ({
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
  await session.page.click("button[data-act='settings']");
  await session.page.click(`button[data-act='theme'][data-id='${theme}']`);
  await session.page.click(".overlay[data-act='close-settings']", { position: { x: 2, y: 2 } });
  const themedStructure = await shellStructure();
  if (JSON.stringify(themedStructure) !== JSON.stringify(warmStructure)) {
    throw new Error(`theme ${theme} changed the shell information architecture: ${JSON.stringify(themedStructure)}`);
  }
}

await session.page.setViewportSize({ width: 1440, height: 900 });
let releaseDocumentRefresh;
const documentRefreshGate = new Promise((resolve) => {
  releaseDocumentRefresh = resolve;
});
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
  if (request?.op === "focusIssue" && request.issueId === "you/garden#2") {
    const response = await route.fetch();
    const result = await response.json();
    result.snapshot.board.selected.document = {
      kind: "stale",
      body: "# Cached issue body\n\nVisible while Tracker refreshes.",
      fetchedAtMs: Date.now() - 60_000,
      failure: { kind: "offline", message: "simulated offline Tracker" },
    };
    await route.fulfill({ response, json: result });
    return;
  }
  if (request?.op === "loadIssueDocument" && request.issueId === "you/garden#2") {
    await documentRefreshGate;
  }
  await route.continue();
};
await session.page.route("**/*", preserveCachedDocumentDuringRefresh);
await session.clickCard(session.page.locator(".issue-card:has-text('child ready') .issue-card-main"));
await session.page.waitForSelector(".detail-hd:has-text('child ready')");
await session.page.waitForSelector('[data-document-state="stale"] .issue-markdown:has-text("Cached issue body")');
if (await session.page.$('button[data-act="edit-issue"]')) {
  throw new Error("Issue editing must stay unavailable for a stale read-only document");
}
await session.page.click('button[data-act="retry-issue-document"]');
await session.page.waitForSelector('[data-document-state="loading"] .issue-markdown:has-text("Cached issue body")');
if (await session.page.$('button[data-act="edit-issue"]')) {
  throw new Error("Issue editing must stay unavailable while the complete document is loading");
}
const loadingStatus = await session.page.$eval('[data-document-state="loading"] .document-status', (node) => node.textContent);
if (!loadingStatus?.includes("数据截至") && !loadingStatus?.includes("Data as of")) {
  throw new Error(`cached Issue document should keep its as-of time while refreshing, got ${loadingStatus}`);
}
releaseDocumentRefresh();
await session.page.waitForSelector('[data-document-state="ready"]');
await session.page.waitForSelector('button[data-act="edit-issue"]');
await session.page.unroute("**/*", preserveCachedDocumentDuringRefresh);
const documentText = await session.page.$eval(".issue-markdown", (node) => node.textContent?.replace(/\s+/g, " ").trim());
if (!documentText?.includes("Can the operator read every constraint") || !documentText.includes("Paragraph six")) {
  throw new Error(`Issue document should render the complete long body, got ${documentText}`);
}
for (const selector of [".issue-markdown h1", ".issue-markdown h2", ".issue-markdown strong", ".issue-markdown code", ".issue-markdown ul"]) {
  if (!(await session.page.$(selector))) throw new Error(`Markdown rendering missing ${selector}`);
}
if (await session.page.$(".issue-markdown script") || await session.page.evaluate(() => window.__ISSUE_HTML_EXECUTED__ === true)) {
  throw new Error("raw Issue HTML must stay escaped and inert");
}
if (!(await session.page.$('.issue-markdown a[data-url="https://github.com/you/garden/issues/2"]'))) {
  throw new Error("safe HTTPS markdown link should remain available");
}
if (await session.page.$('.issue-markdown [data-url^="javascript:"]')) {
  throw new Error("dangerous markdown URLs must not become actions");
}
if (!(await session.page.$(".issue-markdown .unsafe-link"))) {
  throw new Error("dangerous markdown link should be rendered as inert text");
}
const sectionOrder = await session.page.evaluate(() => {
  const documentTop = document.querySelector(".issue-document")?.getBoundingClientRect().top ?? 0;
  const family = document.querySelector(".detail-block")?.getBoundingClientRect().top ?? 0;
  return { document: documentTop, family };
});
if (sectionOrder.family <= sectionOrder.document) {
  throw new Error(`family and Dependency sections should follow the document: ${JSON.stringify(sectionOrder)}`);
}
const headerTopBeforeScroll = await session.page.$eval(".detail-sticky", (node) => node.getBoundingClientRect().top);
await session.page.$eval(".detail-scroll", (node) => { node.scrollTop = node.scrollHeight; });
const headerTopAfterScroll = await session.page.$eval(".detail-sticky", (node) => node.getBoundingClientRect().top);
if (Math.abs(headerTopAfterScroll - headerTopBeforeScroll) > 1) {
  throw new Error("Issue title and actions should stay pinned while document content scrolls");
}
await session.page.$eval(".detail-scroll", (node) => { node.scrollTop = 0; });
await session.capture("issue-98-desktop-detail-1440x900.png");
await session.assertVisual("issue-99-desktop-1440x900.png");
await assertShellRegionsDoNotOverlap(session.page);
const normalDetailWidth = await session.page.$eval(".board-shell > .issue-detail", (node) => node.getBoundingClientRect().width);
if (normalDetailWidth < 340) {
  throw new Error(`Issue document should be readable in the default desktop shell, got ${normalDetailWidth}px`);
}
if (await session.page.$('button[data-act="toggle-issue-width"]')) {
  throw new Error("Issue details should not expose a widen/narrow action");
}
const detailHide = session.page.locator('.issue-detail .detail-title-row button[data-act="toggle-issue"]');
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
await session.page.waitForFunction(() => !document.querySelector(".board-shell > .issue-detail"));
const restoreDetail = session.page.locator('button[data-act="toggle-issue"][aria-label="显示详情"]');
if ((await restoreDetail.count()) !== 1 || (await restoreDetail.locator("svg").count()) !== 1) {
  throw new Error("collapsed Issue details should keep an icon-only restore control in chrome");
}
await restoreDetail.click();
await session.page.waitForSelector(".board-shell > .issue-detail");
await session.page.click(".issue-detail button[data-act='open-issue']");
const openedDetailUrl = await session.page.evaluate(() => window.__OPENED_URLS__.at(-1));
if (openedDetailUrl !== "https://github.com/you/garden/issues/2") {
  throw new Error(`details should open the GitHub Issue, got ${openedDetailUrl}`);
}
const beforeFrontier = await session.page.$$eval('[data-lane="frontier"] .issue-card', (nodes) => nodes.length);

await session.page.click(".name-btn:has-text('#1 parent')");
await session.page.waitForSelector(".detail-hd:has-text('parent')");
const stillFrontier = await session.page.$$eval('[data-lane="frontier"] .issue-card', (nodes) => nodes.length);
if (stillFrontier !== beforeFrontier) {
  throw new Error("clicking a parent link filtered the board");
}

await session.page.click("button:has-text('只看这些子票')");
await session.page.waitForSelector("button:has-text('清除过滤')");
const filtered = await session.page.$$eval('[data-lane="frontier"] .issue-card .issue-title', (nodes) =>
  nodes.map((node) => node.textContent),
);
if (filtered.join(",") !== "child ready") {
  throw new Error(`parent filter should show only children, got ${JSON.stringify(filtered)}`);
}

await session.page.click("button:has-text('清除过滤')");
await session.page.waitForFunction(() => !document.querySelector("button[data-act='clear-filter']"));
}
