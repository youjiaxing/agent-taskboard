import { chromium } from "playwright";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { installDeterministicHostProtocol } from "./visual-regression.mjs";

export async function openIssue100Browser({ screenshotDir = process.env.ISSUE_100_SCREENSHOT_DIR } = {}) {
  const url = process.env.BOARD_URL;
  if (!url) throw new Error("missing BOARD_URL");

  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({ locale: "zh-CN", viewport: { width: 1280, height: 840 } });
  const page = await context.newPage();
  if (screenshotDir) await mkdir(screenshotDir, { recursive: true });
  const capture = async (name) => {
    if (screenshotDir) await page.screenshot({ path: join(screenshotDir, name), fullPage: false });
  };

  await installDeterministicHostProtocol(page, url);
  await page.goto(url, { waitUntil: "domcontentloaded" });
  return { browser, capture, page, url };
}

export async function hostSnapshot(page, url) {
  return page.evaluate(async (protocol) => {
    const response = await fetch(`${protocol}/rpc`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ op: "snapshot" }),
    });
    return (await response.json()).snapshot;
  }, url);
}
