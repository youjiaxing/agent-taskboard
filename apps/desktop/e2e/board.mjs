import { chromium } from "playwright";

const url = process.env.BOARD_URL;
if (!url) {
  console.error("missing BOARD_URL");
  process.exit(1);
}

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ locale: "zh-CN", viewport: { width: 1280, height: 840 } });
const page = await context.newPage();
page.on("pageerror", (error) => {
  console.error("pageerror", error);
});
page.on("console", (msg) => {
  if (msg.type() === "error") {
    console.error("console", msg.text());
  }
});
await page.addInitScript((protocol) => {
  window.__HOST_PROTOCOL__ = protocol;
  window.__OPENED_URLS__ = [];
  window.open = (target) => {
    window.__OPENED_URLS__.push(String(target));
    return null;
  };
}, url);
await page.goto(url, { waitUntil: "domcontentloaded" });
try {
  await page.waitForSelector(".lanes");
} catch (error) {
  const html = await page.content();
  console.error("page html", html.slice(0, 4000));
  throw error;
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

await page.click(".issue-card:has-text('child ready') .issue-card-main");
await page.waitForSelector(".detail-hd:has-text('child ready')");
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
await page.waitForSelector("button:has-text('清除过滤')");
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

await page.click("button[data-act='center-view'][data-id='graph']");
await page.waitForSelector(".dep-graph");
if (await page.$(".lanes")) {
  throw new Error("graph view should replace the four columns");
}
const graphTitles = await page.$$eval(".graph-node .issue-title", (nodes) =>
  nodes.map((node) => node.textContent),
);
if (!graphTitles.includes("unparented ready") || !graphTitles.includes("blocker")) {
  throw new Error(`graph should include all open issues, got ${JSON.stringify(graphTitles)}`);
}
if (graphTitles.includes("old gate") || graphTitles.includes("just closed")) {
  throw new Error("closed context should be off by default");
}
const edge = await page.$('path[data-from="you/garden#9"][data-to="you/garden#3"]');
if (!edge) {
  throw new Error("graph should draw the blocker edge from left to right");
}
if (await page.$('path[data-from="you/garden#1"][data-to="you/garden#2"]')) {
  throw new Error("graph should not draw parent/child as an edge");
}

await page.click(".graph-node:has-text('unparented ready')");
await page.waitForSelector(".detail-hd:has-text('unparented ready')");
if (await page.$("button[data-act='clear-filter']")) {
  throw new Error("clicking a graph node should not filter the board");
}

await page.click("[data-field='closedContext']");
await page.waitForSelector(".graph-node:has-text('old gate')");
if (await page.$(".graph-node:has-text('just closed')")) {
  throw new Error("closed context should only add dependency neighbors");
}

await page.click(".graph-node:has-text('active work')");
await page.waitForSelector(".detail-hd:has-text('active work')");
if (await page.$(".lifted-run")) {
  throw new Error("dependency graph nodes should only change Issue details");
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
const liftedWidths = await page.evaluate(() => {
  const terminal = document.querySelector(".lifted-terminal")?.getBoundingClientRect().width ?? 0;
  const detail = document.querySelector(".lifted-run .issue-detail")?.getBoundingClientRect().width ?? 0;
  return { terminal, detail };
});
if (liftedWidths.terminal < liftedWidths.detail * 1.8 || liftedWidths.terminal > liftedWidths.detail * 2.2) {
  throw new Error(`lifted Run should use about a 2:1 split, got ${JSON.stringify(liftedWidths)}`);
}
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

await page.click("button[data-act='toggle-sidebar']");
if (await page.$(".side")) {
  throw new Error("the sidebar toggle should remove the sidebar from layout");
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
await page.click("button[data-act='new-run']");
await page.waitForSelector(".launch-sheet");
const pick = page.locator("button[data-act='pick-agent']:not([disabled])").first();
if (await pick.count()) {
  await pick.click();
  await page.waitForSelector("textarea[data-field='openingText']");
}
await page.fill("textarea[data-field='openingText']", "e2e unbound run");
await page.click(".launch-sheet button[type='submit']");
await page.waitForSelector(".run-dock");
await page.waitForFunction(() => !document.querySelector(".launch-sheet"));
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

await page.click("button[data-act='mobile-issue']");
await page.waitForSelector(".mobile-issue-view .issue-detail");
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
