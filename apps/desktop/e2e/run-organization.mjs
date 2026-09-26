import assert from "node:assert/strict";
import { chromium } from "playwright";
import { assertNecessaryTextContrast, assertShellRegionsDoNotOverlap, installDeterministicHostProtocol } from "./visual-regression.mjs";
import { runMobileRunOrganizationJourney } from "./mobile-run-organization.mjs";

const url = process.env.BOARD_URL;
const pendingRunId = process.env.PENDING_RUN_ID;
const activeRunId = process.env.ACTIVE_RUN_ID;
const archivedRunId = process.env.ARCHIVED_RUN_ID;
const removedRunId = process.env.REMOVED_RUN_ID;
const removedProjectId = process.env.REMOVED_PROJECT_ID;
const remoteHostId = process.env.REMOTE_HOST_ID;
const remoteRunId = process.env.REMOTE_RUN_ID;
const mobileProjectId = process.env.MOBILE_PROJECT_ID;
const mobileBoundRunId = process.env.MOBILE_BOUND_RUN_ID;
const mobileActiveRunId = process.env.MOBILE_ACTIVE_RUN_ID;
const mobileEndedRunId = process.env.MOBILE_ENDED_RUN_ID;
const mobileRemovedProjectId = process.env.MOBILE_REMOVED_PROJECT_ID;
const mobileRemovedRunId = process.env.MOBILE_REMOVED_RUN_ID;
if (!url || !pendingRunId || !activeRunId || !archivedRunId || !removedRunId || !removedProjectId || !remoteHostId || !remoteRunId
  || !mobileProjectId || !mobileBoundRunId || !mobileActiveRunId || !mobileEndedRunId || !mobileRemovedProjectId || !mobileRemovedRunId) {
  throw new Error("missing Run organization browser fixture environment");
}

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ locale: "zh-CN", viewport: { width: 1280, height: 840 } });
const page = await context.newPage();
page.on("pageerror", (error) => console.error("pageerror", error));
page.on("console", (message) => {
  if (message.type() === "error") console.error("console", message.text());
});
await installDeterministicHostProtocol(page, url);
await page.goto(url, { waitUntil: "domcontentloaded" });

try {
  await page.waitForSelector(".side");
  const localHostId = await page.$eval(".host-picker, .host-line", () => "");
  await page.click("button[data-act='toggle-hosts']");
  const hosts = await page.$$eval(".host-picker button[data-act='focus-host']", (nodes) => nodes.map((node) => ({
    id: node.dataset.id,
    active: node.classList.contains("active"),
  })));
  const focusedLocalHostId = hosts.find((host) => host.active)?.id;
  assert.ok(focusedLocalHostId);
  assert.equal(hosts.some((host) => host.id === remoteHostId), true);
  await page.click("button[data-act='toggle-hosts']");
  void localHostId;

  const pinnedRows = await page.$$eval(".project-block .run-row[data-pinned='true']", (nodes) => nodes.map((node) => ({
    run: node.dataset.run,
    project: node.closest(".project-block")?.dataset.project,
  })));
  assert.deepEqual(pinnedRows.map((row) => row.run).sort(), [activeRunId, pendingRunId].sort(), "pinned Runs should remain visible in their owning Projects");
  assert.equal(new Set(pinnedRows.map((row) => row.project)).size, 2, "pinned Runs from different Projects must not share a Host-level group");
  assert.equal(await page.locator(`.run-row[data-run='${activeRunId}'] [data-act='archive-run']`).count(), 0, "active Run must not offer archive");
  assert.equal(await page.locator(`.run-row[data-run='${pendingRunId}'] [data-act='archive-run']`).count(), 1, "ended Run should offer archive");
  assert.equal(await page.locator(`.run-row[data-run='${activeRunId}'] .run-pin-marker`).count(), 1, "pinned Run should show a visible marker");

  await page.click(`.run-row[data-run='${pendingRunId}'] .run-main`);
  await page.waitForSelector(`[data-terminal-surface='readonly'][data-run='${pendingRunId}']`);
  await page.click(`.run-dock-hd [data-act='archive-run'][data-id='${pendingRunId}']`);
  await page.waitForSelector(".project-board");
  assert.equal(await page.locator("[data-terminal-panel]").count(), 0, "archiving the focused Run must clear its terminal surface");
  assert.equal(await page.locator(`.run-row[data-run='${pendingRunId}']`).count(), 0, "archived Run must leave ordinary navigation");
  assert.equal(await page.locator(".refresh-bar[data-kind='pending'] button[data-act='veto-advance']").count(), 1, "pending confirmation must remain vetoable");
  assert.equal(await page.locator(`.refresh-bar[data-kind='pending'] [data-act='focus-run']`).count(), 0, "pending confirmation must not navigate to the archived Run");

  await page.click("button[data-act='open-run-archive']");
  await page.waitForSelector(".run-archive-page");
  const hostArchived = await page.evaluate(async (protocol) => {
    const response = await fetch(`${protocol.replace(/\/$/, "")}/rpc`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ op: "listArchivedRuns" }),
    });
    return (await response.json()).archivedRuns.map((run) => run.id);
  }, url);
  const renderedArchived = await page.$$eval(".archive-run-row", (nodes) => nodes.map((node) => node.dataset.archivedRun));
  assert.deepEqual(renderedArchived, hostArchived, "archive page must preserve Host archivedAt ordering");
  const filterOptions = await page.$$eval("select[data-archive-filter='project'] option", (nodes) => nodes.map((node) => ({ value: node.value, text: node.textContent })));
  assert.equal(filterOptions.some((option) => option.value === removedProjectId && option.text.includes("已移除 Project")), true, "removed Projects must remain filterable");
  await page.selectOption("select[data-archive-filter='project']", removedProjectId);
  assert.deepEqual(await page.$$eval(".archive-run-row", (nodes) => nodes.map((node) => node.dataset.archivedRun)), [removedRunId]);
  await page.click(`.archive-run-row[data-archived-run='${removedRunId}'] .archive-run-main`);
  assert.match((await page.textContent(".archive-output pre")) ?? "", /legacy archived output/);

  const restoreTrigger = page.locator(`.archive-run-row[data-archived-run='${removedRunId}'] button[data-act='restore-run']`);
  await restoreTrigger.click();
  const restoreDialog = page.locator("[data-dialog-id='restore-run'] [role='dialog']");
  await restoreDialog.waitFor();
  const restoreText = (await restoreDialog.textContent()) ?? "";
  assert.match(restoreText, new RegExp(removedProjectId.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")));
  assert.match(restoreText, /legacy/);
  assert.match(restoreText, /work\/legacy/);
  assert.equal(await page.evaluate(() => document.activeElement?.getAttribute("data-act")), "dismiss-dialog", "dangerous restore should initially focus cancel");

  const conflictSnapshot = await page.evaluate(async (protocol) => {
    const response = await fetch(`${protocol.replace(/\/$/, "")}/rpc`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ op: "snapshot" }),
    });
    return (await response.json()).snapshot;
  }, url);
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
          runRestore: {
            status: "conflict",
            runId: removedRunId,
            projectId: removedProjectId,
            reason: "tombstone-revision-changed",
          },
        }),
      });
      return;
    }
    await route.continue();
  });
  await page.click("[data-dialog-id='restore-run'] button[data-act='confirm-restore-run']");
  await page.waitForSelector("[data-dialog-id='restore-run'] [role='alert']");
  assert.match((await restoreDialog.textContent()) ?? "", /墓碑已经变化/);
  await page.click("[data-dialog-id='restore-run'] button[data-act='dismiss-dialog']");
  await page.waitForFunction(() => !document.querySelector("[data-dialog-id='restore-run']"));
  assert.equal(await restoreTrigger.evaluate((node) => node === document.activeElement), true, "closing restore confirmation should return focus to its trigger");
  await page.unroute("**/rpc");

  await restoreTrigger.click();
  await page.click("[data-dialog-id='restore-run'] button[data-act='confirm-restore-run']");
  await page.waitForFunction(() => !document.querySelector("[data-dialog-id='restore-run']"));
  await page.waitForSelector(".archive-restored button[data-act='open-restored-run']");
  assert.equal(await page.locator(".run-archive-page").count(), 1, "restore should stay on the archive page");
  assert.equal(await page.locator("[data-terminal-panel]").count(), 0, "restore must not open a terminal");
  await page.click(".archive-restored button[data-act='open-restored-run']");
  await page.waitForSelector(`[data-terminal-surface='readonly'][data-run='${removedRunId}']`);
  await page.waitForSelector(`.focus-workspace-layout [data-act='archive-run'][data-id='${removedRunId}']`);
  await page.click("button[data-act='return-page']");
  await page.waitForSelector(".run-archive-page");

  await page.selectOption("select[data-archive-filter='project']", "");
  await page.click(`.archive-run-row[data-archived-run='${archivedRunId}'] button[data-act='restore-run']`);
  await page.waitForSelector(".archive-restored button[data-act='open-restored-run']");
  await page.waitForFunction((runId) => !document.querySelector(`.archive-run-row[data-archived-run='${runId}']`), archivedRunId);
  assert.equal(await page.locator(`.archive-run-row[data-archived-run='${archivedRunId}']`).count(), 0, "restored Run must leave the archive list");

  let stripCapabilityOnce = true;
  let simulateOldHost = false;
  await page.route("**/rpc", async (route) => {
    const request = route.request().postDataJSON();
    if (request?.op === "focusHost" && request.hostId === focusedLocalHostId) simulateOldHost = false;
    if (stripCapabilityOnce && request?.op === "focusHost" && request.hostId === remoteHostId) {
      stripCapabilityOnce = false;
      simulateOldHost = true;
    }
    if (simulateOldHost) {
      const response = await route.fetch();
      const result = await response.json();
      result.snapshot.capabilities = {};
      await route.fulfill({ response, json: result });
      return;
    }
    await route.continue();
  });
  await page.click("button[data-act='toggle-hosts']");
  await page.click(`.host-picker button[data-id='${remoteHostId}']`);
  await page.waitForFunction(() => !document.querySelector("button[data-act='open-run-archive']"));
  assert.equal(await page.locator("[data-pinned-runs]").count(), 0, "Run organization must not render a Host-level pinned group");
  await page.click("button[data-act='toggle-hosts']");
  await page.click(`.host-picker button[data-id='${focusedLocalHostId}']`);
  await page.waitForSelector(`.run-row[data-run='${activeRunId}']`);
  await page.click("button[data-act='toggle-hosts']");
  await page.click(`.host-picker button[data-id='${remoteHostId}']`);
  await page.waitForSelector(`.run-row[data-run='${remoteRunId}']`);
  assert.equal(await page.locator(`.run-row[data-run='${remoteRunId}'][data-pinned='true']`).count(), 1, "remote Host keeps its own Project-local pin");
  await page.unroute("**/rpc");
  await page.click("button[data-act='toggle-hosts']");
  await page.click(`.host-picker button[data-id='${focusedLocalHostId}']`);
  await page.waitForSelector(`.run-row[data-run='${activeRunId}']`);
  await page.click(`.run-row[data-run='${activeRunId}'] .run-main`);
  await page.waitForSelector(`[data-terminal-surface='live'][data-run='${activeRunId}']`);

  let recovery = true;
  await page.route("**/rpc", async (route) => {
    const request = route.request().postDataJSON();
    const response = await route.fetch();
    const result = await response.json();
    if (request?.op === "retryRunPersistenceLoad") recovery = false;
    if (recovery) {
      result.snapshot.capabilities.runPersistenceWrites = false;
      result.snapshot.capabilities.runPersistenceRecovery = true;
      result.snapshot.runPersistenceRecovery = {
        kind: "invalid-json",
        detail: "fixture runs.json is invalid",
        retryOperation: "retryRunPersistenceLoad",
        writesBlocked: true,
      };
    }
    await route.fulfill({ response, json: result });
  });
  await page.reload({ waitUntil: "domcontentloaded" });
  await page.waitForSelector("[data-run-persistence='recovery'] button[data-act='retry-run-persistence']");
  assert.equal(await page.locator(".project-block [data-run-organization]:not(:disabled)").count(), 0, "recovery mode must disable Run writes");
  assert.equal(await page.locator(".side button[data-act='new-run']:not(:disabled)").count(), 0, "recovery mode must disable new Run entry points");
  assert.equal(await page.locator("button[data-act='execute-run']:not(:disabled), button[data-act='continue-run']:not(:disabled), button[data-act='stop-run']:not(:disabled)").count(), 0, "recovery mode must disable desktop Run lifecycle writes");

  await page.setViewportSize({ width: 390, height: 844 });
  await page.reload({ waitUntil: "domcontentloaded" });
  const mobileStop = page.locator(".mobile-workspace-view button[data-act='stop-run']");
  await mobileStop.waitFor();
  assert.equal(await mobileStop.isEnabled(), true, "#183 recovery restrictions must not change mobile Run controls");
  await mobileStop.click();
  const mobileStopConfirm = page.locator("[data-dialog-id='stop-run'] button[data-act='confirm-stop-run']");
  await mobileStopConfirm.waitFor();
  assert.equal(await mobileStopConfirm.isEnabled(), true, "#183 recovery restrictions must not disable the mobile stop confirmation");
  await page.click("[data-dialog-id='stop-run'] button[data-act='dismiss-dialog']");

  await page.setViewportSize({ width: 1280, height: 840 });
  await page.reload({ waitUntil: "domcontentloaded" });
  await page.waitForSelector("[data-run-persistence='recovery'] button[data-act='retry-run-persistence']");
  await page.click("button[data-act='open-run-archive']");
  await page.waitForSelector(".run-archive-page [role='alert']");
  assert.equal(await page.locator(".run-archive-page .archive-empty").count(), 0, "recovery must not render damaged history as empty");
  await page.click("button[data-act='retry-run-persistence']");
  await page.waitForFunction(() => !document.querySelector("[data-run-persistence='recovery']"));
  await page.waitForSelector(".archive-run-row");
  await page.unroute("**/rpc");

  let failWriteOnce = true;
  let exposeWriteError = false;
  await page.route("**/rpc", async (route) => {
    const request = route.request().postDataJSON();
    if (failWriteOnce && request?.op === "setRunPinned") {
      failWriteOnce = false;
      exposeWriteError = true;
      await route.fulfill({ status: 500, contentType: "application/json", body: JSON.stringify({ error: "run-persistence-write-failed", message: "fixture write failed" }) });
      return;
    }
    const response = await route.fetch();
    const result = await response.json();
    if (exposeWriteError && request?.op === "snapshot") {
      exposeWriteError = false;
      result.snapshot.runPersistenceWriteError = { detail: "fixture write failed", retryable: true };
    }
    await route.fulfill({ response, json: result });
  });
  await page.locator(".side button[data-act='focus-project']").first().click();
  await page.waitForSelector(".project-board");
  await page.$eval(`.run-row[data-run='${activeRunId}'] [data-act='set-run-pinned']`, (node) => node.click());
  await page.waitForSelector("[data-run-persistence='write-error'] button[data-act='retry-run-organization']");
  await page.click("[data-run-persistence='write-error'] button[data-act='retry-run-organization']");
  await page.waitForFunction(() => !document.querySelector("[data-run-persistence='write-error']"));
  await page.unroute("**/rpc");

  await page.click("button[data-act='open-run-archive']");
  await page.setViewportSize({ width: 760, height: 720 });
  await page.waitForSelector(".run-archive-page");
  await assertShellRegionsDoNotOverlap(page);
  await assertNecessaryTextContrast(page, "compact desktop Run archive");
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth), true, "compact desktop archive must not overflow horizontally");

  await page.setViewportSize({ width: 390, height: 844 });
  await page.reload({ waitUntil: "domcontentloaded" });
  await page.click("button[data-act='mobile-drawer']");
  await page.click("[data-dialog-id='mobile-drawer'] button[data-act='open-overview']");
  await page.waitForSelector(".overview-page");
  assert.equal(await page.locator(".overview-page [data-run-organization]").count(), 0, "#183 must not add Run organization actions to mobile Host overview");

  await runMobileRunOrganizationJourney(page, {
    url,
    activeRunId,
    remoteHostId,
    remoteRunId,
    mobileProjectId,
    mobileBoundRunId,
    mobileActiveRunId,
    mobileEndedRunId,
    mobileRemovedProjectId,
    mobileRemovedRunId,
  });

  console.log("Run organization desktop and mobile e2e ok");
} finally {
  await browser.close();
}
