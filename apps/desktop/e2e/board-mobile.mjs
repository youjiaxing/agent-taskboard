import { assertShellRegionsDoNotOverlap } from "./board-harness.mjs";

export async function runMobileBoard(session) {
await session.page.waitForSelector(".mobile-nav");
const mobileNavLabels = await session.page.$$eval(".mobile-nav button", (nodes) => nodes.map((node) => node.textContent?.trim()));
if (mobileNavLabels.join("|") !== "看板|票|Run") {
  throw new Error(`mobile bottom navigation should be 看板 | 票 | Run, got ${JSON.stringify(mobileNavLabels)}`);
}
if (await session.page.$(".side")) {
  throw new Error("mobile should move Host and Project lists out of the main layout");
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
if (JSON.stringify(mobileDensity) !== JSON.stringify({ bodyFontSize: "14px", controlHeight: "44px", topbarHeight: "48px", gutter: "16px", panelGap: "0" })) {
  throw new Error(`mobile must only apply the agreed density aliases: ${JSON.stringify(mobileDensity)}`);
}
const mobileProjectOrder = await session.page.evaluate(() => {
  const refresh = document.querySelector(".project-board [data-page-toolbar] .refresh-bar");
  const board = document.querySelector(".project-board > .board-shell");
  return {
    hasRefresh: Boolean(refresh),
    hasBoard: Boolean(board),
    refreshPrecedesBoard: Boolean(refresh && board && (refresh.compareDocumentPosition(board) & Node.DOCUMENT_POSITION_FOLLOWING)),
  };
});
if (!mobileProjectOrder.hasRefresh || !mobileProjectOrder.hasBoard || !mobileProjectOrder.refreshPrecedesBoard) {
  throw new Error(`mobile refresh status should precede work lanes, got ${JSON.stringify(mobileProjectOrder)}`);
}
const visibleMobileLanes = await session.page.waitForFunction(() => {
  if (!window.matchMedia("(max-width: 639.98px)").matches) return false;
  const lanes = [...document.querySelectorAll(".lane")]
    .filter((node) => getComputedStyle(node).display !== "none")
    .map((node) => node.getAttribute("data-lane"));
  return lanes.join("|") === "inProgress|frontier" ? lanes : false;
}).then((handle) => handle.jsonValue());
if (visibleMobileLanes.join("|") !== "inProgress|frontier") {
  throw new Error(`mobile board should prioritize in progress then Frontier, got ${JSON.stringify(visibleMobileLanes)}`);
}
const mobileChangesButtons = await session.page.$$eval('button[data-act="view-changes"]', (nodes) =>
  nodes.filter((node) => getComputedStyle(node).display !== "none").length,
);
if (mobileChangesButtons !== 0) {
  throw new Error(`mobile should not expose full view changes, got ${mobileChangesButtons} buttons`);
}
if (await session.page.$('button[data-act="view-changes"]')) {
  throw new Error("mobile should not render full view changes actions");
}

const mobileFrontierCard = session.page.locator('[data-lane="frontier"] .issue-card').first();
const mobileFrontierIssueId = await mobileFrontierCard.getAttribute("data-issue-id");
await mobileFrontierCard.locator('button[data-act="execute-run"]').click();
await session.page.waitForSelector(".launch-sheet");
const mobileAgentPick = session.page.locator("button[data-act='pick-agent']:not([disabled])").first();
if (await mobileAgentPick.count()) {
  await mobileAgentPick.click();
  await session.page.waitForSelector("form[data-form='launch']");
}
await session.page.click("form[data-form='launch'] button[type='submit']");
await session.page.waitForFunction(() => !document.querySelector(".launch-sheet"));
await session.page.waitForTimeout(100);
const startedIssueRun = await session.page.evaluate(async ({ protocol, issueId }) => {
  const response = await fetch(`${protocol}/rpc`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ op: "snapshot" }),
  });
  const snapshot = (await response.json()).snapshot;
  return snapshot.runs.find((run) => run.issueId === issueId) ?? { debug: snapshot.runs };
}, { protocol: session.url, issueId: mobileFrontierIssueId });
if (!startedIssueRun?.id || startedIssueRun.status !== "running") {
  throw new Error(`mobile should start a Frontier Run through the normal launch form: ${JSON.stringify(startedIssueRun)}`);
}
await session.page.click("button[data-act='mobile-run']");
await session.page.waitForSelector(".mobile-run-view");
await session.page.click(".mobile-run-view button[data-act='stop-run']");
await session.page.waitForSelector(".mobile-board-view");

await session.page.click("button[data-act='mobile-scope']");
await session.page.waitForSelector(".mobile-scope-sheet");
if (!(await session.page.$(".mobile-scope-hosts button[data-act='focus-host']"))) {
  throw new Error("mobile scope switcher should expose the Host list");
}
for (const action of ["register", "edit-project", "remove-project"]) {
  if (!(await session.page.$(`.mobile-scope-sheet [data-act="${action}"]`))) {
    throw new Error(`mobile scope switcher should expose ${action}`);
  }
}
await session.page.click(".mobile-scope-sheet button[data-act='edit-project']");
await session.page.waitForSelector("form[data-form='project']");
await session.page.click("form[data-form='project'] button[data-act='close-form']");
await session.page.click("button[data-act='mobile-scope']");
await session.page.click(".mobile-scope-sheet button[data-act='remove-project']");
await session.page.waitForSelector(".overlay[data-act='close-remove']");
await session.page.click("button[data-act='close-remove']");

const mobileBoardScrollBeforeIssue = await session.page.$eval(".workspace", (node) => {
  node.scrollTop = node.scrollHeight;
  return node.scrollTop;
});
if (mobileBoardScrollBeforeIssue <= 0) {
  throw new Error("mobile Issue navigation regression needs a scrollable board page");
}
await session.clickCard(session.page.locator(".issue-card:has-text('child ready') .issue-card-main"));
await session.page.waitForSelector(".mobile-issue-view .issue-detail");
await session.page.waitForSelector('.mobile-issue-view [data-document-state="ready"]');
const mobileDocument = await session.page.$eval(".mobile-issue-view .issue-markdown", (node) => node.textContent?.replace(/\s+/g, " ").trim());
if (!mobileDocument?.includes("Can the operator read every constraint") || !mobileDocument.includes("Paragraph six")) {
  throw new Error(`390px Issue view should expose the complete document, got ${mobileDocument}`);
}
if (await session.page.$('.mobile-issue-view button[data-act="view-changes"]')) {
  throw new Error("mobile Issue view should still omit full view changes");
}
if (await session.page.$('.mobile-issue-view button[data-act="toggle-issue"]')) {
  throw new Error("mobile Issue view should not expose a desktop panel-collapse control");
}
const mobileIssueGeometry = await session.page.evaluate(() => {
  const detail = document.querySelector(".mobile-issue-view .issue-detail")?.getBoundingClientRect();
  const actions = [...document.querySelectorAll(".mobile-issue-view .detail-meta button")]
    .filter((node) => getComputedStyle(node).display !== "none")
    .map((node) => node.getBoundingClientRect());
  return {
    pageOverflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
    detailLeft: detail?.left ?? -1,
    detailRight: detail?.right ?? 9999,
    actionBounds: actions.map((rect) => [rect.left, rect.right]),
  };
});
if (mobileIssueGeometry.pageOverflow > 0 || mobileIssueGeometry.detailLeft < 0 || mobileIssueGeometry.detailRight > 390) {
  throw new Error(`390px Issue Inspector should stay inside the viewport: ${JSON.stringify(mobileIssueGeometry)}`);
}
if (mobileIssueGeometry.actionBounds.some(([left, right]) => left < 0 || right > 390)) {
  throw new Error(`390px primary Issue actions should not clip: ${JSON.stringify(mobileIssueGeometry.actionBounds)}`);
}
await session.capture("issue-98-mobile-390x844.png");
await assertShellRegionsDoNotOverlap(session.page);
await session.page.click("button[data-act='mobile-board']");
await session.page.waitForSelector(".mobile-board-view");
const mobileBoardScrollAfterIssue = await session.page.$eval(".workspace", (node) => node.scrollTop);
if (Math.abs(mobileBoardScrollAfterIssue - mobileBoardScrollBeforeIssue) > 1) {
  throw new Error(`returning from a mobile Issue must preserve board scroll: ${mobileBoardScrollBeforeIssue} -> ${mobileBoardScrollAfterIssue}`);
}

await session.page.click('[data-lane="inProgress"] .issue-card:has-text("active work") button[data-act="focus-run"]');
await session.page.waitForSelector(".mobile-run-view");
if (await session.page.$(".mobile-run-view .pty-slot")) {
  throw new Error("mobile Run should show recent output before the live terminal escape hatch");
}
await session.page.waitForFunction(() => document.querySelector(".mobile-run-output")?.textContent?.includes("mobile recent output"));
const recentOutput = await session.page.$eval(".mobile-run-output", (node) => node.textContent ?? "");
if (!recentOutput.includes("mobile recent output")) {
  throw new Error(`mobile Run should expose recent read-only output, got ${recentOutput}`);
}
await session.page.fill(".mobile-run-view .inject-row input", "mobile answer");
await session.page.click(".mobile-run-view .inject-row button[type='submit']");
await session.page.waitForFunction(() => document.querySelector(".mobile-run-view .inject-row input")?.value === "");
const injectedRunId = await session.page.$eval(".mobile-run-output", (node) => node.getAttribute("data-run"));
await session.page.waitForFunction(async ({ protocol, runId }) => {
  const response = await fetch(`${protocol}/runs/${encodeURIComponent(runId)}/output?after=0`);
  if (!response.ok) return false;
  const json = await response.json();
  const output = new TextDecoder().decode(Uint8Array.from(atob(json.data), (byte) => byte.charCodeAt(0)));
  return output.includes("mobile answer");
}, { protocol: session.url, runId: injectedRunId });
if (!(await session.page.$(".telemetry-mobile .capsule")) || !(await session.page.$(".telemetry-mobile .telemetry-simple"))) {
  throw new Error("mobile telemetry should keep the main model capsule and simple multi-model list");
}
await session.page.click("button[data-act='mobile-live-terminal']");
await session.page.waitForSelector(".mobile-run-view .pty-slot");
const endedRunId = await session.page.$eval(".mobile-run-view .pty-slot", (node) => node.getAttribute("data-run"));
await session.page.click(".mobile-run-view button[data-act='stop-run']");
await session.page.waitForSelector(".mobile-board-view");
await session.page.click("button[data-act='mobile-run']");
await session.page.waitForSelector(".mobile-run-view");
const endedRecentOutput = await session.page.$eval(".mobile-run-output", (node) => node.textContent ?? "");
if (!endedRecentOutput.includes("mobile recent output") || !endedRecentOutput.includes("mobile answer")) {
  throw new Error(`mobile should retain recent output after a Run ends, got ${endedRecentOutput}`);
}
await session.page.click("button[data-act='mobile-board']");
await session.page.waitForSelector(".mobile-board-view");

await session.page.click("button[data-act='mobile-scope']");
if (!(await session.page.$(".mobile-scope-sheet button[data-act='open-usage']"))) {
  throw new Error("mobile usage should stay reachable from the scope switcher");
}
await session.page.click(".mobile-scope-sheet button[data-act='open-usage']");
await session.page.waitForSelector(".usage-page");
for (const selector of [".usage-ranges", ".usage-filters", ".usage-trend-block", ".usage-full"]) {
  const visible = await session.page.$eval(selector, (node) => getComputedStyle(node).display !== "none");
  if (visible) throw new Error(`mobile usage should hide ${selector}`);
}
if (!(await session.page.$(".token-row.totals"))) {
  throw new Error("mobile usage should retain current totals");
}
const compactProjects = await session.page.$$eval(".usage-compact .usage-row", (nodes) => nodes.length);
if (compactProjects < 1 || compactProjects > 3) {
  throw new Error(`mobile usage should show one to three Project rows, got ${compactProjects}`);
}
await session.page.click("button[data-act='return-page']");

await session.page.click("button[data-act='settings']");
for (const forbidden of ["notifyDesktop", "notifySound", "hostAutoAdvance"]) {
  if (await session.page.$(`[data-field="${forbidden}"]`)) {
    throw new Error(`mobile settings should not expose ${forbidden}`);
  }
}
if (await session.page.$("button[data-act='quit']")) {
  throw new Error("mobile settings should not expose Host quit");
}
await session.page.click("button[data-act='language'][data-id='en']");
await session.page.click("button[data-act='appearance'][data-id='dark']");
const storedMobileAppearance = await session.page.evaluate(() => localStorage.getItem("agent-taskboard-browser-appearance"));
if (!storedMobileAppearance?.includes('"language":"en"') || !storedMobileAppearance.includes('"appearancePreference":"dark"')) {
  throw new Error(`mobile appearance should persist in this browser Client, got ${storedMobileAppearance}`);
}
if ((await session.page.getAttribute("html", "data-theme")) !== "dark") {
  throw new Error("the mobile browser should resolve its own manual dark preference");
}
const hostAppearance = await session.page.evaluate(async (protocol) => {
  const response = await fetch(`${protocol}/rpc`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ op: "snapshot" }),
  });
  return (await response.json()).snapshot.appearance;
}, session.url);
if (hostAppearance.language !== "zh-CN" || hostAppearance.appearancePreference !== "system") {
  throw new Error(`mobile appearance must not overwrite desktop-client settings, got ${JSON.stringify(hostAppearance)}`);
}
const mobileNotificationPermission = await session.page.evaluate(() =>
  typeof Notification === "undefined" ? "unavailable" : Notification.permission,
);
if (mobileNotificationPermission === "granted") {
  throw new Error("mobile browser must not request lock-screen notification permission");
}


}
