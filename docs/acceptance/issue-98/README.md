# Issue #98 可见验收证据

验收日期：2026-08-26。

## 与 #31 `0eb685c` 的逐项对照

- 右侧为可滚的 Issue 文档栏，标题与主要动作固定在滚动区之外。
- Markdown 正文位于家族与 Dependency 之前；长正文独立滚动。
- 详情宽度使用明确的「加宽详情 / 收窄详情」文字动作。
- 进入已有 Run 后终端约占 2/3，右侧仍显示同一份完整 Issue。
- 390×844 手机「票」页可阅读完整正文，仍不提供完整查看改动。

## 自动化证据

`apps/desktop/e2e/board.mjs` 通过 Host loopback + Playwright 覆盖：

- 长正文、标题、段落、列表、粗体、行内代码与 HTTPS 链接；
- 原始 HTML 保持转义，`javascript:` 链接不会变成可执行动作；
- sticky 标题/动作、正文滚动、家族与 Dependency 顺序；
- 加宽/收窄、进入 Run 后正文保留、390px 手机阅读。

对应截图：

- `issue-98-desktop-detail-1440x900.png`
- `issue-98-existing-run-1440x900.png`
- `issue-98-mobile-390x844.png`

## 真实 GitHub Completion gate

使用 live GitHub Tracker Adapter 和本仓真实 Issue #98 完成：看板选中 #98 → 阅读 Problem / Acceptance criteria / Completion gate → 启动并进入本地 Run → 继续阅读同一正文。Run 随后已停止；Issue 未被编辑或关闭。

对应截图：

- `issue-98-real-github-selected-1440x900.png`
- `issue-98-real-github-run-1440x900.png`
