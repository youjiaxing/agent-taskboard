import { assertShellRegionsDoNotOverlap } from "./board-harness.mjs";

/** The mobile Client is one full-screen task at a time: a drawer, a two-item view bar, and a fixed Run input. */
export async function runMobileBoard(session) {
const snapshot = async () =>
  session.page.evaluate(async (protocol) => {
    const response = await fetch(`${protocol}/rpc`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ op: "snapshot" }),
    });
    return (await response.json()).snapshot;
  }, session.url);

await session.page.waitForSelector(".mobile-nav");
await session.page.waitForSelector("[data-current-identity]");

const chromeShape = await session.page.evaluate(() => ({
  actions: [...document.querySelectorAll(".chrome button")].map((node) => node.dataset.act),
  trail: document.querySelectorAll("[data-global-actions] [data-global-action]").length,
  title: document.querySelector("[data-current-identity]")?.textContent?.trim() ?? "",
}));
if (chromeShape.actions.join("|") !== "mobile-drawer") {
  throw new Error(`mobile top bar should only carry the drawer trigger beside back and the title, got ${JSON.stringify(chromeShape.actions)}`);
}
if (chromeShape.trail !== 0) {
  throw new Error(`mobile must not render the desktop global actions, got ${chromeShape.trail}`);
}
if (!chromeShape.title) throw new Error("mobile top bar should show the current object title");

for (const selector of [
  ".side",
  ".fixed-right-rail",
  ".fixed-changes-panel",
  ".more-menu",
  ".appearance-menu",
  ".view-switch",
  ".issue-search",
  "button[data-act='view-changes']",
  "button[data-act='toggle-issue']",
  "button[data-act='new-issue']",
  "button[data-act='view-dependencies']",
]) {
  if (await session.page.$(selector)) {
    throw new Error(`mobile should not render ${selector} at all`);
  }
}

const mobileDensity = await session.page.evaluate(() => {
  const style = getComputedStyle(document.documentElement);
  return {
    bodyFontSize: style.getPropertyValue("--density-body-font-size").trim(),
    controlHeight: style.getPropertyValue("--density-control-height").trim(),
    topbarHeight: style.getPropertyValue("--density-topbar-height").trim(),
    gutter: style.getPropertyValue("--density-gutter-inline").trim(),
    panelGap: style.getPropertyValue("--density-panel-gap").trim(),
  };
});
const expectedDensity = {
  bodyFontSize: "14px",
  controlHeight: "44px",
  topbarHeight: "48px",
  gutter: "16px",
  panelGap: "0",
};
if (JSON.stringify(mobileDensity) !== JSON.stringify(expectedDensity)) {
  throw new Error(`mobile must only apply the agreed density aliases: ${JSON.stringify(mobileDensity)}`);
}
const mobileCss = await session.page.evaluate(() => {
  const sheets = [...document.styleSheets];
  const mediaRules = sheets.flatMap((sheet) => {
    try {
      return [...sheet.cssRules];
    } catch {
      return [];
    }
  }).filter((rule) => rule.media);
  const conditions = mediaRules.map((rule) => rule.conditionText ?? String(rule.media?.mediaText ?? ""));
  const mobileRules = mediaRules
    .filter((rule) => (rule.conditionText ?? rule.media?.mediaText ?? "").includes("639.98px"))
    .flatMap((rule) => [...rule.cssRules]);
  return {
    debug: { sheetCount: sheets.length, conditions, mobileRuleCount: mobileRules.length, selectors: mobileRules.slice(0, 3).map((rule) => rule.selectorText) },
    bottomGap: mobileRules
      .filter((rule) => rule.selectorText === ":root")
      .map((rule) => rule.style.getPropertyValue("--density-bottom-gap").trim())
      .join(""),
    safeAreaUsers: mobileRules
      .filter((rule) => /mobile-nav|mobile-input-row/.test(rule.selectorText ?? "") && rule.cssText.includes("--density-bottom-gap"))
      .map((rule) => rule.selectorText),
  };
});
if (mobileCss.bottomGap !== "env(safe-area-inset-bottom)") {
  throw new Error(`mobile must resolve the bottom gap from the device safe area, got ${JSON.stringify(mobileCss)}`);
}
if (!mobileCss.safeAreaUsers.some((selector) => selector.includes(".mobile-nav")) || !mobileCss.safeAreaUsers.some((selector) => selector.includes(".mobile-input-row"))) {
  throw new Error(`mobile bottom areas must apply the safe-area token: ${JSON.stringify(mobileCss.safeAreaUsers)}`);
}

const assertTouchTargets = async (label) => {
  const small = await session.page.evaluate(() => {
    const selector = "button, select, summary, [role='button'], label.ui-checkbox, input:not([type='checkbox']):not([type='hidden'])";
    return [...document.querySelectorAll(selector)]
      .filter((node) => {
        const rect = node.getBoundingClientRect();
        const style = getComputedStyle(node);
        if (style.display === "none" || style.visibility === "hidden" || rect.width === 0 || rect.height === 0) return false;
        if (node.closest(".xterm") || node.closest(".issue-markdown")) return false;
        const wide = node.tagName === "INPUT" || node.tagName === "SELECT";
        return rect.height < 43.5 || (!wide && rect.width < 43.5);
      })
      .map((node) => `${node.dataset.act ?? node.className}:${Math.round(node.getBoundingClientRect().width)}x${Math.round(node.getBoundingClientRect().height)}`);
  });
  if (small.length) throw new Error(`${label} touch targets under 44px: ${JSON.stringify(small)}`);
};

const laneOrder = await session.page.$$eval(".lane", (nodes) => nodes.map((node) => node.dataset.lane));
if (laneOrder.join("|") !== "inProgress|frontier|blocked|recentlyCompleted") {
  throw new Error(`mobile board should keep all four lanes, worked-on first: ${JSON.stringify(laneOrder)}`);
}
const boardShape = await session.page.evaluate(() => {
  const refresh = document.querySelector(".mobile-board-view .mobile-board-status .refresh-bar");
  const lanes = document.querySelector(".mobile-board-view .lanes");
  return {
    hasRefresh: Boolean(refresh),
    hasLanes: Boolean(lanes),
    refreshPrecedesLanes: Boolean(refresh && lanes && (refresh.compareDocumentPosition(lanes) & Node.DOCUMENT_POSITION_FOLLOWING)),
  };
});
if (!boardShape.hasRefresh || !boardShape.hasLanes || !boardShape.refreshPrecedesLanes) {
  throw new Error(`mobile refresh status should precede the work lanes: ${JSON.stringify(boardShape)}`);
}

const navShape = await session.page.$$eval(".mobile-nav button", (nodes) =>
  nodes.map((node) => ({ act: node.dataset.act, id: node.dataset.id, label: node.textContent.trim(), pressed: node.getAttribute("aria-pressed") })),
);
if (navShape.map((item) => item.label).join("|") !== "看板|专注工作区") {
  throw new Error(`mobile bottom navigation should switch primary views only, got ${JSON.stringify(navShape)}`);
}
if (navShape.some((item) => item.act !== "mobile-nav") || navShape[0].pressed !== "true") {
  throw new Error(`mobile bottom navigation should only carry view switches: ${JSON.stringify(navShape)}`);
}

await session.page.click("button[data-act='mobile-drawer']");
await session.page.waitForSelector("[data-dialog-id='mobile-drawer']");
const drawerActions = await session.page.$$eval("[data-dialog-id='mobile-drawer'] [data-act]", (nodes) => nodes.map((node) => node.dataset.act));
for (const act of [
  "focus-host",
  "focus-project",
  "edit-project",
  "remove-project",
  "mobile-issue-entry",
  "mobile-history-entry",
  "mobile-search-entry",
  "mobile-appearance-entry",
  "mobile-settings-entry",
  "register",
  "pair",
  "open-overview",
  "open-usage",
]) {
  if (!drawerActions.includes(act)) throw new Error(`mobile drawer should expose ${act}, got ${JSON.stringify(drawerActions)}`);
}
const selectedCardBefore = Boolean(await session.page.$(".mobile-board-view .issue-card.sel"));
const issueEntryDisabled = await session.page.$eval("[data-dialog-id='mobile-drawer'] [data-act='mobile-issue-entry']", (node) => node.disabled);
if (issueEntryDisabled === selectedCardBefore) {
  throw new Error(`the drawer's Issue entry follows whether an Issue is selected: selected=${selectedCardBefore} disabled=${issueEntryDisabled}`);
}

await session.page.click("[data-dialog-id='mobile-drawer'] [data-act='mobile-appearance-entry']");
const appearanceChoices = await session.page.$$eval("[data-dialog-id='mobile-drawer'] [data-act='appearance']", (nodes) => nodes.map((node) => node.dataset.id));
if (appearanceChoices.join("|") !== "warm|light|dark|system") {
  throw new Error(`mobile appearance should offer the four agreed preferences, got ${JSON.stringify(appearanceChoices)}`);
}
await session.page.click("[data-dialog-id='mobile-drawer'] [data-act='appearance'][data-id='dark']");
if ((await session.page.getAttribute("html", "data-theme")) !== "dark") {
  throw new Error("the mobile browser should resolve its own manual dark preference");
}
const storedMobileAppearance = await session.page.evaluate(() => localStorage.getItem("agent-taskboard-browser-appearance"));
if (!storedMobileAppearance?.includes('"appearancePreference":"dark"')) {
  throw new Error(`mobile appearance should persist in this browser Client, got ${storedMobileAppearance}`);
}
const hostAppearance = (await snapshot()).appearance;
if (hostAppearance.language !== "zh-CN" || hostAppearance.appearancePreference !== "system") {
  throw new Error(`mobile appearance must not overwrite desktop-client settings, got ${JSON.stringify(hostAppearance)}`);
}
if ((await session.page.evaluate(() => (typeof Notification === "undefined" ? "unavailable" : Notification.permission))) === "granted") {
  throw new Error("mobile browser must not request lock-screen notification permission");
}

await session.page.click("[data-dialog-id='mobile-drawer'] [data-act='mobile-search-entry']");
await session.page.waitForSelector("[data-dialog-id='mobile-search'] form[data-act='issue-search']");
await session.page.fill("#issue-title-search", "child ready");
await session.page.click("[data-dialog-id='mobile-search'] button[type='submit']");
// A successful search hands the result back to the board it filters, so the overlay closes itself.
await session.page.waitForFunction(() => !document.querySelector("[data-dialog-id='mobile-search']"));
await session.page.waitForFunction((protocol) =>
  fetch(`${protocol}/rpc`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ op: "snapshot" }),
  })
    .then((response) => response.json())
    .then((json) => json.snapshot.board.search.title === "child ready"), session.url);
await session.page.waitForFunction(() => document.querySelectorAll(".mobile-board-view .issue-card").length === 1);

await session.page.click("button[data-act='mobile-drawer']");
await session.page.waitForSelector("[data-dialog-id='mobile-drawer']");
await session.page.click("[data-dialog-id='mobile-drawer'] [data-act='mobile-search-entry']");
await session.page.waitForSelector("[data-dialog-id='mobile-search'] form[data-act='issue-search']");
if (await session.page.inputValue("#issue-title-search") !== "child ready") {
  throw new Error("the reopened mobile search should mirror the filter it already applied");
}
await session.page.fill("#issue-title-search", "");
await session.page.click("[data-dialog-id='mobile-search'] button[type='submit']");
await session.page.waitForFunction(() => !document.querySelector("[data-dialog-id='mobile-search']"));
await session.page.waitForFunction(() => document.querySelectorAll(".mobile-board-view .issue-card").length > 1);

const boardScrollBefore = await session.page.$eval(".workspace", (node) => {
  node.scrollTop = node.scrollHeight;
  return node.scrollTop;
});
if (boardScrollBefore <= 0) throw new Error("mobile board navigation regression needs a scrollable board page");

await session.page.click("button[data-act='mobile-drawer']");
await session.page.waitForSelector("[data-dialog-id='mobile-drawer']");
await session.page.click("[data-dialog-id='mobile-drawer'] [data-act='mobile-settings-entry']");
await session.page.waitForSelector(".settings-page");
for (const forbidden of ["notifyDesktop", "notifySound", "hostAutoAdvance"]) {
  if (await session.page.$(`[data-field='${forbidden}']`)) throw new Error(`mobile settings should not expose ${forbidden}`);
}
if (await session.page.$("button[data-act='quit']")) throw new Error("mobile settings should not expose Host quit");
await assertTouchTargets("mobile settings");
await session.page.click("button[data-act='return-page']");
await session.page.waitForSelector(".mobile-board-view");
const boardScrollAfterSettings = await session.page.$eval(".workspace", (node) => node.scrollTop);
if (Math.abs(boardScrollAfterSettings - boardScrollBefore) > 1) {
  throw new Error(`returning from mobile settings must preserve the board scroll: ${boardScrollBefore} -> ${boardScrollAfterSettings}`);
}

await session.page.click("button[data-act='mobile-drawer']");
await session.page.waitForSelector("[data-dialog-id='mobile-drawer']");
await session.page.click("[data-dialog-id='mobile-drawer'] [data-act='open-overview']");
await session.page.waitForSelector(".overview-page");
await session.page.click("button[data-act='return-page']");
await session.page.waitForSelector(".mobile-board-view");

const mobileFrontierCard = session.page.locator('[data-lane="frontier"] .issue-card:has-text("child ready") .issue-card-main');
const mobileFrontierIssueId = await mobileFrontierCard.getAttribute("data-issue-id");
await mobileFrontierCard.click();
await session.page.waitForSelector(".mobile-workspace-view");
if (!(await session.page.$("button[data-act='return-page']"))) {
  throw new Error("entering the mobile workspace should offer the back action");
}
if (await session.page.$(".mobile-nav [data-id='focus-workspace']:not([aria-pressed='true'])")) {
  throw new Error("the mobile bottom navigation should follow the current page");
}
// Selecting an Issue lands on the Issue section, the mobile peer of the desktop rail it replaces.
await session.page.waitForSelector(".mobile-issue-panel [data-document-state='ready']");
const mobileDocument = await session.page.$eval(".mobile-issue-panel .issue-markdown", (node) => node.textContent?.replace(/\s+/g, " ").trim());
if (!mobileDocument?.includes("Can the operator read every constraint") || !mobileDocument.includes("Paragraph six")) {
  throw new Error(`390px Issue view should expose the complete document, got ${mobileDocument}`);
}
const issueSectionActions = await session.page.$$eval(".mobile-issue-panel .detail-meta [data-act]", (nodes) => nodes.map((node) => node.dataset.act));
for (const act of ["execute-run", "edit-issue", "toggle-issue-open", "open-issue"]) {
  if (!issueSectionActions.includes(act)) throw new Error(`mobile Issue panel should keep the ${act} action, got ${JSON.stringify(issueSectionActions)}`);
}
if (issueSectionActions.includes("view-dependencies")) {
  throw new Error("mobile has no dependency-graph page and must not offer its entry");
}
const issueGeometry = await session.page.evaluate(() => {
  const panel = document.querySelector(".mobile-issue-panel")?.getBoundingClientRect();
  const actions = [...document.querySelectorAll(".mobile-issue-panel .detail-meta button")]
    .filter((node) => getComputedStyle(node).display !== "none")
    .map((node) => node.getBoundingClientRect());
  return {
    pageOverflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
    panelLeft: panel?.left ?? -1,
    panelRight: panel?.right ?? 9999,
    actionBounds: actions.map((rect) => [rect.left, rect.right]),
  };
});
if (issueGeometry.pageOverflow > 0 || issueGeometry.panelLeft < 0 || issueGeometry.panelRight > 390) {
  throw new Error(`390px Issue panel should stay inside the viewport: ${JSON.stringify(issueGeometry)}`);
}
if (issueGeometry.actionBounds.some(([left, right]) => left < 0 || right > 390)) {
  throw new Error(`390px primary Issue actions should not clip: ${JSON.stringify(issueGeometry.actionBounds)}`);
}
await assertTouchTargets("mobile Issue panel");

await session.page.click(".mobile-section-switch button[data-id='terminal']");
await session.page.waitForSelector('[data-terminal-surface="empty"]');
if (!(await session.page.$('[data-terminal-surface="empty"] button[data-act="execute-run"]'))) {
  throw new Error("a never-run Issue should keep its start-Run entry in the mobile workspace");
}
await assertTouchTargets("mobile workspace");

await session.page.click(".mobile-section-switch button[data-id='runs']");
await session.page.waitForSelector(".mobile-workspace-panel > p.muted");
const emptyHistory = await session.page.$eval(".mobile-workspace-panel", (node) => node.textContent?.trim());
if (emptyHistory !== "还没有运行记录") {
  throw new Error(`a never-run Issue should explain its empty Run history, got ${emptyHistory}`);
}

await session.page.click(".mobile-section-switch button[data-id='terminal']");
await session.page.click('[data-terminal-surface="empty"] button[data-act="execute-run"]');
await session.page.waitForSelector(".launch-sheet");
const mobileAgentPick = session.page.locator("button[data-act='pick-agent']:not([disabled])").first();
if (await mobileAgentPick.count()) {
  await mobileAgentPick.click();
  await session.page.waitForSelector("form[data-form='launch']");
}
await session.page.click("form[data-form='launch'] button[type='submit']");
await session.page.waitForFunction(() => !document.querySelector(".launch-sheet"));
const startedRun = (await snapshot()).runs.find((run) => run.issueId === mobileFrontierIssueId && run.status !== "ended");
if (!startedRun) throw new Error("mobile should start a Frontier Run through the normal launch form");

await session.page.waitForSelector("[data-mobile-run-input]");
await assertTouchTargets("mobile running workspace");
const inputGeometry = await session.page.evaluate(() => {
  const row = document.querySelector(".mobile-input-row")?.getBoundingClientRect();
  const field = document.querySelector(".mobile-input-row .inject-row")?.getBoundingClientRect();
  const panel = document.querySelector(".mobile-workspace-panel")?.getBoundingClientRect();
  const nav = document.querySelector(".mobile-nav")?.getBoundingClientRect();
  return {
    rowTop: row?.top ?? 0,
    rowBottom: row?.bottom ?? 0,
    fieldLeft: field?.left ?? 0,
    fieldRight: field?.right ?? 0,
    panelBottom: panel?.bottom ?? 0,
    navTop: nav?.top ?? 0,
    navBottom: nav?.bottom ?? 0,
    viewport: window.innerHeight,
  };
});
if (inputGeometry.panelBottom > inputGeometry.navTop + 1 || inputGeometry.navBottom > inputGeometry.rowTop + 1) {
  throw new Error(`the Run input is the last bottom row, under the navigation and below the panel: ${JSON.stringify(inputGeometry)}`);
}
if (inputGeometry.rowBottom > inputGeometry.viewport + 1) {
  throw new Error(`the fixed Run input must stay inside the page: ${JSON.stringify(inputGeometry)}`);
}
if (inputGeometry.fieldLeft < 15.5 || 390 - inputGeometry.fieldRight < 15.5) {
  throw new Error(`the Run input row must keep the mobile page gutter: ${JSON.stringify(inputGeometry)}`);
}
await session.page.fill(".mobile-input-row input[name='text']", "mobile answer");
await session.page.click(".mobile-input-row button[type='submit']");
await session.page.waitForFunction(() => document.querySelector(".mobile-input-row input")?.value === "");
await session.page.waitForFunction(async ({ protocol, runId }) => {
  const response = await fetch(`${protocol}/runs/${encodeURIComponent(runId)}/output?after=0`);
  if (!response.ok) return false;
  const json = await response.json();
  const output = new TextDecoder().decode(Uint8Array.from(atob(json.data), (byte) => byte.charCodeAt(0)));
  return output.includes("mobile answer");
}, { protocol: session.url, runId: startedRun.id });

await session.page.click(".mobile-workspace-view button[data-act='stop-run']");
await session.page.click("[data-dialog-id='stop-run'] button[data-act='confirm-stop-run']");
await session.page.waitForSelector('[data-terminal-surface="readonly"]');
if (await session.page.$("[data-mobile-run-input]")) {
  throw new Error("an ended Run must not keep the mobile input area");
}
const endedStop = await session.page.$eval("[data-terminal-surface='readonly'] button[data-act='stop-run']", (node) => node.disabled);
if (!endedStop) throw new Error("an ended Run should disable the stop action");
const endedOutput = await session.page.$eval(".readonly-terminal-output", (node) => node.textContent ?? "");
if (!endedOutput.includes("mobile answer")) {
  throw new Error(`mobile should keep the ended Run's recent output readable, got ${endedOutput}`);
}

await session.page.click(".mobile-section-switch button[data-id='runs']");
await session.page.waitForSelector(".mobile-workspace-panel .workspace-run-history-item");
const endedRun = await session.page.$eval(".mobile-workspace-panel .workspace-run-history-item", (node) => node.textContent?.replace(/\s+/g, " ").trim());
if (!endedRun?.includes("已结束")) {
  throw new Error(`the mobile Run history should read the ended Run state, got ${endedRun}`);
}
await assertShellRegionsDoNotOverlap(session.page);
await assertTouchTargets("mobile workspace");

await session.page.click(".mobile-nav button[data-id='board']");
await session.page.waitForSelector(".mobile-board-view");
const keptIssueCard = await session.page.$eval(".mobile-board-view .issue-card.sel", (node) => node.getAttribute("data-issue-id"));
if (keptIssueCard !== mobileFrontierIssueId) {
  throw new Error(`switching views must keep the current Issue, got ${keptIssueCard}`);
}
const navBoardScroll = await session.page.$eval(".workspace", (node) => {
  node.scrollTop = node.scrollHeight;
  return node.scrollTop;
});
if (navBoardScroll <= 0) throw new Error("the mobile board round trip needs a scrollable board page");
await session.page.click(".mobile-nav button[data-id='focus-workspace']");
await session.page.waitForSelector(".mobile-workspace-view");
if (!(await session.page.$(".mobile-section-switch button[data-id='runs'][aria-pressed='true']"))) {
  throw new Error("the mobile workspace should keep the section the user left it on");
}
const endedRunChrome = await session.page.evaluate(() => ({
  inputRows: document.querySelectorAll(".mobile-input-row").length,
  navItems: document.querySelectorAll(".mobile-nav button").length,
}));
if (endedRunChrome.inputRows !== 0 || endedRunChrome.navItems !== 2) {
  throw new Error(`an ended Run should keep the navigation but no input row: ${JSON.stringify(endedRunChrome)}`);
}
await session.page.click(".mobile-nav button[data-id='board']");
await session.page.waitForSelector(".mobile-board-view");
const navBoardScrollAfter = await session.page.$eval(".workspace", (node) => node.scrollTop);
if (Math.abs(navBoardScrollAfter - navBoardScroll) > 1) {
  throw new Error(`the bottom navigation must preserve the board scroll: ${navBoardScroll} -> ${navBoardScrollAfter}`);
}

await session.page.click("button[data-act='mobile-drawer']");
await session.page.waitForSelector("[data-dialog-id='mobile-drawer']");
for (const act of ["mobile-issue-entry", "mobile-history-entry"]) {
  if (await session.page.$eval(`[data-dialog-id='mobile-drawer'] [data-act='${act}']`, (node) => node.disabled)) {
    throw new Error(`the drawer's ${act} should be usable once an Issue is current`);
  }
}
await assertTouchTargets("mobile drawer");
await session.page.click("[data-dialog-id='mobile-drawer'] [data-act='mobile-history-entry']");
await session.page.waitForSelector(".mobile-workspace-view");
if (!(await session.page.$(".mobile-section-switch button[data-id='runs'][aria-pressed='true']"))) {
  throw new Error("the drawer's history entry should open the Run history section");
}
await session.page.click(".mobile-nav button[data-id='board']");
await session.page.waitForSelector(".mobile-board-view");
await assertTouchTargets("mobile board");

}
