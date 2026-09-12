import { assertShellRegionsDoNotOverlap } from "./board-harness.mjs";

export async function runDesktopBoardGraph(session) {
const boardActive = await session.page.$eval("button[data-act='center-view'][data-id='board']", (node) =>
  node.classList.contains("active"),
);
if (!boardActive) {
  throw new Error("factory default should be the board view");
}

const graphTabBeforeTick = await session.page.$("button[data-act='center-view'][data-id='graph']");
const graphTabBox = await graphTabBeforeTick?.boundingBox();
if (!graphTabBeforeTick || !graphTabBox) {
  throw new Error("single-click navigation regression needs the graph tab");
}
await session.page.mouse.move(graphTabBox.x + graphTabBox.width / 2, graphTabBox.y + graphTabBox.height / 2);
await session.page.mouse.down();
const navigationTickResponse = session.page.waitForResponse((response) =>
  response.url().endsWith("/rpc") && response.request().postData()?.includes('"op":"tick"'),
);
await session.page.evaluate(() => window.__RUN_INTERVAL_CALLBACKS__());
await navigationTickResponse;
await session.page.waitForTimeout(50);
if (!(await graphTabBeforeTick.evaluate((node) => node.isConnected))) {
  throw new Error("Host tick should not replace a navigation target while its pointer is pressed");
}
await session.page.mouse.up();
await session.page.waitForSelector(".dep-graph", { timeout: 1000 });
const overviewTitles = await session.page.$$eval(".graph-node .issue-title", (nodes) =>
  nodes.map((node) => node.textContent),
);
for (const title of ["parent", "child ready", "child blocked", "unparented ready", "waiting on history", "blocker", "active work"]) {
  if (!overviewTitles.includes(title)) {
    throw new Error(`dependency overview should include open Issue ${title}, got ${JSON.stringify(overviewTitles)}`);
  }
}
if (
  overviewTitles.length !== 7
  || overviewTitles.includes("older closed")
  || (await session.page.$('[data-graph-mode="overview"]')) == null
) {
  throw new Error(`dependency overview should contain all and only open Issues: ${JSON.stringify(overviewTitles)}`);
}
await session.page.click(".graph-node:has-text('child blocked') .graph-node-main");
await session.page.waitForFunction(() => document.querySelector(".graph-center-label")?.textContent?.includes("#3 child blocked"));
await session.page.waitForSelector("button[data-act='graph-overview']");
await session.page.click("button[data-act='graph-overview']");
await session.page.waitForSelector('[data-graph-mode="overview"]');
await session.page.click("button[data-act='center-view'][data-id='board']");
await session.page.waitForSelector(".lanes");

await session.clickCard(session.page.locator(".issue-card:has-text('child blocked') .issue-card-main"));
await session.page.waitForSelector(".detail-hd:has-text('child blocked')");
await session.page.click(".issue-detail button[data-act='view-dependencies']");
await session.page.waitForSelector(".dep-graph");
if (await session.page.$(".lanes")) {
  throw new Error("graph view should replace the four columns");
}
const graphTitles = await session.page.$$eval(".graph-node .issue-title", (nodes) =>
  nodes.map((node) => node.textContent),
);
for (const title of ["blocker", "child blocked", "waiting on history", "active work"]) {
  if (!graphTitles.includes(title)) {
    throw new Error(`one-hop graph should include ${title}, got ${JSON.stringify(graphTitles)}`);
  }
}
for (const unrelated of ["parent", "unparented ready", "older closed"]) {
  if (graphTitles.includes(unrelated)) {
    throw new Error(`centered graph should exclude unrelated Issue ${unrelated}`);
  }
}
const graphCenterText = await session.page.$eval(".graph-center-label", (node) => node.textContent?.trim());
if (graphCenterText !== "中心 Issue：#3 child blocked") {
  throw new Error(`graph should name its stable center Issue, got ${graphCenterText}`);
}
const completeGraphAction = session.page.getByRole("button", { name: "查看完整上下游（61 个 Issue）" });
if ((await completeGraphAction.count()) !== 1) {
  throw new Error("graph should offer the complete connected upstream/downstream closure with a count");
}
const edge = await session.page.$('path[data-from="you/garden#9"][data-to="you/garden#3"]');
if (!edge) {
  throw new Error("graph should draw the blocker edge from left to right");
}
if (!(await session.page.$('path[data-from="you/garden#3"][data-to="you/garden#5"]'))) {
  throw new Error("graph should draw downstream dependencies from the center Issue");
}
if (await session.page.$('path[data-from="you/garden#1"][data-to="you/garden#2"]')) {
  throw new Error("graph should not draw parent/child as an edge");
}

const stableGraphCanvas = await session.page.$(".graph-canvas");
await session.page.click(".graph-node:has-text('blocker') .graph-node-main");
await session.page.waitForSelector(".detail-hd:has-text('blocker')");
if (await session.page.$("button[data-act='clear-filter']")) {
  throw new Error("clicking a graph node should not filter the board");
}
if ((await session.page.$eval(".graph-center-label", (node) => node.textContent?.trim())) !== "中心 Issue：#3 child blocked") {
  throw new Error("clicking a graph node should only change details, not the graph center");
}
if (!stableGraphCanvas || !(await stableGraphCanvas.evaluate((node) => node.isConnected))) {
  throw new Error("changing graph node details should preserve the graph canvas");
}

const expandFromWaiting = session.page.getByRole("button", { name: "从此处展开 #5" });
if ((await expandFromWaiting.count()) !== 1 || !(await expandFromWaiting.textContent())?.includes("从此处展开")) {
  throw new Error("graph nodes should name the re-centering action instead of relying on an unexplained target icon");
}
const waitingViewportBefore = await session.page.$eval(".graph-node:has-text('waiting on history')", (node) => {
  const canvas = node.closest(".graph-canvas");
  const canvasRect = canvas.getBoundingClientRect();
  const nodeRect = node.getBoundingClientRect();
  return {
    x: nodeRect.left - canvasRect.left + nodeRect.width / 2,
    y: nodeRect.top - canvasRect.top + nodeRect.height / 2,
    scrollTop: canvas.scrollTop,
  };
});
await expandFromWaiting.click();
await session.page.waitForFunction(() => document.querySelector(".graph-center-label")?.textContent?.includes("#5 waiting on history"));
await session.page.waitForSelector(".detail-hd:has-text('waiting on history')");
const waitingViewportAfter = await session.page.$eval(".graph-node:has-text('waiting on history')", (node) => {
  const canvas = node.closest(".graph-canvas");
  const canvasRect = canvas.getBoundingClientRect();
  const nodeRect = node.getBoundingClientRect();
  return {
    x: nodeRect.left - canvasRect.left + nodeRect.width / 2,
    y: nodeRect.top - canvasRect.top + nodeRect.height / 2,
    scrollTop: canvas.scrollTop,
  };
});
if (
  Math.abs(waitingViewportAfter.x - waitingViewportBefore.x) > 2
  || Math.abs(waitingViewportAfter.y - waitingViewportBefore.y) > 2
) {
  throw new Error(`expanding from an Issue should preserve its viewport anchor: ${JSON.stringify({ waitingViewportBefore, waitingViewportAfter })}`);
}

const childViewportBefore = await session.page.$eval(".graph-node:has-text('child blocked')", (node) => {
  const canvas = node.closest(".graph-canvas");
  const canvasRect = canvas.getBoundingClientRect();
  const nodeRect = node.getBoundingClientRect();
  return {
    x: nodeRect.left - canvasRect.left + nodeRect.width / 2,
    y: nodeRect.top - canvasRect.top + nodeRect.height / 2,
  };
});
await session.clickGraphAction(session.page.getByRole("button", { name: "从此处展开 #3" }));
await session.page.waitForFunction(() => document.querySelector(".graph-center-label")?.textContent?.includes("#3 child blocked"));
await session.page.waitForSelector(".detail-hd:has-text('child blocked')");
const childViewportAfter = await session.page.$eval(".graph-node:has-text('child blocked')", (node) => {
  const canvas = node.closest(".graph-canvas");
  const canvasRect = canvas.getBoundingClientRect();
  const nodeRect = node.getBoundingClientRect();
  return {
    x: nodeRect.left - canvasRect.left + nodeRect.width / 2,
    y: nodeRect.top - canvasRect.top + nodeRect.height / 2,
  };
});
if (
  Math.abs(childViewportAfter.x - childViewportBefore.x) > 2
  || Math.abs(childViewportAfter.y - childViewportBefore.y) > 2
) {
  throw new Error(`repeated expansion should preserve the clicked Issue anchor: ${JSON.stringify({ childViewportBefore, childViewportAfter })}`);
}
await session.page.getByRole("button", { name: "查看完整上下游（61 个 Issue）" }).click();
await session.page.waitForSelector(".graph-index");
const limitedGraphText = await session.page.$eval(".graph-limit", (node) => node.textContent?.replace(/\s+/g, " ").trim());
if (!limitedGraphText?.includes("画布显示 48/61")) {
  throw new Error(`large complete closure should keep the canvas bounded, got ${limitedGraphText}`);
}
if ((await session.page.$$(".graph-node")).length !== 48) {
  throw new Error("large complete closure should not render every node at once");
}
if ((await session.page.$$(".graph-index-row")).length !== 50 || !(await session.page.$("button[data-act='graph-list-more']"))) {
  throw new Error("complete relationship list should paginate instead of mounting every Issue row");
}
await session.page.fill('[data-field="graphSearch"]', "just closed");
await session.page.waitForFunction(() => document.querySelectorAll(".graph-index-row").length === 1);
const searchedRelationship = await session.page.$eval(".graph-index-row", (node) => node.textContent?.replace(/\s+/g, " ").trim());
if (!searchedRelationship?.includes("just closed")) {
  throw new Error(`complete relationship search should find closed downstream Issues, got ${searchedRelationship}`);
}
await session.page.fill('[data-field="graphSearch"]', "waiting on history");
await session.page.waitForFunction(() => document.querySelectorAll(".graph-index-row").length === 1);
await session.clickGraphAction(session.page.getByRole("button", { name: "从此处展开 #5" }).first());
await session.page.waitForFunction(() => document.querySelector(".graph-center-label")?.textContent?.includes("#5 waiting on history"));
await session.page.waitForSelector(".detail-hd:has-text('waiting on history')");
if (!(await session.page.$(".graph-index")) || !(await session.page.getByRole("button", { name: "收起到一跳上下游" }).count())) {
  throw new Error("re-centering a complete upstream/downstream view should preserve its range");
}
if ((await session.page.inputValue('[data-field="graphSearch"]')) !== "waiting on history") {
  throw new Error("re-centering a complete upstream/downstream view should preserve its search context");
}
await session.page.fill('[data-field="graphSearch"]', "");
await session.page.waitForFunction(() => document.querySelectorAll(".graph-index-row").length === 50);
await session.clickGraphAction(session.page.getByRole("button", { name: "从此处展开 #3" }).first());
await session.page.waitForFunction(() => document.querySelector(".graph-center-label")?.textContent?.includes("#3 child blocked"));
await session.page.waitForSelector(".graph-index");

await session.page.click(".graph-node:has-text('active work') .graph-node-main");
await session.page.waitForSelector(".detail-hd:has-text('active work')");
await session.page.waitForSelector('.issue-markdown:has-text("Active Run Question")');
if (await session.page.$(".lifted-run")) {
  throw new Error("dependency graph nodes should only change Issue details");
}
await session.assertVisual("issue-99-graph-1440x900.png");
await assertShellRegionsDoNotOverlap(session.page);

const graphCanvas = await session.page.$(".graph-canvas");
const graphEdgePath = await session.page.$(".graph-edges path");
const graphScrollLeft = await session.page.$eval(".graph-canvas", (node) => {
  const flow = node.querySelector(".graph-flow");
  flow.style.width = "1600px";
  node.scrollLeft = Math.floor((node.scrollWidth - node.clientWidth) / 2);
  return node.scrollLeft;
});
if (!graphCanvas || graphScrollLeft <= 0) {
  throw new Error(`graph scroll regression needs horizontal overflow, got ${graphScrollLeft}`);
}
const tickResponse = session.page.waitForResponse((response) =>
  response.url().endsWith("/rpc") && response.request().postData()?.includes('"op":"tick"'),
);
await session.page.evaluate(() => window.__RUN_INTERVAL_CALLBACKS__());
await tickResponse;
await session.page.waitForTimeout(50);
const graphCanvasConnected = await graphCanvas.evaluate((node) => node.isConnected);
const graphEdgeConnected = graphEdgePath ? await graphEdgePath.evaluate((node) => node.isConnected) : false;
const graphScrollAfterTick = await session.page.$eval(".graph-canvas", (node) => node.scrollLeft);
if (!graphCanvasConnected || !graphEdgeConnected || graphScrollAfterTick !== graphScrollLeft) {
  throw new Error(
    `Host tick should preserve the dependency graph DOM and viewport, got canvas=${graphCanvasConnected} edge=${graphEdgeConnected} scroll=${graphScrollLeft}->${graphScrollAfterTick}`,
  );
}

await session.page.click("button[data-act='center-view'][data-id='board']");
await session.page.waitForSelector(".lanes");

await session.page.click("button[data-act='open-overview']");
await session.page.waitForSelector(".overview-page");
if (await session.page.$(".lanes")) {
  throw new Error("Host overview should replace the Project board");
}
for (const group of ["running", "stopped"]) {
  if (!(await session.page.$(`[data-run-group="${group}"]`))) {
    throw new Error(`Host overview missing ${group} group`);
  }
}
if (await session.page.$('[data-run-group="ended"]')) {
  throw new Error("ended Runs should be hidden by default");
}
const overviewProjects = await session.page.$$eval(".run-thumbnail .run-project", (nodes) => nodes.map((node) => node.textContent));
if (!overviewProjects.includes("garden") || !overviewProjects.includes("tools")) {
  throw new Error(`Host overview should include Runs from all Projects, got ${JSON.stringify(overviewProjects)}`);
}
await session.page.selectOption('[data-overview-filter="project"]', { label: "garden" });
const filteredProjects = await session.page.$$eval(".run-thumbnail .run-project", (nodes) => nodes.map((node) => node.textContent));
if (filteredProjects.some((name) => name !== "garden")) {
  throw new Error(`Host overview Project filter leaked: ${JSON.stringify(filteredProjects)}`);
}
await session.assertVisual("issue-99-overview-1440x900.png");
await assertShellRegionsDoNotOverlap(session.page);
await session.page.click("button[data-act='return-board']");
await session.page.waitForSelector(".lanes");
}
