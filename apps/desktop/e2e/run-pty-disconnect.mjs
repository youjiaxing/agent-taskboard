import { hostSnapshot, openIssue100Browser } from "./issue-100-harness.mjs";

const { browser, page, url } = await openIssue100Browser({ screenshotDir: "" });
await page.waitForSelector('[data-lane="frontier"] .issue-card:has-text("PTY disconnect issue")');

const beforeLaunch = await hostSnapshot(page, url);
const issueBeforeLaunch = beforeLaunch.board.columns.frontier.find(
  (candidate) => candidate.id === "you/disconnect#1",
);
if (!issueBeforeLaunch || issueBeforeLaunch.claimedBy.length !== 0 || beforeLaunch.runs.length !== 0) {
  throw new Error(`the visible execution entry must start without a claim or Run: ${JSON.stringify({ issueBeforeLaunch, runs: beforeLaunch.runs })}`);
}

let releasePtyRead = () => {};
const ptyReadGate = new Promise((resolve) => {
  releasePtyRead = resolve;
});
await page.route("**/runs/*/output*", async (route) => {
  await ptyReadGate;
  await route.continue();
});

const issue = page.locator('[data-lane="frontier"] .issue-card', { hasText: "PTY disconnect issue" }).first();
await issue.locator('button[data-act="execute-run"]').click();
await page.waitForSelector(".launch-sheet");

const pick = page.locator("button[data-act='select-agent']:not([disabled])").first();
if (await pick.count()) {
  await pick.click();
  await page.click("button[data-act='next-agent']");
  await page.waitForSelector("form[data-form='launch']");
}
await page.locator("form[data-form='launch'] button[type='submit']").click();
await page.waitForFunction(() => !document.querySelector(".launch-sheet"));
await page.waitForSelector(".side .run-row[data-run]");
const firstRunId = await page.locator(".side .run-row[data-run]").last().getAttribute("data-run");
if (!firstRunId) throw new Error("the UI-created Run must be observable in the sidebar");
releasePtyRead();

// CI runners can briefly starve the Host tick loop while Chromium is painting
// the shell. Keep the semantic assertion the same, but allow a bounded 30s
// window for the disconnected PTY to surface as execution-stopped.
for (let attempt = 0; attempt < 60; attempt += 1) {
  const tickResponse = page.waitForResponse((response) =>
    response.url().endsWith("/rpc") && response.request().postData()?.includes('"op":"tick"'),
  );
  await page.evaluate(() => window.__RUN_INTERVAL_CALLBACKS__());
  await tickResponse;
  // The HTTP response can arrive before the Client applies and paints it.
  // Wait for the visible result instead of issuing a raw snapshot that can consume Host events.
  const stopped = await page.locator('[data-lane="inProgress"] .issue-card.execution-stopped', {
    hasText: "PTY disconnect issue",
  }).waitFor({ state: "visible", timeout: 500 }).then(() => true, () => false);
  if (stopped) break;
  if (attempt === 59) throw new Error("timed out waiting for disconnected PTY to become execution-stopped");
}
const card = page.locator('[data-lane="inProgress"] .issue-card', { hasText: "PTY disconnect issue" }).first();
const cardText = (await card.textContent())?.replace(/\s+/g, " ") ?? "";
if (!cardText.includes("执行已停")) {
  throw new Error(`a disconnected PTY must be visible as execution-stopped: ${cardText}`);
}

const stopped = await hostSnapshot(page, url);
const stoppedRun = stopped.runs.find((run) => run.id === firstRunId);
const stoppedIssue = Object.values(stopped.board.columns)
  .flat()
  .find((candidate) => candidate.id === "you/disconnect#1");
if (
  stopped.runs.length !== 1
  || stoppedRun?.status !== "ended"
  || stoppedRun.endedReason !== "abnormal"
  || !stoppedIssue?.claimedBy.length
) {
  throw new Error(`PTY disconnect must end the original Run abnormally while retaining the claim: ${JSON.stringify({ stoppedRun, stoppedIssue, runs: stopped.runs })}`);
}

await page.waitForSelector(".issue-detail .detail-hd:has-text('PTY disconnect issue')");
await page.waitForSelector('.issue-detail button[data-act="continue-run"]');
await page.waitForSelector('.issue-detail button[data-act="release-claim"]');
const detailText = (await page.locator(".issue-detail").textContent())?.replace(/\s+/g, " ") ?? "";
if (!detailText.includes("执行已停")) {
  throw new Error(`the Issue inspector must explain the recoverable stopped state: ${detailText}`);
}

await page.locator('.issue-detail button[data-act="continue-run"]').click();
await page.waitForFunction(() => document.querySelectorAll(".side .run-row[data-run]").length >= 2);
const resumed = await hostSnapshot(page, url);
const resumedRunId = resumed.runs.find(
  (run) => run.issueId === "you/disconnect#1" && run.status === "running",
)?.id;
if (!resumedRunId) throw new Error(`Continue should create a new active Run: ${JSON.stringify(resumed.runs)}`);
await page.click(`.side .run-row[data-run="${resumedRunId}"] .run-main`);
await page.waitForSelector(`.lifted-terminal[data-terminal-surface="live"][data-run="${resumedRunId}"]`);
const resumedRuns = resumed.runs.filter(
  (run) => run.issueId === "you/disconnect#1" && run.status === "running",
);
const resumedRun = resumed.runs.find((run) => run.id === resumedRunId);
const originalRun = resumed.runs.find((run) => run.id === firstRunId);
const resumedIssue = Object.values(resumed.board.columns)
  .flat()
  .find((candidate) => candidate.id === "you/disconnect#1");
if (
  resumed.runs.length !== 2
  || resumedRuns.length !== 1
  || resumedRun?.previousRunId !== firstRunId
  || originalRun?.status !== "ended"
  || originalRun.endedReason !== "abnormal"
  || !resumedIssue?.claimedBy.length
) {
  throw new Error(`Continue must replace the disconnected PTY with one linked active Run: ${JSON.stringify({ resumedRun, originalRun, resumedIssue, runs: resumed.runs })}`);
}

await page.click('.lifted-terminal button[data-act="stop-run"]');
await page.click("[data-dialog-id='stop-run'] button[data-act='confirm-stop-run']");

await browser.close();
console.log("pty disconnect e2e ok");
