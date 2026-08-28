import { hostSnapshot, openIssue100Browser } from "./issue-100-harness.mjs";

const { browser, capture, page, url } = await openIssue100Browser();
await page.waitForSelector(".lanes");

const card = (title) => page.locator(".issue-card", { hasText: title }).first();

let rpcFailure = null;
await page.route("**/rpc", async (route) => {
  let request;
  try {
    request = route.request().postDataJSON();
  } catch {
    await route.continue();
    return;
  }
  if (!rpcFailure || request?.op !== rpcFailure.op || !rpcFailure.matches(request)) {
    await route.continue();
    return;
  }
  rpcFailure.requests += 1;
  if (rpcFailure.failed) {
    await route.continue();
    return;
  }
  await rpcFailure.gate;
  rpcFailure.failed = true;
  await route.fulfill({
    status: 503,
    contentType: "application/json",
    body: JSON.stringify({ message: rpcFailure.message }),
  });
});

const failFirstRpc = (op, message, matches = () => true) => {
  let release;
  const gate = new Promise((resolve) => { release = resolve; });
  rpcFailure = { failed: false, gate, matches, message, op, release, requests: 0 };
  return rpcFailure;
};

let failure = failFirstRpc("searchIssues", "search temporarily unavailable");
await page.fill("#issue-title-search", "active lifecycle issue");
const tickResponse = page.waitForResponse((response) =>
  response.url().endsWith("/rpc") && response.request().postData()?.includes('"op":"tick"'),
);
await page.evaluate(() => window.__RUN_INTERVAL_CALLBACKS__());
await tickResponse;
await page.waitForFunction(() => document.querySelector("#issue-title-search")?.value === "active lifecycle issue");
if ((await page.inputValue("#issue-title-search")) !== "active lifecycle issue") {
  throw new Error("a Host tick must not erase an in-progress search draft");
}
await page.$eval("form[data-act='issue-search']", (form) => {
  form.requestSubmit();
  form.requestSubmit();
});
await page.waitForFunction(() => document.querySelector("form[data-act='issue-search'] button[type='submit']")?.matches(":disabled") === true);
failure.release();
await page.waitForSelector(".form-feedback:has-text('search temporarily unavailable')");
if (failure.requests !== 1 || (await page.inputValue("#issue-title-search")) !== "active lifecycle issue") {
  throw new Error(`search failure must dedupe and preserve the draft: ${JSON.stringify(failure)}`);
}
await page.click("form[data-act='issue-search'] button[type='submit']");
await page.waitForSelector('.issue-card:has-text("active lifecycle issue")');
if (failure.requests !== 2) throw new Error(`search retry should issue one new request: ${failure.requests}`);
rpcFailure = null;
await page.fill("#issue-title-search", "");
await page.press("#issue-title-search", "Enter");
await page.waitForFunction(() => document.querySelectorAll(".issue-card").length >= 3);

await card("active lifecycle issue").locator("button[data-act='focus-run']").click();
await page.waitForSelector(".lifted-run .pty-slot");
await page.waitForSelector(".lifted-run .issue-markdown:has-text('Keep the complete Issue beside the Terminal')");
const liftedText = (await page.locator(".lifted-run").textContent())?.replace(/\s+/g, " ") ?? "";
if (!liftedText.includes("等待操作") || !liftedText.includes("active lifecycle issue")) {
  throw new Error(`active Run must retain Issue identity and waiting state: ${liftedText}`);
}
await capture("issue-100-terminal-and-issue-1280x840.png");

failure = failFirstRpc("injectRunInput", "terminal input temporarily unavailable");
await page.fill(".lifted-terminal .inject-row input", "resume after approval");
await page.$eval(".lifted-terminal .inject-row", (form) => {
  form.requestSubmit();
  form.requestSubmit();
});
await page.waitForFunction(() => document.querySelector(".lifted-terminal .inject-row button[type='submit']")?.matches(":disabled") === true);
failure.release();
await page.waitForSelector(".lifted-terminal .form-feedback:has-text('terminal input temporarily unavailable')");
if (failure.requests !== 1 || (await page.inputValue(".lifted-terminal .inject-row input")) !== "resume after approval") {
  throw new Error(`Terminal injection failure must dedupe and preserve the draft: ${JSON.stringify(failure)}`);
}
await page.click(".lifted-terminal .inject-row button[type='submit']");
if (failure.requests !== 2) throw new Error(`Terminal injection retry should issue one new request: ${failure.requests}`);
rpcFailure = null;
const activeRunId = await page.$eval(".lifted-terminal .pty-slot", (node) => node.dataset.run);
await page.waitForFunction(async ({ protocol, runId }) => {
  const response = await fetch(`${protocol}/runs/${encodeURIComponent(runId)}/output?after=0`);
  if (!response.ok) return false;
  const json = await response.json();
  const bytes = Uint8Array.from(atob(json.data), (byte) => byte.charCodeAt(0));
  return new TextDecoder().decode(bytes).includes("resume after approval");
}, { protocol: url, runId: activeRunId });

await page.click(".lifted-terminal button[data-act='view-changes']");
await page.waitForSelector(".changes-sheet .change-file h4:has-text('notes.txt')");
const diffText = (await page.locator(".changes-sheet").textContent())?.replace(/\s+/g, " ") ?? "";
if (!diffText.includes("changed during the Run")) {
  throw new Error(`view changes must show the live working-tree diff: ${diffText}`);
}
await page.locator(".changes-sheet .diff-line.add[data-act='note-line']").first().click();
await page.fill(".changes-sheet .note-form input", "check this line");
failure = failFirstRpc("writeChangeNote", "note storage temporarily unavailable");
await page.$eval(".changes-sheet .note-form", (form) => {
  form.requestSubmit();
  form.requestSubmit();
});
await page.waitForFunction(() => document.querySelector(".changes-sheet .note-form button[type='submit']")?.matches(":disabled") === true);
failure.release();
await page.waitForSelector(".changes-sheet .form-feedback:has-text('note storage temporarily unavailable')");
if (failure.requests !== 1 || (await page.inputValue(".changes-sheet .note-form input")) !== "check this line") {
  throw new Error(`change-note failure must dedupe and preserve the draft: ${JSON.stringify(failure)}`);
}
await page.click(".changes-sheet .note-form button[type='submit']");
await page.waitForSelector(".changes-sheet .change-note:has-text('check this line')");
if (failure.requests !== 2) throw new Error(`change-note retry should issue one new request: ${failure.requests}`);
rpcFailure = null;
await capture("issue-100-view-changes-1280x840.png");
await page.click("button[data-act='close-changes']");

await page.click(".lifted-terminal button[data-act='open-usage-run']");
await page.waitForSelector(".usage-page");
await page.click("button[data-act='usage-range'][data-id='custom']");
await page.waitForSelector("form[data-act='usage-custom']");
failure = failFirstRpc("setUsageRange", "usage range temporarily unavailable", (request) => request.range === "custom" && "fromMs" in request);
await page.$eval("form[data-act='usage-custom']", (form) => {
  form.requestSubmit();
  form.requestSubmit();
});
await page.waitForFunction(() => document.querySelector("form[data-act='usage-custom'] button[type='submit']")?.matches(":disabled") === true);
failure.release();
await page.waitForSelector(".usage-page .form-feedback:has-text('usage range temporarily unavailable')");
const customDraft = await page.$$eval("form[data-act='usage-custom'] input", (inputs) => inputs.map((input) => input.value));
if (failure.requests !== 1 || customDraft.some((value) => !value)) {
  throw new Error(`custom usage failure must dedupe and preserve both dates: ${JSON.stringify({ failure, customDraft })}`);
}
await page.click("form[data-act='usage-custom'] button[type='submit']");
await page.waitForFunction(() => !document.querySelector(".usage-page .form-feedback"));
if (failure.requests !== 2) throw new Error(`custom usage retry should issue one new request: ${failure.requests}`);
rpcFailure = null;
await page.click("button[data-act='close-usage']");
await page.waitForSelector(".lifted-terminal");
await page.click(".lifted-terminal button[data-act='stop-run']");
await page.click("button[data-act='return-board']");
await page.waitForSelector(".lanes");

await card("continue lifecycle issue").locator(".issue-card-main").click();
await page.waitForSelector(".detail-hd:has-text('continue lifecycle issue')");
await page.click(".issue-detail button[data-act='continue-run']");
await page.waitForSelector(".run-dock .pty-slot");
const continued = await hostSnapshot(page, url);
const continuedRun = continued.runs.find((run) => run.issueId === "you/lifecycle#2" && run.status === "running");
if (!continuedRun?.previousRunId) {
  throw new Error(`Continue must link a new Run to the stopped Run: ${JSON.stringify(continued.runs)}`);
}
const continueText = (await page.locator(".run-dock").textContent())?.replace(/\s+/g, " ") ?? "";
if (!continueText.includes("隔离执行目录已经不在") || !continueText.includes("Project 主目录")) {
  throw new Error(`missing isolated work directory must fall back with a recovery explanation: ${continueText}`);
}
await page.click(".run-dock button[data-act='stop-run']");

await card("release lifecycle issue").locator(".issue-card-main").click();
await page.waitForSelector(".detail-hd:has-text('release lifecycle issue')");
await page.click(".issue-detail button[data-act='release-claim']");
await page.waitForSelector('[data-lane="frontier"] .issue-card:has-text("release lifecycle issue")');
if (await page.locator('[data-lane="recentlyCompleted"] .issue-card:has-text("release lifecycle issue")').count()) {
  throw new Error("ending or releasing a Run must not pretend that the Issue is complete");
}
if (!(await page.locator('[data-lane="inProgress"] .issue-card:has-text("continue lifecycle issue")').count())) {
  throw new Error("stopping a Run must leave the still-claimed open Issue in progress");
}
await capture("issue-100-run-ended-issue-open-1280x840.png");

await browser.close();
console.log("run lifecycle e2e ok");
