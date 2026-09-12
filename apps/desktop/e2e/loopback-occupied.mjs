import { openIssue100Browser } from "./issue-100-harness.mjs";

const occupiedPort = process.env.OCCUPIED_PORT;
if (!occupiedPort) throw new Error("missing OCCUPIED_PORT");

const { browser, page } = await openIssue100Browser({ screenshotDir: "" });
await page.waitForSelector(".lanes");
const notice = (await page.locator(".project-board > .notice").first().textContent())?.replace(/\s+/g, " ").trim() ?? "";
if (!notice.includes(occupiedPort) || !notice.includes("网页入口") || !notice.includes("桌面窗口可以继续用")) {
  throw new Error(`occupied loopback port must explain the unavailable browser entry while the Client remains usable: ${notice}`);
}

await browser.close();
console.log("loopback occupied e2e ok");
