import { chromium } from "playwright";

const url = process.env.BOARD_URL;
if (!url) throw new Error("missing Issue #115 E2E environment");

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ locale: "zh-CN", viewport: { width: 1280, height: 720 } });
const page = await context.newPage();
await page.addInitScript((protocol) => {
  window.__HOST_PROTOCOL__ = protocol;
}, url);
await page.goto(url, { waitUntil: "domcontentloaded" });

await page.click("button[data-act='new-run']");
await page.waitForSelector(".launch-sheet button[data-act='select-agent']");
if (await page.locator(".launch-sheet button[data-act='select-agent'].active").count()) {
  throw new Error("first launch must not preselect an Agent");
}
if (!(await page.locator("button[data-act='next-agent']").isDisabled())) {
  throw new Error("Next must stay disabled until the user selects an Agent");
}

await page.click("button[data-act='select-agent'][data-id='grok-build']");
if (!(await page.locator("button[data-act='select-agent'][data-id='grok-build']").getAttribute("aria-pressed"))?.includes("true")) {
  throw new Error("selected Agent state is not exposed clearly");
}
await page.click("button[data-act='next-agent']");
await page.waitForSelector("textarea[data-field='openingText']");
await page.fill("textarea[data-field='openingText']", "Issue 115 browser supplement");
await page.fill("input[data-launch='model']", "deep");
await page.fill("input[data-launch='effort']", "high");
await page.waitForFunction(() => document.querySelector(".launch-command-preview")?.textContent?.includes("--model deep --effort high"));

const sheet = page.locator(".launch-sheet");
await sheet.evaluate((node) => {
  node.scrollTop = node.scrollHeight;
});
const beforeWait = await sheet.evaluate((node) => node.scrollTop);
if (beforeWait <= 0) throw new Error("launch sheet should be internally scrollable at this viewport");
await page.waitForTimeout(1_250);
const afterWait = await sheet.evaluate((node) => node.scrollTop);
if (afterWait < beforeWait - 2) {
  throw new Error(`launch sheet scroll reset after one second: ${beforeWait} -> ${afterWait}`);
}

await page.click(".launch-sheet button[type='submit']");
await page.waitForFunction(() => !document.querySelector(".launch-sheet"));
await page.waitForSelector(".run-dock");
await page.click(".run-dock button[data-act='stop-run']");
await page.waitForFunction(() => !document.querySelector(".run-dock"));

await page.click("button[data-act='new-run']");
await page.waitForSelector("textarea[data-field='openingText']");
if (await page.locator("button[data-act='select-agent']").count()) {
  throw new Error("last successful Agent should skip the picker");
}
await page.click("button[data-act='switch-agent']");
await page.waitForSelector("button[data-act='select-agent'][data-id='grok-build'].active");
await page.click("button[data-act='select-agent'][data-id='codex']");
await page.click("button[data-act='next-agent']");
await page.waitForSelector("textarea[data-field='openingText']");
await page.fill("textarea[data-field='openingText']", "manual fallback");
await page.fill("input[data-launch='model']", "custom-model");
await page.fill("input[data-launch='effort']", "ultra-special");
await page.waitForFunction(() => document.querySelector(".launch-command-preview")?.textContent?.includes("custom-model"));
if (!(await page.locator(".launch-warnings").textContent())?.includes("ultra-special")) {
  throw new Error("manual value should remain accepted with a readable warning");
}

const previewStyle = await page.locator(".launch-command-preview").evaluate((node) => ({
  whiteSpace: getComputedStyle(node).whiteSpace,
  wordBreak: getComputedStyle(node).wordBreak,
}));
if (previewStyle.whiteSpace !== "pre" || previewStyle.wordBreak === "break-all") {
  throw new Error(`command preview should scroll instead of ugly forced wrapping: ${JSON.stringify(previewStyle)}`);
}

await browser.close();
console.log("Issue #115 browser supplement e2e ok");
