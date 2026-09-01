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
const started = await hostSnapshot(page, url);
const startedRun = started.runs.find((run) => run.issueId === "you/disconnect#1");
const firstRunId = startedRun?.id;
if (!firstRunId) throw new Error(`the UI-created Run must be observable after launch: ${JSON.stringify(started.runs)}`);

for (let attempt = 0; attempt < 40; attempt += 1) {
  const tickResponse = page.waitForResponse((response) =>
    response.url().endsWith("/rpc") && response.request().postData()?.includes('"op":"tick"'),
  );
  await page.evaluate(() => window.__RUN_INTERVAL_CALLBACKS__());
  await tickResponse;
  const stopped = await page.evaluate(() => {
    const card = [...document.querySelectorAll('[data-lane="inProgress"] .issue-card')]
      .find((node) => node.textContent?.includes("PTY disconnect issue"));
    return card?.classList.contains("execution-stopped") === true;
  });
  if (stopped) break;
  if (attempt === 39) throw new Error("timed out waiting for disconnected PTY to become execution-stopped");
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

await page.click('.issue-detail button[data-act="continue-run"]');
await page.waitForFunction((previousRunId) => {
  const currentRunId = document.querySelector(".pty-slot")?.dataset.run;
  return Boolean(currentRunId && currentRunId !== previousRunId);
}, firstRunId);
const resumedRunId = await page.$eval(".pty-slot", (node) => node.dataset.run);
const resumed = await hostSnapshot(page, url);
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

await page.click('.run-dock button[data-act="stop-run"]');

await browser.close();
console.log("pty disconnect e2e ok");
