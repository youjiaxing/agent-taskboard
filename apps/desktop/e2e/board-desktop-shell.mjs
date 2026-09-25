import { assertNecessaryTextContrast, assertShellRegionsDoNotOverlap } from "./board-harness.mjs";

export async function runDesktopBoardShell(session) {
try {
  await session.page.waitForSelector(".lanes");
} catch (error) {
  const html = await session.page.content();
  console.error("page html", html.slice(0, 4000));
  throw error;
}
const originalHostName = await session.page.$eval(".host-line .host-name", (node) => node.textContent?.trim() ?? "");
await session.page.click("button[data-act='toggle-hosts']");
const visibleHosts = await session.page.$$eval(".host-picker button[data-act='focus-host']", (nodes) =>
  nodes.map((node) => ({
    id: node.getAttribute("data-id"),
    text: node.textContent?.replace(/\s+/g, " ").trim(),
    active: node.classList.contains("active"),
  })),
);
if (visibleHosts.length < 2) {
  throw new Error(`daily shell fixture should expose multiple Hosts, got ${JSON.stringify(visibleHosts)}`);
}
const originalHost = visibleHosts.find((host) => host.active);
const otherHost = visibleHosts.find((host) => !host.active);
if (!originalHost?.id || !otherHost?.id) throw new Error(`Host switch fixture is incomplete: ${JSON.stringify(visibleHosts)}`);
await session.page.click(`.host-picker button[data-id="${otherHost.id}"]`);
await session.page.waitForFunction((name) => document.querySelector('.host-line .host-name')?.textContent?.trim() === name, otherHost.text);
const otherHostScope = await session.page.evaluate(() => ({
  host: document.querySelector(".host-line .host-name")?.textContent?.trim(),
  projects: [...document.querySelectorAll(".side .project-block b")].map((node) => node.textContent?.trim()),
  runs: document.querySelectorAll(".side .run-row").length,
}));
if (otherHostScope.projects.includes("garden") || otherHostScope.runs > 0) {
  throw new Error(`sidebar leaked Projects or Runs from another Host: ${JSON.stringify(otherHostScope)}`);
}

// Host 总览与用量只属于当前 Host：从 Host 区域进入，返回恢复来源页面。
// 产品壳按秒重绘，点击用合成事件落到当前节点上，避免重绘吞掉真实鼠标点击。
const clickNode = (selector) => session.page.$eval(selector, (node) => node.click());
const hostAreaEntries = await session.page.$$eval(".host-area [data-act]", (nodes) => nodes.map((node) => node.getAttribute("data-act")));
if (!hostAreaEntries.includes("open-overview") || !hostAreaEntries.includes("open-usage")) {
  throw new Error(`Host overview and usage must be reachable from the current Host area: ${JSON.stringify(hostAreaEntries)}`);
}
await clickNode('.side .project-main:has-text("ledger")');
await session.page.waitForSelector('.project-board .project-heading h1:has-text("ledger")');
await clickNode(".host-area button[data-act='open-overview']");
await session.page.waitForSelector(".overview-page");
const otherHostOverview = await session.page.evaluate(() => ({
  projects: [...document.querySelectorAll(".overview-project .overview-project-head b")].map((node) => node.textContent?.trim()),
  runs: [...document.querySelectorAll(".run-thumbnail .run-project")].map((node) => node.textContent?.trim()),
  stacked: document.querySelectorAll(".lanes, .dep-graph, .focus-workspace-layout").length,
}));
if (JSON.stringify(otherHostOverview.projects) !== JSON.stringify(["ledger"]) || otherHostOverview.runs.length || otherHostOverview.stacked) {
  throw new Error(`Host overview must show only the focused Host without stacking on the board: ${JSON.stringify(otherHostOverview)}`);
}
await clickNode("button[data-act='return-page']");
await session.page.waitForSelector('.project-board .project-heading h1:has-text("ledger")');
await clickNode(".host-area button[data-act='open-usage']");
await session.page.waitForSelector(".usage-page");
const otherHostUsage = await session.page.evaluate(() => ({
  rows: document.querySelectorAll(".usage-row").length,
  empty: document.querySelector(".usage-page .board-empty")?.textContent?.trim() ?? "",
  projectOptions: [...document.querySelectorAll(".usage-page select[data-usage-filter='projectId'] option")].map((node) => node.textContent?.trim()),
  stacked: document.querySelectorAll(".lanes, .dep-graph, .focus-workspace-layout, .overview-page").length,
}));
if (otherHostUsage.rows !== 0 || !otherHostUsage.empty || otherHostUsage.stacked || otherHostUsage.projectOptions.some((name) => name === "garden" || name === "tools")) {
  throw new Error(`usage must show only the focused Host without stacking on the board: ${JSON.stringify(otherHostUsage)}`);
}
await clickNode("button[data-act='return-page']");
await session.page.waitForSelector('.project-board .project-heading h1:has-text("ledger")');

await clickNode("button[data-act='toggle-hosts']");
await session.page.waitForSelector(".host-picker");
await clickNode(`.host-picker button[data-id="${originalHost.id}"]`);
await session.page.waitForFunction((name) => document.querySelector(".host-line .host-name")?.textContent?.trim() === name, originalHostName);
await session.page.waitForSelector(".lanes");
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

// 键盘帮助必须与真实键位一致，每个快捷键都要触发它声称的动作，且不得占用系统或 Agent TUI 的组合键。
const expectedShortcuts = [
  { id: "help", keys: ["?"] },
  { id: "search", keys: ["/"] },
  { id: "next-card", keys: ["J", "↓"] },
  { id: "previous-card", keys: ["K", "↑"] },
  { id: "open-card", keys: ["⏎"] },
  { id: "dismiss", keys: ["Esc"] },
];
await session.page.keyboard.press("?");
await session.page.waitForSelector(".keyboard-help");
const advertisedShortcuts = await session.page.$$eval(".keyboard-help .shortcut-list li", (nodes) =>
  nodes.map((node) => ({
    id: node.getAttribute("data-shortcut"),
    keys: [...node.querySelectorAll("kbd")].map((kbd) => kbd.textContent?.trim()),
    label: node.querySelector(".shortcut-label")?.textContent?.trim() ?? "",
  })),
);
if (JSON.stringify(advertisedShortcuts.map(({ id, keys }) => ({ id, keys }))) !== JSON.stringify(expectedShortcuts)) {
  throw new Error(`keyboard help must advertise every shell shortcut with its real keys: ${JSON.stringify(advertisedShortcuts)}`);
}
if (advertisedShortcuts.some((shortcut) => !shortcut.label || shortcut.keys.some((key) => /⌘|⌥|⌃|⇧|Ctrl|Alt|Meta|Cmd/i.test(key ?? "")))) {
  throw new Error(`shell shortcuts must stay modifier-free and every row must explain itself: ${JSON.stringify(advertisedShortcuts)}`);
}
await session.page.keyboard.press("?");
await session.page.waitForFunction(() => !document.querySelector(".keyboard-help"));

// 每个快捷键都要触发它声称的动作；卡片导航的焦点观察放在同一次求值里，避免定时重绘清掉焦点。
const pressShortcut = (key) => session.page.evaluate((nextKey) => {
  document.body.dispatchEvent(new KeyboardEvent("keydown", { key: nextKey, bubbles: true, cancelable: true }));
  return {
    activeId: document.activeElement?.dataset?.issueId ?? "",
    activeClass: document.activeElement?.className ?? "",
    searchFocused: document.activeElement?.id === "issue-title-search",
  };
}, key);

await session.page.keyboard.press("/");
if (!(await session.page.$eval("#issue-title-search", (node) => node === document.activeElement))) {
  throw new Error("/ should focus the Issue search");
}
await session.page.evaluate(() => (document.activeElement instanceof HTMLElement ? document.activeElement.blur() : undefined));

await session.page.click('.chrome button[data-act="more-menu"]');
await session.page.waitForSelector(".more-menu");
await session.page.keyboard.press("Escape");
await session.page.waitForFunction(() => !document.querySelector(".more-menu"));

const firstCard = await pressShortcut("j");
if (!firstCard.activeClass.includes("issue-card-main") || !firstCard.activeId) {
  throw new Error(`J should focus a board card: ${JSON.stringify(firstCard)}`);
}
const nextCard = await session.page.evaluate((current) => {
  document.body.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true, cancelable: true }));
  return document.activeElement?.dataset?.issueId ?? "";
}, firstCard.activeId);
if (!nextCard || nextCard === firstCard.activeId) {
  throw new Error(`J and the arrow keys should move between board cards: ${firstCard.activeId} -> ${nextCard}`);
}
const previousCard = await session.page.evaluate(() => {
  document.body.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true, cancelable: true }));
  return document.activeElement?.dataset?.issueId ?? "";
});
if (previousCard !== firstCard.activeId) {
  throw new Error(`K and the arrow keys should move back between board cards: ${previousCard}`);
}
const searchAfterCardFocus = await pressShortcut("/");
if (!searchAfterCardFocus.searchFocused) {
  throw new Error(`/ should focus the Issue search from a focused card: ${JSON.stringify(searchAfterCardFocus)}`);
}
await session.page.evaluate(() => (document.activeElement instanceof HTMLElement ? document.activeElement.blur() : undefined));
const scrollRegressionStyle = await session.page.addStyleTag({
  content: '[data-lane="frontier"] { max-height: 120px; } .workspace-rail-section .detail-scroll { max-height: 180px; }',
});
const frontierScrollBeforeFocus = await session.page.$eval('[data-lane="frontier"]', (node) => {
  node.scrollTop = node.scrollHeight;
  return node.scrollTop;
});
if (frontierScrollBeforeFocus <= 0) throw new Error("focus workspace regression needs a scrollable board lane");
await session.page.evaluate(() => {
  document.querySelectorAll(".issue-card-main")[0]?.focus();
  document.body.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
});
await session.page.waitForSelector(".focus-workspace-layout");
await session.page.waitForSelector('[data-terminal-surface]');
await session.page.waitForSelector('.workspace-rail-section[data-workspace-section="issue"][open] .issue-document[data-document-state="ready"]');
if (await session.page.$(".lanes")) throw new Error("selecting an Issue should enter the focus workspace instead of overlaying the board");
const initialWorkspaceSections = await session.page.$$eval(".workspace-rail-section", (sections) =>
  Object.fromEntries(sections.map((section) => [section.dataset.workspaceSection, section.open])),
);
if (!initialWorkspaceSections.issue || initialWorkspaceSections.actions || initialWorkspaceSections.runs) {
  throw new Error(`an Issue without an active Run should default to its body section: ${JSON.stringify(initialWorkspaceSections)}`);
}
const detailScrollBeforeCollapse = await session.page.$eval('.workspace-rail-section[data-workspace-section="issue"] .detail-scroll', (node) => {
  node.scrollTop = node.scrollHeight;
  return node.scrollTop;
});
if (detailScrollBeforeCollapse <= 0) throw new Error("focus workspace regression needs a scrollable Issue document");
await session.page.click("button[data-act='toggle-issue']");
await session.page.waitForFunction(() => !document.querySelector('[data-fixed-panel="right-rail"]'));
if (!(await session.page.$('[data-terminal-surface]')) || !(await session.page.$("button[data-act='toggle-issue']"))) {
  throw new Error("hiding the right rail must keep the fixed Terminal and its restore control");
}
await session.page.click("button[data-act='toggle-issue']");
await session.page.waitForSelector('.workspace-rail-section[data-workspace-section="issue"][open]');
const detailScrollAfterRestore = await session.page.$eval('.workspace-rail-section[data-workspace-section="issue"] .detail-scroll', (node) => node.scrollTop);
if (Math.abs(detailScrollAfterRestore - detailScrollBeforeCollapse) > 1) {
  throw new Error(`restoring the right rail must preserve Issue scroll: ${detailScrollBeforeCollapse} -> ${detailScrollAfterRestore}`);
}
const inspectorHierarchy = await session.page.$eval('.workspace-rail-section[data-workspace-section="issue"]', (node) => ({
  text: node.textContent?.replace(/\s+/g, " ").trim() ?? "",
  commentVisible: Boolean(node.querySelector('form[data-act="issue-comment"]')?.getClientRects().length),
}));
if (!inspectorHierarchy.text.includes("父子关系") || !inspectorHierarchy.text.includes("依赖关系")) {
  throw new Error(`Issue body should use clear relationship headings: ${inspectorHierarchy.text}`);
}
for (const internalPhrase of ["属于 / 子票", "挡住它的 / 它挡住的", "无，可进 Frontier"]) {
  if (inspectorHierarchy.text.includes(internalPhrase)) throw new Error(`Issue body should remove internal copy: ${internalPhrase}`);
}
if (inspectorHierarchy.commentVisible) throw new Error("secondary Issue update forms should stay collapsed until requested");
await session.page.click("button[data-act='return-page']");
await session.page.waitForSelector(".lanes");
const frontierScrollAfterReturn = await session.page.$eval('[data-lane="frontier"]', (node) => node.scrollTop);
if (Math.abs(frontierScrollAfterReturn - frontierScrollBeforeFocus) > 1) {
  throw new Error(`returning from focus must preserve board scroll: ${frontierScrollBeforeFocus} -> ${frontierScrollAfterReturn}`);
}
await session.page.$eval('.issue-card:has(.issue-title:text-is("unparented ready")) .issue-card-main', (node) => node.click());
await session.page.waitForSelector('[data-terminal-surface="empty"]');
await session.page.waitForSelector('.workspace-rail-section[data-workspace-section="issue"][open] .issue-document[data-document-state="ready"]');
await session.page.click("button[data-act='return-page']");
await session.page.waitForSelector(".lanes");
const frontierScrollAfterSwitch = await session.page.$eval('[data-lane="frontier"]', (node) => node.scrollTop);
if (Math.abs(frontierScrollAfterSwitch - frontierScrollBeforeFocus) > 1) {
  throw new Error(`switching Issues through focus must preserve board scroll: ${frontierScrollBeforeFocus} -> ${frontierScrollAfterSwitch}`);
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

// 标题 / 角色 / 开关筛选 + 列滚动 + 已选 Issue：进入专注工作区再返回必须完全一致
const boardRoundTripStyle = await session.page.addStyleTag({ content: '[data-lane="frontier"] { max-height: 90px; }' });
await session.page.fill("#issue-title-search", "ready");
await session.page.selectOption(".issue-search select[name='triageRole']", "ready-for-agent");
await session.page.selectOption(".issue-search select[name='state']", "open");
const filteredSearchResponse = await submitIssueSearch();
if (filteredSearchResponse.snapshot?.board?.search?.triageRole !== "ready-for-agent") {
  throw new Error(`role filter should reach the Host search: ${JSON.stringify(filteredSearchResponse.snapshot?.board?.search)}`);
}
await session.page.waitForFunction(() => document.querySelectorAll('[data-lane="frontier"] .issue-card').length > 1);
const boardRoundTripBefore = await session.page.evaluate(() => {
  const lane = document.querySelector('[data-lane="frontier"]');
  lane.scrollTop = lane.scrollHeight;
  return {
    title: document.querySelector("#issue-title-search")?.value ?? "",
    triageRole: document.querySelector(".issue-search select[name='triageRole']")?.value ?? "",
    state: document.querySelector(".issue-search select[name='state']")?.value ?? "",
    scrollTop: lane.scrollTop,
    cards: [...document.querySelectorAll(".issue-card .issue-title")].map((node) => node.textContent?.trim()),
  };
});
if (boardRoundTripBefore.scrollTop <= 0) throw new Error("board round trip fixture needs a scrollable filtered lane");
await session.clickCard(session.page.locator('[data-lane="frontier"] .issue-card .issue-card-main').first());
await session.page.waitForSelector(".focus-workspace-layout");
const roundTripIssue = await session.page.$eval("[data-current-identity]", (node) => node.textContent?.trim());
await session.page.click("button[data-act='return-page']");
await session.page.waitForSelector(".lanes");
const boardRoundTripAfter = await session.page.evaluate(() => ({
  title: document.querySelector("#issue-title-search")?.value ?? "",
  triageRole: document.querySelector(".issue-search select[name='triageRole']")?.value ?? "",
  state: document.querySelector(".issue-search select[name='state']")?.value ?? "",
  scrollTop: document.querySelector('[data-lane="frontier"]')?.scrollTop ?? -1,
  cards: [...document.querySelectorAll(".issue-card .issue-title")].map((node) => node.textContent?.trim()),
  selected: document.querySelector(".issue-card.sel .issue-title")?.textContent?.trim() ?? "",
  inspector: document.querySelector(".board-shell > .issue-detail .detail-hd")?.textContent?.trim() ?? "",
}));
if (
  boardRoundTripAfter.title !== boardRoundTripBefore.title
  || boardRoundTripAfter.triageRole !== boardRoundTripBefore.triageRole
  || boardRoundTripAfter.state !== boardRoundTripBefore.state
  || Math.abs(boardRoundTripAfter.scrollTop - boardRoundTripBefore.scrollTop) > 1
  || boardRoundTripAfter.cards.join("|") !== boardRoundTripBefore.cards.join("|")
) {
  throw new Error(`returning from the focus workspace must restore filters, lane scroll and the Issue set: ${JSON.stringify({ boardRoundTripBefore, boardRoundTripAfter })}`);
}
if (!roundTripIssue || boardRoundTripAfter.selected !== roundTripIssue || !boardRoundTripAfter.inspector.includes(roundTripIssue)) {
  throw new Error(`returning from the focus workspace must keep the selected Issue: ${JSON.stringify({ roundTripIssue, boardRoundTripAfter })}`);
}
await boardRoundTripStyle.evaluate((node) => node.remove());
await session.page.fill("#issue-title-search", "");
await session.page.selectOption(".issue-search select[name='triageRole']", "");
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
  const root = getComputedStyle(document.documentElement);
  return {
    opacity: Number.parseFloat(getComputedStyle(node).opacity),
    titleColor: title ? getComputedStyle(title).color : "",
    secondaryColor: root.getPropertyValue("--color-text-secondary").trim(),
    decoration: title ? getComputedStyle(title).textDecorationLine : "",
    actionable: Boolean(node.querySelector("button:not([disabled])")),
  };
});
if (recentHierarchy.opacity !== 1 || recentHierarchy.titleColor !== "rgb(86, 87, 91)" || recentHierarchy.secondaryColor !== "#56575b" || !recentHierarchy.decoration.includes("line-through") || !recentHierarchy.actionable) {
  throw new Error(`recently completed Issues should stay readable, visibly subdued and actionable: ${JSON.stringify(recentHierarchy)}`);
}

const dailyShellGeometry = await session.page.evaluate(() => {
  const rect = (selector) => document.querySelector(selector)?.getBoundingClientRect();
  const chrome = rect(".chrome");
  const side = rect(".side");
  const lanes = [...document.querySelectorAll(".lane")].map((node) => node.getBoundingClientRect());
  const boardTabs = [...document.querySelectorAll('[data-page-toolbar] [data-act="center-view"]')];
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
if (dailyShellGeometry.chromeHeight !== 44) {
  throw new Error(`desktop chrome should stay native and compact, got ${dailyShellGeometry.chromeHeight}px`);
}
if (dailyShellGeometry.sideWidth < 220 || dailyShellGeometry.sideWidth > 250) {
  throw new Error(`desktop Host / Project hierarchy should keep a stable native rail, got ${dailyShellGeometry.sideWidth}px`);
}
if (dailyShellGeometry.boardTabs.join("|") !== "看板|依赖图") {
  throw new Error(`board and graph controls should live in the content toolbar, got ${JSON.stringify(dailyShellGeometry.boardTabs)}`);
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
const shellControlOwnership = await session.page.evaluate(() => ({
  globalActions: [...document.querySelectorAll("[data-global-actions] [data-global-action]")]
    .map((node) => node.getAttribute("data-global-action")),
  topLevelSwitches: document.querySelectorAll('.chrome [data-act="center-view"]').length,
  toolbarSwitches: document.querySelectorAll('[data-page-toolbar] [data-act="center-view"]').length,
  toolbarSearches: document.querySelectorAll('[data-page-toolbar] .issue-search').length,
  toolbarProjectNames: document.querySelectorAll('[data-page-toolbar] .project-heading h1').length,
  toolbarKeyboardHelp: document.querySelectorAll('[data-page-toolbar] [data-act="keyboard-help"]').length,
  topProjectName: document.querySelector('[data-current-identity]')?.textContent?.trim(),
}));
const expectedGlobalOrder = ["right-rail", "appearance", "settings", "more"];
if (shellControlOwnership.globalActions.join("|") !== expectedGlobalOrder.join("|")) {
  throw new Error(`global action order is wrong: ${JSON.stringify(shellControlOwnership)}`);
}
if (shellControlOwnership.topLevelSwitches !== 0 || shellControlOwnership.toolbarSwitches !== 2 || shellControlOwnership.toolbarSearches !== 1 || shellControlOwnership.toolbarProjectNames !== 1 || shellControlOwnership.toolbarKeyboardHelp !== 0) {
  throw new Error(`page controls must belong only to the content toolbar: ${JSON.stringify(shellControlOwnership)}`);
}
if (shellControlOwnership.topProjectName === "garden") {
  throw new Error("the global identity must not duplicate the Project name from the content toolbar");
}
await session.page.focus('[data-global-actions] [data-global-action="right-rail"]');
for (const expected of expectedGlobalOrder.slice(1)) {
  await session.page.keyboard.press("Tab");
  const focusedAction = await session.page.evaluate(() => document.activeElement?.getAttribute("data-global-action"));
  if (focusedAction !== expected) {
    throw new Error(`global actions must follow their visual keyboard order, expected ${expected}, got ${focusedAction}`);
  }
}
await session.page.click('.chrome button[data-act="more-menu"]');
await session.page.waitForSelector('.more-menu button[data-act="keyboard-help"]');
const keyboardHelpOwnership = await session.page.evaluate(() => ({
  total: document.querySelectorAll('[data-act="keyboard-help"]').length,
  inMoreMenu: document.querySelectorAll('.more-menu [data-act="keyboard-help"]').length,
  inToolbar: document.querySelectorAll('[data-page-toolbar] [data-act="keyboard-help"]').length,
}));
if (keyboardHelpOwnership.total !== 1 || keyboardHelpOwnership.inMoreMenu !== 1 || keyboardHelpOwnership.inToolbar !== 0) {
  throw new Error(`keyboard help must belong only to the global more menu: ${JSON.stringify(keyboardHelpOwnership)}`);
}
await session.page.click('.chrome button[data-act="more-menu"]');
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
const readCssTokens = async (names) => session.page.evaluate((tokenNames) => {
  const style = getComputedStyle(document.documentElement);
  return Object.fromEntries(tokenNames.map((name) => [name, style.getPropertyValue(name).trim()]));
}, names);
const assertCssTokens = async (expected, label) => {
  const actual = await readCssTokens(Object.keys(expected));
  const mismatches = Object.entries(expected).filter(([name, value]) => actual[name] !== value);
  if (mismatches.length) {
    throw new Error(`${label} token mismatch: ${JSON.stringify({ mismatches, actual })}`);
  }
};
const commonTokens = {
  "--terminal-canvas": "#171717", "--terminal-surface": "#1f1f1f", "--terminal-text": "#f5f5f5",
  "--terminal-text-muted": "#a3a3a3", "--terminal-cursor": "#f5f5f5", "--terminal-selection": "#314766",
  "--font-family-ui": 'ui-sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
  "--font-family-mono": "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
  "--font-weight-regular": "400", "--font-weight-medium": "500", "--font-weight-semibold": "600",
  "--font-size-xs": "11px", "--font-size-sm": "12px", "--font-size-md": "13px", "--font-size-lg": "14px",
  "--font-size-xl": "16px", "--font-size-2xl": "20px", "--font-size-3xl": "24px",
  "--line-height-xs": "16px", "--line-height-sm": "18px", "--line-height-md": "20px", "--line-height-lg": "20px",
  "--line-height-xl": "24px", "--line-height-2xl": "28px", "--line-height-3xl": "32px",
  "--letter-spacing-heading": "-.01em", "--letter-spacing-body": "0",
  "--space-0": "0", "--space-2": "2px", "--space-4": "4px", "--space-6": "6px", "--space-8": "8px",
  "--space-10": "10px", "--space-12": "12px", "--space-16": "16px", "--space-20": "20px", "--space-24": "24px",
  "--space-32": "32px", "--space-40": "40px", "--space-48": "48px",
  "--radius-xs": "4px", "--radius-sm": "6px", "--radius-md": "8px", "--radius-lg": "12px", "--radius-xl": "16px", "--radius-full": "999px",
  "--control-height-sm": "28px", "--control-height-md": "32px", "--control-height-lg": "36px", "--touch-target-min": "44px",
  "--topbar-height-desktop": "44px", "--topbar-height-mobile": "48px",
  "--sidebar-width-min": "216px", "--sidebar-width-default": "248px", "--sidebar-width-max": "320px",
  "--right-rail-width-min": "280px", "--right-rail-width-default": "320px", "--right-rail-width-max": "420px",
  "--changes-panel-width-min": "420px", "--changes-panel-width-default": "520px", "--changes-panel-width-max": "640px",
  "--dialog-width-confirm": "420px", "--dialog-width-form": "560px", "--dialog-width-wide": "880px",
  "--shadow-focus": "0 0 0 3px color-mix(in srgb, #2563eb 22%, transparent)",
  "--shadow-raised-light": "0 1px 2px rgb(0 0 0 / 7%), 0 8px 24px rgb(0 0 0 / 8%)",
  "--shadow-dialog-light": "0 2px 8px rgb(0 0 0 / 12%), 0 18px 60px rgb(0 0 0 / 18%)",
  "--shadow-raised-dark": "0 1px 2px rgb(0 0 0 / 28%), 0 10px 30px rgb(0 0 0 / 28%)",
  "--shadow-dialog-dark": "0 20px 70px rgb(0 0 0 / 46%)",
  "--layer-base": "0", "--layer-sticky": "10", "--layer-rail": "20", "--layer-menu": "100",
  "--layer-overlay": "200", "--layer-dialog": "210", "--layer-toast": "300", "--layer-tooltip": "400",
  "--duration-fast": ".1s", "--duration-normal": ".16s", "--duration-slow": ".24s",
  "--ease-standard": "cubic-bezier(.2, 0, 0, 1)", "--ease-exit": "cubic-bezier(.4, 0, 1, 1)",
  "--density-body-font-size": "13px", "--density-control-height": "32px", "--density-topbar-height": "44px",
  "--density-gutter-inline": "12px", "--density-panel-gap": "8px", "--density-bottom-gap": "0",
};
const themeColorTokens = {
  light: ["#f7f7f5", "#ffffff", "#f3f3f1", "#ffffff", "#eeeeeb", "#e6e6e2", "#202123", "#56575b", "#75767a", "#ffffff", "#e2e2de", "#c6c7c3", "#2563eb", "#202123", "#0f1012", "#ffffff", "#dce7f9", "#237a43", "#eaf6ee", "#8a6100", "#fff4d6", "#b42318", "#ffece9", "#1d5db8", "#eaf2ff", "rgb(0 0 0 / 48%)"],
  dark: ["#171717", "#1f1f1f", "#262626", "#2b2b2b", "#303030", "#3a3a3a", "#f3f3f3", "#c8c8c8", "#a0a0a0", "#171717", "#373737", "#555555", "#60a5fa", "#f3f3f3", "#ffffff", "#171717", "#23456b", "#6bcb8b", "#173725", "#f2c15c", "#3e3217", "#ff8075", "#421d1a", "#7cb7ff", "#173052", "rgb(0 0 0 / 64%)"],
  warm: ["#f5efe7", "#fffaf3", "#faf2e8", "#fffdf9", "#f3e7d9", "#ead9c5", "#2a231c", "#62584e", "#7b7065", "#fffaf3", "#e4d6c5", "#cbb9a3", "#a64b25", "#a94722", "#873817", "#ffffff", "#f3d7c1", "#237a43", "#eaf6ee", "#8a6100", "#fff4d6", "#b42318", "#ffece9", "#1d5db8", "#eaf2ff", "rgb(42 35 28 / 45%)"],
};
const themeColorNames = ["--color-canvas", "--color-surface", "--color-surface-subtle", "--color-surface-raised", "--color-surface-hover", "--color-surface-active", "--color-text", "--color-text-secondary", "--color-text-muted", "--color-text-inverse", "--color-border", "--color-border-strong", "--color-focus", "--color-accent", "--color-accent-hover", "--color-on-accent", "--color-selection", "--color-success", "--color-success-surface", "--color-warning", "--color-warning-surface", "--color-danger", "--color-danger-surface", "--color-info", "--color-info-surface", "--color-overlay"];
const relativeLuminance = (hex) => {
  const channels = [1, 3, 5].map((offset) => parseInt(hex.slice(offset, offset + 2), 16) / 255)
    .map((channel) => channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4);
  return channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722;
};
const contrastRatio = (foreground, background) => {
  const values = [relativeLuminance(foreground), relativeLuminance(background)].sort((left, right) => right - left);
  return (values[0] + 0.05) / (values[1] + 0.05);
};
for (const [theme, colors] of Object.entries(themeColorTokens)) {
  const pairs = [
    [colors[6], colors[0]], [colors[6], colors[1]], [colors[6], colors[2]],
    [colors[7], colors[0]], [colors[7], colors[1]], [colors[7], colors[2]],
    [colors[8], colors[1]],
  ];
  const failing = pairs.filter(([foreground, background]) => contrastRatio(foreground, background) < 4.5);
  if (failing.length) throw new Error(`${theme} necessary text tokens must meet 4.5:1: ${JSON.stringify(failing)}`);
}
await assertCssTokens(commonTokens, "design foundation");
await assertCssTokens(Object.fromEntries(themeColorNames.map((name, index) => [name, themeColorTokens.light[index]])), "light theme");
const initialStructure = await shellStructure();
if ((await session.page.getAttribute("html", "data-theme")) !== "light") {
  throw new Error("a new browser Client should resolve system appearance to the current light color scheme");
}
await session.assertVisual("desktop-main-light.png");
await assertNecessaryTextContrast(session.page, "light desktop board");
await session.page.emulateMedia({ colorScheme: "dark" });
await session.page.waitForFunction(() => document.documentElement.dataset.theme === "dark");
await assertCssTokens(Object.fromEntries(themeColorNames.map((name, index) => [name, themeColorTokens.dark[index]])), "dark theme");
await session.assertVisual("desktop-main-dark.png");
await assertNecessaryTextContrast(session.page, "dark desktop board");
await session.page.click("button[data-act='appearance-menu']");
const appearanceLabels = await session.page.$$eval(".appearance-menu [role='menuitemradio']", (nodes) =>
  nodes.map((node) => node.textContent?.trim()),
);
if (appearanceLabels.join("|") !== "暖纸|素纸|素纸夜间|跟随系统") {
  throw new Error(`appearance menu should expose the four agreed choices, got ${JSON.stringify(appearanceLabels)}`);
}
if ((await session.page.evaluate(() => document.activeElement?.getAttribute("data-id"))) !== "warm") {
  throw new Error("opening the appearance menu should focus its first choice");
}
await session.assertVisual("appearance-menu.png");
await assertNecessaryTextContrast(session.page, "appearance menu");
await session.page.keyboard.press("ArrowDown");
if ((await session.page.evaluate(() => document.activeElement?.getAttribute("data-id"))) !== "light") {
  throw new Error("appearance menu arrow navigation should move between choices");
}
const focusRing = await session.page.evaluate(() => getComputedStyle(document.activeElement).boxShadow);
if (!focusRing || focusRing === "none" || !/0px 0px 0px 3px/.test(focusRing)) {
  throw new Error(`keyboard-focused appearance choices need the shared 3px focus ring, got ${focusRing}`);
}
await session.page.keyboard.press("Escape");
if (await session.page.$(".appearance-menu")) throw new Error("Escape should close the appearance menu");
await session.page.click("button[data-act='appearance-menu']");
await session.page.click(".appearance-menu button[data-act='appearance'][data-id='warm']");
await session.page.waitForFunction(() => document.documentElement.dataset.theme === "warm");
await assertCssTokens(Object.fromEntries(themeColorNames.map((name, index) => [name, themeColorTokens.warm[index]])), "warm theme");
await session.page.emulateMedia({ colorScheme: "light" });
await session.page.waitForTimeout(50);
if ((await session.page.getAttribute("html", "data-theme")) !== "warm") {
  throw new Error("a manual appearance preference must stop following system changes");
}
const storedBrowserAppearance = await session.page.evaluate(() => localStorage.getItem("agent-taskboard-browser-appearance"));
if (!storedBrowserAppearance?.includes('"appearancePreference":"warm"')) {
  throw new Error(`desktop browser appearance should persist in this origin, got ${storedBrowserAppearance}`);
}
const hostAppearanceAfterBrowserChoice = await session.page.evaluate(async (protocol) => {
  const response = await fetch(`${protocol}/rpc`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ op: "snapshot" }),
  });
  return (await response.json()).snapshot.appearance;
}, session.url);
if (hostAppearanceAfterBrowserChoice.appearancePreference !== "system") {
  throw new Error(`desktop browser appearance must not overwrite desktop-client settings, got ${JSON.stringify(hostAppearanceAfterBrowserChoice)}`);
}
const stableAddress = session.page.url();
for (const appearancePreference of ["light", "dark", "warm", "system"]) {
  await session.page.click("button[data-act='settings']");
  await session.page.waitForSelector('.settings-page[data-primary-page="settings"]');
  if (appearancePreference === "light") {
    await session.capture("issue-147-settings-1440x900.png");
    const settingsSections = await session.page.$$eval("[data-settings-section]", (nodes) =>
      nodes.map((node) => node.getAttribute("data-settings-section")),
    );
    if (settingsSections.join("|") !== "appearance|startup|updates|refresh|notifications|host") {
      throw new Error(`settings page should retain all agreed sections: ${JSON.stringify(settingsSections)}`);
    }
  }
  await session.page.click(`.settings-page button[data-act='appearance'][data-id='${appearancePreference}']`);
  if (appearancePreference === "light") {
    await session.page.waitForFunction(() => document.documentElement.dataset.theme === "light");
    await session.assertVisual("settings-appearance.png");
    await assertNecessaryTextContrast(session.page, "appearance settings");
  }
  await session.page.click("button[data-act='return-page']");
  await session.page.waitForSelector(".lanes");
  if (session.page.url() !== stableAddress) throw new Error("internal page navigation must keep the browser address stable");
  const resolvedTheme = await session.page.getAttribute("html", "data-theme");
  if (resolvedTheme === "system") {
    throw new Error("data-theme must only contain a resolved theme");
  }
  const expectedTheme = appearancePreference === "system" ? "light" : appearancePreference;
  await assertCssTokens(
    Object.fromEntries(themeColorNames.map((name, index) => [name, themeColorTokens[expectedTheme][index]])),
    `${appearancePreference} appearance`,
  );
  await assertNecessaryTextContrast(session.page, `${appearancePreference} desktop board`);
  const themedStructure = await shellStructure();
  if (JSON.stringify(themedStructure) !== JSON.stringify(initialStructure)) {
    throw new Error(`appearance ${appearancePreference} changed the shell information architecture: ${JSON.stringify(themedStructure)}`);
  }
}

await session.page.fill("#issue-title-search", "ready");
await submitIssueSearch();
await session.page.waitForFunction(() => document.querySelectorAll(".issue-card").length >= 2);
const settingsReturnStyle = await session.page.addStyleTag({ content: '[data-lane="frontier"] { max-height: 70px; }' });
const boardStateBeforeSettings = await session.page.evaluate(() => {
  const lane = document.querySelector('[data-lane="frontier"]');
  if (lane) lane.scrollTop = lane.scrollHeight;
  return { title: document.querySelector("#issue-title-search")?.value, scrollTop: lane?.scrollTop ?? 0 };
});
if (boardStateBeforeSettings.scrollTop <= 0) throw new Error("settings return fixture needs non-zero board scroll");
await session.clickCard(session.page.locator(".issue-card:has-text('child ready') .issue-card-main"));
await session.page.waitForSelector('[data-terminal-surface="empty"]');
await session.page.waitForSelector('.workspace-rail-section[data-workspace-section="issue"][open] [data-document-state="ready"]');
await session.page.click("button[data-act='settings']");
await session.page.waitForSelector(".settings-page");
await session.page.click("button[data-act='return-page']");
await session.page.waitForSelector('[data-terminal-surface="empty"]');
const focusStateAfterSettings = await session.page.evaluate(() => ({
  title: document.querySelector("[data-current-identity]")?.textContent?.trim(),
  issueOpen: document.querySelector('.workspace-rail-section[data-workspace-section="issue"]')?.open,
}));
if (focusStateAfterSettings.title !== "child ready" || !focusStateAfterSettings.issueOpen) {
  throw new Error(`settings return must restore the focused Issue workspace: ${JSON.stringify(focusStateAfterSettings)}`);
}
await session.page.click("button[data-act='return-page']");
await session.page.waitForSelector(".lanes");
const boardStateAfterSettings = await session.page.evaluate(() => ({
  title: document.querySelector("#issue-title-search")?.value,
  scrollTop: document.querySelector('[data-lane="frontier"]')?.scrollTop ?? 0,
}));
if (JSON.stringify(boardStateAfterSettings) !== JSON.stringify(boardStateBeforeSettings)) {
  throw new Error(`returning from settings through focus must restore filter and scroll: ${JSON.stringify({ boardStateBeforeSettings, boardStateAfterSettings })}`);
}
await settingsReturnStyle.evaluate((node) => node.remove());
await session.page.fill("#issue-title-search", "");
await submitIssueSearch();

await session.page.emulateMedia({ colorScheme: "light", reducedMotion: "reduce" });
await session.page.waitForFunction(() => document.documentElement.dataset.theme === "light");
await session.page.click("button[data-act='settings']");
const reducedMotion = await session.page.$eval(".settings-page", (node) => {
  const style = getComputedStyle(node);
  const milliseconds = (duration) => duration.endsWith("ms") ? parseFloat(duration) : parseFloat(duration) * 1000;
  return {
    animationDurationMs: milliseconds(style.animationDuration),
    animationIterationCount: style.animationIterationCount,
    transitionDurationMs: milliseconds(style.transitionDuration),
  };
});
if (reducedMotion.animationDurationMs > 5 || reducedMotion.transitionDurationMs > 5 || reducedMotion.animationIterationCount === "infinite") {
  throw new Error(`reduced motion should remove perceptible movement: ${JSON.stringify(reducedMotion)}`);
}
await session.page.click("button[data-act='return-page']");
await session.page.emulateMedia({ colorScheme: "light", reducedMotion: "no-preference" });

for (const [width, expectedViewport, expectedMobile] of [
  [640, "compact-desktop", "false"],
  [639, "mobile", "true"],
  [899, "compact-desktop", "false"],
  [900, "full-desktop", "false"],
]) {
  await session.page.setViewportSize({ width, height: 840 });
  await session.page.waitForFunction(
    ({ viewport, mobile }) => document.documentElement.dataset.viewport === viewport && document.documentElement.dataset.mobile === mobile,
    { viewport: expectedViewport, mobile: expectedMobile },
  );
  const responsiveState = await session.page.evaluate(() => ({
    horizontalOverflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
    boardMainDisplay: document.querySelector(".board-main") ? getComputedStyle(document.querySelector(".board-main")).display : "missing",
  }));
  if (responsiveState.horizontalOverflow > 0) throw new Error(`${width}px viewport must not create horizontal overflow: ${responsiveState.horizontalOverflow}`);
  if (width === 899 && responsiveState.boardMainDisplay !== "none") {
    throw new Error(`899px must use the compact desktop fixed-region composition: ${JSON.stringify(responsiveState)}`);
  }
  if (width === 900 && responsiveState.boardMainDisplay === "none") {
    throw new Error(`900px must use the full desktop composition: ${JSON.stringify(responsiveState)}`);
  }
}
await session.page.setViewportSize({ width: 1440, height: 900 });
await session.page.waitForFunction(() => document.documentElement.dataset.viewport === "full-desktop");

// 紧凑桌面上的高内容车道必须留在工作区内，由车道自身滚动，而不是把 board-main 撑出视口。
await session.page.setViewportSize({ width: 800, height: 600 });
await session.page.waitForSelector(".lanes");
const compactIssueToggle = session.page.locator("button[data-act='toggle-issue']");
if (await compactIssueToggle.count()) await compactIssueToggle.click();
await session.page.waitForFunction(() => document.querySelector(".board-main") && getComputedStyle(document.querySelector(".board-main")).display !== "none");
const compactBoardStyle = await session.page.addStyleTag({
  content: '[data-lane="frontier"] { height: 180px; min-height: 0; }',
});
const compactBoardBefore = await session.page.$eval('[data-lane="frontier"]', (node) => {
  const rect = node.getBoundingClientRect();
  const board = node.closest(".board-shell").getBoundingClientRect();
  return {
    scrollHeight: node.scrollHeight,
    clientHeight: node.clientHeight,
    boardBottom: board.bottom,
    viewportHeight: window.innerHeight,
    rectBottom: rect.bottom,
  };
});
if (compactBoardBefore.scrollHeight <= compactBoardBefore.clientHeight) {
  throw new Error(`compact board scroll fixture needs vertical overflow: ${JSON.stringify(compactBoardBefore)}`);
}
if (compactBoardBefore.boardBottom > compactBoardBefore.viewportHeight + 1 || compactBoardBefore.rectBottom > compactBoardBefore.viewportHeight + 1) {
  throw new Error(`compact board must stay inside the viewport: ${JSON.stringify(compactBoardBefore)}`);
}
const compactBoardBox = await session.page.locator('[data-lane="frontier"]').boundingBox();
if (!compactBoardBox) throw new Error("compact board scroll fixture has no geometry");
await session.page.mouse.move(compactBoardBox.x + compactBoardBox.width / 2, compactBoardBox.y + compactBoardBox.height / 2);
await session.page.mouse.wheel(0, 420);
const compactBoardAfter = await session.page.$eval('[data-lane="frontier"]', (node) => node.scrollTop);
if (compactBoardAfter <= 0) {
  throw new Error(`compact board lane should respond to a real mouse wheel: ${compactBoardBefore.clientHeight}/${compactBoardBefore.scrollHeight} -> ${compactBoardAfter}`);
}
await compactBoardStyle.evaluate((node) => node.remove());
await session.page.setViewportSize({ width: 1440, height: 900 });
await session.page.waitForFunction(() => document.documentElement.dataset.viewport === "full-desktop");

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
await session.page.waitForSelector('[data-terminal-surface="empty"]');
await session.page.waitForSelector('[data-document-state="stale"] .issue-markdown:has-text("Cached issue body")');
await session.page.click('.workspace-rail-section[data-workspace-section="actions"] > summary');
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
for (const selector of [
  ".issue-markdown h1",
  ".issue-markdown h2",
  ".issue-markdown strong",
  ".issue-markdown code",
  ".issue-markdown .unsafe-image",
  ".issue-markdown blockquote",
  ".issue-markdown pre > code.language-ts",
  ".issue-markdown input[type='checkbox'][disabled]",
  ".issue-markdown table > thead",
  ".issue-markdown table > tbody",
  ".issue-markdown ul ul",
  ".issue-markdown ul ol",
]) {
  if (!(await session.page.$(selector))) throw new Error(`Markdown rendering missing ${selector}`);
}
if (
  await session.page.$(".issue-markdown script, .issue-markdown [onclick], .issue-markdown [onerror]")
  || await session.page.evaluate(() => window.__ISSUE_HTML_EXECUTED__ === true)
) {
  throw new Error("raw Issue HTML must stay escaped and inert");
}
if (!(await session.page.$('.issue-markdown a[data-url="https://github.com/you/garden/issues/2"]'))) {
  throw new Error("safe HTTPS markdown link should remain available");
}
if (await session.page.$('.issue-markdown [data-url^="javascript:"]')) {
  throw new Error("dangerous markdown URLs must not become actions");
}
if (await session.page.$('.issue-markdown [data-url^="data:"]')) {
  throw new Error("data markdown URLs must not become actions");
}
if ((await session.page.$$(".issue-markdown .unsafe-link")).length !== 2) {
  throw new Error("dangerous markdown links should be rendered as inert text");
}
const imageLabel = await session.page.$eval(".issue-markdown .unsafe-image", (node) => node.textContent?.trim());
if (imageLabel !== "the Tracker image label" || await session.page.$(".issue-markdown img")) {
  throw new Error(`Markdown images must stay inert with readable labels, got ${JSON.stringify(imageLabel)}`);
}
const sectionOrder = await session.page.evaluate(() => {
  const body = document.querySelector('.workspace-rail-section[data-workspace-section="issue"]');
  const documentTop = body?.querySelector(".issue-document")?.getBoundingClientRect().top ?? 0;
  const family = body?.querySelector(".detail-block")?.getBoundingClientRect().top ?? 0;
  return { document: documentTop, family };
});
if (sectionOrder.family <= sectionOrder.document) {
  throw new Error(`family and Dependency sections should follow the document: ${JSON.stringify(sectionOrder)}`);
}
await session.capture("issue-98-desktop-detail-1440x900.png");
await assertShellRegionsDoNotOverlap(session.page);
const normalDetailWidth = await session.page.$eval('[data-fixed-panel="right-rail"]', (node) => node.getBoundingClientRect().width);
if (Math.abs(normalDetailWidth - 320) > 2) {
  throw new Error(`Issue document should use the default fixed right-rail width, got ${normalDetailWidth}px`);
}
if (await session.page.$('button[data-act="toggle-issue-width"]')) throw new Error("Issue details should not expose a widen/narrow action");
if (await session.page.$('.issue-detail button[data-act="toggle-issue"]')) throw new Error("the right-rail toggle must exist only in the global top bar");
const detailHide = session.page.locator('.chrome button[data-act="toggle-issue"]');
if ((await detailHide.count()) !== 1 || (await detailHide.getAttribute("aria-label")) !== "收起详情") {
  throw new Error("the global top bar should expose one right-rail hide control");
}
await detailHide.click();
await session.page.waitForFunction(() => !document.querySelector('[data-fixed-panel="right-rail"]'));
if (!(await session.page.$('[data-terminal-surface="empty"]'))) throw new Error("hiding Issue details must keep the fixed Terminal empty state");
const restoreDetail = session.page.locator('button[data-act="toggle-issue"][aria-label="显示详情"]');
if ((await restoreDetail.count()) !== 1 || (await restoreDetail.locator("svg").count()) !== 1) {
  throw new Error("collapsed Issue details should keep an icon-only restore control in chrome");
}
await restoreDetail.click();
await session.page.waitForSelector('[data-fixed-panel="right-rail"]');
await session.page.click('.workspace-rail-section[data-workspace-section="actions"] button[data-act="open-issue"]');
const openedDetailUrl = await session.page.evaluate(() => window.__OPENED_URLS__.at(-1));
if (openedDetailUrl !== "https://github.com/you/garden/issues/2") {
  throw new Error(`details should open the GitHub Issue, got ${openedDetailUrl}`);
}
await session.page.click("button[data-act='return-page']");
await session.page.waitForSelector(".lanes");
const beforeFrontier = await session.page.$$eval('[data-lane="frontier"] .issue-card', (nodes) => nodes.length);
await session.clickCard(session.page.locator(".issue-card:has-text('child ready') .issue-card-main"));
await session.page.waitForSelector('[data-terminal-surface="empty"]');
await session.page.click(".name-btn:has-text('#1 parent')");
await session.page.waitForFunction(() => document.querySelector("[data-current-identity]")?.textContent?.trim() === "parent");
if (await session.page.$(".lanes")) throw new Error("clicking a parent link should stay in the focus workspace without filtering the board");
await session.page.click("button:has-text('只看这些子票')");
await session.page.click("button[data-act='return-page']");
await session.page.waitForSelector("button:has-text('清除过滤')");
const filtered = await session.page.$$eval('[data-lane="frontier"] .issue-card .issue-title', (nodes) => nodes.map((node) => node.textContent));
if (filtered.join(",") !== "child ready") {
  throw new Error(`parent filter should show only children, got ${JSON.stringify(filtered)}`);
}
if (beforeFrontier <= filtered.length) throw new Error("the parent filter should narrow the Frontier");
await session.page.click("button:has-text('清除过滤')");
await session.page.waitForFunction(() => !document.querySelector("button[data-act='clear-filter']"));
}
