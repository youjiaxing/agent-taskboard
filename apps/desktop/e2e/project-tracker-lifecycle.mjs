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
await page.evaluate(({ path, name }) => {
  const pathInput = document.querySelector("#project-path");
  const nameInput = document.querySelector("#project-name");
  const hostInput = document.querySelector("#project-host");
  const repoInput = document.querySelector("#project-repo");
  if (
    !(pathInput instanceof HTMLInputElement) ||
    !(nameInput instanceof HTMLInputElement) ||
    !(hostInput instanceof HTMLInputElement) ||
    !(repoInput instanceof HTMLInputElement)
  ) {
    throw new Error("Project form inputs are missing");
  }
  // Simulate Computer Use/AX updates that change the visible control value
  // without dispatching the input/change events used by the draft state.
  pathInput.value = path;
  nameInput.value = name;
  hostInput.value = "local";
  repoInput.value = path;
}, { path: localProjectDir, name: "local-tracker" });
await page.click("form[data-form='project'] button[type='submit']");
await page.waitForFunction(() => !document.querySelector("form[data-form='project']"));
await projectRow("local-tracker").waitFor();
await page.waitForSelector('.issue-card:has-text("local issue")');
if (!/本地 Markdown|Local Markdown/.test(await projectRow("local-tracker").textContent())) {
  throw new Error("Local Markdown registration should be visible in the Project row");
}

await page.click("button[data-act='register']");
await page.fill("#project-name", "manual-local");
await page.fill("#project-host", "manual.example.com");
await page.fill("#project-repo", "manual/kept");
await page.fill("#project-path", localProjectDir);
await page.locator("#project-path").dispatchEvent("change");
await page.waitForFunction(
  () =>
    document.querySelector("#project-host")?.value === "manual.example.com" &&
    document.querySelector("#project-repo")?.value === "manual/kept",
);
await page.click("form[data-form='project'] .dialog-actions button[data-act='dismiss-dialog']");
await page.waitForFunction(() => !document.querySelector("form[data-form='project']"));

await page.click("button[data-act='register']");
await page.fill("#project-path", remoteProjectDir);
await page.locator("#project-path").dispatchEvent("change");
await page.waitForSelector("[data-inference='candidate']");
await page.click("[data-inference='candidate'] button[data-act='apply-infer']");
await page.waitForFunction(
  () =>
    document.querySelector("#project-host")?.value === "github.enterprise.example.com" &&
    document.querySelector("#project-repo")?.value === "acme/garden",
);
await page.fill("#project-name", "enterprise-project");
await page.click("form[data-form='project'] button[type='submit']");
await page.waitForFunction(() => !document.querySelector("form[data-form='project']"));
await projectRow("enterprise-project").waitFor();
await page.waitForSelector('.issue-card:has-text("self-hosted issue")');
if (!(await projectRow("enterprise-project").textContent()).includes("github.enterprise.example.com/acme/garden")) {
  throw new Error("self-hosted GitHub remote registration should preserve its full namespace");
}

await page.click("button[data-act='register']");
await page.fill("#project-path", fallbackProjectDir);
await page.locator("#project-path").dispatchEvent("change");
await page.waitForSelector("[data-inference='candidate']");
await page.click("[data-inference='candidate'] button[data-act='apply-infer']");
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

let row = await openProjectMenu("enterprise-project");
await row.locator("button[data-act='edit-project']").click();
await page.waitForSelector("form[data-form='project']");
await page.fill("#project-name", "enterprise-renamed");
await page.click("form[data-form='project'] button[type='submit']");
await page.waitForFunction(() => !document.querySelector("form[data-form='project']"));
await projectRow("enterprise-renamed").waitFor();

await page.locator(".side .project-main", { hasText: "enterprise-renamed" }).click();
row = await openProjectMenu("enterprise-renamed");
await row.locator("button[data-act='remove-project']").click();
await page.waitForSelector("button[data-act='confirm-remove']");
await page.click("button[data-act='confirm-remove']");
await page.waitForFunction(() => !document.querySelector("[data-dialog-id='remove-project']"));
if (await projectRow("enterprise-renamed").count()) {
  throw new Error("removed self-hosted GitHub Project should leave the sidebar");
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
