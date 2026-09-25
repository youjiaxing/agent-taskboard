import { chromium } from "playwright";

const url = process.env.BOARD_URL;
const gardenProjectId = process.env.GARDEN_PROJECT_ID;
const notesProjectId = process.env.NOTES_PROJECT_ID;
const navigationBudgetMs = Number(process.env.E2E_NAVIGATION_BUDGET_MS ?? "500");
if (!url || !gardenProjectId || !notesProjectId) {
  throw new Error("missing slow Issue write E2E environment");
}

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({
  locale: "zh-CN",
  viewport: { width: 1280, height: 840 },
});
const page = await context.newPage();
await page.addInitScript((protocol) => {
  window.__HOST_PROTOCOL__ = protocol;
}, url);
page.on("pageerror", (error) => console.error("pageerror", error));
page.on("console", (message) => {
  if (message.type() === "error") console.error("console", message.text());
});
await page.goto(url, { waitUntil: "domcontentloaded" });
await page.waitForSelector(".project-board");

const issueSection = ".issue-detail";
const waitForRpcResponse = (op, timeout) => page.waitForResponse((response) => {
  try {
    const request = response.request();
    return request.method() === "POST" && request.postDataJSON()?.op === op;
  } catch {
    return false;
  }
}, { timeout });
const clickCard = async (locator) => {
  await locator.waitFor({ state: "visible" });
  const box = await locator.boundingBox();
  if (!box) throw new Error("card is not visible");
  await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
};
const openRailSection = async (name) => {
  const section = page.locator(`.workspace-rail-section[data-workspace-section="${name}"]`);
  if (await section.count() === 0) return;
  await section.waitFor({ state: "attached" });
  if (!(await section.evaluate((node) => node.open))) await section.locator("summary").click();
};
const focusProject = async (projectId, expectedName) => {
  const started = Date.now();
  const responsePromise = waitForRpcResponse("focusProject", navigationBudgetMs);
  await page.click(`button[data-act="focus-project"][data-id="${projectId}"]`);
  const response = await responsePromise;
  if (!response.ok()) throw new Error(`focusProject failed: ${response.status()} ${await response.text()}`);
  await page.waitForSelector(`.project-heading h1:has-text("${expectedName}")`, { timeout: navigationBudgetMs });
  const elapsed = Date.now() - started;
  if (elapsed >= navigationBudgetMs) {
    throw new Error(`Project focus waited ${elapsed}ms for the slow Issue write`);
  }
};
const clickIssue = async (issueId, timeout) => {
  const responsePromise = waitForRpcResponse("focusIssue", timeout);
  await clickCard(page.locator(`[data-act="focus-issue"][data-id="${issueId}"]`));
  const response = await responsePromise;
  if (!response.ok()) throw new Error(`focusIssue failed: ${response.status()} ${await response.text()}`);
};
const focusIssue = async (issueId, expectedTitle, timeout) => {
  const started = Date.now();
  await clickIssue(issueId, timeout);
  await page.waitForSelector(".board-shell > .issue-detail", { timeout });
  await page.waitForFunction(
    (title) => document.querySelector(".board-shell > .issue-detail .detail-hd")?.textContent?.includes(title),
    expectedTitle,
    { timeout },
  );
  return Date.now() - started;
};

await focusProject(gardenProjectId, "garden");
await focusIssue("you/garden#1", "garden issue", 30_000);
await page.waitForSelector(`${issueSection} section.issue-document[data-document-state="ready"]`);
await openRailSection("actions");
await page.click('button[data-act="edit-issue"]');
await page.fill("#issue-edit-title", "garden issue after slow write");

const updateResponsePromise = waitForRpcResponse("updateIssue", 30_000);
await page.click('form[data-act="issue-edit"] button[type="submit"]');
await page.waitForSelector('form[data-act="issue-edit"][aria-busy="true"]');

await focusProject(notesProjectId, "notes");
const issueFocusElapsed = await focusIssue("you/notes#1", "notes issue", navigationBudgetMs);
if (issueFocusElapsed >= navigationBudgetMs) {
  throw new Error(`Issue focus waited ${issueFocusElapsed}ms for the slow Issue write`);
}

const updateResponse = await updateResponsePromise;
if (!updateResponse.ok()) {
  throw new Error(`slow updateIssue failed: ${updateResponse.status()} ${await updateResponse.text()}`);
}
const updateResult = await updateResponse.json();
if (
  updateResult.snapshot?.focusedProjectId !== notesProjectId
  || updateResult.snapshot?.board?.selected?.id !== "you/notes#1"
) {
  throw new Error(`slow write returned stale navigation: ${JSON.stringify({
    project: updateResult.snapshot?.focusedProjectId,
    selected: updateResult.snapshot?.board?.selected?.id,
  })}`);
}

await page.waitForTimeout(100);
const identityAfterWrite = await page.evaluate(() => ({
  project: document.querySelector(".project-row.active b")?.textContent?.trim(),
  selected: document.querySelector(".board-shell > .issue-detail .detail-hd")?.textContent?.replace(/\s+/g, " ").trim(),
  editForm: Boolean(document.querySelector('form[data-act="issue-edit"]')),
}));
if (
  identityAfterWrite.project !== "notes"
  || !identityAfterWrite.selected?.includes("notes issue")
  || identityAfterWrite.editForm
) {
  throw new Error(`slow write completion rolled back navigation: ${JSON.stringify(identityAfterWrite)}`);
}

await focusProject(gardenProjectId, "garden");
await page.waitForSelector('.issue-card:has-text("garden issue after slow write")');

await context.close();
await browser.close();
console.log("slow Issue write navigation e2e ok");
