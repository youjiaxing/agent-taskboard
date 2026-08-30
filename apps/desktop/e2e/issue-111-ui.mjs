import { readFile, readdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { chromium } from "playwright";

const url = process.env.BOARD_URL;
const localProjectDir = process.env.LOCAL_PROJECT_DIR;
if (!url || !localProjectDir) throw new Error("missing Issue #111 E2E environment");

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ locale: "zh-CN", viewport: { width: 1280, height: 840 } });
const page = await context.newPage();
await page.addInitScript((protocol) => {
  window.__HOST_PROTOCOL__ = protocol;
}, url);
await page.goto(url, { waitUntil: "domcontentloaded" });
const issueId = (number) => `${localProjectDir}#${number}`;
const issueFiles = async () => {
  const directory = join(localProjectDir, ".scratch", "feature", "issues");
  const names = await readdir(directory);
  return Promise.all(
    names.filter((name) => name.endsWith(".md")).map((name) => readFile(join(directory, name), "utf8")),
  );
};
const issueText = async (needle) => {
  const files = await issueFiles();
  const match = files.find((contents) => contents.includes(needle));
  if (!match) throw new Error(`could not find ${needle} in Local Markdown files`);
  return match;
};
const waitForIssueText = async (needle) => {
  let lastError;
  for (let attempt = 0; attempt < 40; attempt += 1) {
    try {
      return await issueText(needle);
    } catch (error) {
      lastError = error;
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
  }
  throw lastError;
};
const waitForIssueTextWithout = async (needle) => {
  for (let attempt = 0; attempt < 40; attempt += 1) {
    const files = await issueFiles();
    if (!files.some((contents) => contents.includes(needle))) return;
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  throw new Error(`Local Markdown files still contain ${needle}`);
};

await page.waitForSelector(".empty button[data-act='register']");
await page.click(".empty button[data-act='register']");
await page.fill("#project-path", localProjectDir);
await page.locator("#project-path").dispatchEvent("change");
await page.fill("#project-name", "issue-111-ui");
await page.fill("#project-host", "local");
await page.fill("#project-repo", localProjectDir);
await page.click("form[data-form='project'] button[type='submit']");
await page.waitForFunction(() => !document.querySelector("form[data-form='project']"));
await page.waitForSelector(".issue-card:has-text('Child')");

await page.click("button[data-act='new-issue']");
await page.fill("#issue-create-title", "Created from desktop UI");
await page.fill("#issue-create-body", "created body from desktop UI");
await page.click("form[data-act='issue-create'] button[type='submit']");
await page.waitForFunction(() => !document.querySelector("form[data-act='issue-create']"));
await page.waitForSelector(".issue-card:has-text('Created from desktop UI')");
await waitForIssueText("created body from desktop UI");

await page.locator(".issue-card-main", { hasText: "Child" }).click();
await page.waitForSelector(".issue-detail");
await page.waitForSelector("section.issue-document[data-document-state='ready']");

let failNextUpdate = true;
await page.route("**/rpc", async (route) => {
  let request;
  try {
    request = route.request().postDataJSON();
  } catch {
    await route.continue();
    return;
  }
  if (request?.op === "updateIssue" && failNextUpdate) {
    failNextUpdate = false;
    await route.fulfill({
      status: 503,
      contentType: "application/json",
      body: JSON.stringify({ error: "simulated Local Markdown write failure" }),
    });
    return;
  }
  await route.continue();
});

await page.click("button[data-act='edit-issue']");
if ((await page.inputValue("#issue-edit-body")).includes("Status:")) {
  throw new Error("Local Markdown edit form should expose body content without tracker metadata");
}
await page.fill("#issue-edit-title", "Failed draft title");
await page.fill("#issue-edit-body", "failed draft body");
await page.click("form[data-act='issue-edit'] button[type='submit']");
await page.waitForSelector("form[data-act='issue-edit'] .form-feedback");
if (await page.inputValue("#issue-edit-title") !== "Failed draft title") {
  throw new Error("failed edit must retain the title draft");
}
if (!(await page.locator("form[data-act='issue-edit'] .form-feedback").textContent())?.includes("simulated Local Markdown write failure")) {
  throw new Error("failed edit must show the tracker error");
}

await page.fill("#issue-edit-title", "Child edited from desktop UI");
await page.fill("#issue-edit-body", "edited body from desktop UI");
await page.click("form[data-act='issue-edit'] button[type='submit']");
await page.waitForFunction(() => !document.querySelector("form[data-act='issue-edit']"));
await page.waitForSelector(".detail-hd:has-text('Child edited from desktop UI')");
await waitForIssueText("edited body from desktop UI");

await page.click(".detail-maintenance > summary");
await page.fill("form[data-act='issue-comment'] textarea[name='body']", "comment from desktop UI");
await page.click("form[data-act='issue-comment'] button[type='submit']");
await page.waitForFunction(() => document.querySelector("form[data-act='issue-comment'] textarea[name='body']")?.value === "");
await waitForIssueText("comment from desktop UI");

await page.selectOption("#issue-parent", issueId(1));
await page.click("form[data-act='issue-parent'] button[type='submit']");
await page.waitForFunction((expected) => document.querySelector("#issue-parent")?.value === expected, issueId(1));
await waitForIssueText("Part of: 1");

await page.selectOption("#issue-blocked-by", [issueId(1)]);
await page.click("form[data-act='issue-blockers'] button[type='submit']");
await waitForIssueText("Blocked by: 1");

await page.selectOption("#issue-parent", "");
await page.click("form[data-act='issue-parent'] button[type='submit']");
await waitForIssueTextWithout("Part of: 1");
await page.click("button[data-act='clear-issue-blockers']");
await page.click("form[data-act='issue-blockers'] button[type='submit']");
await waitForIssueTextWithout("Blocked by: 1");

await page.click("button[data-act='toggle-issue-open']");
await page.waitForSelector("button[data-act='toggle-issue-open']");
await waitForIssueText("Status: resolved");
await page.click("button[data-act='toggle-issue-open']");
await waitForIssueText("Status: ready-for-agent");

await page.reload({ waitUntil: "domcontentloaded" });
await page.waitForSelector(".issue-card:has-text('Child edited from desktop UI')");
await page.locator(".issue-card-main", { hasText: "Child edited from desktop UI" }).click();
await page.waitForSelector("section.issue-document[data-document-state='ready']");
if (await page.locator(".issue-detail").count() !== 1) throw new Error("Issue detail should survive reload");

const childPath = join(localProjectDir, ".scratch", "feature", "issues", "02-child.md");
const childMarkdown = await readFile(childPath, "utf8");
await writeFile(
  childPath,
  childMarkdown.replace("edited body from desktop UI", "externally changed body from markdown"),
);
await page.waitForSelector("section.issue-document[data-document-state='ready']", { timeout: 5_000 });
await page.waitForFunction(
  () => document.querySelector("section.issue-document")?.textContent?.includes("externally changed body from markdown"),
  null,
  { timeout: 5_000 },
);
await page.click("button[data-act='edit-issue']");
if (await page.inputValue("#issue-edit-body") !== "externally changed body from markdown") {
  throw new Error("external Local Markdown changes must reload into the open Issue editor");
}

await browser.close();
console.log("Issue #111 desktop UI e2e ok");
