import { openIssue100Browser } from "./issue-100-harness.mjs";

const { browser, capture, page } = await openIssue100Browser();
await page.waitForSelector("button[data-act='new-run']");
await page.click("button[data-act='new-run']");
await page.waitForSelector(".launch-sheet .agent-picks");

const unavailable = page.locator(".agent-choice-unavailable").first();
await unavailable.waitFor();
const text = (await unavailable.textContent())?.replace(/\s+/g, " ").trim() ?? "";
if (!text.includes("Grok Build") || !text.includes("找不到 grok") || !text.includes("已搜 PATH") || !text.includes("已知安装位置")) {
  throw new Error(`missing Agent should explain the command and searched locations: ${text}`);
}
if (!(await unavailable.locator("button[data-act='pick-agent']").isDisabled())) {
  throw new Error("missing Agent launch action must remain disabled");
}
await capture("issue-100-agent-unavailable-1280x840.png");

await browser.close();
console.log("agent unavailable e2e ok");
