# Issue #99 主壳恢复验收

验收日期：2026-08-27。

## 原型基线 / 产品结果 / 有意偏离

| 原型基线 `prototype/codex-shell-refresh` | 产品结果 | 有意偏离 |
| --- | --- | --- |
| 左侧连续展示 Host、Project、进行中与执行已停 Run | 产品壳使用固定宽度的原生层级栏；自动化 fixture 含 2 个 Host、2 个 Project、进行中与执行已停 Run | 保留真实 Host 配对、Project 登记、独立菜单和状态修复入口，不复制原型假数据 |
| 中间顶栏固定放看板 / 依赖图与折叠入口 | 看板 / 依赖图进入 38px chrome；左右区域折叠后从 grid 消失，恢复入口仍在 chrome；Issue header 也提供 icon-only 收起入口 | 保留产品已有的刷新、搜索和键盘入口，压成工作区工具条，不放进原型控制条 |
| 四列是可扫读的主工作面，右侧是完整 Issue | 1440×900 与 1280×840 均为四列并排；列使用无边框 surface；默认详情宽度至少 340px | 暖纸仍按 ADR 0011 作为系统浅色首次默认；素纸与素纸夜间只改颜色，不改 DOM 和几何 |
| 点已有 Run 的 Issue 进入约 2:1 的 Embedded Terminal / Issue 工作面 | 产品壳自动收起左侧，Embedded Terminal 与 Issue 直接共边，返回看板恢复原布局 | Embedded Terminal 使用真实 xterm / PTY，不复制原型中的假 TUI 文案与终端控制 |
| 依赖图替换看板；Host 总览按状态分组 | Dependency graph 以一张中心 Issue 为根，默认显示一跳上下游；可查看包含 closed Issue 的完整 Dependency 闭包，但不加入无关 Project 历史；总览先显示各 Project 的 Issue 数量与分布，再显示可过滤的 Run 分组 | 单击节点只换详情；“从此处展开”同时选中该 Issue 并重新构图，保留当前一跳/完整范围与搜索上下文；大闭包的数据完整，但画布和关系列表渐进挂载，避免同时铺满全部节点；尚无 Run 时仍保留 Project 态势 |
| 手机聚焦当前 Project，底栏为看板 / 票 / Run | 390×844 仍只显示进行中与 Frontier；Host / Project 在切换面板；完整 Issue 独立滚动 | 手机默认显示只读最近输出，完整官方 TUI 仍是逃生入口，不升级为手机主路径 |

## 可复现场景

开发 Host 运行时，可直接在浏览器打开 `http://127.0.0.1:10529/`，或使用当前由 `npm run tauri dev` 启动的桌面窗口。不要打开 `/Applications/Agent Taskboard.app`，那是已安装版本，不包含本分支尚未合并的改动。

主壳场景统一由真实 `HostKernel` loopback fixture 驱动：

```sh
npm --prefix apps/desktop run build
cargo test -p host-kernel browser_renders_incomplete_state_then_recovers_all_board_flows -- --nocapture
```

该场景覆盖：多 Host、多 Project、四列、最近完成、普通 Issue、已有 Run、执行已停、Dependency graph、Host 总览、无 Run 时的 Project 态势、Project 过滤、左右折叠、Run 返回看板、三种主题同结构、1280×840、1440×900 与 390×844。它还会把一次 Host tick 精确插入顶栏按钮的按下/松开之间，确保“看板 / 依赖图”单击不会被全量 render 吞掉。

Dependency graph 的回归场景以 #3 为中心：默认只画一跳上游与下游；单击节点只切换详情，点击明确命名的「从此处展开」会同步选中该 Issue 并重新构图。完整模式重新展开时仍保持完整上下游与搜索上下文，不会突然退回一跳。fixture 构造 61 节点闭包，完整数据包含 closed 链路、排除无关 Issue 且可搜索；画布首批只挂载 48 节点、关系列表首批只挂载 50 行，可继续渐进显示。

同一场景还会制造横向 overflow，再手动触发一次 Host tick，断言原 graph canvas、SVG Dependency path 与横向视野均未被替换或重置。graph 数据和中心未变化时，后续每秒 tick 复用既有 DOM；仅切换右侧详情时也不重建 graph canvas。

边界状态也由真实产品壳 + HostKernel loopback fixture 渲染并截图：

```sh
cargo test -p host-kernel browser_renders_shell_edge_state_fixtures -- --nocapture
```

该 browser fixture 分别恢复空 Host、单 Project、Frontier 全认领、离线保留上次数据、限流与鉴权失败；对应 kernel 单元测试继续验证状态语义和写入边界。

## 视觉回归基线

`apps/desktop/e2e/visual-regression.mjs` 使用 `pixelmatch` 比较产品截图；差异超过 0.2% 时写出 `target/visual-diffs/*.actual.png` 与 `*.diff.png` 并失败。DOM 几何断言另行检查主要区域存在、关键区域不相交、四列顺序与互不遮挡、最小列宽、详情宽度、Run 约 2:1、折叠退场和页面级横纵向溢出。

- [1280×840 日常看板](../../../apps/desktop/e2e/baselines/issue-99-desktop-1280x840.png)
- [1440×900 普通 Issue](../../../apps/desktop/e2e/baselines/issue-99-desktop-1440x900.png)
- [1440×900 Dependency graph](../../../apps/desktop/e2e/baselines/issue-99-graph-1440x900.png)
- [1440×900 Host 总览](../../../apps/desktop/e2e/baselines/issue-99-overview-1440x900.png)
- [1440×900 已有 Run](../../../apps/desktop/e2e/baselines/issue-99-run-1440x900.png)
- [390×844 手机 Issue](../../../apps/desktop/e2e/baselines/issue-99-mobile-390x844.png)
- [1280×840 空 Host](../../../apps/desktop/e2e/baselines/issue-99-edge-empty-host-1280x840.png)
- [1280×840 单 Project](../../../apps/desktop/e2e/baselines/issue-99-edge-single-project-1280x840.png)
- [1280×840 Frontier 空](../../../apps/desktop/e2e/baselines/issue-99-edge-frontier-empty-1280x840.png)
- [1280×840 offline](../../../apps/desktop/e2e/baselines/issue-99-edge-offline-1280x840.png)
- [1280×840 rate-limit](../../../apps/desktop/e2e/baselines/issue-99-edge-rate-limited-1280x840.png)
- [1280×840 auth-failure](../../../apps/desktop/e2e/baselines/issue-99-edge-auth-failed-1280x840.png)

更新基线仅用于有意视觉变更：

```sh
UPDATE_VISUAL_BASELINES=1 cargo test -p host-kernel browser_renders_incomplete_state_then_recovers_all_board_flows -- --nocapture
```

## Completion gate

产品壳与自动化证据已完成；Issue 仍需提出者查看上述产品结果并确认默认主壳不再与 `0eb685c` 相差甚远。未确认前不合并关闭 #99。
