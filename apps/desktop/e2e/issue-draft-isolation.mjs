import assert from "node:assert/strict";
import { openIssue100Browser } from "./issue-100-harness.mjs";

const { browser, page, capture } = await openIssue100Browser();
try {
  const issueTitle = "shared issue";
  const issueSection = '.workspace-rail-section[data-workspace-section="issue"]';
  const openRailSection = async (name) => {
    const section = page.locator(`.workspace-rail-section[data-workspace-section="${name}"]`);
    await section.waitFor({ state: "attached" });
    if (!(await section.evaluate((node) => node.open))) await section.locator("summary").click();
  };
  const focusProject = async (projectId) => {
    await page.click(`button[data-act="focus-project"][data-id="${projectId}"]`);
    const name = projectId === process.env.FIRST_PROJECT_ID ? "first" : "second";
    await page.waitForFunction((expected) => document.querySelector(".project-heading h1")?.textContent === expected, name);
  };
  const focus = async (projectId) => {
    await focusProject(projectId);
    await page.click('[data-act="focus-issue"][data-id="you/shared#1"]');
    await page.waitForSelector(".focus-workspace-layout");
    await page.waitForSelector("[data-terminal-surface]");
    await page.waitForFunction((expected) => document.querySelector("[data-current-identity]")?.textContent?.trim() === expected, issueTitle);
    assert.deepEqual(
      await page.$$eval(".workspace-rail-section", (sections) => sections.map((section) => section.dataset.workspaceSection)),
      ["actions", "issue", "runs"],
      "focusing an Issue must render the fixed three-section right rail",
    );
    await openRailSection("issue");
    await page.waitForSelector(`${issueSection} section.issue-document[data-document-state="ready"]`);
    const maintenance = page.locator(`${issueSection} details.detail-maintenance`);
    if (!(await maintenance.evaluate((element) => element.open))) await maintenance.locator("summary").click();
  };
  const comment = `${issueSection} form[data-act="issue-comment"] textarea`;

  await focus(process.env.FIRST_PROJECT_ID);
  await page.fill(comment, "comment for first Project");
  await openRailSection("actions");
  await page.click('button[data-act="edit-issue"]');
  await page.fill("#issue-edit-title", "draft for first Project");
  await focus(process.env.SECOND_PROJECT_ID);
  assert.equal(await page.locator('form[data-act="issue-edit"]').count(), 0, "another Project must not inherit the open editor");
  assert.equal(await page.inputValue(comment), "", "another Project must not inherit the comment draft");
  await page.fill(comment, "comment for second Project");
  await openRailSection("actions");
  await page.click('button[data-act="edit-issue"]');
  await page.fill("#issue-edit-title", "draft for second Project");
  await focus(process.env.FIRST_PROJECT_ID);
  assert.equal(await page.inputValue("#issue-edit-title"), "draft for first Project");
  assert.equal(await page.inputValue(comment), "comment for first Project");
  await focus(process.env.SECOND_PROJECT_ID);
  assert.equal(await page.inputValue("#issue-edit-title"), "draft for second Project");
  assert.equal(await page.inputValue(comment), "comment for second Project");
  await capture("issue-100-draft-isolation-1280x840.png");
  for (const projectId of [process.env.SECOND_PROJECT_ID, process.env.FIRST_PROJECT_ID]) {
    await focus(projectId);
    await openRailSection("actions");
    const closed = page.waitForResponse((response) => response.request().postDataJSON()?.op === "setIssueOpen");
    await page.click('button[data-act="toggle-issue-open"]');
    const result = await (await closed).json();
    const selected = result.snapshot.board?.selected;
    const closedCard = result.snapshot.board?.columns?.recentlyCompleted?.find((card) => card.id === "you/shared#1");
    assert.equal(
      selected ? selected.open : closedCard?.open,
      false,
      "Issue writes must update the selected Project, even when another Project has the same Issue ID",
    );
  }
  console.log("Issue draft isolation e2e ok");
} finally {
  await browser.close();
}
