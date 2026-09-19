import { chromium } from "playwright";
import {
  assertShellRegionsDoNotOverlap,
  installDeterministicHostProtocol,
} from "./visual-regression.mjs";

const url = process.env.BOARD_URL;
const state = process.env.SHELL_EDGE_STATE;
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
