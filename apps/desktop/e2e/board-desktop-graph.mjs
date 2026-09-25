import { assertShellRegionsDoNotOverlap } from "./board-harness.mjs";

export async function runDesktopBoardGraph(session) {
const nodeViewport = (title) =>
  session.page.$eval(`.graph-node:has-text('${title}')`, (node) => {
    const canvas = node.closest(".graph-canvas");
    const canvasRect = canvas.getBoundingClientRect();
    const nodeRect = node.getBoundingClientRect();
    return {
      x: nodeRect.left - canvasRect.left + nodeRect.width / 2,
      y: nodeRect.top - canvasRect.top + nodeRect.height / 2,
      scrollLeft: canvas.scrollLeft,
      scrollTop: canvas.scrollTop,
    };
  });
const assertViewportRestored = (before, after, message) => {
  if (
    Math.abs(after.scrollLeft - before.scrollLeft) > 1
    || Math.abs(after.scrollTop - before.scrollTop) > 1
  ) {
    throw new Error(`${message}: ${JSON.stringify({ before, after })}`);
  }
};
const graphViewport = async () => ({
  ...(await session.page.$eval(".graph-canvas", (node) => ({ scrollLeft: node.scrollLeft, scrollTop: node.scrollTop }))),
  ...(await nodeViewport("child blocked")),
});
const selectedGraphNodeTitle = async () =>
  (await session.page.$$eval(".graph-node.sel .issue-title", (nodes) => nodes.map((node) => node.textContent?.trim()))).join("|");
const graphCenterLabel = () => session.page.$eval(".graph-center-label", (node) => node.textContent?.trim());

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

// 依赖图选中无活跃 Run 的节点 → Board inspector：必须保留同一张图、同一中心与同一选中 Issue
const graphPageAddress = session.page.url();
await session.page.$eval(".graph-node:has-text('blocker') .graph-node-main", (node) => node.click());
await session.page.waitForSelector(".dep-graph");
await session.page.waitForSelector(".board-shell > .issue-detail .issue-document[data-document-state='ready']");
if (await session.page.$(".focus-workspace-layout") || await session.page.$("[data-terminal-surface]")) {
  throw new Error("selecting a dependency graph node without an active Run should stay on the graph with its Board inspector");
}
if (session.page.url() !== graphPageAddress) {
  throw new Error("opening the graph Issue inspector must keep the browser address stable");
}
if ((await graphCenterLabel()) !== "中心 Issue：#3 child blocked") {
  throw new Error("opening the graph Issue inspector must preserve the same graph center");
}
if (await session.page.$("button[data-act='clear-filter']")) {
  throw new Error("clicking a graph node should not filter the board");
}

const expandFromWaiting = session.page.getByRole("button", { name: "从此处展开 #5" });
if ((await expandFromWaiting.count()) !== 1 || !(await expandFromWaiting.textContent())?.includes("从此处展开")) {
  throw new Error("graph nodes should name the re-centering action instead of relying on an unexplained target icon");
}
await expandFromWaiting.click();
await session.page.waitForFunction(() => document.querySelector(".graph-center-label")?.textContent?.includes("#5 waiting on history"));
await session.page.waitForSelector(".detail-hd:has-text('waiting on history')");

await session.clickGraphAction(session.page.getByRole("button", { name: "从此处展开 #3" }));
await session.page.waitForFunction(() => document.querySelector(".graph-center-label")?.textContent?.includes("#3 child blocked"));
await session.page.waitForSelector(".detail-hd:has-text('child blocked')");
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

// 完整上下游视图：滚动后的图视口在进入工作区再返回后必须一致
await session.page.$eval(".graph-canvas", (node) => {
  node.scrollLeft = Math.floor((node.scrollWidth - node.clientWidth) * 0.55);
  node.scrollTop = Math.floor((node.scrollHeight - node.clientHeight) * 0.6);
});
const completeViewportBefore = await graphViewport();
if (completeViewportBefore.scrollLeft <= 0) {
  throw new Error(`complete graph viewport anchor fixture needs a scrolled canvas: ${JSON.stringify(completeViewportBefore)}`);
}
await session.page.$eval(".graph-node:has-text('active work') .graph-node-main", (node) => node.click());
await session.page.waitForSelector(".focus-workspace-layout");
await session.page.waitForSelector('[data-terminal-surface="live"]');
if (await session.page.$(".dep-graph")) {
  throw new Error("selecting a graph node with an active Run should enter that Run");
}
await session.page.click("button[data-act='return-page']");
await session.page.waitForSelector(".dep-graph");
if (await session.page.$(".lifted-run")) {
  throw new Error("returning from a graph Run must leave the focus workspace");
}
assertViewportRestored(completeViewportBefore, await graphViewport(), "returning from a graph Run must restore the scrolled graph viewport");
if ((await graphCenterLabel()) !== "中心 Issue：#3 child blocked") {
  throw new Error("returning from a graph Run must restore the same graph center");
}
if ((await selectedGraphNodeTitle()) !== "child blocked") {
  throw new Error(`returning from a graph Run must restore the graph context Issue, got ${await selectedGraphNodeTitle()}`);
}
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
if (!(await session.page.$('[data-run-group="running"]'))) {
  throw new Error("Host overview missing running group");
}
for (const group of ["stopped", "ended"]) {
  if (await session.page.$(`[data-run-group="${group}"]`)) {
    throw new Error(`${group} Runs should be hidden by default`);
  }
}
await session.page.check('input[data-field="showEndedRuns"]');
if (!(await session.page.$('[data-run-group="stopped"]'))) {
  throw new Error("Host overview should reveal execution-stopped Runs with the ended toggle");
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
await assertShellRegionsDoNotOverlap(session.page);
await session.page.click("button[data-act='return-page']");
await session.page.waitForSelector(".lanes");
}
