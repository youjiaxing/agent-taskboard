import { chromium } from "playwright";
import {
  assertShellRegionsDoNotOverlap,
  installDeterministicHostProtocol,
} from "./visual-regression.mjs";

const url = process.env.BOARD_URL;
const state = process.env.SHELL_EDGE_STATE;
const gardenProjectId = process.env.GARDEN_PROJECT_ID;
if (!url || !state) {
  console.error("missing BOARD_URL or SHELL_EDGE_STATE");
  process.exit(1);
}

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ locale: "zh-CN", viewport: { width: 1280, height: 840 } });
page.on("pageerror", (error) => console.error("pageerror", error));
await installDeterministicHostProtocol(page, url);
await page.goto(url, { waitUntil: "domcontentloaded" });

if (state === "empty-host") {
  await page.waitForSelector(".empty");
  if (await page.$(".project-row")) throw new Error("empty Host should not render a Project row");
} else {
  await page.waitForSelector(".project-board");
  if (["offline", "rate-limited", "auth-failed"].includes(state)) {
    const refreshResponse = page.waitForResponse((response) => {
      if (!response.url().includes("/rpc")) return false;
      try {
        return response.request().postDataJSON()?.op === "refresh";
      } catch {
        return false;
      }
    });
    await page.click('.refresh-bar button[data-act="refresh"]');
    const response = await refreshResponse;
    if (!response.ok()) {
      throw new Error(`manual refresh failed at the protocol boundary: ${response.status()}`);
    }
    await page.waitForSelector(`.refresh-bar[data-kind="${state}"]`);
  }
  if (state === "single-project") {
    const projects = await page.$$(".project-row");
    if (projects.length !== 1) throw new Error(`single Project fixture rendered ${projects.length} Project rows`);
    await page.waitForSelector(".lanes");
    await page.click("button[data-act='open-overview']");
    await page.waitForSelector(".overview-page");
    await page.waitForSelector(".overview-project");
    const empty = await page.$eval(".overview-runs-empty", (node) => node.textContent?.replace(/\s+/g, " ").trim());
    if (!empty?.includes("尚无通过 Agent Taskboard 启动的 Run") || !empty.includes("Project 态势仍在上方可见")) {
      throw new Error(`Host overview should keep Project status above a compact Run empty state: ${empty}`);
    }
    await page.click("button[data-act='return-page']");
    await page.waitForSelector(".lanes");
    await page.click(".host-area button[data-act='open-usage']");
    await page.waitForSelector(".usage-page");
    const usagePage = await page.evaluate(() => ({
      title: document.querySelector(".usage-page h1")?.textContent?.trim() ?? "",
      stacked: document.querySelectorAll(".lanes, .dep-graph, .focus-workspace-layout, .overview-page").length,
      ranges: [...document.querySelectorAll(".usage-page button[data-act='usage-range']")].map((node) => node.getAttribute("data-id")),
      activeRanges: document.querySelectorAll(".usage-page button[data-act='usage-range'].active").length,
      filters: [...document.querySelectorAll(".usage-page select[data-usage-filter]")].map((node) => node.getAttribute("data-usage-filter")),
      allFilterLabel: document.querySelector(".usage-page select[data-usage-filter='projectId'] option")?.textContent?.trim() ?? "",
    }));
    if (usagePage.stacked !== 0) {
      throw new Error("the usage page must replace the board instead of stacking over it");
    }
    if (usagePage.title !== "用量" && usagePage.title !== "Usage") {
      throw new Error(`usage page title, got ${usagePage.title}`);
    }
    if (JSON.stringify(usagePage.ranges) !== JSON.stringify(["24-hours", "today", "7-days", "30-days", "custom"]) || usagePage.activeRanges !== 1) {
      throw new Error(`usage ranges must offer one selected range from the shared option group: ${JSON.stringify(usagePage)}`);
    }
    if (JSON.stringify(usagePage.filters) !== JSON.stringify(["projectId", "agentId", "model"]) || !["全部", "All"].includes(usagePage.allFilterLabel)) {
      throw new Error(`usage filters must offer Project, Agent, and model choices: ${JSON.stringify(usagePage)}`);
    }
    await page.click("button[data-act='return-page']");
    await page.waitForSelector(".lanes");
  } else if (state === "overview-usage-truth") {
    if (!gardenProjectId) throw new Error("missing GARDEN_PROJECT_ID");
    await page.click("button[data-act='open-overview']");
    await page.waitForSelector(".overview-page");
    const readOverview = () => page.evaluate(() => ({
      stats: [...document.querySelectorAll(".overview-stats > div")].map((node) => ({
        value: Number(node.querySelector("b")?.textContent ?? "NaN"),
        label: node.querySelector("span")?.textContent?.trim() ?? "",
      })),
      runTotal: {
        label: document.querySelector(".overview-run-section > .lane-hd")?.childNodes[0]?.textContent?.trim() ?? "",
        value: Number(document.querySelector(".overview-run-section > .lane-hd span")?.textContent ?? "NaN"),
      },
      groups: [...document.querySelectorAll(".overview-group")].map((node) => ({
        id: node.getAttribute("data-run-group"),
        count: Number(node.querySelector(".lane-hd span")?.textContent ?? "NaN"),
        runs: node.querySelectorAll(".run-thumbnail").length,
      })),
      runs: document.querySelectorAll(".overview-group .run-thumbnail").length,
    }));
    const assertOverview = (actual, expected) => {
      const active = actual.stats.find((item) => item.label === "活跃 Run");
      if (active?.value !== expected.active) {
        throw new Error(`active Run KPI must use the filtered Run collection: ${JSON.stringify(actual)}`);
      }
      if (actual.runTotal.label !== "筛选后的 Run" || actual.runTotal.value !== expected.total || actual.runs !== expected.total) {
        throw new Error(`filtered Run heading and rendered groups must agree: ${JSON.stringify(actual)}`);
      }
      const groups = Object.fromEntries(actual.groups.map((group) => [group.id, group]));
      for (const [id, count] of Object.entries(expected.groups)) {
        if (groups[id]?.count !== count || groups[id]?.runs !== count) {
          throw new Error(`Run group ${id} must contain ${count} Runs: ${JSON.stringify(actual)}`);
        }
      }
      for (const id of expected.absent) {
        if (groups[id]) throw new Error(`Run group ${id} must be hidden: ${JSON.stringify(actual)}`);
      }
    };

    assertOverview(await readOverview(), {
      active: 3,
      total: 3,
      groups: { waiting: 1, running: 2 },
      absent: ["stopped", "ended"],
    });
    await page.check('input[data-field="showEndedRuns"]');
    assertOverview(await readOverview(), {
      active: 3,
      total: 6,
      groups: { waiting: 1, running: 2, stopped: 1, ended: 2 },
      absent: [],
    });
    await page.selectOption('select[data-overview-filter="project"]', gardenProjectId);
    assertOverview(await readOverview(), {
      active: 2,
      total: 4,
      groups: { waiting: 1, running: 1, stopped: 1, ended: 1 },
      absent: [],
    });
    await page.uncheck('input[data-field="showEndedRuns"]');
    assertOverview(await readOverview(), {
      active: 2,
      total: 2,
      groups: { waiting: 1, running: 1 },
      absent: ["stopped", "ended"],
    });

    await page.click("button[data-act='return-page']");
    await page.click(".host-area button[data-act='open-usage']");
    await page.waitForSelector(".usage-page");
    const trends = await page.evaluate(() => [...document.querySelectorAll(".usage-trend-block")].map((block) => ({
      label: block.querySelector(".tiny")?.textContent?.trim() ?? "",
      bars: [...block.querySelectorAll(".usage-trend > i")].map((bar) => ({
        title: bar.getAttribute("title"),
        slow: bar.classList.contains("slow"),
      })),
      missing: [...block.querySelectorAll(".usage-trend-missing")].map((node) => node.textContent?.trim()),
    })));
    const [ttft, generationRate] = trends;
    if (ttft.bars.length !== 1 || ttft.bars[0]?.title !== "180" || ttft.missing.length === 0 || ttft.missing.some((value) => value !== "—")) {
      throw new Error(`mixed TTFT data must render only the real sample as a bar: ${JSON.stringify(trends)}`);
    }
    if (generationRate.bars.length !== 0 || generationRate.missing.length === 0 || generationRate.missing.some((value) => value !== "—")) {
      throw new Error(`missing generation-rate data must render no bars and keep missing semantics: ${JSON.stringify(trends)}`);
    }
    await page.click("button[data-act='return-page']");
    await page.waitForSelector(".lanes");
  } else if (state === "frontier-empty") {
    await page.waitForSelector('[data-lane="frontier"] .lane-empty');
    const text = await page.$eval('[data-lane="frontier"] .lane-empty', (node) => node.textContent?.trim());
    if (!text?.includes("阻塞") && !text?.includes("认领")) {
      throw new Error(`Frontier empty reason is not visible: ${text}`);
    }
  } else if (state === "offline") {
    await page.waitForSelector('.refresh-bar[data-kind="offline"]');
    await page.waitForSelector(".lanes");
    const text = (await page.locator(".refresh-bar").textContent())?.replace(/\s+/g, " ") ?? "";
    if (!text.includes("检查运行 Host 的电脑网络") || !text.includes("刷新")) {
      throw new Error(`offline state must provide a concrete manual recovery step: ${text}`);
    }
  } else if (state === "rate-limited") {
    await page.waitForSelector('.refresh-bar[data-kind="rate-limited"]');
    await page.waitForSelector(".lanes");
    const text = (await page.locator(".refresh-bar").textContent())?.replace(/\s+/g, " ") ?? "";
    if (!text.includes("大约可再刷新") || !text.includes("刷新")) {
      throw new Error(`rate-limited state must show retry timing and the retry action: ${text}`);
    }
  } else if (state === "auth-failed") {
    await page.waitForSelector('.refresh-bar[data-kind="auth-failed"]');
    await page.waitForSelector(".notice.bad");
    const text = (await page.locator(".refresh-bar").textContent())?.replace(/\s+/g, " ") ?? "";
    if (!text.includes("更新 GitHub 凭据") || !text.includes("刷新")) {
      throw new Error(`auth failure must explain where to repair credentials and how to retry: ${text}`);
    }
  } else if (state !== "empty-host") {
    throw new Error(`unknown shell edge state ${state}`);
  }

  if (state !== "empty-host") {
    const refreshBars = await page.$$(".project-board [data-page-toolbar] > .refresh-bar");
    if (refreshBars.length !== 1) {
      throw new Error(`a Project board must show exactly one refresh status bar: ${refreshBars.length}`);
    }
    const refreshBar = await page.$eval(".project-board [data-page-toolbar] > .refresh-bar", (node) => {
      const rect = node.getBoundingClientRect();
      const toolbar = node.parentElement?.getBoundingClientRect();
      return {
        kind: node.getAttribute("data-kind"),
        text: node.textContent?.replace(/\s+/g, " ").trim() ?? "",
        overflow: node.scrollWidth - node.clientWidth,
        left: rect.left,
        right: rect.right,
        toolbarLeft: toolbar?.left ?? 0,
        toolbarRight: toolbar?.right ?? 0,
        recoveryActions: node.querySelectorAll("button[data-act='refresh']").length,
      };
    });
    if (refreshBar.recoveryActions !== 1) {
      throw new Error(`the refresh status bar must render exactly the caller's recovery action: ${JSON.stringify(refreshBar)}`);
    }
    if (refreshBar.overflow > 0 || refreshBar.left < refreshBar.toolbarLeft - 1 || refreshBar.right > refreshBar.toolbarRight + 1) {
      throw new Error(`refresh status must stay inside the content toolbar: ${JSON.stringify(refreshBar)}`);
    }
    if (!["never-fetched", "refreshing"].includes(refreshBar.kind) && !refreshBar.text.includes("数据截至") && !refreshBar.text.includes("Data as of")) {
      throw new Error(`refresh status must keep the data as-of time: ${JSON.stringify(refreshBar)}`);
    }
  }
}

await assertShellRegionsDoNotOverlap(page);
await browser.close();
console.log(`shell edge state ${state} ok`);
