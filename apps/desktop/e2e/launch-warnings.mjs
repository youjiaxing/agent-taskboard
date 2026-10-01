import assert from "node:assert/strict";
import { chromium } from "playwright";

const url = process.env.BOARD_URL;
if (!url) throw new Error("missing launch warnings E2E environment");

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage({ locale: "zh-CN", viewport: { width: 1280, height: 840 } });
  page.setDefaultTimeout(10_000);
  await page.addInitScript((protocol) => { window.__HOST_PROTOCOL__ = protocol; }, url);
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.click("button[data-act='new-run']");
  await page.waitForSelector("textarea[data-field='openingText']");
  await page.waitForFunction(() => {
    const button = document.querySelector(".launch-sheet button[type='submit']");
    return button instanceof HTMLButtonElement && !button.disabled;
  });
  const items = () => page.locator(".launch-warnings p").allTextContents();
  const shared = "将与其他运行中的 Run 共用工作目录，文件修改可能互相覆盖。";
  const lock = "检测到 Git 锁文件 `.git/index.lock`，Git 写入操作可能失败。";
  assert.deepEqual((await items()).map((item) => item.trim()), [shared, lock],
    "shared-directory and detected-lock warnings must be separately readable");
  assert.equal(await page.locator(".launch-sheet button[type='submit']").isEnabled(), true,
    "warnings must not prevent launching");

  const isolation = page.locator("input[data-launch='isolation']");
  await isolation.check();
  await page.waitForFunction(() =>
    document.querySelector(".launch-warnings")?.textContent?.includes("index.lock")
      && !document.querySelector(".launch-warnings")?.textContent?.includes("共用工作目录"));
  assert.deepEqual((await items()).map((item) => item.trim()), [lock],
    "effective isolation must remove only the shared-directory warning");

  const gate = () => {
    let release;
    const promise = new Promise((resolve) => { release = resolve; });
    return {
      promise,
      release,
      async wait() {
        let timer;
        try {
          await Promise.race([
            promise,
            new Promise((_, reject) => {
              timer = setTimeout(() => reject(new Error("preview gate timed out")), 10_000);
            }),
          ]);
        } finally {
          clearTimeout(timer);
        }
      },
    };
  };
  const firstReceived = gate();
  const releaseFirst = gate();
  const secondReceived = gate();
  const releaseSecond = gate();
  let previews = 0;
  await page.route("**/rpc", async (route) => {
    const request = route.request().postDataJSON();
    if (request.op !== "updateRunLaunch") {
      await route.continue();
      return;
    }
    const index = ++previews;
    if (index === 1) {
      const response = await route.fetch();
      firstReceived.release();
      await releaseFirst.promise;
      await route.fulfill({ response });
    } else if (index === 2) {
      secondReceived.release();
      await releaseSecond.promise;
      await route.fulfill({
        status: 503,
        contentType: "application/json",
        body: JSON.stringify({ message: "preview unavailable" }),
      });
    } else {
      await route.continue();
    }
  });
  await isolation.uncheck();
  await firstReceived.wait();
  await isolation.check();
  releaseFirst.release();
  await secondReceived.wait();
  await page.setViewportSize({ width: 390, height: 844 });
  await page.waitForFunction(() => document.documentElement.dataset.viewport === "mobile");
  assert.equal(await page.locator(".launch-warnings").isVisible(), false,
    "obsolete warnings must stay hidden during the latest preview, even after a full render");
  const secondResponse = page.waitForResponse(async (response) =>
    response.request().postDataJSON()?.op === "updateRunLaunch" && response.status() === 503);
  releaseSecond.release();
  await secondResponse;
  await page.setViewportSize({ width: 1280, height: 840 });
  await page.waitForFunction(() => document.documentElement.dataset.viewport === "full-desktop");
  assert.equal((await page.locator(".launch-warnings").innerText()).includes(shared), false,
    "a failed latest preview must not restore the obsolete shared-directory warning");

  await isolation.uncheck();
  await page.waitForFunction(() =>
    document.querySelector(".launch-warnings")?.textContent?.includes("共用工作目录"));
  assert.deepEqual((await items()).map((item) => item.trim()), [shared, lock],
    "retry must restore both current warnings as separate items");
  await isolation.check();
  await page.waitForFunction(() =>
    document.querySelector(".launch-warnings")?.textContent?.includes("index.lock")
      && !document.querySelector(".launch-warnings")?.textContent?.includes("共用工作目录"));
  assert.deepEqual((await items()).map((item) => item.trim()), [lock]);
} finally {
  await browser.close();
}
console.log("Launch warning items and obsolete preview handling e2e ok");
