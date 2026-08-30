import { chromium } from "playwright";

const url = process.env.BOARD_URL;
const gardenProjectId = process.env.GARDEN_PROJECT_ID;
const notesProjectId = process.env.NOTES_PROJECT_ID;
if (!url || !gardenProjectId || !notesProjectId) {
  throw new Error("missing Client isolation E2E environment");
}

const browser = await chromium.launch({ headless: true });
const desktopContext = await browser.newContext({
  locale: "zh-CN",
  viewport: { width: 1280, height: 840 },
});
const browserContext = await browser.newContext({
  locale: "zh-CN",
  viewport: { width: 1280, height: 840 },
});
const desktop = await desktopContext.newPage();
const web = await browserContext.newPage();
for (const page of [desktop, web]) {
  await page.addInitScript((protocol) => {
    window.__HOST_PROTOCOL__ = protocol;
  }, url);
  page.on("pageerror", (error) => console.error("pageerror", error));
  page.on("console", (message) => {
    if (message.type() === "error") console.error("console", message.text());
  });
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.waitForSelector(".project-board");
}

const clientIds = await Promise.all(
  [desktop, web].map((page) =>
    page.evaluate(() => sessionStorage.getItem("agent-taskboard-client-id")),
  ),
);
if (!clientIds[0] || !clientIds[1] || clientIds[0] === clientIds[1]) {
  throw new Error(`real Client tabs need distinct identities, got ${JSON.stringify(clientIds)}`);
}

const focusProject = async (page, projectId, expectedName) => {
  await page.click(`button[data-act="focus-project"][data-id="${projectId}"]`);
  await page.waitForSelector(`.project-heading h1:has-text("${expectedName}")`);
};

await focusProject(desktop, gardenProjectId, "garden");
await desktop.click('[data-act="focus-issue"][data-id="you/garden#1"]');
await desktop.waitForSelector('.detail-hd:has-text("garden issue 1")');
await desktop.click('button[data-act="center-view"][data-id="graph"]');
await desktop.waitForSelector('button[data-act="center-view"][data-id="graph"].active');

await focusProject(web, notesProjectId, "notes");
await web.click('[data-act="focus-issue"][data-id="you/notes#1"]');
await web.waitForSelector('.detail-hd:has-text("notes issue")');

const desktopIdentity = await desktop.evaluate(() => ({
  project: document.querySelector(".project-heading h1")?.textContent?.trim(),
  selected: document.querySelector(".detail-hd")?.textContent?.replace(/\s+/g, " ").trim(),
  view: document.querySelector('button[data-act="center-view"].active')?.getAttribute("data-id"),
}));
if (
  desktopIdentity.project !== "garden"
  || !desktopIdentity.selected?.includes("garden issue 1")
  || desktopIdentity.view !== "graph"
) {
  throw new Error(`browser actions displaced the desktop Client: ${JSON.stringify(desktopIdentity)}`);
}

await desktop.click('button[data-act="center-view"][data-id="board"]');
await desktop.waitForSelector(".lanes");
await desktop.addStyleTag({ content: '[data-lane="frontier"] { max-height: 120px; }' });
const scrollBefore = await desktop.$eval('[data-lane="frontier"]', (node) => {
  node.scrollTop = node.scrollHeight;
  return node.scrollTop;
});
if (scrollBefore <= 0) throw new Error("slow-refresh regression needs a scrollable board lane");

await desktop.click('button[data-act="new-issue"]');
await desktop.fill("#issue-create-title", "draft survives slow refresh");
await desktop.fill("#issue-create-body", "body remains local and editable");
await desktop.focus("#issue-create-title");
const titleHandle = await desktop.$("#issue-create-title");
if (!titleHandle) throw new Error("missing create Issue title field");

await desktop.click('.refresh-bar button[data-act="refresh"]');
await desktop.waitForSelector('.refresh-bar[data-kind="refreshing"]');
const responsiveDuringSlowRead = await titleHandle.evaluate((node) => ({
  connected: node.isConnected,
  value: node.value,
}));
if (!responsiveDuringSlowRead.connected || responsiveDuringSlowRead.value !== "draft survives slow refresh") {
  throw new Error(`slow refresh reset the open form: ${JSON.stringify(responsiveDuringSlowRead)}`);
}

await desktop.click('button[data-act="center-view"][data-id="graph"]');
await desktop.waitForSelector('button[data-act="center-view"][data-id="graph"].active', {
  timeout: 500,
});
await desktop.waitForFunction(() => document.querySelector(".refresh-bar")?.getAttribute("data-kind") !== "refreshing");
if ((await desktop.inputValue("#issue-create-title")) !== "draft survives slow refresh") {
  throw new Error("view switching during slow refresh lost the unsubmitted title");
}
if (!(await desktop.textContent(".detail-hd"))?.includes("garden issue 1")) {
  throw new Error("refresh completion closed or replaced the current Inspector");
}

await desktop.click('button[data-act="center-view"][data-id="board"]');
await desktop.waitForSelector(".lanes");
const scrollAfter = await desktop.$eval('[data-lane="frontier"]', (node) => node.scrollTop);
if (Math.abs(scrollAfter - scrollBefore) > 1) {
  throw new Error(`board/graph switching lost lane scroll: ${scrollBefore} -> ${scrollAfter}`);
}

const webIdentity = await web.evaluate(() => ({
  project: document.querySelector(".project-heading h1")?.textContent?.trim(),
  selected: document.querySelector(".detail-hd")?.textContent?.replace(/\s+/g, " ").trim(),
  view: document.querySelector('button[data-act="center-view"].active')?.getAttribute("data-id"),
}));
if (
  webIdentity.project !== "notes"
  || !webIdentity.selected?.includes("notes issue")
  || webIdentity.view !== "board"
) {
  throw new Error(`desktop actions displaced the browser Client: ${JSON.stringify(webIdentity)}`);
}

await desktop.click('[data-act="settings"]');
const originalRefreshInterval = await desktop.inputValue("#refresh-interval");
const desktopClientId = clientIds[0];
const readHostRefreshInterval = () => desktop.evaluate(async (clientInstanceId) => {
  const response = await fetch(`${window.__HOST_PROTOCOL__ ?? ""}/rpc`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ op: "snapshot", clientInstanceId }),
  });
  if (!response.ok) throw new Error(`snapshot RPC failed: ${response.status} ${await response.text()}`);
  const result = await response.json();
  return result.snapshot.refreshIntervalMs;
}, desktopClientId);
const setRefreshInterval = async (seconds) => {
  const field = desktop.locator("#refresh-interval");
  await field.fill(seconds);
  await field.blur();
  const expectedMs = Number(seconds) * 1000;
  const deadline = Date.now() + 2_000;
  while (await readHostRefreshInterval() !== expectedMs) {
    if (Date.now() >= deadline) {
      throw new Error(`Host did not persist refresh interval ${expectedMs}`);
    }
    await desktop.waitForTimeout(25);
  }
  if ((await desktop.inputValue("#refresh-interval")) !== seconds) {
    throw new Error(`settings UI did not reflect refresh interval ${seconds}`);
  }
};
await setRefreshInterval("15");
await setRefreshInterval("999999");
await setRefreshInterval(originalRefreshInterval);

await desktopContext.close();
await browserContext.close();
await browser.close();
