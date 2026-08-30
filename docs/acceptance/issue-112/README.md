# Issue #112 看板信息层级与 Issue Inspector 验收

验收日期：2026-08-30

## 结论

Issue #112：**PASS**。

| 尺寸 / Client | 结果 | 证据 |
| --- | --- | --- |
| 桌面 Release Tauri | **PASS** | review 修复后的最终源码重新构建 Release `.app`；实际点击 6 张 Issue，并反复打开/收起 Inspector。 |
| 390×844 Browser Client | **PASS** | Playwright 以 390×844 实际打开 Issue、操作主要按钮并检查页面级横向溢出、Inspector 与按钮边界。 |

390×844 按产品定义由 Browser Client 验收；Tauri 主窗口配置的最小宽度是 880px，未伪造为 390px 桌面窗口。浏览器证据不替代桌面壳证据：上表第一行是本轮单独完成的真实 Release Tauri 验收。

## 桌面 Release Tauri：PASS

验收 bundle：

`target/release/bundle/macos/Agent Taskboard.app`

- 2026-08-30 14:59（Asia/Shanghai）在所有 review 修复完成后重新执行 `tauri build --bundles app`；`codesign --verify --deep --strict` 通过。
- 启动该最终 bundle 的独立进程（PID 73248，process identity `1788073150144502`），在真实 Tauri WebView 中依次点击 #116、#115、#114、#113、#112，并在每张 Issue 上打开、收起 Inspector。
- 收起 #116 后，使用固定在顶栏的“显示详情”重新打开同一张 Issue；选择与 Inspector 上下文保持一致。随后再次收起，再继续切换其余四张 Issue。
- 额外点击已弱化显示的“最近完成” #111；卡片仍可辨认并成功打开 Inspector。
- Inspector 以右侧覆盖层展示；打开与关闭不改变四列看板本身的宽度。普通看板不再重复解释列顺序和“最近完成”规则。
- Inspector 首屏保留标题、执行、编辑、关闭和浏览器打开；正文之后使用“父子关系”“依赖关系”，评论与关系编辑默认收起。
- 15:01 与 15:02 从同一最终进程重新截取下方 Inspector / 看板证据；不沿用 review 修复前的图片。

截图：

- [完整看板与弱化的最近完成](issue-112-desktop-board-release-tauri.png)
- [浮动 Issue Inspector](issue-112-desktop-inspector-release-tauri.png)

## 390×844 Browser Client：PASS

- 手机主界面只显示进行中与 Frontier；Issue 从底部“票”页完整打开。
- Inspector、正文和主要操作均位于 390px 视口内，没有页面级强制横向滚动。
- 主要按钮边界均未超出视口；桌面专属的 Inspector 收起按钮和“查看改动”不出现在手机 Issue 页。
- 从看板页打开 Issue、再返回看板后，`.workspace` 页面滚动位置保持不变。
- 视觉基线：[390×844 手机 Issue](../../../apps/desktop/e2e/baselines/issue-99-mobile-390x844.png)。

## 自动化验证

以下回归均通过：

```sh
npm --prefix apps/desktop run build
cargo test --workspace --all-targets
cargo test -p host-kernel --test board browser_renders_incomplete_state_then_recovers_all_board_flows -- --exact
cargo test -p host-kernel --test board browser_covers_local_markdown_issue_111_write_forms -- --exact
npm --prefix apps/desktop run verify:release
```

专项浏览器场景覆盖：

- Inspector 开关前后泳道宽度保持一致；
- 收起/恢复同一 Issue 时保留 Inspector 滚动；
- 收起和切换 Issue 时保留看板泳道滚动；
- 390×844 从 Issue 返回看板时保留页面滚动；
- 最近完成低对比度 + 删除线，但保留可用按钮；
- 默认看板删除重复说明；Inspector 删除重复状态和内部化关系词；
- 390×844 下无页面级横向溢出或主要操作裁切。
