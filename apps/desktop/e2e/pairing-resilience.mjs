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

  const exercisePairingOperation = async (op, selector, closesDialog = false) => {
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
    await page.locator(selector).click({ clickCount: 2 });
    try {
      assert.ok(await page.locator(selector).isDisabled(), `${op} must disable its submit control immediately`);
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
    await page.click(selector);
    assert.ok((await retry).ok());
    assert.equal(calls, 2, "explicit retry must send one new request");
    await page.unroute("**/rpc", intercept);
    if (closesDialog) await page.waitForFunction(() => !document.querySelector(".pairing-sheet"));
  };

  await exercisePairingOperation("beginPairingOffer", ".pairing-sheet button[data-act='show-offer']");
  await exercisePairingOperation("pairRemoteHost", ".pairing-sheet button[data-act='connect-host']", true);

  await page.click("button[data-act='settings']");
  await page.waitForSelector("[data-settings-section='host'] [data-paired-clients]");
  const revoke = page.locator("[data-settings-section='host'] button[data-act='revoke']").first();
  const clientName = (await revoke.locator("xpath=ancestor::*[contains(@class, 'client-row')]").locator(":scope > span").textContent())?.trim();
  assert.ok(clientName);
  await revoke.click();
  const confirmation = page.locator("[data-dialog-id='revoke-client']");
  await confirmation.waitFor();
  assert.match((await confirmation.textContent()) ?? "", new RegExp(clientName));
  assert.match((await confirmation.textContent()) ?? "", /重新配对|pair again/i);

  let releaseRevoke;
  const revokeGate = new Promise((resolve) => { releaseRevoke = resolve; });
  let revokeCalls = 0;
  const interceptRevoke = async (route) => {
    if (route.request().postDataJSON()?.op !== "revokeClient") return route.continue();
    revokeCalls += 1;
    if (revokeCalls !== 1) return route.continue();
    await revokeGate;
    await route.fulfill({ status: 503, contentType: "application/json", body: JSON.stringify({ message: "revokeClient retry needed" }) });
  };
  await page.route("**/rpc", interceptRevoke);
  await page.$eval("button[data-act='confirm-revoke-client']", (button) => {
    button.click();
    button.click();
  });
  releaseRevoke();
  await confirmation.getByText("revokeClient retry needed").waitFor();
  assert.equal(revokeCalls, 1, "revoke double click must send one request");
  const retry = page.waitForResponse((response) => response.request().postDataJSON()?.op === "revokeClient");
  await page.click("button[data-act='confirm-revoke-client']");
  assert.ok((await retry).ok());
  assert.equal(revokeCalls, 2);
  await page.unroute("**/rpc", interceptRevoke);
  await page.waitForFunction(() => !document.querySelector("[data-dialog-id='revoke-client']"));
  assert.equal(await page.locator("[data-settings-section='host'] button[data-act='revoke']").count(), 0);

  console.log("pairing resilience e2e ok");
} finally {
  await browser.close();
}
