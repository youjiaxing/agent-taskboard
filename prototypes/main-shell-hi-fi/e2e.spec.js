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
  await expect(page.getByRole("button", { name: "执行" }).first()).toBeVisible();
  await expect(page).toHaveURL(/variant=A/);

  await page.getByRole("button", { name: /B · 票内/ }).click();
  await expect(page).toHaveURL(/variant=B/);

  await page.getByRole("button", { name: /C · 每次/ }).click();
  await expect(page).toHaveURL(/variant=C/);

  await page.keyboard.press("ArrowRight");
  await expect(page).toHaveURL(/variant=A/);
  await page.keyboard.press("ArrowLeft");
  await expect(page).toHaveURL(/variant=C/);
});

test("A：记住上次 Agent 直接填表；换一家才回名单；Claude 未安装不能启动", async ({ page }) => {
  await open(page, "&variant=A");
  await startFromIssue(page);

  await expect(page.locator(".launch-modal-card")).toBeVisible();
  await expect(page.locator(".launch-palette")).toHaveCount(0);
  await expect(page.locator(".launch-modal-card")).toContainText("这个 Project 上次启动用的");
  await expect(page.locator(".launch-banner").first()).toContainText("预填自这个 Project");
  await expect(page.locator(".launch-modal-card")).toContainText("权限模式");

  await page.getByRole("button", { name: "换一家" }).click();
  await expect(page.locator(".launch-palette")).toBeVisible();
  await page.locator('.launch-palette .agent-row[data-id="agy"]').click();
  await expect(page.locator(".launch-modal-card")).toContainText("第一次用 Antigravity CLI");
  await expect(page.locator(".launch-modal-card")).toContainText("执行模式");
  await expect(page.locator(".launch-modal-card")).toContainText("子 Agent");

  await page.getByRole("button", { name: "换一家" }).click();
  await page.locator('.launch-palette .agent-row[data-id="claude"]').click();
  await expect(page.locator(".launch-modal-card")).toContainText("未安装");
  await expect(page.locator('[data-act=launch-commit]').first()).toBeDisabled();

  await page.getByRole("button", { name: "换一家" }).click();
  await page.locator('.launch-palette .agent-row[data-id="codex"]').click();
  await expect(page.locator(".launch-modal-card")).toContainText("approval");
  await expect(page.locator('[data-act=launch-iso]')).toBeDisabled();

  await page.getByRole("button", { name: "换一家" }).click();
  await page.locator('.launch-palette .agent-row[data-id="grok"]').click();
  await expect(page.locator(".cmd-preview")).toContainText("grok --model");
  await page.locator('[data-act=launch-commit]').first().click();
  await expect(page.locator(".run-stage, .term").first()).toBeVisible();
  await expect(page.locator(".launch-modal-card")).toHaveCount(0);
});

test("进行中卡片展示 Agent 与简单执行状态", async ({ page }) => {
  await open(page, "&variant=A");

  const col = page.locator(".col.in-run");
  await expect(col).toContainText("Grok Build");
  await expect(col).toContainText("在改 src/ui/IssueDetail.tsx");
  await expect(col).toContainText("Codex");
  await expect(col).toContainText("等你确认要不要写入 src/settings.rs");
  await expect(col).toContainText("停在跑 npm test 之前");
});

test("点已有 Run 的票直接进中间终端，不打开配置表", async ({ page }) => {
  await open(page, "&variant=A");

  await page.locator('.lanes [data-act=issue][data-id="50"]').click();
  await expect(page.locator(".run-stage")).toBeVisible();
  await expect(page.locator(".launch-dock, .launch-drawer, .launch-palette, .launch-modal-card")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "返回看板" })).toBeVisible();
});

test("不绑票入口在 Project 行右侧图标；默认开场白是空的；不认领", async ({ page }) => {
  await open(page, "&variant=A");

  await page.locator('.map-side [data-act=open-launch][data-kind=free][data-id=tb]').click();
  await expect(page.locator(".launch-modal-card")).toBeVisible();
  await expect(page.locator(".launch-modal-card")).toContainText("未绑定 Issue");
  await expect(page.locator("textarea[data-act=launch-prompt]")).toHaveValue("");
  await expect(page.locator("textarea[data-act=launch-prompt]")).toHaveAttribute("placeholder", "要 Agent 做什么，写在这里。");
  await page.locator('.launch-modal-card [data-act=launch-commit]').first().click();
  await expect(page.locator(".run-stage, .term").first()).toBeVisible();
  await expect(page.locator(".tag").filter({ hasText: "未绑定 Issue" }).first()).toBeVisible();
});

test("C：每次都先选 Agent；不绑票也从 Project 下方进", async ({ page }) => {
  await open(page, "&variant=C");
  await startFromIssue(page);

  await expect(page.locator(".launch-palette")).toBeVisible();
  await expect(page.locator(".launch-palette")).toContainText("选 Agent");
  await page.locator('.launch-palette .agent-row[data-id="grok"]').click();
  await expect(page.locator(".launch-modal-card")).toBeVisible();
  await page.getByRole("button", { name: "换一家" }).click();
  await expect(page.locator(".launch-palette")).toBeVisible();

  await page.getByRole("button", { name: "×" }).click();
  await page.locator('.map-side [data-act=open-launch][data-kind=free][data-id=tb]').click();
  await expect(page.locator(".launch-palette")).toContainText("新建");
});

test("字段校验与启动失败夹具", async ({ page }) => {
  await open(page, "&variant=A&launch_fail=1");
  await startFromIssue(page);
  await page.locator('[data-act=launch-commit]').first().click();
  await expect(page.locator(".launch-banner.bad")).toContainText("启动失败");
  await expect(page.locator(".launch-modal-card")).toBeVisible();
});

test("手机：Run 页收敛成配置表，仍可开停", async ({ page }) => {
  await open(page, "&variant=A&viewport=phone");

  await expect(page.locator(".phone")).toBeVisible();
  await page.locator('[data-act=phone][data-id=issue]').click();
  await expect(page.locator(".issue-hd [data-act=open-launch]")).toBeVisible();
  await page.locator(".issue-hd [data-act=open-launch]").click();
  await expect(page.locator(".launch-modal-card, .launch-form").first()).toBeVisible();
});
