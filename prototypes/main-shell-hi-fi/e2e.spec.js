const { test, expect } = require("@playwright/test");

const prototypeUrl = process.env.PROTOTYPE_URL || "http://127.0.0.1:4173";

async function open(page, params = "") {
  await page.goto(`${prototypeUrl}/?direction=codex-map&scenario=daily${params}`);
}

async function startFromIssue(page) {
  await page.locator(".issue-hd [data-act=open-launch]").click();
}

test("三种开 Run 结构切换，URL 与键盘稳定", async ({ page }) => {
  await open(page, "&variant=A");

  await expect(page.locator(".map-side")).toBeVisible();
  await expect(page.locator(".lanes")).toBeVisible();
  await expect(page.getByRole("button", { name: "开 Run" }).first()).toBeVisible();
  await expect(page).toHaveURL(/variant=A/);

  await page.getByRole("button", { name: /B · 票内/ }).click();
  await expect(page).toHaveURL(/variant=B/);

  await page.getByRole("button", { name: /C · 先选/ }).click();
  await expect(page).toHaveURL(/variant=C/);

  await page.keyboard.press("ArrowRight");
  await expect(page).toHaveURL(/variant=A/);
  await page.keyboard.press("ArrowLeft");
  await expect(page).toHaveURL(/variant=C/);
});

test("A：底栏配置台；切 Agent 换字段；Claude 未安装不能启动；预填可改", async ({ page }) => {
  await open(page, "&variant=A");
  await startFromIssue(page);

  await expect(page.locator(".launch-dock")).toBeVisible();
  await expect(page.locator(".launch-banner").first()).toContainText("预填自这个 Project");
  await expect(page.locator(".launch-dock")).toContainText("权限模式");
  await expect(page.locator(".launch-dock")).toContainText("alwaysApprove");

  await page.locator('.agent-seg [data-id="agy"]').click();
  await expect(page.locator(".launch-banner").first()).toContainText("第一次用 Antigravity CLI");
  await expect(page.locator(".launch-dock")).toContainText("执行模式");
  await expect(page.locator(".launch-dock")).toContainText("跳过权限确认");
  await expect(page.locator(".launch-dock")).toContainText("子 Agent");
  await expect(page.locator(".launch-dock")).toContainText("没有原生建隔离执行目录");

  await page.locator('.agent-seg [data-id="claude"]').click();
  await expect(page.locator(".launch-dock")).toContainText("未安装");
  await expect(page.locator('[data-act=launch-commit]').first()).toBeDisabled();

  await page.locator('.agent-seg [data-id="codex"]').click();
  await expect(page.locator(".launch-dock")).toContainText("approval");
  await expect(page.locator('[data-act=launch-iso]')).toBeDisabled();

  await page.locator('.agent-seg [data-id="grok"]').click();
  await expect(page.locator(".cmd-preview")).toContainText("grok --model");
  await page.locator('[data-act=launch-commit]').first().click();
  await expect(page.locator(".run-stage, .term").first()).toBeVisible();
  await expect(page.locator(".launch-dock")).toHaveCount(0);
});

test("点已有 Run 的票直接进中间终端，不打开配置表", async ({ page }) => {
  await open(page, "&variant=A");

  await page.locator('.lanes [data-act=issue][data-id="50"]').click();
  await expect(page.locator(".run-stage")).toBeVisible();
  await expect(page.locator(".launch-dock, .launch-drawer, .launch-palette")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "返回看板" })).toBeVisible();
});

test("B：表单在票详情；游离挂在 Host 区且不认领", async ({ page }) => {
  await open(page, "&variant=B");
  await startFromIssue(page);

  await expect(page.locator(".map-detail-col .launch-drawer")).toBeVisible();
  await expect(page.locator(".agent-list")).toBeVisible();
  await page.getByRole("button", { name: "×" }).first().click();
  await expect(page.locator(".launch-drawer")).toHaveCount(0);

  await page.locator('.map-side [data-act=open-launch][data-kind=free]').click();
  await expect(page.locator(".launch-free-sheet")).toBeVisible();
  await expect(page.locator(".launch-free-sheet")).toContainText("未绑定 Issue");
  await page.locator('.launch-free-sheet [data-act=launch-commit]').first().click();
  await expect(page.locator(".run-stage, .term").first()).toBeVisible();
  await expect(page.locator(".tag").filter({ hasText: "未绑定 Issue" }).first()).toBeVisible();
});

test("C：先命令面板选 Agent，再居中表单；游离共用面板", async ({ page }) => {
  await open(page, "&variant=C");
  await startFromIssue(page);

  await expect(page.locator(".launch-palette")).toBeVisible();
  await expect(page.locator(".launch-palette")).toContainText("选 Agent");
  await page.locator('.launch-palette .agent-row[data-id="grok"]').click();
  await expect(page.locator(".launch-modal-card")).toBeVisible();
  await expect(page.locator(".launch-palette")).toHaveCount(0);
  await page.getByRole("button", { name: "换一家" }).click();
  await expect(page.locator(".launch-palette")).toBeVisible();

  await page.getByRole("button", { name: "×" }).click();
  await page.locator('.map-chrome-trail [data-act=open-launch][data-kind=free]').click();
  await expect(page.locator(".launch-palette")).toContainText("开游离 Run");
});

test("字段校验与启动失败夹具", async ({ page }) => {
  await open(page, "&variant=A");
  await page.getByRole("button", { name: "失败夹具" }).click();
  await startFromIssue(page);
  await page.locator('[data-act=launch-commit]').first().click();
  await expect(page.locator(".launch-banner.bad")).toContainText("启动失败");
  await expect(page.locator(".launch-dock")).toBeVisible();
});

test("手机：Run 页收敛成配置表，仍可开停", async ({ page }) => {
  await open(page, "&variant=A&viewport=phone");

  await expect(page.locator(".phone")).toBeVisible();
  await page.getByRole("button", { name: "票" }).click();
  await expect(page.getByRole("button", { name: "开 Run" }).first()).toBeVisible();
  await page.getByRole("button", { name: "开 Run" }).first().click();
  await expect(page.locator(".launch-dock, .launch-form").first()).toBeVisible();
  await expect(page.locator(".agent-seg")).toBeVisible();
});
