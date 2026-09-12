import assert from "node:assert/strict";
import { openIssue100Browser } from "./issue-100-harness.mjs";

const { browser, page, capture } = await openIssue100Browser();
try {
  await page.click('button[data-act="pair"]');
  await page.waitForSelector(".pairing-sheet");
  const address = "http://127.0.0.1:10529";
  const payload = process.env.PAIRING_PAYLOAD;
  assert.ok(payload);
  await page.fill("#pairing-address", address);
  await page.fill('[data-field="paste"]', payload);

  for (const [op, action] of [
    ["beginPairingOffer", "show-offer"],
    ["revokeClient", "revoke"],
    ["pairRemoteHost", "connect-host"],
  ]) {
    let release;
    const gate = new Promise((resolve) => { release = resolve; });
    let calls = 0;
    const intercept = async (route) => {
      if (route.request().postDataJSON()?.op !== op) return route.continue();
      calls += 1;
      if (calls !== 1) return route.continue();
      await gate;
      await route.fulfill({ status: 503, contentType: "application/json", body: JSON.stringify({ message: `${op} retry needed` }) });
    };
    await page.route("**/rpc", intercept);
    const button = `.pairing-sheet button[data-act="${action}"]`;
    await page.locator(button).click({ clickCount: 2 });
    try {
      assert.ok(await page.locator(button).isDisabled(), `${op} must disable its submit control immediately`);
      assert.ok(await page.locator("#pairing-address").isDisabled(), "pending pairing must freeze the submitted draft");
    } finally {
      release();
    }
    await page.waitForSelector(`.pairing-sheet :text('${op} retry needed')`);
    assert.equal(calls, 1, "double click must send one request");
    assert.equal(await page.inputValue("#pairing-address"), address);
    assert.equal(await page.inputValue('[data-field="paste"]'), payload);
    await page.locator(".pairing-sheet .form-feedback").scrollIntoViewIfNeeded();
    await capture("issue-100-pairing-retry-1280x840.png");
    const retry = page.waitForResponse((response) => response.request().postDataJSON()?.op === op);
    await page.click(button);
    assert.ok((await retry).ok());
    assert.equal(calls, 2, "explicit retry must send one new request");
    await page.unroute("**/rpc", intercept);
    if (op === "revokeClient") await page.waitForFunction(() => !document.querySelector('[data-act="revoke"]'));
  }
  await page.waitForFunction(() => !document.querySelector(".pairing-sheet"));
  console.log("pairing resilience e2e ok");
} finally {
  await browser.close();
}
