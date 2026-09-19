import assert from "node:assert/strict";
import { openIssue100Browser } from "./issue-100-harness.mjs";

const { browser, page } = await openIssue100Browser({ screenshotDir: "" });
try {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.locator('[data-act="execute-run"]').first().click();
  await page.locator('[data-act="select-agent"]').first().click();
  await page.locator('[data-act="next-agent"]').click();
  await page.locator('form[data-form="launch"] button[type="submit"]').click();
  await page.waitForSelector('.launch-sheet', { state: 'hidden' });
  await page.getByRole('button', { name: 'Run', exact: true }).click();
  await page.waitForFunction(() => document.querySelector('.mobile-run-output')?.textContent.includes('TASKBOARD_OUTPUT_OK'));
  const readable = async () => {
    const text = await page.locator('.mobile-run-output').textContent();
    assert.match(text, /成功 TASKBOARD_OUTPUT_OK/);
    assert.doesNotMatch(text, /[\u001b\u0007]|Loading\.\.\.|private terminal title/);
  };
  await readable();
  await page.getByRole('button', { name: '停止', exact: true }).click();
  await page.locator("[data-dialog-id='stop-run'] button[data-act='confirm-stop-run']").click();
  await page.waitForSelector('.mobile-board-view');
  await page.getByRole('button', { name: 'Run', exact: true }).click();
  await page.waitForSelector('.mobile-run-output');
  await readable();
  assert.equal(await page.getByRole('button', { name: '停止', exact: true }).isDisabled(), true);
} finally {
  await browser.close();
}
console.log('mobile readable terminal output e2e ok');
