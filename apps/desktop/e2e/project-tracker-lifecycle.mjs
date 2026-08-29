import { openIssue100Browser } from "./issue-100-harness.mjs";

const localProjectDir = process.env.LOCAL_PROJECT_DIR;
const remoteProjectDir = process.env.REMOTE_PROJECT_DIR;
const fallbackProjectDir = process.env.FALLBACK_PROJECT_DIR;
if (!localProjectDir || !remoteProjectDir || !fallbackProjectDir) {
  throw new Error("missing Project / Tracker lifecycle e2e environment");
}

const { browser, capture, page } = await openIssue100Browser();
await page.waitForSelector(".empty button[data-act='register']");

const projectRow = (name) => page.locator(".side .project-row", { hasText: name });
const openProjectMenu = async (name) => {
  const row = projectRow(name);
  await row.locator("button[data-act='project-menu']").click();
  await row.locator(".project-menu").waitFor();
  return row;
};

await page.click(".empty button[data-act='register']");
await page.fill("#project-path", localProjectDir);
await page.locator("#project-path").dispatchEvent("change");
await page.waitForFunction(
  (expected) =>
    document.querySelector("#project-host")?.value === "local" &&
    document.querySelector("#project-repo")?.value === expected,
  localProjectDir,
);
await page.fill("#project-name", "local-tracker");
await page.click("form[data-form='project'] button[type='submit']");
await page.waitForFunction(() => !document.querySelector("form[data-form='project']"));
await projectRow("local-tracker").waitFor();
await page.waitForSelector('.issue-card:has-text("local issue")');
if (!(await projectRow("local-tracker").textContent()).includes("Local Markdown")) {
  throw new Error("Local Markdown registration should be visible in the Project row");
}

await page.click("button[data-act='register']");
await page.fill("#project-path", remoteProjectDir);
await page.locator("#project-path").dispatchEvent("change");
await page.waitForFunction(
  () =>
    document.querySelector("#project-host")?.value === "gitlab.example.com" &&
    document.querySelector("#project-repo")?.value === "acme/platform/garden",
);
await page.fill("#project-name", "gitlab-project");
await page.click("form[data-form='project'] button[type='submit']");
await page.waitForFunction(() => !document.querySelector("form[data-form='project']"));
await projectRow("gitlab-project").waitFor();
await page.waitForSelector('.issue-card:has-text("self-hosted issue")');
if (!(await projectRow("gitlab-project").textContent()).includes("gitlab.example.com/acme/platform/garden")) {
  throw new Error("self-hosted Git remote registration should preserve its full namespace");
}

await page.click("button[data-act='register']");
await page.fill("#project-path", fallbackProjectDir);
await page.locator("#project-path").dispatchEvent("change");
await page.waitForFunction(
  (expected) =>
    document.querySelector("#project-host")?.value === "local" &&
    document.querySelector("#project-repo")?.value === expected,
  fallbackProjectDir,
);
await page.fill("#project-name", "fallback-project");
await page.click("form[data-form='project'] button[type='submit']");
await page.waitForFunction(() => !document.querySelector("form[data-form='project']"));
await projectRow("fallback-project").waitFor();

let row = await openProjectMenu("gitlab-project");
await row.locator("button[data-act='edit-project']").click();
await page.waitForSelector("form[data-form='project']");
await page.fill("#project-name", "gitlab-renamed");
await page.click("form[data-form='project'] button[type='submit']");
await page.waitForFunction(() => !document.querySelector("form[data-form='project']"));
await projectRow("gitlab-renamed").waitFor();

await page.locator(".side .project-main", { hasText: "gitlab-renamed" }).click();
row = await openProjectMenu("gitlab-renamed");
await row.locator("button[data-act='remove-project']").click();
await page.waitForSelector("button[data-act='confirm-remove']");
await page.click("button[data-act='confirm-remove']");
await page.waitForFunction(() => !document.querySelector(".overlay[data-act='close-remove']"));
if (await projectRow("gitlab-renamed").count()) {
  throw new Error("removed self-hosted Project should leave the sidebar");
}
const heading = (await page.locator(".project-heading h1").textContent())?.trim();
if (heading !== "fallback-project") {
  throw new Error(`removing the current Project should fall back to fallback-project, got ${heading}`);
}
if (await page.getByText("self-hosted issue", { exact: true }).count()) {
  throw new Error("removed Project Issue content must not remain visible");
}

await page.locator(".side .project-main", { hasText: "local-tracker" }).click();
await page.waitForSelector('.issue-card:has-text("local issue")');
await capture("issue-110-local-and-self-hosted-lifecycle-1280x840.png");

await browser.close();
console.log("Project / Tracker lifecycle e2e ok");
