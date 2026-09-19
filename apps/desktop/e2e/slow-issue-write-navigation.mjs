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

const issueSection = '.workspace-rail-section[data-workspace-section="issue"]';
const openRailSection = async (name) => {
  const section = page.locator(`.workspace-rail-section[data-workspace-section="${name}"]`);
  await section.waitFor({ state: "attached" });
  if (!(await section.evaluate((node) => node.open))) await section.locator("summary").click();
};
const focusProject = async (projectId, expectedName) => {
  const started = Date.now();
  await page.click(`button[data-act="focus-project"][data-id="${projectId}"]`);
  await page.waitForSelector(`.project-heading h1:has-text("${expectedName}")`, { timeout: navigationBudgetMs });
  const elapsed = Date.now() - started;
  if (elapsed >= navigationBudgetMs) {
    throw new Error(`Project focus waited ${elapsed}ms for the slow Issue write`);
  }
};
const focusIssue = async (issueId, expectedTitle, timeout) => {
  const started = Date.now();
  await page.click(`[data-act="focus-issue"][data-id="${issueId}"]`);
  await page.waitForSelector(".focus-workspace-layout", { timeout });
  await page.waitForSelector("[data-terminal-surface]", { timeout });
  await page.waitForFunction(
    (title) => document.querySelector("[data-current-identity]")?.textContent?.trim() === title,
    expectedTitle,
    { timeout },
  );
  return Date.now() - started;
};

await focusProject(gardenProjectId, "garden");
await page.click('[data-act="focus-issue"][data-id="you/garden#1"]');
await page.waitForSelector(".focus-workspace-layout");
await page.waitForFunction(() => document.querySelector("[data-current-identity]")?.textContent?.trim() === "garden issue");
const gardenSections = await page.$$eval(".workspace-rail-section", (sections) =>
  sections.map((section) => section.dataset.workspaceSection),
);
if (gardenSections.join(",") !== "actions,issue,runs") {
  throw new Error(`desktop Issue focus must use the three-section right rail: ${JSON.stringify(gardenSections)}`);
}
await page.waitForSelector(`${issueSection} section.issue-document[data-document-state="ready"]`);
await openRailSection("actions");
await page.click('button[data-act="edit-issue"]');
await page.fill("#issue-edit-title", "garden issue after slow write");

const updateResponsePromise = page.waitForResponse((response) => {
  try {
    return response.request().postDataJSON()?.op === "updateIssue";
  } catch {
    return false;
  }
});
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
  selected: document.querySelector("[data-current-identity]")?.textContent?.trim(),
  editForm: Boolean(document.querySelector('form[data-act="issue-edit"]')),
}));
if (
  identityAfterWrite.project !== "notes"
  || identityAfterWrite.selected !== "notes issue"
  || identityAfterWrite.editForm
) {
  throw new Error(`slow write completion rolled back navigation: ${JSON.stringify(identityAfterWrite)}`);
}

await focusProject(gardenProjectId, "garden");
await page.waitForSelector('.issue-card:has-text("garden issue after slow write")');

await context.close();
await browser.close();
console.log("slow Issue write navigation e2e ok");
