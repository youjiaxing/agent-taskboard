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
const sheet = page.locator(".launch-sheet");
const scrollArea = page.locator(".launch-sheet .dialog-content");
// A preview response can replace the sheet between locating it and reading layout.
// Read attached geometry in one browser task, retaining the actual width assertion.
const sheetBox = await page.waitForFunction(() => {
  const rect = document.querySelector(".launch-sheet")?.getBoundingClientRect();
  return rect?.width ? { width: rect.width, height: rect.height } : null;
}, undefined, { timeout: 2000 }).then((handle) => handle.jsonValue());
if (!sheetBox || sheetBox.width < 700) {
  throw new Error(`launch sheet is too narrow: ${sheetBox?.width}`);
}
const checkboxMetrics = async (id) => {
  const input = page.locator(`.launch-sheet input[type='checkbox'][data-launch='${id}']`);
  return input.evaluate((node) => {
    const label = node.closest("label");
    if (!(node instanceof HTMLInputElement) || !label) throw new Error("checkbox label missing");
    const inputBox = node.getBoundingClientRect();
    const labelBox = label.getBoundingClientRect();
    return { inputWidth: inputBox.width, labelHeight: labelBox.height, labelWidth: labelBox.width };
  });
};
const isolationMetrics = await checkboxMetrics("isolation");
if (isolationMetrics.inputWidth > 40) {
  throw new Error(`isolation checkbox is stretched: ${isolationMetrics.inputWidth}`);
}
if (isolationMetrics.labelHeight > 40) {
  throw new Error(`isolation label is stacked vertically: ${isolationMetrics.labelHeight}`);
}
const alwaysApproveMetrics = await checkboxMetrics("always-approve");
if (alwaysApproveMetrics.inputWidth > 40) {
  throw new Error(`alwaysApprove checkbox is stretched: ${alwaysApproveMetrics.inputWidth}`);
}
if (alwaysApproveMetrics.labelWidth > alwaysApproveMetrics.inputWidth + 180) {
  throw new Error(`alwaysApprove label is stretched across the form: ${alwaysApproveMetrics.labelWidth}`);
}
if (await page.locator("select[data-launch-select='sandbox']").count()) {
  throw new Error("sandbox with no candidates must not render as an empty select");
}
if (!(await page.locator("input[data-launch='sandbox']").count())) {
  throw new Error("sandbox with no candidates should stay a text field");
}
await page.click(".launch-sheet summary[data-act='toggle-folded']");
const overflow = await sheet.evaluate((node) => ({
  scrollWidth: node.scrollWidth,
  clientWidth: node.clientWidth,
}));
if (overflow.scrollWidth > overflow.clientWidth + 2) {
  throw new Error(`launch sheet requires horizontal scrolling: ${overflow.scrollWidth} > ${overflow.clientWidth}`);
}
const setLaunchValue = async (id, value) => {
  const select = page.locator(`select[data-launch-select='${id}']`);
  if (await select.count()) {
    const available = await select.locator("option:not([value='']):not([value='__custom__'])").evaluateAll((nodes) => nodes.map((node) => node.value));
    const selected = available.at(-1);
    if (!selected) throw new Error(`missing discovered ${id} options`);
    await select.selectOption(selected);
    return selected;
  }
  await page.fill(`input[data-launch='${id}']`, value);
  return value;
};
const selectedModel = await setLaunchValue("model", "deep");
const effortSelect = page.locator("select[data-launch-select='effort']");
let selectedEffort = "high";
if (await effortSelect.count()) {
  const efforts = await effortSelect.locator("option:not([value='']):not([value='__custom__'])").evaluateAll((nodes) => nodes.map((node) => node.value));
  selectedEffort = efforts.at(-1);
  if (!selectedEffort) throw new Error("model-specific effort options were not rendered");
  await effortSelect.selectOption(selectedEffort);
} else {
  await page.fill("input[data-launch='effort']", selectedEffort);
}
await page.waitForFunction(({ model, effort }) => document.querySelector(".launch-command-preview")?.textContent?.includes(`--model ${model} --effort ${effort}`), { model: selectedModel, effort: selectedEffort });

await scrollArea.evaluate((node) => {
  node.scrollTop = node.scrollHeight;
});
const beforeWait = await scrollArea.evaluate((node) => node.scrollTop);
if (beforeWait <= 0) throw new Error("launch sheet should be internally scrollable at this viewport");
await page.waitForTimeout(1_250);
const afterWait = await scrollArea.evaluate((node) => node.scrollTop);
if (afterWait < beforeWait - 2) {
  throw new Error(`launch sheet scroll reset after one second: ${beforeWait} -> ${afterWait}`);
}

await page.click(".launch-sheet button[type='submit']");
await page.waitForFunction(() => !document.querySelector(".launch-sheet"));
await page.waitForSelector(".run-dock");
await page.click(".run-dock button[data-act='stop-run']");
await page.click("[data-dialog-id='stop-run'] button[data-act='confirm-stop-run']");
await page.waitForFunction(() => !document.querySelector(".run-dock"));

await page.click("button[data-act='new-run']");
await page.waitForSelector("textarea[data-field='openingText']");
if (await page.locator("button[data-act='select-agent']").count()) {
  throw new Error("last successful Agent should skip the picker");
}
await page.click("button[data-act='switch-agent']");
await page.waitForSelector("button[data-act='select-agent'][data-id='grok-build'].active");
await page.click("button[data-act='select-agent'][data-id='codex']");
await page.waitForTimeout(1_250);
if (!(await page.locator("button[data-act='select-agent'][data-id='codex']").getAttribute("aria-pressed"))?.includes("true")) {
  throw new Error("picker selection must survive a tick snapshot");
}
await page.click("button[data-act='next-agent']");
await page.waitForSelector("textarea[data-field='openingText']");
const switchedAgent = (await page.locator(".launch-agent b").textContent())?.trim() ?? "";
if (!switchedAgent.toLowerCase().includes("codex")) {
  throw new Error(`next step must keep the clicked Agent, got ${JSON.stringify(switchedAgent)}`);
}
await page.fill("textarea[data-field='openingText']", "manual fallback");
if (await page.locator("select[data-launch-select='model']").count()) {
  await page.selectOption("select[data-launch-select='model']", "__custom__");
  await page.waitForTimeout(500);
  const customPreview = (await page.locator(".launch-command-preview").textContent()) ?? "";
  if (customPreview.includes("__custom__")) {
    throw new Error("the custom-entry sentinel must never reach the command preview");
  }
  await page.fill("input[data-launch-custom='model']", "custom-model");
} else {
  await page.fill("input[data-launch='model']", "custom-model");
}
if (await page.locator("select[data-launch-select='effort']").count()) {
  await page.selectOption("select[data-launch-select='effort']", "__custom__");
  await page.fill("input[data-launch-custom='effort']", "ultra-special");
} else {
  await page.fill("input[data-launch='effort']", "ultra-special");
}
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
