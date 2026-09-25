import assert from "node:assert/strict";
import { openIssue100Browser } from "./issue-100-harness.mjs";

const { browser, page } = await openIssue100Browser();
try {
  const dialogRoot = (id) => page.locator(`[data-dialog-root='true'][data-dialog-id='${id}']`);
  const assertDialogSemantics = async (id) => {
    const root = dialogRoot(id);
    await root.waitFor();
    const panel = root.getByRole("dialog");
    await page.waitForTimeout(180);
    assert.equal(await panel.getAttribute("aria-modal"), "true");
    assert.ok(await panel.getAttribute("aria-labelledby"));
    return { root, panel };
  };
  const width = async (panel) => Math.round((await panel.boundingBox())?.width ?? 0);

  const registerTrigger = page.locator(".side button[data-act='register']");
  await registerTrigger.click();
  let { root: projectRoot, panel: projectPanel } = await assertDialogSemantics("project-form");
  const formWidth = await width(projectPanel);
  assert.ok(formWidth <= 560 && formWidth >= 540, `form dialog should respect the 560px maximum, got ${formWidth}`);
  assert.equal(await page.evaluate(() => document.activeElement?.id), "project-name", "form dialog should focus the first field");
  const projectClose = projectPanel.locator(".dialog-close");
  await projectClose.focus();
  await page.keyboard.press("Shift+Tab");
  assert.equal(await page.evaluate(() => document.activeElement?.getAttribute("type")), "submit", "Shift+Tab should wrap within the dialog");
  await page.keyboard.press("Escape");
  await projectRoot.waitFor({ state: "detached" });
  assert.equal(await registerTrigger.evaluate((node) => node === document.activeElement), true, "Escape should return focus to the trigger");

  await registerTrigger.click();
  projectRoot = dialogRoot("project-form");
  await projectRoot.click({ position: { x: 2, y: 2 } });
  await projectRoot.waitFor({ state: "detached" });
  assert.equal(await registerTrigger.evaluate((node) => node === document.activeElement), true, "backdrop dismiss should return focus");

  await registerTrigger.click();
  projectPanel = dialogRoot("project-form").getByRole("dialog");
  const dismissActions = projectPanel.locator("button[data-act='dismiss-dialog']");
  assert.equal(await dismissActions.count(), 2, "close and cancel should share the same dismiss action");
  await dismissActions.last().click();
  await dialogRoot("project-form").waitFor({ state: "detached" });

  await page.click(".side button[data-act='new-run']");
  let launch = await assertDialogSemantics("launch");
  const picker = launch.panel.locator("button[data-act='select-agent']");
  if (await picker.count()) {
    const pickerWidth = await width(launch.panel);
    assert.ok(pickerWidth <= 640 && pickerWidth >= 540, `Agent picker should use the form-sized dialog, got ${pickerWidth}`);
    await launch.panel.locator("button[data-act='select-agent']:not([disabled])").first().click();
    await launch.panel.locator("button[data-act='next-agent']").click();
    await page.waitForSelector("textarea[data-field='openingText']");
    launch = await assertDialogSemantics("launch");
  }
  const wideWidth = await width(launch.panel);
  assert.ok(wideWidth <= 880 && wideWidth >= 840, `Run launch should respect the 880px maximum, got ${wideWidth}`);
  await page.keyboard.press("Escape");
  await launch.root.waitFor({ state: "detached" });

  await page.click("button[data-act='pair']");
  const pairing = await assertDialogSemantics("pairing");
  assert.equal(await pairing.root.locator("[data-paired-clients], button[data-act='revoke']").count(), 0, "pairing dialog must not contain paired Clients");
  await pairing.panel.locator(".dialog-close").click();

  const activeProject = page.locator(".side .project-row", { hasText: "dialog-project" });
  await activeProject.locator("button[data-act='project-menu']").click();
  await activeProject.locator("button[data-act='remove-project']").click();
  const remove = await assertDialogSemantics("remove-project");
  const confirmWidth = await width(remove.panel);
  assert.ok(confirmWidth <= 420 && confirmWidth >= 400, `confirmation should respect the 420px maximum, got ${confirmWidth}`);
  assert.match((await remove.panel.textContent()) ?? "", /活跃 Run：1|Active Runs .*: 1/);
  assert.equal(await remove.panel.locator("button[data-act='confirm-remove']").count(), 0, "active Run must keep Project removal blocked");
  assert.equal(await page.evaluate(() => document.activeElement?.getAttribute("data-act")), "dismiss-dialog", "confirmation should focus cancel");
  await page.keyboard.press("Escape");
  assert.equal(await dialogRoot("remove-project").count(), 1, "Escape must not dismiss a confirmation");
  await remove.root.click({ position: { x: 2, y: 2 } });
  assert.equal(await dialogRoot("remove-project").count(), 1, "backdrop must not dismiss a confirmation");
  await remove.panel.locator(".dialog-actions button[data-act='dismiss-dialog']").click();

  await page.click("button[data-act='settings']");
  const settingsClients = page.locator("[data-settings-section='host'] [data-paired-clients]");
  await settingsClients.waitFor();
  assert.equal(await settingsClients.locator("button[data-act='revoke']").count(), 1, "paired Clients should live in Host settings");
  await settingsClients.locator("button[data-act='revoke']").click();
  const revoke = await assertDialogSemantics("revoke-client");
  assert.match((await revoke.panel.textContent()) ?? "", /dialog client/);
  assert.match((await revoke.panel.textContent()) ?? "", /重新配对|pair again/i);
  await page.keyboard.press("Escape");
  assert.equal(await dialogRoot("revoke-client").count(), 1);
  await revoke.panel.locator(".dialog-actions button[data-act='dismiss-dialog']").click();

  await page.click("button[data-act='quit']");
  const quit = await assertDialogSemantics("quit-host");
  assert.match((await quit.panel.textContent()) ?? "", /活跃 Run：1|Active Runs .*: 1/);
  await quit.panel.locator(".dialog-actions button[data-act='dismiss-dialog']").click();
  await page.waitForFunction(() => document.activeElement?.getAttribute("data-act") === "quit");
  await page.click("button[data-act='return-page']");

  const stop = page.locator("[data-lane='inProgress'] .issue-card button[data-act='stop-run']").first();
  await stop.click();
  const stopDialog = await assertDialogSemantics("stop-run");
  assert.match((await stopDialog.panel.textContent()) ?? "", /Grok Build.*you\/dialogs#1|you\/dialogs#1.*Grok Build/);
  assert.equal(await page.evaluate(() => document.activeElement?.getAttribute("data-act")), "dismiss-dialog");
  await page.keyboard.press("Escape");
  assert.equal(await dialogRoot("stop-run").count(), 1);
  await stopDialog.root.click({ position: { x: 2, y: 2 } });
  assert.equal(await dialogRoot("stop-run").count(), 1);
  await stopDialog.panel.locator(".dialog-actions button[data-act='dismiss-dialog']").click();

  await page.setViewportSize({ width: 390, height: 844 });
  await page.click("button[data-act='mobile-drawer']");
  await page.click("[data-dialog-id='mobile-drawer'] button[data-act='register']");
  const mobileProject = await assertDialogSemantics("project-form");
  const mobileBox = await mobileProject.panel.boundingBox();
  assert.ok(mobileBox);
  assert.ok(Math.abs(mobileBox.x - 12) <= 1, `mobile left gutter should be 12px, got ${mobileBox.x}`);
  assert.ok(Math.abs(390 - (mobileBox.x + mobileBox.width) - 12) <= 1, "mobile right gutter should be 12px");
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth), true, "mobile dialogs must not scroll horizontally");
  await mobileProject.panel.locator(".dialog-actions button[data-act='dismiss-dialog']").click();

  await page.setViewportSize({ width: 390, height: 500 });
  await page.click("button[data-act='mobile-drawer']");
  await page.click("[data-dialog-id='mobile-drawer'] button[data-act='register']");
  const compact = await assertDialogSemantics("project-form");
  const before = await compact.panel.evaluate((panel) => {
    const header = panel.querySelector(".dialog-header").getBoundingClientRect();
    const footer = panel.querySelector(".dialog-actions").getBoundingClientRect();
    const content = panel.querySelector(".dialog-content");
    return { headerTop: header.top, footerBottom: footer.bottom, canScroll: content.scrollHeight > content.clientHeight };
  });
  assert.equal(before.canScroll, true, "compact dialog content should scroll independently");
  await compact.panel.locator(".dialog-content").evaluate((node) => { node.scrollTop = node.scrollHeight; });
  const after = await compact.panel.evaluate((panel) => ({
    headerTop: panel.querySelector(".dialog-header").getBoundingClientRect().top,
    footerBottom: panel.querySelector(".dialog-actions").getBoundingClientRect().bottom,
  }));
  assert.deepEqual(after, { headerTop: before.headerTop, footerBottom: before.footerBottom }, "header and actions must stay fixed while content scrolls");
  await compact.panel.locator(".dialog-actions button[data-act='dismiss-dialog']").click();

  await page.setViewportSize({ width: 1280, height: 840 });
  await page.locator("[data-lane='inProgress'] .issue-card button[data-act='stop-run']").first().click();
  await page.click("[data-dialog-root='true'][data-dialog-id='stop-run'] button[data-act='confirm-stop-run']");
  await page.waitForFunction(() => !document.querySelector("[data-dialog-root='true'][data-dialog-id='stop-run']"));

  console.log("dialog contracts e2e ok");
} finally {
  await browser.close();
}
