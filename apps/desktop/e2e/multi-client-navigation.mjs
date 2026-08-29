import { chromium } from "playwright";
import { installDeterministicHostProtocol } from "./visual-regression.mjs";

const url = process.env.BOARD_URL;
if (!url) throw new Error("missing BOARD_URL");

const browser = await chromium.launch({ headless: true });
const firstContext = await browser.newContext({ locale: "zh-CN", viewport: { width: 1280, height: 840 } });
const secondContext = await browser.newContext({ locale: "zh-CN", viewport: { width: 1280, height: 840 } });
const first = await firstContext.newPage();
const second = await secondContext.newPage();

for (const page of [first, second]) {
  await installDeterministicHostProtocol(page, url);
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.waitForSelector(".lanes");
}

await first.click(".issue-card:has-text('#1') .issue-card-main");
await first.waitForSelector(".detail-hd:has-text('first client issue')");

await second.click(".issue-card:has-text('#2') .issue-card-main");
await second.waitForSelector(".detail-hd:has-text('second client issue')");

for (const page of [first, second]) {
  const tick = page.waitForResponse((response) =>
    response.url().endsWith("/rpc") && response.request().postData()?.includes('"op":"tick"'),
  );
  await page.evaluate(() => window.__RUN_INTERVAL_CALLBACKS__());
  await tick;
}

const firstTitle = await first.$eval(".detail-hd", (node) => node.textContent?.replace(/\s+/g, " ").trim());
const secondTitle = await second.$eval(".detail-hd", (node) => node.textContent?.replace(/\s+/g, " ").trim());
if (!firstTitle?.includes("first client issue")) {
  throw new Error(`first Client lost its Issue selection: ${firstTitle}`);
}
if (!secondTitle?.includes("second client issue")) {
  throw new Error(`second Client lost its Issue selection: ${secondTitle}`);
}

const removeFirstIssueFromRefresh = async (route) => {
  let request;
  try {
    request = route.request().postDataJSON();
  } catch {
    await route.continue();
    return;
  }
  if (request?.op !== "tick" || request.clientView?.selectedIssueId !== "you/multi-client#1") {
    await route.continue();
    return;
  }
  const response = await route.fetch();
  const result = await response.json();
  result.snapshot.board.selected = null;
  result.snapshot.board.refresh = {
    kind: "ready",
    fetchedAtMs: Date.now(),
    nextRefreshInMs: 60_000,
  };
  for (const cards of Object.values(result.snapshot.board.columns ?? {})) {
    if (Array.isArray(cards)) {
      const index = cards.findIndex((card) => card.id === "you/multi-client#1");
      if (index >= 0) cards.splice(index, 1);
    }
  }
  await route.fulfill({ response, json: result });
};
await first.route("**/rpc", removeFirstIssueFromRefresh);
const deletionTick = first.waitForResponse((response) =>
  response.url().endsWith("/rpc") && response.request().postData()?.includes('"op":"tick"'),
);
await first.evaluate(() => window.__RUN_INTERVAL_CALLBACKS__());
await deletionTick;
await first.waitForFunction(
  () => !document.querySelector(".detail-hd")?.textContent?.includes("first client issue"),
  undefined,
  { timeout: 1_000 },
);
const firstClientView = await first.evaluate(() => {
  const entry = Object.entries(localStorage).find(([key]) => key.includes("client-view"));
  return entry ? JSON.parse(entry[1]) : null;
});
if (firstClientView?.selectedIssueId !== null) {
  throw new Error(`a successful refresh must clear only the missing Issue selection: ${JSON.stringify(firstClientView)}`);
}
await first.unroute("**/rpc", removeFirstIssueFromRefresh);

const secondAfterDeletion = await second.$eval(".detail-hd", (node) => node.textContent?.replace(/\s+/g, " ").trim());
if (!secondAfterDeletion?.includes("second client issue")) {
  throw new Error(`one Client clearing a deleted Issue must not move another Client: ${secondAfterDeletion}`);
}

await first.click('[data-act="open-usage"]');
await first.waitForSelector(".usage-page");
const usageTick = second.waitForResponse((response) =>
  response.url().endsWith("/rpc") && response.request().postData()?.includes('"op":"tick"'),
);
await second.evaluate(() => window.__RUN_INTERVAL_CALLBACKS__());
await usageTick;
if (await second.locator(".usage-page").count()) {
  throw new Error("one Client opening Usage must not open it in another Client");
}
await first.click('[data-act="close-usage"]');

await first.click('.issue-card:has-text("#2") [data-act="execute-run"]');
await first.waitForSelector(".launch-sheet");
const secondTick = second.waitForResponse((response) =>
  response.url().endsWith("/rpc") && response.request().postData()?.includes('"op":"tick"'),
);
await second.evaluate(() => window.__RUN_INTERVAL_CALLBACKS__());
await secondTick;
if (await second.locator(".launch-sheet").count()) {
  throw new Error("one Client opening the Run launch form must not open it in another Client");
}

await browser.close();
console.log("multi-client navigation e2e ok");
