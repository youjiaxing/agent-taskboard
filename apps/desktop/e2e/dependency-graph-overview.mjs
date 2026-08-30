import { chromium } from "playwright";
import { installDeterministicHostProtocol } from "./visual-regression.mjs";

const url = process.env.BOARD_URL;
if (!url) {
  console.error("missing BOARD_URL");
  process.exit(1);
}

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ locale: "zh-CN", viewport: { width: 1280, height: 840 } });
const page = await context.newPage();
await installDeterministicHostProtocol(page, url);
await page.goto(url, { waitUntil: "domcontentloaded" });
await page.waitForSelector(".lanes");
await page.click("button[data-act='center-view'][data-id='graph']");
await page.waitForSelector('[data-graph-mode="overview"]');

const nodes = await page.$$eval(".graph-node", (items) =>
  items.map((item) => item.querySelector(".issue-title")?.textContent ?? ""),
);
if (nodes.length !== 60) {
  throw new Error(`dependency overview should render all 60 Host-selected nodes, got ${nodes.length}`);
}
for (const title of ["dependency origin", "dependency target", "open 60"]) {
  if (!nodes.includes(title)) {
    throw new Error(`dependency overview is missing ${title}`);
  }
}
if (await page.$("button[data-act='graph-more']")) {
  throw new Error("dependency overview should not require focused-canvas pagination");
}

await browser.close();
