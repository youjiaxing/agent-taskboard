const { test, expect } = require("@playwright/test");

const prototypeUrl = process.env.PROTOTYPE_URL || "http://127.0.0.1:4173";

const DIRECTIONS = ["codex-map", "codex", "paper"];

function urlWithDirection(page, direction) {
  return new RegExp(`[?&]direction=${direction}(&|$)`);
}

test("三个方向可切换，底部切换与 URL 参数稳定", async ({ page }) => {
  await page.goto(`${prototypeUrl}/?direction=codex-map&scenario=daily`);

  // 原貌映射：左侧原生栏 + 大片白主区 + 底部悬浮操作面，四列都在
  await expect(page.getByText("Codex 原貌映射", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("结构已动 · 对照原貌", { exact: true }).first()).toBeVisible();
  await expect(page.locator(".map-side")).toBeVisible();
  await expect(page.locator(".map-dock")).toBeVisible();
  await expect(page.locator(".map-detail")).toHaveCount(0);
  await page.locator('[data-act="issue"][data-id="50"]').first().click();
  await expect(page.locator(".map-detail")).toBeVisible();
  await page.locator('[data-act="map-close-detail"]').click();
  await expect(page.locator(".map-detail")).toHaveCount(0);
  await expect(page.locator(".map-lanes, .map-board .lanes").first()).toBeVisible();
  await expect(page.locator(".col-hd").filter({ hasText: /^阻塞中/ })).toBeVisible();
  await expect(page.locator(".col-hd").filter({ hasText: /^Frontier/ })).toBeVisible();
  await expect(page.locator(".col-hd").filter({ hasText: /^进行中/ })).toBeVisible();
  await expect(page.locator(".col-hd").filter({ hasText: /^最近完成/ })).toBeVisible();
  await expect(page.getByText("查看改动", { exact: true }).first()).toBeVisible();

  // 切到气质版：结构未改，顶栏仍在
  await page.getByRole("button", { name: "Codex 气质", exact: true }).first().click();
  await expect(page).toHaveURL(urlWithDirection(page, "codex"));
  await expect(page.getByText("Codex 气质", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("结构未改", { exact: true }).first()).toBeVisible();
  await expect(page.locator(".topbar")).toBeVisible();
  await expect(page.locator(".col-hd").filter({ hasText: /^Frontier/ })).toBeVisible();

  // 切到纸面精修
  await page.getByRole("button", { name: "纸面精修", exact: true }).first().click();
  await expect(page).toHaveURL(urlWithDirection(page, "paper"));
  await expect(page.getByText("纸面精修", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("结构未改", { exact: true }).first()).toBeVisible();
  await expect(page.locator(".col-hd").filter({ hasText: /^最近完成/ })).toBeVisible();

  // 键盘 ←/→ 循环切换（非输入焦点）
  await page.keyboard.press("ArrowLeft");
  await expect(page).toHaveURL(urlWithDirection(page, "codex"));
  await page.keyboard.press("ArrowRight");
  await expect(page).toHaveURL(urlWithDirection(page, "paper"));
  await page.keyboard.press("ArrowRight");
  await expect(page).toHaveURL(urlWithDirection(page, "codex-map"));
});

test("输入焦点时键盘 ←/→ 不切换方向", async ({ page }) => {
  await page.goto(`${prototypeUrl}/?direction=codex-map&scenario=daily`);

  const search = page.locator(".ms-search");
  await search.focus();
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("ArrowLeft");
  await expect(page).toHaveURL(urlWithDirection(page, "codex-map"));

  // 移开焦点后恢复切换
  await page.locator(".ms-brand").click();
  await page.keyboard.press("ArrowRight");
  await expect(page).toHaveURL(urlWithDirection(page, "codex"));
});

test("三个方向都能查看改动（这一轮 / 未提交）", async ({ page }) => {
  for (const direction of DIRECTIONS) {
    await page.goto(`${prototypeUrl}/?direction=${direction}&scenario=daily`);
    await page.locator('[data-act="toggle-vc"]').first().click();
    await expect(page.getByText("这一轮", { exact: true })).toBeVisible();
    await expect(page.getByText("未提交", { exact: true })).toBeVisible();
  }
});

test("三个方向都保留 Host/Project、Issue、Run 与空状态", async ({ page }) => {
  for (const direction of DIRECTIONS) {
    await page.goto(`${prototypeUrl}/?direction=${direction}&scenario=daily`);

    // Host/Project：原貌映射在侧栏分组直接点项目；其余方向先切 Host 再点项目
    if (direction === "codex-map") {
      await page.locator('[data-act="project"][data-id="shop"]').first().click();
    } else {
      await page.locator('[data-act="host"][data-id="mini"]').click();
      await page.locator('[data-act="project"][data-id="shop"]').first().click();
    }
    await expect(page.getByText("#30 修支付超时", { exact: true }).first()).toBeVisible();

    // Issue 详情
    await page.locator('[data-act="issue"][data-id="30"]').first().click();
    await expect(page.getByText("属于 / 子票", { exact: true })).toBeVisible();
    await expect(page.getByText("挡住它的", { exact: true }).first()).toBeVisible();

    // Run：切到 r2（已停），终端给出恢复提示
    await page.locator('[data-act="run"][data-id="r2"]').first().click();
    await expect(page.getByText("认领还在。优先恢复原生会话。", { exact: false })).toBeVisible();

    // 设置面板列出三个方向
    if (direction === "codex-map") {
      await page.locator('[data-act="map-nav"][data-id="settings"]').click();
    } else {
      await page.locator('[data-act="settings"]').first().click();
    }
    await expect(page.getByText("观感方向", { exact: true })).toBeVisible();
    await expect(page.getByText(/没有取：线程列表、聊天输入框和会话主表面/)).toBeVisible();
    await expect(page.getByText("Codex 原貌映射", { exact: true }).last()).toBeVisible();
    await page.locator('[data-act="settings"]').last().click();

    // 空状态：未配对 / 无 Project / Frontier 为空 / 执行已停
    await page.locator('select[data-act="scenario"]').selectOption("unpaired");
    await expect(page.getByText("这个窗口还没连上 Host", { exact: true })).toBeVisible();
    await page.locator('select[data-act="scenario"]').selectOption("noproject");
    await expect(page.getByText("这台 Host 上还没有 Project", { exact: true }).first()).toBeVisible();
    await page.locator('select[data-act="scenario"]').selectOption("emptyfront");
    await expect(page.getByText("没有可领的票。剩下的都还被挡住。", { exact: true })).toBeVisible();
    await page.locator('select[data-act="scenario"]').selectOption("stopped");
    await expect(page.getByText("认领还在。优先恢复原生会话。", { exact: false })).toBeVisible();
  }
});

test("390px 手机：三个方向都可切换并查看降级界面", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  for (const direction of DIRECTIONS) {
    await page.goto(`${prototypeUrl}/?direction=${direction}&viewport=phone&scenario=daily`);
    await expect(page.getByText("手机只看态势和开停。查看改动请到电脑。")).toBeVisible();
    await expect(page.getByText("态势", { exact: true })).toBeVisible();
    await expect(page.getByText("Run", { exact: true }).last()).toBeVisible();
  }

  // 手机内切换方向
  await page.getByRole("button", { name: "纸面精修", exact: true }).first().click();
  await expect(page).toHaveURL(urlWithDirection(page, "paper"));
  await page.getByRole("button", { name: "Codex 原貌映射", exact: true }).first().click();
  await expect(page).toHaveURL(urlWithDirection(page, "codex-map"));
});
