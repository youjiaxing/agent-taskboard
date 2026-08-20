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
  await expect(page.locator(".issue-body")).toContainText("详情栏现在只画家族");
  await expect(page.getByRole("button", { name: "加宽", exact: true })).toBeVisible();
  await expect(page.locator(".lanes").first()).toBeVisible();
  await expect(page.locator(".col-hd").filter({ hasText: /^阻塞中/ })).toBeVisible();
  await expect(page.locator(".col-hd").filter({ hasText: /^Frontier/ })).toBeVisible();
  await expect(page.locator(".col-hd").filter({ hasText: /^进行中/ })).toBeVisible();
  await expect(page.locator(".col-hd").filter({ hasText: /^最近完成/ })).toBeVisible();
  await expect(page.getByRole("button", { name: "看板", exact: true }).first()).toBeVisible();
  await expect(page.getByRole("button", { name: "依赖图", exact: true }).first()).toBeVisible();
  await expect(page.getByRole("button", { name: "总览", exact: true }).first()).toBeVisible();
  await expect(page.getByRole("button", { name: "这次 Run", exact: true })).toHaveCount(0);
  await expect(page.locator(".map-side [data-act=\"focus-run\"][data-id=\"r1\"]")).toBeVisible();
  await expect(page.locator(".map-side").getByRole("button", { name: "所有 Run", exact: true })).toHaveCount(0);
  await expect(page.locator(".dock").getByRole("button", { name: "所有 Run", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "查看更多最近完成", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "占满右侧", exact: true }).first()).toBeVisible();
  await expect(page.getByRole("button", { name: "浅色终端", exact: true }).first()).toBeVisible();
  await expect(page.getByRole("button", { name: "停止", exact: true }).first()).toBeVisible();
  await expect(page.getByText("查看改动", { exact: true }).first()).toBeVisible();

  // 打开终端：抬到中间；放回底栏恢复。顶栏没有「这次 Run」。
  await page.getByRole("button", { name: "打开终端", exact: true }).click();
  await expect(page).toHaveURL(/mid=run/);
  await expect(page.locator(".run-stage")).toBeVisible();
  await expect(page.locator(".dock")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "这次 Run", exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "放回底栏", exact: true }).first()).toBeVisible();
  await expect(page.getByText("属于 / 子票", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "放回底栏", exact: true }).first().click();
  await expect(page.locator(".dock")).toBeVisible();
  await expect(page.locator(".lanes").first()).toBeVisible();
  await expect(page.locator(".term")).toBeVisible();

  // 依赖图：点节点只换详情；第一次打开默认折终端
  await page.getByRole("button", { name: "依赖图", exact: true }).first().click();
  await expect(page).toHaveURL(/mid=graph/);
  await expect(page.locator(".graph-canvas")).toBeVisible();
  await expect(page.locator(".dock")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "展开终端", exact: true })).toBeVisible();
  await page.locator('[data-act="graph-node"][data-id="51"]').click();
  await expect(page.getByText("#51 实现：依赖图", { exact: true }).first()).toBeVisible();
  await page.getByRole("button", { name: "看板", exact: true }).first().click();
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

test("折叠栏不占布局，只留浮层展开", async ({ page }) => {
  await page.goto(`${prototypeUrl}/?direction=codex-map&scenario=daily`);
  const lanesBefore = await page.locator(".lanes").first().boundingBox();
  const toggle = page.locator(".map-chrome [data-act=\"toggle-side\"]");
  const foldBox = await toggle.boundingBox();
  await expect(toggle).toHaveAttribute("aria-label", "收起左侧");
  await toggle.click();
  await expect(page.locator(".map-side")).toHaveCount(0);
  await expect(toggle).toHaveAttribute("aria-label", "展开左侧");
  const unfoldBox = await toggle.boundingBox();
  expect(Math.abs(unfoldBox.x - foldBox.x)).toBeLessThan(2);
  expect(Math.abs(unfoldBox.y - foldBox.y)).toBeLessThan(2);
  const lanesAfter = await page.locator(".lanes").first().boundingBox();
  expect(lanesAfter.x).toBeLessThan(lanesBefore.x - 80);

  await page.getByRole("button", { name: "总览", exact: true }).first().click();
  const toggleOnOverview = await toggle.boundingBox();
  const allProjects = await page.getByRole("button", { name: "全部 Project", exact: true }).boundingBox();
  expect(Math.abs(toggleOnOverview.x - foldBox.x)).toBeLessThan(2);
  expect(toggleOnOverview.y + toggleOnOverview.height).toBeLessThan(allProjects.y + 2);
  await page.getByRole("button", { name: "看板", exact: true }).first().click();

  await page.getByRole("button", { name: "收起终端", exact: true }).click();
  await expect(page.locator(".dock")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "展开终端", exact: true })).toBeVisible();

  const detailToggle = page.locator(".map-chrome-trail [data-act=\"toggle-detail\"]");
  const detailBox = await detailToggle.boundingBox();
  await expect(detailToggle).toHaveAttribute("aria-label", "收起 Issue");
  await detailToggle.click();
  await expect(page.locator(".map-detail-col")).toHaveCount(0);
  await expect(detailToggle).toHaveAttribute("aria-label", "展开 Issue");
  const detailUnfold = await detailToggle.boundingBox();
  expect(Math.abs(detailUnfold.x - detailBox.x)).toBeLessThan(2);
  expect(Math.abs(detailUnfold.y - detailBox.y)).toBeLessThan(2);
  await expect(page.locator(".mid-bar").getByRole("button", { name: "展开 Issue", exact: true })).toHaveCount(0);

  const settingsBtn = page.locator(".map-chrome-lead [data-act=\"settings\"]");
  await expect(settingsBtn).toBeVisible();
  const settingsBox = await settingsBtn.boundingBox();
  const sideBox = await toggle.boundingBox();
  expect(settingsBox.x).toBeGreaterThan(sideBox.x);
  expect(Math.abs(settingsBox.y - sideBox.y)).toBeLessThan(4);
  await settingsBtn.click();
  await expect(page.getByText("界面语言", { exact: true })).toBeVisible();
  await expect(page.locator(".settings-modal").getByText("观感方向", { exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: "素纸夜间", exact: true }).click();
  await expect(page).toHaveURL(/skin=plain-dark/);
  await page.locator('[data-act="settings"]').last().click();
});

test("右侧 Issue 第一屏能读正文，加宽后栏变宽", async ({ page }) => {
  await page.goto(`${prototypeUrl}/?direction=codex-map&scenario=daily`);
  await page.locator('[data-act="issue"][data-id="50"]').first().click();
  await expect(page.locator(".issue-title")).toHaveText("#50 实现：详情挡住名单");
  await expect(page.locator(".issue-body")).toContainText("Question");
  await expect(page.locator(".issue-body")).toContainText("加宽");
  await page.getByRole("button", { name: "加宽", exact: true }).click();
  await expect(page).toHaveURL(/iw=1/);
  await expect(page.locator(".map-cols.issue-wide")).toBeVisible();
  await page.locator('[data-act="issue"][data-id="1"]').first().click();
  await expect(page.locator(".issue-body")).toContainText("这张地图为什么这么长");
});

test("点 Issue 时底栏跟 Run：有则切换，无则给出开 Run", async ({ page }) => {
  await page.goto(`${prototypeUrl}/?direction=codex-map&scenario=daily`);
  await page.locator('[data-act="issue"][data-id="50"]').first().click();
  await expect(page.locator(".dock .term")).toBeVisible();
  await expect(page.locator(".dock")).toContainText("#50");

  await page.locator('[data-act="issue"][data-id="51"]').first().click();
  await expect(page.locator(".dock .term")).toHaveCount(0);
  await expect(page.locator(".dock-idle")).toBeVisible();
  await expect(page.locator(".dock").getByRole("button", { name: "开 Run", exact: true })).toBeVisible();
  await expect(page.getByText("这张 Issue 没有进行中的 Run", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "认领", exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "只认领", exact: true })).toBeVisible();

  await page.locator(".dock").getByRole("button", { name: "开 Run", exact: true }).click();
  await expect(page.locator(".dock .term")).toBeVisible();
  await expect(page.getByText("已认领", { exact: true }).first()).toBeVisible();
  await expect(page.locator('.lanes [data-act="issue"][data-id="51"]').first()).toBeVisible();

  await page.locator('[data-act="issue"][data-id="90"]').first().click();
  await expect(page.locator(".dock .term")).toBeVisible();
  await expect(page.locator(".dock")).toContainText("#90");
});

test("总览按终端状态分组，可按 Project 过滤", async ({ page }) => {
  await page.goto(`${prototypeUrl}/?direction=codex-map&scenario=daily`);
  await page.getByRole("button", { name: "总览", exact: true }).first().click();
  await expect(page).toHaveURL(/mid=overview/);
  await expect(page.locator(".ov")).toBeVisible();
  await expect(page.locator(".ov-group-hd").filter({ hasText: "在等人" })).toBeVisible();
  await expect(page.locator(".ov-group-hd").filter({ hasText: "进行中" })).toBeVisible();
  await expect(page.locator(".ov-group-hd").filter({ hasText: "执行已停" })).toBeVisible();
  await expect(page.locator(".ov-tile")).toHaveCount(4);
  await expect(page.locator(".dock")).toHaveCount(0);

  await page.locator('[data-act="ov-run"][data-id="r5"]').click();
  await expect(page).toHaveURL(/mid=run/);
  await expect(page.locator(".run-stage")).toBeVisible();
  await expect(page.locator(".issue-title")).toHaveText("#90 实现：设置页");
  await expect(page.getByRole("button", { name: "返回总览", exact: true }).first()).toBeVisible();
  await page.getByRole("button", { name: "下一条", exact: true }).click();
  await expect(page.locator(".run-stage")).toBeVisible();
  await page.getByRole("button", { name: "返回总览", exact: true }).first().click();
  await expect(page).toHaveURL(/mid=overview/);
  await expect(page.locator(".ov")).toBeVisible();
  await expect(page.locator(".ov-tile.sel")).toBeVisible();

  await page.locator('[data-act="ov-proj"][data-id="notes"]').click();
  await expect(page.getByText("这个过滤下没有 Run", { exact: true })).toBeVisible();
  await page.locator('[data-act="ov-proj"][data-id="all"]').click();
  await expect(page.locator(".ov-tile")).toHaveCount(4);

  await page.getByRole("button", { name: "显示已结束", exact: true }).click();
  await expect(page.locator(".ov-group-hd").filter({ hasText: "已结束" })).toBeVisible();
  await expect(page.locator(".ov-tile")).toHaveCount(5);
});

test("占满右侧只放大现有终端，不复制一份", async ({ page }) => {
  await page.goto(`${prototypeUrl}/?direction=codex-map&scenario=daily`);
  await page.getByRole("button", { name: "打开终端", exact: true }).click();
  await expect(page.locator(".run-stage")).toBeVisible();
  await expect(page.locator(".dock")).toHaveCount(0);
  await expect(page.locator(".term")).toHaveCount(1);

  await page.getByRole("button", { name: "占满右侧", exact: true }).first().click();
  await expect(page).toHaveURL(/zoom=right/);
  await expect(page.locator(".zoom-stage")).toHaveCount(0);
  await expect(page.locator(".term")).toHaveCount(1);
  await expect(page.locator(".map-detail-col")).toHaveCount(0);
  await expect(page.locator(".run-stage")).toBeVisible();

  await page.getByRole("button", { name: "看板", exact: true }).first().click();
  await expect(page.locator(".zoom-stage")).toHaveCount(0);
  await expect(page.locator(".term")).toHaveCount(1);
  await expect(page.locator(".map-term-col")).toBeVisible();
  await expect(page.locator(".lanes")).toBeVisible();
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
    await expect(page.getByText("挡住它的 / 它挡住的", { exact: true })).toBeVisible();
    await expect(page.locator(".issue-body")).toContainText("支付回调超过 8 秒");

    // 依赖图仍在，不因观感方向丢失
    await page.getByRole("button", { name: "依赖图", exact: true }).first().click();
    await expect(page.locator(".graph-canvas")).toBeVisible();
    await page.locator('[data-act="graph-node"]').first().click();
    await expect(page.locator(".graph-canvas")).toBeVisible();
    await page.getByRole("button", { name: "看板", exact: true }).first().click();

    // 点有 Run 的 Issue，底栏切到对应终端
    await page.locator('[data-act="issue"][data-id="30"]').first().click();
    await expect(page.getByText("认领还在。优先恢复原生会话。", { exact: false })).toBeVisible();

    // 设置面板：语言、主题清单；观感方向只在原型底栏
    await page.locator('[data-act="settings"]').first().click();
    await expect(page.getByText("界面语言", { exact: true })).toBeVisible();
    await expect(page.getByText("主题", { exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "暖纸", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "素纸", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "素纸夜间", exact: true })).toBeVisible();
    await expect(page.locator(".settings-modal").getByText("观感方向", { exact: true })).toHaveCount(0);
    await expect(page.getByText("自动推进", { exact: true })).toBeVisible();
    await expect(page.getByText("这台 Host", { exact: true })).toBeVisible();
    await expect(page.locator('[data-act="auto-advance"][data-id="off"]')).toHaveClass(/active/);
    await expect(page.getByText("冷启动后恢复自动推进", { exact: true })).toHaveCount(0);
    await page.locator('[data-act="auto-advance"][data-id="on"]').click();
    await expect(page.getByText("冷启动后恢复自动推进", { exact: true })).toBeVisible();
    await expect(page.locator('[data-act="cold-resume"][data-id="off"]')).toHaveClass(/active/);
    await expect(page.getByText("等待秒数", { exact: true })).toHaveCount(0);
    await page.locator('[data-act="cold-resume"][data-id="on"]').click();
    await expect(page.getByText("等待秒数", { exact: true })).toBeVisible();
    await expect(page.locator("[data-role=cold-wait-val]")).toHaveText("60");
    await page.locator('[data-act="settings"]').last().click();
    await expect(page.getByRole("button", { name: "自动推进 开", exact: true })).toBeVisible();

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
