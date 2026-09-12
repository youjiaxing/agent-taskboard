import { openIssue100Browser } from "./issue-100-harness.mjs";

const addedProjectDir = process.env.ADDED_PROJECT_DIR;
if (!addedProjectDir) throw new Error("missing ADDED_PROJECT_DIR");

const { browser, capture, page } = await openIssue100Browser();
await page.waitForSelector(".side .project-row");

const projectRow = (name) => page.locator(".side .project-row", { hasText: name });
const openProjectMenu = async (name) => {
  const row = projectRow(name);
  await row.locator("button[data-act='project-menu']").click();
  await row.locator(".project-menu").waitFor();
  return row;
};

await page.click(".side button[data-act='register']");
await page.waitForSelector("form[data-form='project']");
await page.fill("#project-path", addedProjectDir);
await page.locator("#project-path").blur();
await page.waitForSelector("[data-inference='candidate'], [data-inference='failed']");
await page.fill("#project-name", "added-project");
await page.fill("#project-host", "github.com");
await page.fill("#project-repo", "you/added");
await page.click("form[data-form='project'] button[type='submit']");
await page.waitForFunction(() => !document.querySelector("form[data-form='project']"));
await projectRow("added-project").waitFor();
await page.waitForSelector('.issue-card:has-text("added issue")');

let row = await openProjectMenu("active-project");
await row.locator("button[data-act='remove-project']").click();
await page.waitForSelector(".overlay[data-act='close-remove']");
const activeRemovalText = (await page.locator(".overlay[data-act='close-remove']").textContent())?.replace(/\s+/g, " ") ?? "";
if (!activeRemovalText.includes("活跃 Run") || await page.locator("button[data-act='confirm-remove']").count()) {
  throw new Error(`active Run must block Project removal with a visible reason: ${activeRemovalText}`);
}
await capture("issue-100-active-run-removal-blocked-1280x840.png");
await page.click("button[data-act='close-remove']");

row = await openProjectMenu("stopped-project");
await row.locator("button[data-act='remove-project']").click();
await page.waitForSelector("button[data-act='confirm-remove']");
const stoppedRemovalText = (await page.locator(".overlay[data-act='close-remove']").textContent())?.replace(/\s+/g, " ") ?? "";
if (!stoppedRemovalText.includes("Tracker") || !stoppedRemovalText.includes("认领")) {
  throw new Error(`execution-stopped removal must warn that the Tracker claim stays: ${stoppedRemovalText}`);
}

let releaseRemoval;
const removalGate = new Promise((resolve) => {
  releaseRemoval = resolve;
});
let removeRequests = 0;
await page.route("**/rpc", async (route) => {
  let request;
  try {
    request = route.request().postDataJSON();
  } catch {
    await route.continue();
    return;
  }
  if (request?.op !== "removeProject") {
    await route.continue();
    return;
  }
  removeRequests += 1;
  if (removeRequests === 1) await removalGate;
  await route.continue();
});
await page.$eval("button[data-act='confirm-remove']", (button) => {
  button.click();
  button.click();
});
await page.waitForFunction(() => document.querySelector("button[data-act='confirm-remove']")?.matches(":disabled") === true);
releaseRemoval();
await page.waitForFunction(() => !document.querySelector(".overlay[data-act='close-remove']"));
if (removeRequests !== 1) throw new Error(`Project removal must submit once, got ${removeRequests}`);
if (await projectRow("stopped-project").count()) throw new Error("removed Project must leave the sidebar");
const heading = await page.$eval(".project-heading h1", (node) => node.textContent?.trim());
if (heading === "stopped-project") throw new Error("removing the current Project must fall back to a neighbor");
if (await page.getByText("stopped issue", { exact: true }).count()) {
  throw new Error("the removed Project's Issue content must not remain in the main area");
}

row = await openProjectMenu("fallback-project");
await row.locator("button[data-act='edit-project']").click();
await page.waitForSelector("form[data-form='project']");
await page.fill("#project-name", "fallback-renamed");
await page.click("form[data-form='project'] button[type='submit']");
await page.waitForFunction(() => !document.querySelector("form[data-form='project']"));
await projectRow("fallback-renamed").waitFor();
await capture("issue-100-project-fallback-1280x840.png");

await browser.close();
console.log("project management e2e ok");
