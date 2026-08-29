import { chromium } from "playwright";

const url = process.env.BOARD_URL;
const firstProjectDir = process.env.FIRST_PROJECT_DIR;
const staleProjectDir = process.env.STALE_PROJECT_DIR;
const missingProjectDir = process.env.MISSING_PROJECT_DIR;
const retryProjectDir = process.env.RETRY_PROJECT_DIR;
if (!url || !firstProjectDir || !staleProjectDir || !missingProjectDir || !retryProjectDir) {
  console.error("missing project registration e2e environment");
  process.exit(1);
}

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ locale: "zh-CN", viewport: { width: 1280, height: 840 } });
const page = await context.newPage();
let inferenceRequests = 0;
await page.addInitScript((protocol) => {
  window.__HOST_PROTOCOL__ = protocol;
}, url);
await page.route("**/rpc", async (route) => {
  const request = route.request();
  let op = "";
  try {
    op = request.postDataJSON()?.op ?? "";
  } catch {
    // Non-JSON requests continue unchanged.
  }
  if (op === "inferProject") {
    inferenceRequests += 1;
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  await route.continue();
});
await page.goto(url, { waitUntil: "domcontentloaded" });

await page.waitForSelector(".empty button[data-act='register']");
await page.click(".empty button[data-act='register']");
await page.waitForSelector("form[data-form='project']");
await page.fill("#project-host", "manual.example.com");
await page.fill("#project-repo", "manual/kept");
await page.fill("#project-path", staleProjectDir);
await page.locator("#project-path").dispatchEvent("change");
await page.waitForSelector("form[data-form='project'] [data-inference='pending']");
if (await page.locator("form[data-form='project'] button[type='submit']").isDisabled()) {
  throw new Error("inference must not disable manual submission");
}
if (await page.locator("form[data-form='project'] button[data-act='close-form']").isDisabled()) {
  throw new Error("inference must not disable cancellation");
}
if (await page.locator("#project-path").isDisabled()) {
  throw new Error("inference must not disable changing the path");
}
await page.fill("#project-host", "");
await page.fill("#project-repo", "");
await page.fill("#project-path", firstProjectDir);
await page.locator("#project-path").dispatchEvent("change");
await page.waitForFunction(() => document.querySelector("#project-host")?.value === "github.com" && document.querySelector("#project-repo")?.value === "you/first");
if ((await page.inputValue("#project-name")) !== "first") {
  throw new Error("changing the path should replace the previous directory-derived display name");
}
if (await page.locator("[data-inference='candidate']").count()) {
  throw new Error("a unique Git remote should be adopted automatically instead of showing a confirmation candidate");
}
await page.click("form[data-form='project'] button[type='submit']");
await page.waitForSelector('[data-lane="frontier"] .issue-card:has-text("first tracker issue")');

await page.click(".project-row button[data-act='project-menu']");
await page.click(".project-menu button[data-act='edit-project']");
await page.waitForSelector("form[data-form='project']");
if ((await page.inputValue("#project-path")) !== firstProjectDir || (await page.inputValue("#project-repo")) !== "you/first") {
  throw new Error("editing an existing Project should reopen the confirmed registration values");
}
await page.fill("#project-name", "first renamed");
await page.click("form[data-form='project'] button[type='submit']");
await page.waitForSelector(".project-row:has-text('first renamed')");

await page.click("button[data-act='register']");
await page.fill("#project-name", "cancelled draft");
await page.fill("#project-repo", "manual/cancelled");
await page.fill("#project-path", staleProjectDir);
await page.locator("#project-path").dispatchEvent("change");
await page.waitForSelector("[data-inference='pending']");
await page.click("button[data-act='close-form']");
await page.waitForFunction(() => !document.querySelector("form[data-form='project']"));

await page.click("button[data-act='register']");
await page.fill("#project-name", "failed draft");
await page.fill("#project-host", "github.com");
await page.fill("#project-repo", "manual/retry");
await page.fill("#project-path", missingProjectDir);
await page.locator("#project-path").dispatchEvent("change");
await page.waitForSelector("[data-inference='failed'] button[data-act='retry-infer']");
for (const [selector, expected] of [
  ["#project-name", "failed draft"],
  ["#project-host", "github.com"],
  ["#project-repo", "manual/retry"],
  ["#project-path", missingProjectDir],
]) {
  const actual = await page.inputValue(selector);
  if (actual !== expected) throw new Error(`inference failure lost ${selector}: ${actual}`);
}
const requestsBeforeRetry = inferenceRequests;
await page.click("button[data-act='retry-infer']");
for (let attempt = 0; inferenceRequests <= requestsBeforeRetry && attempt < 20; attempt += 1) {
  await page.waitForTimeout(25);
}
if (inferenceRequests <= requestsBeforeRetry) throw new Error("retry should issue a new inference request");
await page.waitForSelector("[data-inference='failed'] button[data-act='retry-infer']");
await page.click("form[data-form='project'] button[type='submit']");
await page.waitForSelector("form[data-form='project'] > .notice.bad");
if ((await page.inputValue("#project-name")) !== "failed draft" || (await page.inputValue("#project-repo")) !== "manual/retry") {
  throw new Error("registration failure must preserve the complete draft");
}
await page.fill("#project-path", retryProjectDir);
await page.locator("#project-path").dispatchEvent("change");
await page.waitForSelector("[data-inference='pending']");
await page.click("form[data-form='project'] button[type='submit']");
await page.waitForFunction(() => !document.querySelector("form[data-form='project']"));
await page.waitForSelector(".project-row:has-text('manual/retry')");

await browser.close();
