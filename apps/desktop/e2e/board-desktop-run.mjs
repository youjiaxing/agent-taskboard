import { assertShellRegionsDoNotOverlap } from "./board-harness.mjs";

export async function runDesktopBoardRun(session) {
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
  if (request?.op !== "openHostOverview") {
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
await session.page.route("**/*", emptyRunsOverviewResponse);
await session.page.click("button[data-act='open-overview']");
await session.page.waitForSelector(".overview-page");
const toolsOverview = await session.page.$$eval(".overview-project:has-text('tools') .overview-project-metrics > span", (nodes) =>
  Object.fromEntries(nodes.map((node) => [node.querySelector("i")?.textContent, node.querySelector("b")?.textContent])),
);
if (toolsOverview.Open !== "1" || toolsOverview.Frontier !== "1") {
  throw new Error(`Host overview should discard a stale cross-Host Project filter and keep Issue data without Runs, got ${JSON.stringify(toolsOverview)}`);
}
if (!(await session.page.$(".overview-runs-empty"))) {
  throw new Error("a Host with no Runs should keep a compact Run empty state below Project data");
}
await session.page.unroute("**/*", emptyRunsOverviewResponse);
await session.page.click("button[data-act='return-page']");
await session.page.waitForSelector(".lanes");

await session.clickCard(session.page.locator('[data-lane="inProgress"] .issue-card:has-text("active work") .issue-card-main'));
await session.page.waitForSelector(".lifted-run");
if (await session.page.$(".lanes")) {
  throw new Error("lifting a Run should replace the board");
}
if (!(await session.page.$(".side"))) {
  throw new Error("the global shell should keep the current Host sidebar while a Run is focused");
}
await session.page.waitForSelector(".lifted-run .issue-detail .detail-hd:has-text('active work')");
await session.page.waitForSelector('.lifted-run [data-document-state="ready"]');
await session.page.waitForSelector(".lifted-terminal .xterm-viewport");
const focusedRunGlobalActions = await session.page.$$eval(
  "[data-global-actions] [data-global-action]",
  (nodes) => nodes.map((node) => node.getAttribute("data-global-action")),
);
const expectedFocusedRunActions = ["right-rail", "changes", "appearance", "settings", "more"];
if (focusedRunGlobalActions.join("|") !== expectedFocusedRunActions.join("|")) {
  throw new Error(`focused Run global action order is wrong: ${JSON.stringify(focusedRunGlobalActions)}`);
}
const terminalPalettes = [];
for (const appearancePreference of ["light", "dark", "warm"]) {
  await session.page.click("button[data-act='appearance-menu']");
  await session.page.click(`.appearance-menu button[data-act='appearance'][data-id='${appearancePreference}']`);
  terminalPalettes.push(await session.page.evaluate(() => {
    const root = getComputedStyle(document.documentElement);
    const container = document.querySelector(".lifted-terminal .pty-slot");
    const viewport = document.querySelector(".lifted-terminal .xterm-viewport");
    return {
      tokens: ["--terminal-canvas", "--terminal-surface", "--terminal-text", "--terminal-text-muted", "--terminal-cursor", "--terminal-selection"]
        .map((name) => root.getPropertyValue(name).trim()),
      containerBackground: container ? getComputedStyle(container).backgroundColor : "",
      viewportBackground: viewport ? getComputedStyle(viewport).backgroundColor : "",
    };
  }));
}
const expectedTerminalTokens = ["#171717", "#1f1f1f", "#f5f5f5", "#a3a3a3", "#f5f5f5", "#314766"];
for (const palette of terminalPalettes) {
  if (JSON.stringify(palette.tokens) !== JSON.stringify(expectedTerminalTokens)) {
    throw new Error(`terminal tokens must stay fixed across shell themes: ${JSON.stringify(terminalPalettes)}`);
  }
  if (palette.containerBackground !== "rgb(23, 23, 23)" || palette.viewportBackground !== "rgb(23, 23, 23)") {
    throw new Error(`terminal container and xterm must share the fixed canvas: ${JSON.stringify(terminalPalettes)}`);
  }
}
const liftedDocument = await session.page.$eval(".lifted-run .issue-markdown", (node) => node.textContent?.replace(/\s+/g, " ").trim());
if (!liftedDocument?.includes("Active Run Question") || !liftedDocument.includes("same complete Issue")) {
  throw new Error(`entering a Run should retain the complete Issue document, got ${liftedDocument}`);
}
await session.capture("issue-98-existing-run-1440x900.png");
await assertShellRegionsDoNotOverlap(session.page);
const telemetryCapsules = await session.page.$$(".lifted-terminal .telemetry-desktop .capsule");
if (!telemetryCapsules.length) {
  throw new Error("desktop Run should expose per-model telemetry capsules");
}
await telemetryCapsules[0].click();
await session.page.waitForSelector(".lifted-terminal .telemetry-cards .telemetry-card");
if (!(await session.page.$(".lifted-terminal .telemetry-meta")) || !(await session.page.$(".lifted-terminal .telemetry-cards .tiny"))) {
  throw new Error("expanded telemetry should expose timing/rate details and the observation-only disclaimer");
}
const liftedWidths = await session.page.evaluate(() => {
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
if (Math.abs(liftedWidths.detail - 320) > 2 || liftedWidths.terminal <= liftedWidths.detail) {
  throw new Error(`focused Run should keep the bounded right rail and a larger terminal main area: ${JSON.stringify(liftedWidths)}`);
}
if (liftedWidths.gap !== "0px" || liftedWidths.padding !== "0px") {
  throw new Error(`lifted Run and Issue should share one continuous workspace seam, got ${JSON.stringify(liftedWidths)}`);
}
if (liftedWidths.horizontalOverflow > 0) {
  throw new Error(`lifted Run should not create page-level horizontal scrolling: ${liftedWidths.horizontalOverflow}px`);
}
await session.page.click("button[data-act='return-page']");
await session.page.waitForSelector(".lanes");
await session.page.waitForSelector(".side");
if (!(await session.page.$(".run-dock"))) {
  throw new Error("returning to the board should restore the active Issue terminal dock");
}
await session.clickCard(session.page.locator(".issue-card:has-text('child ready') .issue-card-main"));
await session.page.waitForSelector(".detail-hd:has-text('child ready')");
if (await session.page.$(".run-dock")) {
  throw new Error("selecting an Issue without an active Run should remove the terminal dock");
}

const issueToggleLeftBeforeSidebarFold = await session.page.$eval("button[data-act='toggle-issue']", (node) =>
  node.getBoundingClientRect().left,
);
await session.page.click("button[data-act='toggle-sidebar']");
if (await session.page.$(".side")) {
  throw new Error("the sidebar toggle should remove the sidebar from layout");
}
const issueToggleLeftAfterSidebarFold = await session.page.$eval("button[data-act='toggle-issue']", (node) =>
  node.getBoundingClientRect().left,
);
if (Math.abs(issueToggleLeftAfterSidebarFold - issueToggleLeftBeforeSidebarFold) > 1) {
  throw new Error(`Issue detail toggle should keep its chrome coordinate when the sidebar folds: ${issueToggleLeftBeforeSidebarFold} -> ${issueToggleLeftAfterSidebarFold}`);
}
await session.clickCard(session.page.locator('[data-lane="inProgress"] .issue-card:has-text("active work") .issue-card-main'));
await session.page.waitForSelector(".lifted-run");
await session.page.click("button[data-act='return-page']");
await session.page.waitForSelector(".lanes");
if (await session.page.$(".side")) {
  throw new Error("returning should preserve a sidebar that was already collapsed");
}
await session.page.click("button[data-act='toggle-sidebar']");
await session.page.waitForSelector(".side");

await session.page.click("button[data-act='open-usage']");
await session.page.waitForSelector(".usage-page");
if (await session.page.$(".lanes")) {
  throw new Error("usage page should replace the board, not sit as a tab or overlay");
}
const usageTitle = await session.page.$eval(".usage-page h1", (node) => node.textContent ?? "");
if (usageTitle !== "用量" && usageTitle !== "Usage") {
  throw new Error(`usage page title, got ${usageTitle}`);
}
if (!(await session.page.$("button.active[data-act='usage-range'][data-id='today']"))) {
  throw new Error("usage range should default to today");
}
if ((await session.page.$$(".usage-trend-block")).length !== 2 || !(await session.page.$(".usage-page > .tiny"))) {
  throw new Error("desktop usage should expose TTFT/rate trends and the observation-only disclaimer");
}
await session.page.click("button[data-act='return-page']");
await session.page.waitForSelector(".lanes");

const afterGraphFrontier = await session.page.$$eval('[data-lane="frontier"] .issue-card .issue-title', (nodes) =>
  nodes.map((node) => node.textContent),
);
if (!afterGraphFrontier.includes("unparented ready")) {
  throw new Error("returning to the board should keep the unfiltered Frontier");
}

const inProgress = session.page.locator('[data-lane="inProgress"] .issue-card:has-text("active work")');
for (const action of ["focus-run", "stop-run"]) {
  if (!(await inProgress.locator(`button[data-act="${action}"]`).count())) {
    throw new Error(`in-progress row should expose ${action}`);
  }
}
if (await inProgress.locator('button[data-act="view-changes"]').count()) {
  throw new Error("the change-panel toggle must exist only in the global top bar");
}

if (await session.page.$(".board-shell > .issue-detail")) {
  await session.page.click('.chrome button[data-act="toggle-issue"]');
  await session.page.waitForFunction(() => !document.querySelector(".board-shell > .issue-detail"));
}
const recentOpen = session.page.locator('[data-lane="recentlyCompleted"] button[data-act="open-issue"]').first();
await recentOpen.click();
const openedRecentUrl = await session.page.evaluate(() => window.__OPENED_URLS__.at(-1));
if (!openedRecentUrl?.startsWith("https://github.com/you/garden/issues/")) {
  throw new Error(`recently completed should open its GitHub Issue, got ${openedRecentUrl}`);
}
if (await session.page.locator('[data-lane="recentlyCompleted"] button[data-act="view-changes"]').count()) {
  throw new Error("recently completed rows must not duplicate the global change-panel toggle");
}

const frontierRun = session.page.locator('[data-lane="frontier"] button[data-act="execute-run"]').first();
if (!(await frontierRun.count())) {
  throw new Error("Frontier row should expose Run");
}

const newLabel = await session.page.$eval("button[data-act='new-run']", (node) => node.getAttribute("aria-label"));
if (newLabel !== "新建" && newLabel !== "New") {
  throw new Error(`project row plus should be New, got ${newLabel}`);
}
await session.page.click("button[data-act='register']");
await session.page.waitForSelector("form[data-form='project']");
if (!(await session.page.$("button[data-act='choose-project-directory']"))) {
  throw new Error("local Host registration should keep a folder-picker action next to the path field");
}
if (await session.page.$("button[data-act='infer']")) {
  throw new Error("registration should infer automatically instead of asking for a manual infer click");
}
await session.page.click(".overlay.modal[data-act='close-form']", { position: { x: 2, y: 2 } });
if (!(await session.page.$("form[data-form='project']"))) {
  throw new Error("clicking outside the registration sheet should keep the form open");
}
await session.page.click("button[data-act='choose-project-directory']");
await session.page.waitForSelector("form[data-form='project'] .notice.bad");
const pickerNotice = await session.page.$eval("form[data-form='project'] .notice.bad", (node) => node.textContent?.trim());
if (!pickerNotice?.includes("系统目录选择只在本机桌面窗口可用")) {
  throw new Error(`browser Client should explain the desktop-only folder picker, got ${pickerNotice}`);
}
await session.page.click("form[data-form='project'] button[data-act='close-form']");
await session.page.waitForFunction(() => !document.querySelector("form[data-form='project']"));
await session.page.click("button[data-act='new-run']");
await session.page.waitForSelector(".launch-sheet");
const pick = session.page.locator("button[data-act='select-agent']:not([disabled])").first();
if (await pick.count()) {
  await pick.click();
  await session.page.click("button[data-act='next-agent']");
  await session.page.waitForSelector("textarea[data-field='openingText']");
}
await session.page.click(".launch-sheet button[data-act='intent'][data-id='modify']");
const openingText = session.page.locator("textarea[data-field='openingText']");
await openingText.fill("");
await openingText.pressSequentially("e2e unbound run");
const customIntent = await session.page.$eval(".launch-sheet button[data-act='intent-custom']", (node) => ({
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
await session.page.click(".launch-sheet button[type='submit']");
await session.page.waitForSelector(".run-dock");
await session.page.waitForFunction(() => !document.querySelector(".launch-sheet"));
const dockText = await session.page.$eval(".run-dock", (node) => node.textContent.replace(/\s+/g, " ").trim());
if (!dockText.includes("Grok Build") || (!dockText.includes("未绑定 Issue") && !dockText.includes("Unbound Issue"))) {
  throw new Error(`unbound Run dock missing identity, got ${dockText}`);
}
if (!(await session.page.$(".pty-slot"))) {
  throw new Error("Embedded Terminal slot missing");
}
await session.page.click(".xterm-helper-textarea");
await session.page.keyboard.press("?");
if (await session.page.$(".keyboard-help")) {
  throw new Error("terminal focus should keep ? in the official TUI");
}
await session.page.click(".run-dock button[data-act='stop-run']");
await session.page.waitForFunction(() => !document.querySelector(".run-dock"));

await session.page.click("button[data-act='settings']");
await session.page.waitForSelector(".settings-page #recent-limit");
const browserUpdateText = await session.page.$eval(".update-settings", (node) => node.textContent?.replace(/\s+/g, " ").trim());
if (!browserUpdateText?.includes("浏览器 Client 不能给 Host 换包")) {
  throw new Error(`browser Client should not expose update installation: ${browserUpdateText}`);
}
if (await session.page.$("button[data-act='check-updates']") || await session.page.$("button[data-act='install-update']")) {
  throw new Error("browser Client must not expose updater actions");
}
const initialPreviewSetting = await session.page.$eval("input[data-field='commandPreview']", (node) => node.checked);
if (!initialPreviewSetting) {
  throw new Error("command preview should be enabled by default");
}
await session.page.locator("input[data-field='commandPreview']").uncheck();
await session.page.waitForFunction(async (protocol) => {
  const response = await fetch(`${protocol}/rpc`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ op: "snapshot" }),
  });
  return (await response.json()).snapshot.showCommandPreview === false;
}, session.url);
const browserStartupText = await session.page.$eval(".startup-settings", (node) => node.textContent?.replace(/\s+/g, " ").trim());
if (!browserStartupText?.includes("只能在桌面应用中修改")) {
  throw new Error(`browser Client should explain the desktop startup boundary: ${browserStartupText}`);
}
if (await session.page.$("button[data-act='host-mode']") || await session.page.$("input[data-field='startAtLogin']")) {
  throw new Error("browser Client must not expose Host mode or system autostart controls");
}
await session.page.click("button[data-act='refresh-launch-environment']");
await session.page.waitForSelector('[data-launch-environment-status="ready"]');
const launchEnvironmentText = await session.page.$eval("[data-launch-environment-status]", (node) => node.textContent?.replace(/\s+/g, " ").trim());
if (!launchEnvironmentText?.includes("启动环境已更新")) {
  throw new Error(`launch environment refresh should report success: ${launchEnvironmentText}`);
}
await session.page.fill("#recent-limit", "1");
await session.page.locator("#recent-limit").dispatchEvent("change");
await session.page.click("button[data-act='return-page']");
await session.page.waitForFunction(() => document.querySelectorAll('[data-lane="recentlyCompleted"] .issue-card').length === 1);
}
