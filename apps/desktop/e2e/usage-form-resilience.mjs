import { openIssue100Browser } from "./issue-100-harness.mjs";

const { browser, page } = await openIssue100Browser();
await page.waitForSelector(".lanes");
await page.click("button[data-act='open-usage']");
await page.waitForSelector(".usage-page");
await page.click("button[data-act='usage-range'][data-id='custom']");
await page.waitForSelector("form[data-act='usage-custom']");

let releaseFailure;
let markStarted;
let requests = 0;
let failed = false;
let forceTickRender = false;
const failureGate = new Promise((resolve) => { releaseFailure = resolve; });
const started = new Promise((resolve) => { markStarted = resolve; });
await page.route("**/rpc", async (route) => {
  const request = route.request().postDataJSON();
  if (request?.op === "tick" && forceTickRender) {
    const response = await route.fetch();
    const result = await response.json();
    result.snapshot.notifySound = !result.snapshot.notifySound;
    forceTickRender = false;
    await route.fulfill({ response, json: result });
    return;
  }
  if (
    request?.op !== "setUsageRange"
    || request?.range !== "custom"
    || typeof request?.fromMs !== "number"
    || typeof request?.toMs !== "number"
  ) {
    await route.continue();
    return;
  }
  requests += 1;
  markStarted();
  if (failed) {
    await route.continue();
    return;
  }
  await failureGate;
  failed = true;
  await route.fulfill({
    status: 503,
    contentType: "application/json",
    body: JSON.stringify({ message: "usage range temporarily unavailable" }),
  });
});

await page.focus("form[data-act='usage-custom'] input[name='from']");
forceTickRender = true;
const changedTick = page.waitForResponse((response) =>
  response.url().endsWith("/rpc") && response.request().postData()?.includes('"op":"tick"'),
);
await page.evaluate(() => window.__RUN_INTERVAL_CALLBACKS__());
await changedTick;
await page.waitForTimeout(50);
const activeUsageField = await page.evaluate(() => document.activeElement?.getAttribute("name"));
if (activeUsageField !== "from") {
  throw new Error(`business snapshot redraw must preserve the active Usage field: ${activeUsageField}`);
}

await page.$eval("form[data-act='usage-custom']", (form) => {
  form.requestSubmit();
  form.requestSubmit();
});
await Promise.race([
  started,
  new Promise((_, reject) => setTimeout(() => reject(new Error("custom Usage submit needs a stable request identity")), 2000)),
]);
await page.waitForFunction(() =>
  document.querySelector("form[data-act='usage-custom'] button[type='submit']")?.matches(":disabled") === true,
);
releaseFailure();
await page.waitForSelector(
  ".usage-page .form-feedback:has-text('usage range temporarily unavailable')",
  { timeout: 3000 },
);
const customDraft = await page.$$eval(
  "form[data-act='usage-custom'] input",
  (inputs) => inputs.map((input) => input.value),
);
if (requests !== 1 || customDraft.some((value) => !value)) {
  throw new Error(`custom Usage failure must dedupe and preserve both dates: ${JSON.stringify({ requests, customDraft })}`);
}

await page.click("form[data-act='usage-custom'] button[type='submit']");
await page.waitForFunction(() => !document.querySelector(".usage-page .form-feedback"));
if (requests !== 2) throw new Error(`custom Usage retry should issue one new request: ${requests}`);

await page.unroute("**/rpc");
let queryRequests = 0;
let releaseQuery;
const queryGate = new Promise((resolve) => { releaseQuery = resolve; });
await page.route("**/rpc", async (route) => {
  const request = route.request().postDataJSON();
  if (request?.op !== "setUsageFilter") return route.continue();
  queryRequests += 1;
  if (queryRequests > 1) return route.continue();
  await queryGate;
  await route.fulfill({
    status: 503,
    contentType: "application/json",
    body: JSON.stringify({ message: "usage filter temporarily unavailable" }),
  });
});
const projectId = await page.$eval('[data-usage-filter="projectId"]', (select) => select.options[1].value);
await page.focus('[data-usage-filter="projectId"]');
await page.selectOption('[data-usage-filter="projectId"]', projectId);
await page.waitForFunction(() => document.querySelector('[data-usage-filter="projectId"]')?.disabled, null, { timeout: 3000 });
await page.$eval('[data-usage-filter="projectId"]', (select) => select.dispatchEvent(new Event("change", { bubbles: true })));
if (queryRequests !== 1) throw new Error("pending Usage filter requests must dedupe");
releaseQuery();
await page.waitForSelector(".usage-page .form-feedback:has-text('usage filter temporarily unavailable')");
if (await page.inputValue('[data-usage-filter="projectId"]') !== "") {
  throw new Error("a failed Usage filter must retain the last applied filter, not claim the failed query succeeded");
}
await page.click('button[data-act="retry-usage-query"]');
await page.waitForFunction((id) => document.querySelector('[data-usage-filter="projectId"]')?.value === id, projectId);
if (queryRequests !== 2 || await page.$(".usage-page .form-feedback")) {
  throw new Error("retry must apply the failed Usage filter exactly once and clear its error");
}
const filterFocus = await page.evaluate(() => document.activeElement?.id);
if (filterFocus !== "usage-project") throw new Error(`Usage query retry must return keyboard focus to its filter: ${filterFocus}`);
await page.keyboard.press("Tab");
if (await page.evaluate(() => document.activeElement?.id) !== "usage-agent") {
  throw new Error("successful Usage queries must allow continued keyboard filtering");
}

await page.unroute("**/rpc");
let rangeRequests = 0;
await page.route("**/rpc", async (route) => {
  if (route.request().postDataJSON()?.op !== "setUsageRange") return route.continue();
  rangeRequests += 1;
  if (rangeRequests > 1) return route.continue();
  await route.fulfill({
    status: 503,
    contentType: "application/json",
    body: JSON.stringify({ message: "usage period temporarily unavailable" }),
  });
});
await page.click('button[data-act="usage-range"][data-id="7-days"]');
await page.waitForSelector(".usage-page .form-feedback:has-text('usage period temporarily unavailable')");
if (!await page.$('button.active[data-act="usage-range"][data-id="custom"]')) {
  throw new Error("a failed Usage range must keep the last successful range selected");
}
await page.click('button[data-act="retry-usage-query"]');
await page.waitForSelector('button.active[data-act="usage-range"][data-id="7-days"]');
if (rangeRequests !== 2 || await page.$(".usage-page .form-feedback")) {
  throw new Error("retry must apply the captured Usage range once and clear its error");
}
if (await page.evaluate(() => document.activeElement?.getAttribute("data-id")) !== "7-days") {
  throw new Error("Usage period retry must return focus to the requested range");
}

await page.click('button[data-act="usage-range"][data-id="custom"]');
await page.waitForSelector(".usage-custom");
await page.unroute("**/rpc");
let releaseCustomQuery;
let oldRangeRequests = 0;
let conflictingFilters = 0;
const customQueryGate = new Promise((resolve) => { releaseCustomQuery = resolve; });
await page.route("**/rpc", async (route) => {
  const request = route.request().postDataJSON();
  if (request?.op === "setUsageFilter") conflictingFilters += 1;
  if (request?.op === "setUsageRange" && request.range === "custom" && typeof request.fromMs === "number") {
    await customQueryGate;
    return route.continue();
  }
  if (request?.op !== "setUsageRange" || request.range !== "7-days") return route.continue();
  oldRangeRequests += 1;
  if (oldRangeRequests > 1) return route.continue();
  await route.fulfill({
    status: 503,
    contentType: "application/json",
    body: JSON.stringify({ message: "old usage range unavailable" }),
  });
});
await page.click('button[data-act="usage-range"][data-id="7-days"]');
await page.waitForSelector(".form-feedback:has-text('old usage range unavailable')");
await page.fill(".usage-custom [name=from]", "2000-01-01T00:00");
await page.fill(".usage-custom [name=to]", "2000-01-02T00:00");
await page.click(".usage-custom button[type=submit]");
await page.waitForFunction(() => document.querySelector(".usage-custom")?.getAttribute("aria-busy") === "true");
if (!await page.locator('[data-act="retry-usage-query"]').isDisabled()) {
  throw new Error("old Usage retry must be disabled while a new custom range is pending");
}
await page.$eval('[data-act="retry-usage-query"]', (button) => button.dispatchEvent(new MouseEvent("click", { bubbles: true })));
await page.$eval('[data-usage-filter="projectId"]', (select) => select.dispatchEvent(new Event("change", { bubbles: true })));
releaseCustomQuery();
await page.waitForFunction(() => document.querySelector(".usage-custom")?.getAttribute("aria-busy") === "false");
await page.waitForTimeout(100);
if (
  oldRangeRequests !== 1 || conflictingFilters !== 0
  || await page.$(".usage-page .form-feedback")
  || await page.$('[data-act="retry-usage-query"]')
  || !await page.$('button.active[data-act="usage-range"][data-id="custom"]')
  || await page.inputValue(".usage-custom [name=from]") !== "2000-01-01T00:00"
) {
  throw new Error("a successful custom query must supersede its old error/retry and reject conflicting requests");
}

await browser.close();
