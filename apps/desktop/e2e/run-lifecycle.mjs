import { hostSnapshot, openIssue100Browser } from "./issue-100-harness.mjs";

const { browser, capture, page, url } = await openIssue100Browser();
await page.waitForSelector(".lanes");

const card = (title) => page.locator(".issue-card", { hasText: title }).first();
const closeInspectorIfOpen = async () => {
  const close = page.locator('.chrome button[data-act="toggle-issue"]');
  if (!(await close.count())) return;
  await close.click();
  await page.waitForFunction(() => !document.querySelector(".board-shell > .issue-detail"));
};

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

let failure = failFirstRpc(
  "searchIssues",
  "search temporarily unavailable",
  () => true,
);
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
const initialRailSections = await page.$$eval(".workspace-rail-section", (sections) =>
  Object.fromEntries(sections.map((section) => [section.dataset.workspaceSection, section.open])),
);
if (!initialRailSections.runs || initialRailSections.issue || initialRailSections.actions) {
  throw new Error(`an active Run should default to Run history only: ${JSON.stringify(initialRailSections)}`);
}
await page.click('.workspace-rail-section[data-workspace-section="issue"] > summary');
await page.waitForSelector(".lifted-run .issue-markdown:has-text('Keep the complete Issue beside the Terminal')");
const liftedText = (await page.locator(".lifted-run").textContent())?.replace(/\s+/g, " ") ?? "";
if (!liftedText.includes("等待操作") || !liftedText.includes("active lifecycle issue")) {
  throw new Error(`active Run must retain Issue identity and waiting state: ${liftedText}`);
}
await capture("issue-100-terminal-and-issue-1280x840.png");

const activeRunId = await page.$eval(".lifted-terminal .pty-slot", (node) => node.dataset.run);
await page.evaluate(() => { window.__FOCUS_TERMINAL_HOST__ = document.querySelector(".lifted-terminal .pty-host"); });
await page.click(".lifted-terminal .pty-host");
await page.keyboard.type("resume after approval");
await page.keyboard.press("Enter");
await page.waitForFunction(async ({ protocol, runId }) => {
  const response = await fetch(`${protocol}/runs/${encodeURIComponent(runId)}/output?after=0`);
  if (!response.ok) return false;
  const json = await response.json();
  const bytes = Uint8Array.from(atob(json.data), (byte) => byte.charCodeAt(0));
  return new TextDecoder().decode(bytes).includes("resume after approval");
}, { protocol: url, runId: activeRunId });

await page.click(".chrome button[data-act='view-changes']");
await page.waitForSelector(".changes-sheet .change-file h4:has-text('notes.txt')");
const terminalWhileChangesOpen = await page.evaluate(() => ({
  sameHost: window.__FOCUS_TERMINAL_HOST__ === document.querySelector(".lifted-terminal .pty-host"),
  visible: Boolean(document.querySelector(".focus-workspace-main")?.getClientRects().length),
}));
if (!terminalWhileChangesOpen.sameHost || !terminalWhileChangesOpen.visible) {
  throw new Error(`opening changes must keep the live Terminal mounted: ${JSON.stringify(terminalWhileChangesOpen)}`);
}
await page.click(".lifted-terminal .pty-host");
await page.keyboard.type("continuous while changes");
await page.keyboard.press("Enter");
await page.waitForFunction(async ({ protocol, runId }) => {
  const response = await fetch(`${protocol}/runs/${encodeURIComponent(runId)}/output?after=0`);
  if (!response.ok) return false;
  const json = await response.json();
  const bytes = Uint8Array.from(atob(json.data), (byte) => byte.charCodeAt(0));
  return new TextDecoder().decode(bytes).includes("continuous while changes");
}, { protocol: url, runId: activeRunId });
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
await page.waitForSelector('.workspace-rail-section[data-workspace-section="runs"]');
const restoredWorkspaceState = await page.evaluate(() => ({
  sameHost: window.__FOCUS_TERMINAL_HOST__ === document.querySelector(".lifted-terminal .pty-host"),
  sections: Object.fromEntries([...document.querySelectorAll(".workspace-rail-section")].map((section) => [section.dataset.workspaceSection, section.open])),
}));
if (!restoredWorkspaceState.sameHost || !restoredWorkspaceState.sections.runs || !restoredWorkspaceState.sections.issue) {
  throw new Error(`closing changes must restore the Terminal and rail expansion state: ${JSON.stringify(restoredWorkspaceState)}`);
}

await page.click(".lifted-terminal button[data-act='open-usage-run']");
await page.waitForSelector(".usage-page");
await page.click("button[data-act='usage-range'][data-id='custom']");
await page.waitForSelector("form[data-act='usage-custom']");
failure = failFirstRpc(
  "setUsageRange",
  "usage range temporarily unavailable",
  (request) =>
    request.range === "custom"
    && typeof request.fromMs === "number"
    && typeof request.toMs === "number",
);
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
await page.click("button[data-act='return-page']");
await page.waitForSelector(".lifted-terminal");
await page.click(".lifted-terminal button[data-act='stop-run']");
await page.click("[data-dialog-id='stop-run'] button[data-act='confirm-stop-run']");
await page.click("button[data-act='return-page']");
try {
  await page.waitForSelector(".lanes", { timeout: 2000 });
} catch {
  const diagnostic = await page.evaluate(() => ({
    clientView: Object.entries(localStorage).filter(([key]) => key.includes("client-view")),
    workspace: document.querySelector(".workspace")?.className,
    lifted: Boolean(document.querySelector(".lifted-run")),
    overview: Boolean(document.querySelector(".overview-page")),
    usage: Boolean(document.querySelector(".usage-page")),
    empty: document.querySelector(".board-empty")?.textContent,
  }));
  throw new Error(`returning from a stopped Run did not restore the board: ${JSON.stringify(diagnostic)}`);
}

await closeInspectorIfOpen();
let endedOutputRequests = 0;
const countEndedOutput = (request) => {
  if (request.url().includes("/runs/") && request.url().includes("/output")) endedOutputRequests += 1;
};
page.on("request", countEndedOutput);
await card("continue lifecycle issue").locator(".issue-card-main").click();
await page.waitForSelector('[data-terminal-surface="readonly"]');
await new Promise((resolve) => setTimeout(resolve, 200));
page.off("request", countEndedOutput);
if (endedOutputRequests !== 0 || await page.$('[data-terminal-surface="readonly"] .pty-slot') || await page.$('[data-terminal-surface="readonly"] input')) {
  throw new Error(`an ended Run must use recentOutput without PTY reads or input: ${endedOutputRequests}`);
}
const endedDefaults = await page.$$eval(".workspace-rail-section", (sections) =>
  Object.fromEntries(sections.map((section) => [section.dataset.workspaceSection, section.open])),
);
if (!endedDefaults.issue || endedDefaults.runs || endedDefaults.actions) {
  throw new Error(`an Issue without an active Run should default to its body: ${JSON.stringify(endedDefaults)}`);
}
await page.click(".chrome button[data-act='view-changes']");
await page.waitForSelector(".changes-sheet .notice.bad");
const missingIsolationChanges = await page.$eval(".changes-sheet .notice.bad", (node) => node.textContent ?? "");
if (!missingIsolationChanges.includes("隔离执行目录") || missingIsolationChanges.includes("Project 主目录")) {
  throw new Error(`missing isolation changes must stay unavailable instead of falling back: ${missingIsolationChanges}`);
}
await page.click("button[data-act='close-changes']");
await page.click('.workspace-rail-section[data-workspace-section="actions"] > summary');
await page.click(".issue-detail button[data-act='continue-run']");
await page.waitForSelector(".lifted-terminal .pty-slot");
const continued = await hostSnapshot(page, url);
const continuedRun = continued.runs.find((run) => run.issueId === "you/lifecycle#2" && run.status === "running");
if (!continuedRun?.previousRunId) {
  throw new Error(`Continue must link a new Run to the stopped Run: ${JSON.stringify(continued.runs)}`);
}
const continueText = (await page.locator(".lifted-terminal").textContent())?.replace(/\s+/g, " ") ?? "";
if (!continueText.includes("隔离执行目录已经不在") || !continueText.includes("Project 主目录")) {
  throw new Error(`missing isolated work directory must fall back with a recovery explanation: ${continueText}`);
}
await page.click(".lifted-terminal button[data-act='stop-run']");
await page.click("[data-dialog-id='stop-run'] button[data-act='confirm-stop-run']");
await page.click("button[data-act='return-page']");
await page.waitForSelector(".lanes");

await closeInspectorIfOpen();
await card("release lifecycle issue").locator(".issue-card-main").click();
await page.waitForSelector('[data-terminal-surface="readonly"]');
await page.click('.workspace-rail-section[data-workspace-section="actions"] > summary');
await page.click(".issue-detail button[data-act='release-claim']");
await page.click("button[data-act='return-page']");
await page.waitForSelector('[data-lane="frontier"] .issue-card:has-text("release lifecycle issue")');
if (await page.locator('[data-lane="recentlyCompleted"] .issue-card:has-text("release lifecycle issue")').count()) {
  throw new Error("ending or releasing a Run must not pretend that the Issue is complete");
}
if (!(await page.locator('[data-lane="inProgress"] .issue-card:has-text("continue lifecycle issue")').count())) {
  throw new Error("stopping a Run must leave the still-claimed open Issue in progress");
}
await card("never run lifecycle issue").locator(".issue-card-main").click();
await page.waitForSelector('[data-terminal-surface="empty"] button[data-act="execute-run"]');
if (await page.$(".pty-slot") || await page.$('[data-global-action="changes"]')) {
  throw new Error("an Issue that never ran must show only the Terminal empty state and its launch entry");
}
await capture("issue-100-run-ended-issue-open-1280x840.png");

await browser.close();
console.log("run lifecycle e2e ok");
