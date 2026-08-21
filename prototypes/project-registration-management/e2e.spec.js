const { test, expect } = require("@playwright/test");

const prototypeUrl = process.env.PROTOTYPE_URL || "http://127.0.0.1:4173";

async function open(page, params = "") {
  await page.goto(`${prototypeUrl}/?variant=A${params}`);
}

async function openMenu(page, name) {
  await page.getByRole("button", { name: `管理 ${name}` }).click();
}

test("三种管理入口通过 URL、按钮和键盘稳定切换", async ({ page }) => {
  await open(page);
  await expect(page).toHaveURL(/variant=A/);
  await expect(page.getByText("A · 行尾原生菜单")).toBeVisible();
  await expect(page.getByRole("button", { name: "登记 Project", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "配对 Host", exact: true })).toBeVisible();
  await expect(page.locator(".side-foot")).toHaveCount(0);
  await openMenu(page, "agent-taskboard");
  await expect(page.getByRole("button", { name: "编辑登记…" })).toBeVisible();

  await page.getByRole("button", { name: "下一个方案" }).click();
  await expect(page).toHaveURL(/variant=B/);
  await expect(page.getByText("PROJECT 管理", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "编辑", exact: true }).first()).toBeVisible();
  await expect(page.getByRole("button", { name: "管理 agent-taskboard" })).toHaveCount(0);

  await page.keyboard.press("ArrowRight");
  await expect(page).toHaveURL(/variant=C/);
  await page.getByRole("button", { name: "管理 Project", exact: true }).last().click();
  await expect(page.getByRole("dialog", { name: "Project 管理" })).toBeVisible();
});

test("桌面可登记、确认采用推断、编辑并保存完整字段", async ({ page }) => {
  await open(page);
  await page.getByRole("button", { name: "登记 Project", exact: true }).click();
  const form = page.locator('[data-form="project"]');
  await form.getByLabel("显示名称").fill("docs-site");
  await form.getByLabel("本地目录").fill("/Users/you/Code/docs-site");
  await form.getByRole("button", { name: "从本地目录推断" }).click();
  await expect(form).toContainText("发现 GitHub · you/docs-site");
  await form.getByRole("button", { name: "使用这份推断结果" }).click();
  await form.getByRole("button", { name: "登记 Project" }).click();
  await expect(page.locator(".board-head h1")).toHaveText("docs-site");

  await openMenu(page, "docs-site");
  await page.getByRole("button", { name: "编辑登记…" }).click();
  await page.getByLabel("Issue Tracker 类型").selectOption("GitLab");
  await page.getByLabel("连接信息").fill("gitlab.example.com/docs/site");
  await page.getByRole("button", { name: "保存登记" }).click();
  await expect(page.locator(".board-head")).toContainText("GitLab · gitlab.example.com/docs/site");
});

test("普通 Project 可移除，当前 Project 按邻近项回退", async ({ page }) => {
  await open(page);
  await openMenu(page, "agent-taskboard");
  await page.getByRole("button", { name: "移除 Project…" }).click();
  await expect(page.getByText("不会删除本地目录、git 仓库", { exact: false })).toBeVisible();
  await page.getByRole("button", { name: "只移除登记" }).click();
  await expect(page.locator(".board-head h1")).toHaveText("garden-notes");
  await expect(page.getByRole("button", { name: "打开 agent-taskboard" })).toHaveCount(0);
  await expect(page.locator(".inspector")).toContainText("移除 agent-taskboard；当前回退到 garden-notes");
});

test("活跃 Run 禁止移除且不给危险确认按钮", async ({ page }) => {
  await open(page);
  await openMenu(page, "shop-api");
  await page.getByRole("button", { name: "移除 Project…" }).click();
  await expect(page.getByRole("heading", { name: "现在不能移除 shop-api" })).toBeVisible();
  await expect(page.getByText("先停止或结束 Run", { exact: false })).toBeVisible();
  await expect(page.getByRole("button", { name: "只移除登记" })).toHaveCount(0);
});

test("执行已停要求确认认领仍保留，然后才允许移除", async ({ page }) => {
  await open(page);
  await openMenu(page, "garden-notes");
  await page.getByRole("button", { name: "移除 Project…" }).click();
  const remove = page.getByRole("button", { name: "只移除登记" });
  await expect(remove).toBeDisabled();
  await expect(page.getByText("Tracker 上现有认领不会自动释放", { exact: false })).toBeVisible();
  await page.getByText("我知道 Tracker 上的认领仍会保留").click();
  await expect(remove).toBeEnabled();
  await remove.click();
  await expect(page.getByRole("button", { name: "打开 garden-notes" })).toHaveCount(0);
});

test("尚未拉取 Tracker 的 Project 仍可移除", async ({ page }) => {
  await open(page);
  await openMenu(page, "new-catalog");
  await page.getByRole("button", { name: "移除 Project…" }).click();
  await expect(page.getByText("尚未成功拉取 Tracker 数据", { exact: false })).toBeVisible();
  await page.getByRole("button", { name: "只移除登记" }).click();
  await expect(page.getByRole("button", { name: "打开 new-catalog" })).toHaveCount(0);
});

test("空 Host 从主区正式登记，配对入口保持分开", async ({ page }) => {
  await open(page, "&scenario=empty-host");
  await expect(page.getByRole("heading", { name: "这个 Host 上还没有 Project" })).toBeVisible();
  await expect(page.getByRole("button", { name: "登记第一个 Project" })).toBeVisible();
  await expect(page.getByRole("button", { name: "配对另一个 Host", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "登记第一个 Project" }).click();
  await page.getByLabel("显示名称").fill("first-project");
  await page.getByLabel("本地目录").fill("/tmp/first-project");
  await page.getByLabel("连接信息").fill("you/first-project");
  await page.locator('[data-form="project"]').getByRole("button", { name: "登记 Project", exact: true }).click();
  await expect(page.locator(".board-head h1")).toHaveText("first-project");
});

test("390px 手机在切换范围内登记、编辑和移除，不平铺到顶栏", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await open(page, "&viewport=phone");
  await page.getByRole("button", { name: "切换" }).click();
  await expect(page.getByText("切换范围", { exact: true })).toBeVisible();
  await expect(page.locator(".phone-head")).toContainText("agent-taskboard");
  await expect(page.locator(".phone-head")).not.toContainText("garden-notes");

  await openMenu(page, "agent-taskboard");
  await page.getByRole("button", { name: /编辑 agent-taskboard/ }).click();
  await page.getByLabel("连接信息").fill("you/taskboard-mobile");
  await page.getByRole("button", { name: "保存登记" }).click();
  await expect(page.locator(".scope")).toContainText("agent-taskboard");

  await page.getByRole("button", { name: "＋ 登记 Project" }).click();
  await page.getByLabel("显示名称").fill("phone-added");
  await page.getByLabel("本地目录").fill("/tmp/phone-added");
  await page.getByLabel("连接信息").fill("you/phone-added");
  await page.getByRole("button", { name: "登记 Project", exact: true }).click();
  await expect(page.locator(".phone-head")).toContainText("phone-added");
  await expect(page.locator(".scope")).toBeVisible();

  await openMenu(page, "phone-added");
  await page.getByRole("button", { name: /移除 Project/ }).click();
  await page.getByRole("button", { name: "只移除登记" }).click();
  await expect(page.locator(".phone-head")).toContainText("new-catalog");
});

test("390px 空 Host 的登记入口留在切换范围", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await open(page, "&viewport=phone&scenario=empty-host");
  await expect(page.locator(".scope")).toBeVisible();
  await expect(page.locator(".scope").getByRole("button", { name: "＋ 登记 Project" })).toBeVisible();
  await expect(page.locator(".phone-head")).toContainText("我的 MacBook");
});
