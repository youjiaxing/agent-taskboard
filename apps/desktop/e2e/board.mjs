import { chromium } from "playwright";

const url = process.env.BOARD_URL;
if (!url) {
  console.error("missing BOARD_URL");
  process.exit(1);
}

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage();
await page.addInitScript((protocol) => {
  window.__HOST_PROTOCOL__ = protocol;
}, url);
await page.goto(url, { waitUntil: "networkidle" });
await page.waitForSelector(".lanes");
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
const afterGraphFrontier = await page.$$eval('[data-lane="frontier"] .issue-card .issue-title', (nodes) =>
  nodes.map((node) => node.textContent),
);
if (!afterGraphFrontier.includes("unparented ready")) {
  throw new Error("returning to the board should keep the unfiltered Frontier");
}

await page.click("button:has-text('设置')");
await page.waitForSelector("#recent-limit");
await page.fill("#recent-limit", "1");
await page.locator("#recent-limit").dispatchEvent("change");
await page.waitForFunction(() => document.querySelectorAll('[data-lane="recentlyCompleted"] .issue-card').length === 1);

await browser.close();
console.log("board e2e ok");
