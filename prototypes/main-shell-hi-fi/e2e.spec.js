const { test, expect } = require("@playwright/test");

const prototypeUrl = process.env.PROTOTYPE_URL || "http://127.0.0.1:4173";

const DIRECTIONS = ["codex-map", "codex", "paper"];

function urlWithDirection(page, direction) {
  return new RegExp(`[?&]direction=${direction}(&|$)`);
}

test("三个方向可切换，底部切换与 URL 参数稳定", async ({ page }) => {
  await page.goto(`${prototypeUrl}/?direction=codex-map&scenario=daily`);

  // 原貌映射：左侧原生栏 + 中间看板/图 + 底栏占自己的高度（不悬浮遮挡）
  await expect(page.getByText("Codex 原貌映射", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("结构已动 · 对照原貌", { exact: true }).first()).toBeVisible();
  await expect(page.locator(".map-side")).toBeVisible();
  await expect(page.getByRole("button", { name: "本机 · MacBook" })).toBeVisible();
  await expect(page.getByRole("button", { name: "书房 Mini" })).toBeVisible();
  await expect(page.getByRole("button", { name: "公司台式机" })).toBeVisible();
  await expect(page.getByRole("button", { name: "+ 配对" })).toBeVisible();
  await expect(page.locator(".dock")).toBeVisible();
  await expect(page.locator(".map-detail-col")).toBeVisible();
  await expect(page.locator(".map-dock")).toHaveCount(0);
  await page.locator('[data-act="issue"][data-id="50"]').first().click();
  await expect(page.getByText("属于 / 子票", { exact: true })).toBeVisible();
  await expect(page.locator(".lanes").first()).toBeVisible();
  await expect(page.locator(".col-hd").filter({ hasText: /^阻塞中/ })).toBeVisible();
  await expect(page.locator(".col-hd").filter({ hasText: /^Frontier/ })).toBeVisible();
  await expect(page.locator(".col-hd").filter({ hasText: /^进行中/ })).toBeVisible();
  await expect(page.locator(".col-hd").filter({ hasText: /^最近完成/ })).toBeVisible();
  await expect(page.getByRole("button", { name: "看板视图", exact: true }).first()).toBeVisible();
  await expect(page.getByRole("button", { name: "依赖图", exact: true }).first()).toBeVisible();
  await expect(page.locator(".map-side [data-act=\"focus-run\"][data-id=\"r1\"]")).toBeVisible();
  await expect(page.locator(".map-side").getByRole("button", { name: "全部 Run", exact: true })).toHaveCount(0);
  await expect(page.locator(".dock").getByRole("button", { name: "全部 Run", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "查看更多最近完成", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "占满右侧", exact: true }).first()).toBeVisible();
  await expect(page.getByRole("button", { name: "浅色终端", exact: true }).first()).toBeVisible();
  await expect(page.getByText("查看改动", { exact: true }).first()).toBeVisible();

  // 左侧进行中的 Run：点进中间这次 Run，右侧仍是 Issue
  await page.locator('.map-side [data-act="focus-run"][data-id="r1"]').click();
  await expect(page).toHaveURL(/mid=run/);
  await expect(page.locator(".run-stage")).toBeVisible();
  await expect(page.getByText("属于 / 子票", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "看板视图", exact: true }).first().click();

  // 依赖图：点节点只换详情；第一次打开默认折终端
  await page.getByRole("button", { name: "依赖图", exact: true }).first().click();
  await expect(page).toHaveURL(/mid=graph/);
  await expect(page.locator(".graph-canvas")).toBeVisible();
  await expect(page.locator(".dock.slim")).toBeVisible();
  await page.locator('[data-act="graph-node"][data-id="51"]').click();
  await expect(page.getByText("#51 实现：依赖图", { exact: true }).first()).toBeVisible();
  await page.getByRole("button", { name: "看板视图", exact: true }).first().click();
  await expect(page.locator(".col-hd").filter({ hasText: /^Frontier/ })).toBeVisible();

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

  await page.locator('[data-act="toggle-vc"]').first().click();
  await page.locator('[data-act="note-line"]').first().click();
  const draft = page.locator("textarea[data-act=note-draft]");
  await draft.focus();
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("ArrowLeft");
  await expect(page).toHaveURL(urlWithDirection(page, "codex-map"));

  // 移开焦点后恢复切换
  await page.getByRole("button", { name: "本机 · MacBook" }).click();
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

    // Host/Project：先切 Host，再点这个 Host 上的 Project
    await page.locator('[data-act="host"][data-id="mini"]').click();
    await page.locator('[data-act="project"][data-id="shop"]').first().click();
    await expect(page.getByText("#30 修支付超时", { exact: true }).first()).toBeVisible();

    // Issue 详情
    await page.locator('[data-act="issue"][data-id="30"]').first().click();
    await expect(page.getByText("属于 / 子票", { exact: true })).toBeVisible();
    await expect(page.getByText("挡住它的", { exact: true }).first()).toBeVisible();

    // 依赖图仍在，不因观感方向丢失
    await page.getByRole("button", { name: "依赖图", exact: true }).first().click();
    await expect(page.locator(".graph-canvas")).toBeVisible();
    await page.locator('[data-act="graph-node"]').first().click();
    await expect(page.locator(".graph-canvas")).toBeVisible();
    await page.getByRole("button", { name: "看板视图", exact: true }).first().click();

    // Run：切到 r2（已停），终端给出恢复提示
    await page.locator('[data-act="run"][data-id="r2"]').first().click();
    await expect(page.getByText("认领还在。优先恢复原生会话。", { exact: false })).toBeVisible();

    // 设置面板列出三个方向
    await page.locator('[data-act="settings"]').first().click();
    await expect(page.getByText("观感方向", { exact: true })).toBeVisible();
    await expect(page.getByText(/没有取：线程列表、聊天输入框和会话主表面/)).toBeVisible();
    await expect(page.getByText("Codex 原貌映射", { exact: true }).last()).toBeVisible();
    await page.locator('[data-act="settings"]').last().click();

    // 空状态：未配对 / 无 Project / Frontier 为空 / 执行已停
    await page.locator('select[data-act="scenario"]').selectOption("unpaired");
    await expect(page.getByText("这个窗口还没连上 Host", { exact: true }).first()).toBeVisible();
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
