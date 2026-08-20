const { test, expect } = require("@playwright/test");

const prototypeUrl = process.env.PROTOTYPE_URL || "http://127.0.0.1:4173";

async function open(page, params = "") {
  await page.goto(`${prototypeUrl}/?direction=codex-map&scenario=daily${params}`);
}

async function setRefresh(page, state) {
  await page.locator('select[data-act="refresh-state"]').selectOption(state);
}

test("三种摆位在同一主壳切换，URL 与键盘稳定", async ({ page }) => {
  await open(page, "&variant=A&refresh=normal");

  await expect(page.locator(".map-side")).toBeVisible();
  await expect(page.locator(".lanes")).toBeVisible();
  await expect(page.locator(".mid-bar .refresh-compact")).toBeVisible();
  await expect(page.locator('[data-refresh-placement="alert"]')).toHaveCount(0);
  await expect(page).toHaveURL(/variant=A/);

  await page.getByRole("button", { name: /B · Project 行/ }).click();
  await expect(page).toHaveURL(/variant=B/);
  await expect(page.locator(".map-side .refresh-project-row")).toBeVisible();
  await expect(page.locator(".mid-bar .refresh-compact")).toHaveCount(0);

  await page.locator('.map-chrome [data-act="toggle-side"]').click();
  await expect(page.locator(".map-side")).toHaveCount(0);
  await expect(page.locator(".mid-bar .refresh-compact")).toBeVisible();

  await page.getByRole("button", { name: /C · 态势时间轴/ }).click();
  await expect(page).toHaveURL(/variant=C/);
  await expect(page.locator('[data-refresh-placement="rail"]')).toBeVisible();
  await expect(page.locator(".mid-bar .refresh-compact")).toHaveCount(0);

  await page.keyboard.press("ArrowRight");
  await expect(page).toHaveURL(/variant=A/);
  await page.keyboard.press("ArrowLeft");
  await expect(page).toHaveURL(/variant=C/);
});

test("正常轮询可手动刷新，完成后按设置间隔重新倒计时", async ({ page }) => {
  await open(page, "&variant=A&refresh=normal");

  await page.locator(".mid-bar .refresh-compact").click();
  const details = page.getByRole("dialog", { name: "Tracker 刷新详情" });
  await expect(details).toContainText("42 秒后自动刷新");
  await expect(details).toContainText("态势截至 今天 14:32");
  await details.getByRole("button", { name: "刷新设置" }).click();

  const interval = page.getByRole("combobox", { name: "Tracker 刷新间隔" });
  await expect(interval).toHaveValue("60");
  await interval.selectOption("120");
  await expect(page).toHaveURL(/interval=120/);
  await page.locator(".settings-modal").getByRole("button", { name: "×" }).click();

  await page.locator(".mid-bar .refresh-compact").click();
  await page.getByRole("dialog", { name: "Tracker 刷新详情" }).getByRole("button", { name: "立即刷新" }).click();
  await expect(page.locator(".mid-bar .refresh-compact")).toContainText("正在刷新");
  await expect(page.locator(".lanes")).toBeVisible();
  await expect(page.locator(".dock")).toBeVisible();

  await expect(page.locator(".mid-bar .refresh-compact")).toContainText("120 秒", { timeout: 3000 });
});

test("慢刷新继续画上次态势，不重复出现刷新按钮", async ({ page }) => {
  await open(page, "&variant=C&refresh=slow");

  const rail = page.locator('[data-refresh-placement="rail"]');
  await expect(rail).toContainText("刷新还在继续");
  await expect(rail).toContainText("已等待 18 秒");
  await expect(rail.getByRole("button", { name: "立即刷新" })).toHaveCount(0);
  await expect(page.locator(".lanes")).toBeVisible();
});

test("离线但有上次态势：四列保留，Tracker 写动作暂停，已有 Run 仍可打开", async ({ page }) => {
  await open(page, "&variant=A&refresh=offline-cached");

  const alert = page.locator('[data-refresh-placement="alert"]');
  await expect(alert).toContainText("Project 离线");
  await expect(alert).toContainText("截至 今天 14:32");
  await expect(page.locator(".lanes")).toBeVisible();
  await expect(page.getByText(/认领、放领和自动推进暂停/)).toBeVisible();

  await page.locator('[data-act="issue"][data-id="24"]').first().click();
  await expect(page.locator('[data-act="fake-start"]:disabled').first()).toBeVisible();
  await expect(page.locator('[data-act="fake-claim"]:disabled').first()).toBeVisible();

  await page.locator('[data-act="issue"][data-id="50"]').first().click();
  await page.locator('.issue-hd [data-act="focus-run"]').click();
  await expect(page.locator(".run-stage")).toBeVisible();
  await expect(page.locator(".term")).toBeVisible();
});

test("离线且没有上次态势：看板、依赖图、Issue 和底栏都不伪造数据", async ({ page }) => {
  await open(page, "&variant=C&refresh=offline-empty");

  await expect(page.locator('[data-refresh-empty="true"]')).toContainText("还没有可显示的态势");
  await expect(page.locator(".lanes")).toHaveCount(0);
  await expect(page.locator(".map-detail-col")).toHaveCount(0);
  await expect(page.locator(".dock")).toHaveCount(0);

  await page.getByRole("button", { name: "依赖图", exact: true }).click();
  await expect(page.locator('[data-refresh-empty="true"]')).toBeVisible();
  await expect(page.locator(".graph-canvas")).toHaveCount(0);
});

test("限流有恢复时间和无恢复时间分开，均允许手动试一次", async ({ page }) => {
  await open(page, "&variant=C&refresh=limited-until");
  let rail = page.locator('[data-refresh-placement="rail"]');
  await expect(rail).toContainText("约 15:10 恢复");
  await expect(rail.getByRole("button", { name: "立即刷新" })).toBeVisible();

  await setRefresh(page, "limited-unknown");
  rail = page.locator('[data-refresh-placement="rail"]');
  await expect(rail).toContainText("未给恢复时间");
  await rail.getByRole("button", { name: "立即刷新" }).click();
  await expect(page.locator('[data-refresh-placement="rail"]')).toContainText("正在刷新");
  await expect(page.locator(".lanes")).toBeVisible();
});

test("鉴权失败与离线、限流使用不同文案和处理入口", async ({ page }) => {
  await open(page, "&variant=B&refresh=auth");

  await expect(page.locator(".refresh-project-row")).toContainText("GitHub 鉴权失败");
  const alert = page.locator('[data-refresh-placement="alert"]');
  await expect(alert).toContainText("更新这个 Project 的凭据");
  await expect(alert.getByRole("button", { name: "Project 设置" })).toBeVisible();
  await expect(alert.getByRole("button", { name: "立即刷新" })).toHaveCount(0);
  await alert.getByRole("button", { name: "Project 设置" }).click();
  await expect(page.locator(".settings-modal")).toBeVisible();
});

test("看板与依赖图共享 Project 刷新状态；Host 总览和 Run 不误挂 Project 状态", async ({ page }) => {
  await open(page, "&variant=C&refresh=offline-cached");
  await expect(page.locator('[data-refresh-placement="rail"]')).toBeVisible();

  await page.getByRole("button", { name: "依赖图", exact: true }).click();
  await expect(page.locator(".graph-canvas")).toBeVisible();
  await expect(page.locator('[data-refresh-placement="rail"]')).toContainText("Project 离线");

  await page.getByRole("button", { name: "总览", exact: true }).first().click();
  await expect(page.locator(".ov")).toBeVisible();
  await expect(page.locator('[data-refresh-placement="rail"]')).toHaveCount(0);

  await page.locator('[data-act="ov-run"]').first().click();
  await expect(page.locator(".run-stage")).toBeVisible();
  await expect(page.locator('[data-refresh-placement="rail"]')).toHaveCount(0);
});

test("390px 手机：三个摆位都可读，离线空状态不画真实 Issue", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });

  await open(page, "&viewport=phone&variant=A&refresh=offline-cached");
  await expect(page.locator(".phone-bar .refresh-compact")).toContainText("离线 · 截至 14:32");
  await page.locator(".phone-bar .refresh-compact").click();
  await expect(page.getByRole("dialog", { name: "Tracker 刷新详情" })).toBeVisible();
  await page.getByRole("dialog", { name: "Tracker 刷新详情" }).getByRole("button", { name: "关闭" }).click();

  await page.getByRole("button", { name: "B", exact: true }).click();
  await expect(page.locator('[data-refresh-placement="phone-project"]')).toContainText("离线 · 截至 14:32");

  await page.getByRole("button", { name: "C", exact: true }).click();
  await setRefresh(page, "limited-until");
  await expect(page.locator('[data-refresh-placement="phone-card"]')).toContainText("约 15:10 恢复");

  await setRefresh(page, "offline-empty");
  await expect(page.locator('[data-refresh-empty="true"]')).toContainText("还没有可显示的态势");
  await expect(page.locator(".phone-body .issue-card")).toHaveCount(0);
});

test("390px 手机周边流程：切 Host、读 Issue、打开和停止已有 Run", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await open(page, "&viewport=phone&variant=A&refresh=normal");

  await page.getByRole("button", { name: "书房 Mini" }).click();
  await expect(page.getByRole("button", { name: "shop-api" })).toBeVisible();
  await page.getByRole("button", { name: /#30 修支付超时/ }).click();
  await expect(page.locator(".issue-body")).toContainText("支付回调超过 8 秒");

  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(page.locator(".term")).toBeVisible();
  await page.locator('[data-act="fake-continue"]').click();
  await page.locator('[data-act="ask-stop"]').click();
  await expect(page.locator('[data-act="stop-soft"]')).toBeVisible();
  await page.locator('[data-act="stop-soft"]').click();
  await expect(page.locator(".term")).toContainText("认领还在");
  await expect(page.locator('[data-act="fake-continue"]')).toBeVisible();
});
