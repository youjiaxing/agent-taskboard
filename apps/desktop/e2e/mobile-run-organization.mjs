import assert from "node:assert/strict";
import { assertNecessaryTextContrast, assertShellRegionsDoNotOverlap } from "./visual-regression.mjs";

async function rpc(page, protocol, body) {
  return page.evaluate(async ({ protocol, body }) => {
    const response = await fetch(`${protocol.replace(/\/$/, "")}/rpc`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(body),
    });
    if (!response.ok) throw new Error(await response.text());
    return response.json();
  }, { protocol, body });
}

async function openDrawer(page) {
  if (await page.locator("[data-dialog-id='mobile-drawer']").count()) return;
  await page.click("button[data-act='mobile-drawer']");
  await page.waitForSelector("[data-dialog-id='mobile-drawer']");
}

async function openRunMenu(page, runId, scope = "") {
  const prefix = scope ? `${scope} ` : "";
  const trigger = page.locator(`${prefix}[data-act='mobile-run-menu'][data-id='${runId}']`);
  await trigger.click();
  await page.waitForSelector(`${prefix}.run-organization-menu`);
  return page.locator(`${prefix}.run-organization-menu`);
}

async function assertTouchTargets(page, label) {
  const small = await page.evaluate(() => {
    const selector = "button, select, summary, [role='button'], input:not([type='checkbox']):not([type='hidden'])";
    return [...document.querySelectorAll(selector)]
      .filter((node) => {
        const rect = node.getBoundingClientRect();
        const style = getComputedStyle(node);
        if (style.display === "none" || style.visibility === "hidden" || rect.width === 0 || rect.height === 0) return false;
        if (node.closest(".xterm") || node.closest(".issue-markdown")) return false;
        const wide = node.tagName === "INPUT" || node.tagName === "SELECT";
        return rect.height < 43.5 || (!wide && rect.width < 43.5);
      })
      .map((node) => `${node.dataset.act ?? node.className}:${Math.round(node.getBoundingClientRect().width)}x${Math.round(node.getBoundingClientRect().height)}`);
  });
  assert.deepEqual(small, [], `${label} touch targets must be at least 44px`);
}

async function assertMobileShell(page, label) {
  assert.equal(await page.locator(".side, .fixed-right-rail, .fixed-changes-panel, [data-global-action='changes']").count(), 0, `${label} must not render desktop panels`);
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth), true, `${label} must not overflow horizontally`);
  await assertShellRegionsDoNotOverlap(page);
  await assertNecessaryTextContrast(page, label);
  await assertTouchTargets(page, label);
}

export async function runMobileRunOrganizationJourney(page, options) {
  const {
    url,
    mobileProjectId,
    mobileBoundRunId,
    mobileActiveRunId,
    mobileEndedRunId,
    mobileRemovedProjectId,
    mobileRemovedRunId,
  } = options;

  await page.setViewportSize({ width: 390, height: 844 });
  await rpc(page, url, { op: "setRunPinned", runId: mobileActiveRunId, pinned: true });
  await rpc(page, url, { op: "setRunPinned", runId: mobileEndedRunId, pinned: true });
  await page.reload({ waitUntil: "domcontentloaded" });
  await page.waitForSelector("button[data-act='mobile-drawer']");

  await openDrawer(page);
  const drawerActions = await page.$$eval("[data-dialog-id='mobile-drawer'] [data-act]", (nodes) => nodes.map((node) => node.dataset.act));
  assert.equal(drawerActions.includes("open-run-pinned"), false);
  assert.equal(drawerActions.includes("open-run-archive"), true);
  await page.click(`[data-dialog-id='mobile-drawer'] [data-act='focus-project'][data-id='${mobileProjectId}']`);
  await page.waitForSelector(".mobile-board-view");
  await openDrawer(page);
  await page.click("[data-dialog-id='mobile-drawer'] [data-act='mobile-history-entry']");
  await page.waitForSelector(".mobile-project-history");
  const mobileHistoryIds = await page.$$eval(".mobile-project-history .workspace-run-history-item", (nodes) => nodes.map((node) => ({
    id: node.dataset.id,
    pinned: node.dataset.pinned,
  })));
  assert.equal(mobileHistoryIds.find((run) => run.id === mobileActiveRunId)?.pinned, "true");
  assert.equal(mobileHistoryIds.find((run) => run.id === mobileEndedRunId)?.pinned, "true");
  assert.equal(mobileHistoryIds.find((run) => run.id === mobileBoundRunId)?.pinned, "false");
  assert.ok(mobileHistoryIds.findIndex((run) => run.id === mobileEndedRunId) < mobileHistoryIds.findIndex((run) => run.id === mobileBoundRunId), "pinned history must precede unpinned history");
  assert.equal(await page.locator(`.workspace-run-history-item[data-id='${mobileActiveRunId}'] .run-pin-marker`).count(), 1, "mobile Project history should show the pin marker");

  let menu = await openRunMenu(page, mobileActiveRunId, `.mobile-project-history .workspace-run-history-item[data-id='${mobileActiveRunId}']`);
  assert.equal(await menu.locator("[data-act='archive-run']").count(), 0, "active waiting Run must not offer archive");
  assert.equal(await menu.locator("[data-act='set-run-pinned']").count(), 1);
  const activeMenuTrigger = page.locator(`.mobile-project-history .workspace-run-history-item[data-id='${mobileActiveRunId}'] [data-act='mobile-run-menu']`);
  assert.match((await activeMenuTrigger.getAttribute("aria-label")) ?? "", /Run/);
  await page.keyboard.press("Escape");
  await page.waitForFunction(() => !document.querySelector(".run-organization-menu"));
  assert.equal(await activeMenuTrigger.evaluate((node) => node === document.activeElement), true, "Escape should close the Run menu and restore trigger focus");
  menu = await openRunMenu(page, mobileEndedRunId, `.mobile-project-history .workspace-run-history-item[data-id='${mobileEndedRunId}']`);
  assert.equal(await menu.locator("[data-act='archive-run']").count(), 1, "ended Run should offer archive");
  await page.click(`.mobile-project-history .workspace-run-history-item[data-id='${mobileEndedRunId}'] [data-act='mobile-run-menu']`);

  await page.click(`.mobile-project-history .workspace-run-history-item[data-id='${mobileActiveRunId}'] .workspace-run-history-main`);
  await page.waitForSelector(`[data-terminal-surface='readable'][data-run='${mobileActiveRunId}']`);
  await page.waitForFunction(() => document.querySelector(".mobile-run-output")?.textContent?.includes("mobile active output"));
  assert.match((await page.textContent(".mobile-run-output")) ?? "", /mobile active output/);
  assert.match((await page.textContent(".mobile-output-panel")) ?? "", /等待操作/);
  await page.fill("[data-mobile-run-input] input[name='text']", "mobile draft survives organization navigation");
  await page.click("button[data-act='mobile-project-history-return']");
  await page.waitForSelector(".mobile-project-history");
  await page.click(`.mobile-project-history .workspace-run-history-item[data-id='${mobileActiveRunId}'] .workspace-run-history-main`);
  await page.waitForSelector(`[data-terminal-surface='readable'][data-run='${mobileActiveRunId}']`);
  assert.equal(await page.inputValue("[data-mobile-run-input] input[name='text']"), "mobile draft survives organization navigation");
  await page.click("button[data-act='mobile-project-history-return']");
  await page.waitForSelector(".mobile-project-history");

  await openDrawer(page);
  await page.click(`[data-dialog-id='mobile-drawer'] [data-act='focus-project'][data-id='${mobileProjectId}']`);
  await page.waitForSelector(".mobile-board-view");
  await openDrawer(page);
  await page.click("[data-dialog-id='mobile-drawer'] [data-act='mobile-history-entry']");
  await page.waitForSelector(".mobile-project-history");
  assert.equal(await page.locator(`.mobile-project-history .workspace-run-history-item[data-id='${mobileActiveRunId}']`).count(), 1);
  assert.equal(await page.locator(`.mobile-project-history .workspace-run-history-item[data-id='${mobileEndedRunId}']`).count(), 1);
  menu = await openRunMenu(page, mobileActiveRunId, `.mobile-project-history .workspace-run-history-item[data-id='${mobileActiveRunId}']`);
  assert.equal(await menu.locator("[data-act='archive-run']").count(), 0);
  await page.click(`.mobile-project-history .workspace-run-history-item[data-id='${mobileActiveRunId}'] [data-act='mobile-run-menu']`);
  menu = await openRunMenu(page, mobileEndedRunId, `.mobile-project-history .workspace-run-history-item[data-id='${mobileEndedRunId}']`);
  await menu.locator("[data-act='archive-run']").click();
  await page.waitForFunction((runId) => !document.querySelector(`.mobile-project-history .workspace-run-history-item[data-id='${runId}']`), mobileEndedRunId);

  await openDrawer(page);
  await page.click("[data-dialog-id='mobile-drawer'] [data-act='open-run-archive']");
  await page.waitForSelector(".mobile-run-archive-page");
  await page.selectOption("select[data-archive-filter='project']", mobileProjectId);
  await page.waitForSelector(`[data-archived-run='${mobileEndedRunId}']`);
  await page.click(`[data-archived-run='${mobileEndedRunId}'] .archive-run-main`);
  assert.match((await page.textContent(".archive-output pre")) ?? "", /mobile ended output/);
  menu = await openRunMenu(page, mobileEndedRunId, `[data-archived-run='${mobileEndedRunId}']`);
  await menu.locator("[data-act='restore-run']").click();
  await page.waitForFunction((runId) => !document.querySelector(`[data-archived-run='${runId}']`), mobileEndedRunId);
  assert.equal(await page.locator(".mobile-run-archive-page").count(), 1, "restore must remain on mobile archive page");
  assert.equal(await page.locator("[data-terminal-panel]").count(), 0, "restore must not open a terminal");

  await page.selectOption("select[data-archive-filter='project']", mobileRemovedProjectId);
  await page.waitForSelector(`[data-archived-run='${mobileRemovedRunId}']`);
  menu = await openRunMenu(page, mobileRemovedRunId, `[data-archived-run='${mobileRemovedRunId}']`);
  await menu.locator("[data-act='restore-run']").click();
  const restoreDialog = page.locator("[data-dialog-id='restore-run'] [role='dialog']");
  await restoreDialog.waitFor();
  const confirmationText = (await restoreDialog.textContent()) ?? "";
  assert.match(confirmationText, /mobile-removed/);
  assert.match(confirmationText, new RegExp(mobileRemovedProjectId.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")));
  await page.click("[data-dialog-id='restore-run'] [data-act='dismiss-dialog']");
  await page.waitForFunction(() => !document.querySelector("[data-dialog-id='restore-run']"));
  assert.equal(await page.locator(`[data-archived-run='${mobileRemovedRunId}']`).count(), 1, "cancel must leave the archived Run unchanged");

  menu = await openRunMenu(page, mobileRemovedRunId, `[data-archived-run='${mobileRemovedRunId}']`);
  await menu.locator("[data-act='restore-run']").click();
  const conflictSnapshot = (await rpc(page, url, { op: "snapshot" })).snapshot;
  let rejectRestoreOnce = true;
  await page.route("**/rpc", async (route) => {
    const request = route.request().postDataJSON();
    if (rejectRestoreOnce && request?.op === "restoreRun" && request.confirmProjectRecreate) {
      rejectRestoreOnce = false;
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          snapshot: conflictSnapshot,
          process: "keep-running",
          runRestore: { status: "conflict", runId: mobileRemovedRunId, projectId: mobileRemovedProjectId, reason: "tombstone-revision-changed" },
        }),
      });
      return;
    }
    await route.continue();
  });
  await page.click("[data-dialog-id='restore-run'] [data-act='confirm-restore-run']");
  await page.waitForSelector("[data-dialog-id='restore-run'] [role='alert']");
  assert.match((await restoreDialog.textContent()) ?? "", /墓碑已经变化/);
  await page.click("[data-dialog-id='restore-run'] [data-act='dismiss-dialog']");
  await page.unroute("**/rpc");
  menu = await openRunMenu(page, mobileRemovedRunId, `[data-archived-run='${mobileRemovedRunId}']`);
  await menu.locator("[data-act='restore-run']").click();
  await page.click("[data-dialog-id='restore-run'] [data-act='confirm-restore-run']");
  await page.waitForFunction(() => !document.querySelector("[data-dialog-id='restore-run']"));
  await page.waitForSelector(".archive-restored");
  assert.equal(await page.locator("[data-terminal-panel]").count(), 0);

  await openDrawer(page);
  await page.click(`[data-dialog-id='mobile-drawer'] [data-act='focus-project'][data-id='${mobileProjectId}']`);
  await page.waitForSelector(".mobile-board-view");
  await page.click(`[data-act='focus-issue'][data-id='you/mobile#1']`);
  await page.waitForSelector(".mobile-workspace-view");
  await page.click(".mobile-section-switch [data-act='mobile-workspace-section'][data-id='runs']");
  await page.waitForSelector(`.workspace-run-history-item[data-id='${mobileBoundRunId}']`);
  assert.equal(await page.locator(`.workspace-run-history-item[data-id='${mobileActiveRunId}']`).count(), 0, "unbound Run must not enter Issue history");
  menu = await openRunMenu(page, mobileBoundRunId, `.workspace-run-history-item[data-id='${mobileBoundRunId}']`);
  assert.equal(await menu.locator("[data-act='archive-run']").count(), 1, "Issue history ended Run should offer archive");
  await page.click(`.workspace-run-history-item[data-id='${mobileBoundRunId}'] [data-act='mobile-run-menu']`);

  await openDrawer(page);
  await page.click("[data-dialog-id='mobile-drawer'] [data-act='open-run-archive']");
  await page.waitForSelector(".mobile-run-archive-page");
  let recovery = true;
  await page.route("**/rpc", async (route) => {
    const request = route.request().postDataJSON();
    const response = await route.fetch();
    const result = await response.json();
    if (request?.op === "retryRunPersistenceLoad") recovery = false;
    if (recovery) {
      result.snapshot.capabilities.runPersistenceWrites = false;
      result.snapshot.capabilities.runPersistenceRecovery = true;
      result.snapshot.runPersistenceRecovery = { kind: "invalid-json", detail: "mobile fixture runs.json is invalid", retryOperation: "retryRunPersistenceLoad", writesBlocked: true };
    }
    try {
      await route.fulfill({ response, json: result });
    } catch (error) {
      if (!(error instanceof Error) || !error.message.includes("Route is already handled")) throw error;
    }
  });
  await page.reload({ waitUntil: "domcontentloaded" });
  await openDrawer(page);
  await page.click(`[data-dialog-id='mobile-drawer'] [data-act='focus-project'][data-id='${mobileProjectId}']`);
  await page.waitForSelector(".mobile-board-view");
  await openDrawer(page);
  await page.click("[data-dialog-id='mobile-drawer'] [data-act='mobile-history-entry']");
  await page.waitForSelector(".mobile-project-history");
  await page.waitForSelector("[data-run-persistence='recovery'] [data-act='retry-run-persistence']");
  menu = await openRunMenu(page, mobileActiveRunId, `.mobile-project-history .workspace-run-history-item[data-id='${mobileActiveRunId}']`);
  assert.equal(await menu.locator("[data-act='set-run-pinned']:not(:disabled), [data-act='archive-run']:not(:disabled)").count(), 0, "recovery mode must disable mobile organization writes");
  await page.click("[data-run-persistence='recovery'] [data-act='retry-run-persistence']");
  await page.waitForFunction(() => !document.querySelector("[data-run-persistence='recovery']"));
  await page.unroute("**/rpc");

  let failWriteOnce = true;
  await page.route("**/rpc", async (route) => {
    const request = route.request().postDataJSON();
    if (failWriteOnce && request?.op === "setRunPinned") {
      failWriteOnce = false;
      await route.fulfill({ status: 500, contentType: "application/json", body: JSON.stringify({ error: "run-persistence-write-failed", message: "mobile fixture write failed" }) });
      return;
    }
    await route.continue();
  });
  menu = await openRunMenu(page, mobileActiveRunId, `.mobile-project-history .workspace-run-history-item[data-id='${mobileActiveRunId}']`);
  await menu.locator("[data-act='set-run-pinned']").click();
  await page.waitForSelector("[data-run-persistence='write-error'] [data-act='retry-run-organization']");
  assert.equal(await page.locator(".mobile-project-history").count(), 1, "write failure must stay on the current mobile Project history");
  await page.click("[data-run-persistence='write-error'] [data-act='retry-run-organization']");
  await page.waitForFunction(() => !document.querySelector("[data-run-persistence='write-error']"));
  await page.unroute("**/rpc");

  await assertMobileShell(page, "mobile Run organization");
  console.log("Run organization mobile e2e ok");
}
