import { chromium } from "playwright";

const url = process.env.BOARD_URL;
if (!url) {
  console.error("missing BOARD_URL");
  process.exit(1);
}

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage();
page.on("pageerror", (error) => {
  console.error("pageerror", error);
});
page.on("console", (msg) => {
  if (msg.type() === "error") {
    console.error("console", msg.text());
  }
});
await page.addInitScript((protocol) => {
  window.__HOST_PROTOCOL__ = protocol;
}, url);
await page.goto(url, { waitUntil: "networkidle" });
try {
  await page.waitForSelector(".lanes");
} catch (error) {
  const html = await page.content();
  console.error("page html", html.slice(0, 4000));
  throw error;
}
await page.waitForSelector(".refresh-bar");
const refreshText = await page.$eval(".refresh-bar", (node) => node.textContent.replace(/\s+/g, " ").trim());
if (!refreshText.includes("数据截至") && !refreshText.includes("Data as of")) {
  throw new Error(`refresh bar missing as-of time: ${refreshText}`);
}
await page.click(".refresh-bar button[data-act='refresh']");
await page.waitForSelector(".lanes");

const headers = await page.$$eval(".lane-hd", (nodes) =>
  nodes.map((node) => node.textContent.replace(/\s+/g, " ").trim()),
);
if (headers.length !== 4) {
  throw new Error(`expected 4 columns, got ${JSON.stringify(headers)}`);
}
if (!headers[0].startsWith("阻塞中") || !headers[1].startsWith("Frontier") || !headers[2].startsWith("进行中") || !headers[3].startsWith("最近完成")) {
  throw new Error(`unexpected column order: ${JSON.stringify(headers)}`);
}

await page.click(".issue-card:has-text('child ready')");
await page.waitForSelector(".detail-hd:has-text('child ready')");
const beforeFrontier = await page.$$eval('[data-lane="frontier"] .issue-card', (nodes) => nodes.length);

await page.click(".name-btn:has-text('#1 parent')");
await page.waitForSelector(".detail-hd:has-text('parent')");
const stillFrontier = await page.$$eval('[data-lane="frontier"] .issue-card', (nodes) => nodes.length);
if (stillFrontier !== beforeFrontier) {
  throw new Error("clicking a parent link filtered the board");
}

await page.click("button:has-text('只看这些子票')");
await page.waitForSelector("button:has-text('清除过滤')");
const filtered = await page.$$eval('[data-lane="frontier"] .issue-card .issue-title', (nodes) =>
  nodes.map((node) => node.textContent),
);
if (filtered.join(",") !== "child ready") {
  throw new Error(`parent filter should show only children, got ${JSON.stringify(filtered)}`);
}

await page.click("button:has-text('清除过滤')");
await page.waitForFunction(() => !document.querySelector("button[data-act='clear-filter']"));

const boardActive = await page.$eval("button[data-act='center-view'][data-id='board']", (node) =>
  node.classList.contains("active"),
);
if (!boardActive) {
  throw new Error("factory default should be the board view");
}

await page.click("button[data-act='center-view'][data-id='graph']");
await page.waitForSelector(".dep-graph");
if (await page.$(".lanes")) {
  throw new Error("graph view should replace the four columns");
}
const graphTitles = await page.$$eval(".graph-node .issue-title", (nodes) =>
  nodes.map((node) => node.textContent),
);
if (!graphTitles.includes("unparented ready") || !graphTitles.includes("blocker")) {
  throw new Error(`graph should include all open issues, got ${JSON.stringify(graphTitles)}`);
}
if (graphTitles.includes("old gate") || graphTitles.includes("just closed")) {
  throw new Error("closed context should be off by default");
}
const edge = await page.$('path[data-from="you/garden#9"][data-to="you/garden#3"]');
if (!edge) {
  throw new Error("graph should draw the blocker edge from left to right");
}
if (await page.$('path[data-from="you/garden#1"][data-to="you/garden#2"]')) {
  throw new Error("graph should not draw parent/child as an edge");
}

await page.click(".graph-node:has-text('unparented ready')");
await page.waitForSelector(".detail-hd:has-text('unparented ready')");
if (await page.$("button[data-act='clear-filter']")) {
  throw new Error("clicking a graph node should not filter the board");
}

await page.click("[data-field='closedContext']");
await page.waitForSelector(".graph-node:has-text('old gate')");
if (await page.$(".graph-node:has-text('just closed')")) {
  throw new Error("closed context should only add dependency neighbors");
}

await page.click("button[data-act='center-view'][data-id='board']");
await page.waitForSelector(".lanes");

await page.click("button[data-act='open-usage']");
await page.waitForSelector(".usage-page");
if (await page.$(".lanes")) {
  throw new Error("usage page should replace the board, not sit as a tab or overlay");
}
const usageTitle = await page.$eval(".usage-page h1", (node) => node.textContent ?? "");
if (usageTitle !== "用量" && usageTitle !== "Usage") {
  throw new Error(`usage page title, got ${usageTitle}`);
}
if (!(await page.$("button.active[data-act='usage-range'][data-id='today']"))) {
  throw new Error("usage range should default to today");
}
await page.click("button[data-act='close-usage']");
await page.waitForSelector(".lanes");

const afterGraphFrontier = await page.$$eval('[data-lane="frontier"] .issue-card .issue-title', (nodes) =>
  nodes.map((node) => node.textContent),
);
if (!afterGraphFrontier.includes("unparented ready")) {
  throw new Error("returning to the board should keep the unfiltered Frontier");
}

const newLabel = await page.$eval("button[data-act='new-run']", (node) => node.getAttribute("aria-label"));
if (newLabel !== "新建" && newLabel !== "New") {
  throw new Error(`project row plus should be New, got ${newLabel}`);
}
await page.click("button[data-act='new-run']");
await page.waitForSelector(".launch-sheet");
const pick = page.locator("button[data-act='pick-agent']:not([disabled])").first();
if (await pick.count()) {
  await pick.click();
  await page.waitForSelector("textarea[data-field='openingText']");
}
await page.fill("textarea[data-field='openingText']", "e2e unbound run");
await page.click("button[type='submit']");
await page.waitForSelector(".run-dock");
if (await page.$(".launch-sheet")) {
  await page.click("button[data-act='close-launch']");
}
const dockText = await page.$eval(".run-dock", (node) => node.textContent.replace(/\s+/g, " ").trim());
if (!dockText.includes("Grok Build") || (!dockText.includes("未绑定 Issue") && !dockText.includes("Unbound Issue"))) {
  throw new Error(`unbound Run dock missing identity, got ${dockText}`);
}
if (!(await page.$(".pty-slot"))) {
  throw new Error("Embedded Terminal slot missing");
}
await page.click("button[data-act='stop-run']");
await page.waitForFunction(() => document.querySelector("button[data-act='stop-run']")?.disabled);

await page.click("button:has-text('设置')");
await page.waitForSelector("#recent-limit");
await page.fill("#recent-limit", "1");
await page.locator("#recent-limit").dispatchEvent("change");
await page.waitForFunction(() => document.querySelectorAll('[data-lane="recentlyCompleted"] .issue-card').length === 1);

await browser.close();
console.log("board e2e ok");
