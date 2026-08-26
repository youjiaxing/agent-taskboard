import { chromium } from "playwright";
import { assertShellRegionsDoNotOverlap, createVisualAssert } from "./visual-regression.mjs";

const url = process.env.BOARD_URL;
const state = process.env.SHELL_EDGE_STATE;
if (!url || !state) {
  console.error("missing BOARD_URL or SHELL_EDGE_STATE");
  process.exit(1);
}

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ locale: "zh-CN", viewport: { width: 1280, height: 840 } });
page.on("pageerror", (error) => console.error("pageerror", error));
await page.addInitScript((protocol) => {
  window.__HOST_PROTOCOL__ = protocol;
}, url);
await page.goto(url, { waitUntil: "domcontentloaded" });

if (state === "empty-host") {
  await page.waitForSelector(".empty");
  if (await page.$(".project-row")) throw new Error("empty Host should not render a Project row");
} else {
  await page.waitForSelector(".project-board");
  if (state === "single-project") {
    const projects = await page.$$(".project-row");
    if (projects.length !== 1) throw new Error(`single Project fixture rendered ${projects.length} Project rows`);
    await page.waitForSelector(".lanes");
  } else if (state === "frontier-empty") {
    await page.waitForSelector('[data-lane="frontier"] .lane-empty');
    const text = await page.$eval('[data-lane="frontier"] .lane-empty', (node) => node.textContent?.trim());
    if (!text?.includes("阻塞") && !text?.includes("认领")) {
      throw new Error(`Frontier empty reason is not visible: ${text}`);
    }
  } else if (state === "offline") {
    await page.waitForSelector('.refresh-bar[data-kind="offline"]');
    await page.waitForSelector(".lanes");
  } else if (state === "rate-limited") {
    await page.waitForSelector('.refresh-bar[data-kind="rate-limited"]');
    await page.waitForSelector(".lanes");
  } else if (state === "auth-failed") {
    await page.waitForSelector('.refresh-bar[data-kind="auth-failed"]');
    await page.waitForSelector(".notice.bad");
  } else {
    throw new Error(`unknown shell edge state ${state}`);
  }
}

await assertShellRegionsDoNotOverlap(page);
await createVisualAssert(page)(`issue-99-edge-${state}-1280x840.png`);
await browser.close();
console.log(`shell edge state ${state} ok`);
