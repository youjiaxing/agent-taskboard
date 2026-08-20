const { test, expect } = require("@playwright/test");

const prototypeUrl = process.env.PROTOTYPE_URL || "http://127.0.0.1:4173";

async function open(page, params = "") {
  await page.goto(`${prototypeUrl}/?direction=codex-map&scenario=daily${params}`);
}

test("三种用量层级切换，URL 与键盘稳定", async ({ page }) => {
  await open(page, "&variant=A");

  await expect(page.locator(".map-side")).toBeVisible();
  await expect(page.getByRole("button", { name: "用量", exact: true }).first()).toBeVisible();
  await expect(page.locator(".usage-pane")).toBeVisible();
  await expect(page.locator(".usage-ledger").first()).toBeVisible();
  await expect(page).toHaveURL(/variant=A/);
  await expect(page).toHaveURL(/mid=usage/);

  await page.getByRole("button", { name: /B · 浮层/ }).click();
  await expect(page).toHaveURL(/variant=B/);
  await expect(page.locator(".lanes")).toBeVisible();
  await expect(page.locator(".usage-sheet")).toBeVisible();
  await expect(page.locator(".usage-prompt")).toContainText("先选时间范围");

  await page.getByRole("button", { name: /C · 总览里的账本/ }).click();
  await expect(page).toHaveURL(/variant=C/);
  await expect(page.locator(".usage-pane")).toBeVisible();
  await expect(page.locator(".usage-ledger").first()).toBeVisible();

  await page.keyboard.press("ArrowRight");
  await expect(page).toHaveURL(/variant=A/);
  await page.keyboard.press("ArrowLeft");
  await expect(page).toHaveURL(/variant=C/);
});

test("A：左侧入口 + 流水；今天与 24 小时不同；缺字段是 —；可自定义范围", async ({ page }) => {
  await open(page, "&variant=A&mid=board");

  await expect(page.locator(".lanes")).toBeVisible();
  await page.locator('.map-side [data-act="mid-mode"][data-id="usage"]').click();
  await expect(page.locator(".usage-pane")).toBeVisible();
  await expect(page.getByText("这台 Host 合计")).toBeVisible();
  await expect(page.getByRole("button", { name: "全部模型" })).toBeVisible();
  await expect(page.getByRole("button", { name: "grok-4" })).toBeVisible();
  await expect(page.getByText("缓存命中率")).toBeVisible();
  await expect(page.getByText("最近请求")).toBeVisible();
  await expect(page.locator('[data-act="usage-range"][data-id="today"]')).toHaveClass(/active/);
  await expect(page.locator(".usage-pane")).toContainText("今天 11:20");
  await expect(page.locator(".usage-pane")).not.toContainText("昨天 21:40");

  await page.getByRole("button", { name: "最近 24 小时" }).click();
  await expect(page.locator(".usage-pane")).toContainText("昨天 21:40");
  await expect(page.locator(".usage-pane")).not.toContainText("昨天 14:02");

  await page.getByRole("button", { name: "Codex", exact: true }).click();
  await expect(page.locator(".usage-kpi.missing .v").first()).toHaveText("—");

  await page.locator('.usage-ledger tr[data-act="usage-run"]').first().click();
  await expect(page.locator("[data-usage-detail]")).toBeVisible();
  await expect(page.locator("[data-usage-detail]")).toContainText("这家怎么记账");
  await expect(page.locator("[data-usage-detail]")).toContainText("合计是 Agent 自己报的");

  await page.getByRole("button", { name: "自定义" }).click();
  await expect(page.locator('input[data-act="usage-from"]')).toBeVisible();
  await expect(page.locator('input[data-act="usage-to"]')).toBeVisible();
  await page.locator('input[data-act="usage-from"]').fill("2026-08-01");
  await page.locator('input[data-act="usage-to"]').fill("2026-08-20");
  await expect(page.locator(".usage-ledger").first()).toBeVisible();

  await page.getByRole("button", { name: "返回看板" }).click();
  await expect(page.locator(".lanes")).toBeVisible();
  await expect(page.locator(".usage-pane")).toHaveCount(0);
});

test("B：浮层不拆掉看板，关掉回到原工作面；没筛选不出总量", async ({ page }) => {
  await open(page, "&variant=B&mid=board");

  await expect(page.locator(".lanes")).toBeVisible();
  await expect(page.locator(".usage-sheet")).toBeVisible();
  await expect(page.locator(".usage-prompt")).toContainText("先选时间范围");
  await expect(page.locator(".usage-kpis")).toHaveCount(0);

  await page.getByRole("button", { name: "近 7 天" }).click();
  await expect(page.locator(".usage-kpis")).toBeVisible();
  await expect(page.locator(".usage-caveat, .usage-note").first()).toBeVisible();

  await page.locator(".usage-sheet-hd [data-act='usage-close']").click();
  await expect(page.locator(".usage-sheet")).toHaveCount(0);
  await expect(page.locator(".lanes")).toBeVisible();

  await page.locator('.map-chrome-lead [data-act="usage-open"]').click();
  await expect(page.locator(".usage-sheet")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.locator(".usage-sheet")).toHaveCount(0);
});

test("C：总览切到用量账本，点一行看六个字段", async ({ page }) => {
  await open(page, "&variant=C");

  await expect(page.locator('.mid-bar [data-act="usage-tab"][data-id="usage"]')).toHaveClass(/active/);
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(page.locator(".ov")).toBeVisible();

  await page.locator('.mid-bar [data-act="usage-tab"][data-id="usage"]').click();
  await expect(page.locator(".usage-ledger").first()).toBeVisible();
  await page.locator('.usage-ledger tr[data-act="usage-run"]').first().click();
  await expect(page.locator("[data-usage-detail]")).toBeVisible();
  await expect(page.locator("[data-usage-detail]")).toContainText("输入");
  await expect(page.locator("[data-usage-detail]")).toContainText("合计（Agent 自报）");
});

test("空数据和远程 Host 不可达不画假数字", async ({ page }) => {
  await open(page, "&variant=A&usage=empty");
  await expect(page.locator(".usage-empty")).toContainText("还没有可统计的 Run");
  await expect(page.locator(".usage-kpis")).toHaveCount(0);

  await page.locator('select[data-act="usage-state"]').selectOption("unreachable");
  await expect(page.locator(".usage-empty")).toContainText("暂时连不上");
  await expect(page.getByText("不画上次数字")).toBeVisible();
});

test("手机只留合计，丢掉拆分表和六字段明细", async ({ page }) => {
  await open(page, "&variant=A&viewport=phone");

  await expect(page.locator(".phone")).toBeVisible();
  await expect(page.locator(".phone-usage")).toBeVisible();
  await expect(page.locator(".phone-usage")).toContainText("手机只保留合计");
  await expect(page.locator(".usage-split")).toHaveCount(0);
  await expect(page.locator("[data-usage-detail]")).toHaveCount(0);
});
