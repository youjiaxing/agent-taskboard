import { hostSnapshot, openIssue100Browser } from "./issue-100-harness.mjs";

const { browser, capture, page, url } = await openIssue100Browser();
page.on("pageerror", (error) => console.error("pageerror", error));
page.on("console", (msg) => {
  if (msg.type() === "error") console.error("console", msg.text());
});
await page.waitForSelector('[data-lane="frontier"] .issue-card');

const issueId = await page.$eval('[data-lane="frontier"] .issue-card', (node) => node.dataset.issueId);
const beforeLaunch = await hostSnapshot(page, url);
const issueBeforeLaunch = beforeLaunch.board.columns.frontier.find((issue) => issue.id === issueId);
if (!issueBeforeLaunch || issueBeforeLaunch.claimedBy.length !== 0 || beforeLaunch.runs.length !== 0) {
  throw new Error(`opening the form must not claim or create a Run: ${JSON.stringify({ issueBeforeLaunch, runs: beforeLaunch.runs })}`);
}

await page.click(`[data-lane="frontier"] .issue-card[data-issue-id="${issueId}"] button[data-act="execute-run"]`);
await page.waitForSelector(".launch-sheet");
const formOpened = await hostSnapshot(page, url);
const issueWithFormOpen = formOpened.board.columns.frontier.find((issue) => issue.id === issueId);
if (!issueWithFormOpen || issueWithFormOpen.claimedBy.length !== 0 || formOpened.runs.length !== 0) {
  throw new Error(`opening the launch form must not claim or create a Run: ${JSON.stringify({ issueWithFormOpen, runs: formOpened.runs })}`);
}
const pick = page.locator("button[data-act='pick-agent']:not([disabled])").first();
if (await pick.count()) {
  await pick.click();
  await page.waitForSelector("form[data-form='launch']");
}
const opening = page.locator("textarea[data-field='openingText']");
await opening.fill("preserve this launch draft");

let releaseFirstLaunch;
const firstLaunchGate = new Promise((resolve) => {
  releaseFirstLaunch = resolve;
});
let launchRequests = 0;
await page.route("**/rpc", async (route) => {
  let request;
  try {
    request = route.request().postDataJSON();
  } catch {
    await route.continue();
    return;
  }
  if (request?.op !== "startUnboundRun") {
    await route.continue();
    return;
  }
  launchRequests += 1;
  if (launchRequests === 1) await firstLaunchGate;
  await route.continue();
});

await page.$eval("form[data-form='launch']", (form) => {
  form.requestSubmit();
  form.requestSubmit();
});
await page.waitForFunction(() => document.querySelector("form[data-form='launch'] button[type='submit']")?.matches(":disabled") === true);
releaseFirstLaunch();

await page.waitForSelector("form[data-form='launch'] .notice.bad:has-text('pty unavailable')");
await page.waitForTimeout(250);
if (launchRequests !== 1) {
  throw new Error(`double submit must result in one launch request, got ${launchRequests}`);
}
if ((await opening.inputValue()) !== "preserve this launch draft") {
  throw new Error("launch failure must preserve the opening draft");
}
await page.$eval(".launch-sheet", (node) => { node.scrollTop = node.scrollHeight; });
await capture("issue-100-launch-retry-1280x840.png");
const afterFailure = await hostSnapshot(page, url);
if (afterFailure.runs.some((run) => run.status === "running")) {
  throw new Error(`a failed launch must not become a running Run: ${JSON.stringify(afterFailure.runs)}`);
}
const issueAfterFailure = Object.values(afterFailure.board.columns)
  .flat()
  .find((issue) => issue.id === issueId);
if (!issueAfterFailure || issueAfterFailure.claimedBy.length !== 0) {
  throw new Error(`a failed launch must release its provisional claim: ${JSON.stringify(issueAfterFailure)}`);
}

await page.click("form[data-form='launch'] button[type='submit']");
await page.waitForFunction(() => !document.querySelector(".launch-sheet"));
await page.waitForSelector(".run-dock");
const afterRetry = await hostSnapshot(page, url);
const running = afterRetry.runs.filter((run) => run.issueId === issueId && run.status === "running");
const claimed = Object.values(afterRetry.board.columns)
  .flat()
  .find((issue) => issue.id === issueId);
if (running.length !== 1 || launchRequests !== 2 || !claimed?.claimedBy.length) {
  throw new Error(`explicit retry should claim the Issue and create exactly one running Run: ${JSON.stringify({ claimed, running, launchRequests })}`);
}
await capture("issue-100-bound-run-1280x840.png");

await browser.close();
console.log("run launch resilience e2e ok");
