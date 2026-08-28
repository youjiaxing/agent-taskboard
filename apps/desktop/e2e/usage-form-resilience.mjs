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
  if (request?.op !== "snapshot" || request?.clientAction !== "setUsageRange") {
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

await browser.close();
