import assert from "node:assert/strict";

export async function verifyUsageHostIsolation(session) {
  const page = session.page;
  const click = (selector) => page.locator(selector).first().click();
  const snapshot = () => page.evaluate(async (protocol) => {
    const response = await fetch(`${protocol}/rpc`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        op: "snapshot",
        clientInstanceId: sessionStorage.getItem("agent-taskboard-client-id"),
      }),
    });
    assertResponse(response);
    return (await response.json()).snapshot;

    function assertResponse(response) {
      if (!response.ok) throw new Error(`snapshot failed: ${response.status}`);
    }
  }, session.url);
  const initial = await snapshot();
  const original = initial.hosts.find((host) => host.id === initial.focusedHostId);
  const other = initial.hosts.find((host) => host.id !== initial.focusedHostId);
  assert.ok(original && other, "usage isolation requires two real Hosts");
  const switchHost = async (host) => {
    await click("[data-act='toggle-hosts']");
    await click(`.host-picker [data-act='focus-host'][data-id='${host.id}']`);
    await page.waitForFunction((name) =>
      document.querySelector(".host-line .host-name")?.textContent.trim() === name,
    host.displayName);
  };
  const openUsage = async () => {
    await click("[data-act='open-usage']");
    await page.waitForSelector(".usage-page");
  };
  await switchHost(other);
  await openUsage();
  const otherBefore = await snapshot();
  await switchHost(original);
  await openUsage();
  const projectId = await page.$eval("#usage-project", (select) => select.options[1].value);
  let releaseQuery;
  let queryStarted;
  const queryGate = new Promise((resolve) => { releaseQuery = resolve; });
  const queryStart = new Promise((resolve) => { queryStarted = resolve; });
  let phase = "query-first";
  let releaseSwitch;
  let switchStarted;
  let switchGate = new Promise((resolve) => { releaseSwitch = resolve; });
  let switchStart = new Promise((resolve) => { switchStarted = resolve; });
  let staleFilterRequests = 0;
  let staleCustomRequests = 0;
  const routeQuery = async (route) => {
    const request = route.request().postDataJSON();
    if (phase === "query-first" && request?.op === "setUsageFilter") {
      queryStarted();
      await queryGate;
      return route.fulfill({
        status: 503,
        contentType: "application/json",
        body: JSON.stringify({ message: "original Host query unavailable" }),
      });
    }
    if (["switch-first", "custom-switch"].includes(phase) && request?.op === "focusHost") {
      switchStarted();
      await switchGate;
    }
    if (phase === "switch-first" && request?.op === "setUsageFilter") staleFilterRequests += 1;
    if (phase === "custom-switch" && request?.op === "setUsageRange") staleCustomRequests += 1;
    await route.continue();
  };
  await page.route("**/rpc", routeQuery);
  try {
    await page.selectOption("#usage-project", projectId);
    await queryStart;
    const switchAfterQuery = switchHost(other);
    await page.waitForSelector(".host-picker");
    releaseQuery();
    await switchAfterQuery;
    await openUsage();
    const afterFailedQuery = await snapshot();
    assert.deepEqual(afterFailedQuery.usage.filter, otherBefore.usage.filter, "failed A query cannot alter B");
    assert.equal(afterFailedQuery.focusedHostId, other.id);
    assert.equal(await page.locator(".usage-page .form-feedback, [data-act='retry-usage-query']").count(), 0);
    await switchHost(original);
    await openUsage();
    phase = "switch-first";
    const switchBeforeQuery = switchHost(other);
    await switchStart;
    await page.selectOption("#usage-project", projectId);
    releaseSwitch();
    await switchBeforeQuery;
    await page.waitForFunction(() =>
      document.querySelector(".usage-query-controls")?.getAttribute("aria-busy") !== "true");
    const afterStaleQuery = await snapshot();
    assert.deepEqual(afterStaleQuery.usage.filter, otherBefore.usage.filter,
      "a query submitted on A while switching cannot write B");
    assert.equal(staleFilterRequests, 0, "obsolete Host query must not be sent");
    assert.equal(afterStaleQuery.focusedHostId, other.id);
    assert.ok(!afterStaleQuery.projects.some((project) => project.id === projectId));
    assert.equal(await page.locator(".usage-page .form-feedback, [data-act='retry-usage-query']").count(), 0);
    phase = "";
    await switchHost(original);
    await openUsage();
    await click("[data-act='usage-range'][data-id='custom']");
    await page.waitForSelector(".usage-custom");
    await page.fill(".usage-custom [name='from']", "2000-01-01T00:00");
    await page.fill(".usage-custom [name='to']", "2000-01-02T00:00");
    switchGate = new Promise((resolve) => { releaseSwitch = resolve; });
    switchStart = new Promise((resolve) => { switchStarted = resolve; });
    phase = "custom-switch";
    const switchBeforeCustom = switchHost(other);
    await switchStart;
    await click(".usage-custom button[type='submit']");
    releaseSwitch();
    await switchBeforeCustom;
    await page.waitForFunction(() =>
      document.querySelector(".usage-custom")?.getAttribute("aria-busy") !== "true");
    const afterStaleCustom = await snapshot();
    assert.equal(afterStaleCustom.usage.range, otherBefore.usage.range, "A custom dates cannot change B range");
    assert.equal(afterStaleCustom.usage.customFromMs, otherBefore.usage.customFromMs);
    assert.equal(afterStaleCustom.usage.customToMs, otherBefore.usage.customToMs);
    assert.equal(staleCustomRequests, 0, "obsolete custom-date query must not be sent");
  } finally {
    releaseQuery();
    releaseSwitch();
    await page.unroute("**/rpc", routeQuery);
  }
  await switchHost(original);
  await openUsage();
  await click("[data-act='usage-range'][data-id='today']");
  await page.selectOption("#usage-project", "");
  await page.waitForFunction(() => document.querySelector("#usage-project")?.value === "");
  await click("[data-act='return-page']");
  await page.waitForSelector(".lanes");
}
