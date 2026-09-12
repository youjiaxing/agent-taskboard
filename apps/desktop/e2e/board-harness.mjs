import { chromium } from "playwright";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import {
  assertShellRegionsDoNotOverlap,
  createVisualAssert,
  installDeterministicHostProtocol,
} from "./visual-regression.mjs";

export async function createBoardSession() {
  const url = process.env.BOARD_URL;
  if (!url) {
    console.error("missing BOARD_URL");
    process.exit(1);
  }
  const screenshotDir = process.env.ISSUE_DOCUMENT_SCREENSHOT_DIR;
  if (screenshotDir) await mkdir(screenshotDir, { recursive: true });
  const browser = await chromium.launch({ headless: true });
  const session = {
    url,
    screenshotDir,
    browser,
    context: null,
    page: null,
    capture: async () => {},
    assertVisual: null,
    clickGraphAction: null,
    clickCard: null,
    configurePage: null,
  };
  const bindPage = (page) => {
    session.page = page;
    session.capture = async (name) => {
      if (screenshotDir) await page.screenshot({ path: join(screenshotDir, name), fullPage: false });
    };
    session.clickGraphAction = async (locator) => {
      const box = await locator.boundingBox();
      if (!box) throw new Error("graph action has no clickable geometry");
      await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
    };
    session.clickCard = async (locator) => {
      const box = await locator.boundingBox();
      if (!box) throw new Error("Issue card has no clickable geometry");
      await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
    };
    session.assertVisual = createVisualAssert(page);
    session.configurePage = async () => {
      page.on("pageerror", (error) => { console.error("pageerror", error); });
      page.on("console", (msg) => {
        if (msg.type() === "error") console.error("console", msg.text());
      });
      await installDeterministicHostProtocol(page, url);
      await page.addInitScript(() => {
        window.__OPENED_URLS__ = [];
        window.open = (target) => {
          window.__OPENED_URLS__.push(String(target));
          return null;
        };
      });
      await page.goto(url, { waitUntil: "domcontentloaded" });
    };
  };
  session.context = await browser.newContext({ locale: "zh-CN", viewport: { width: 1280, height: 840 } });
  bindPage(await session.context.newPage());
  session.bindPage = bindPage;
  session.switchMobile = async () => {
    await session.context.close();
    session.context = await browser.newContext({ locale: "zh-CN", viewport: { width: 390, height: 844 } });
    bindPage(await session.context.newPage());
    await session.configurePage();
  };
  session.close = async () => { await browser.close(); };
  return session;
}
export { assertShellRegionsDoNotOverlap };
