# Issue #116 验收记录

Issue #116：**PASS**。

证据边界：浏览器 E2E 验证 Client 隔离、重启持久化、面板交互、状态/滚动保留；桌面体验以下列真实 Release Tauri 壳结果为准。

| 场景 | 结果 | 证据 |
| --- | --- | --- |
| 桌面宽屏 Inspector 拖动、缩放、浮窗/停靠 | **PASS** | 2026-08-30 新构建 Release DMG；实际将 Inspector 从右侧拖动到中部，缩放反馈从 `400 × 600` 到 `466 × 600`，并切换“停靠/浮窗”。 |
| 桌面宽屏 Terminal 拖动、缩放、浮窗化 | **PASS** | 同一 Release Tauri；打开已有 Run，实际拖动 Terminal 从底栏转为浮窗，再从 `760 × 280` 缩放到 `845 × 324`。截图：[拖动后的浮窗](issue-116-tauri-terminal-drag.png)、[缩放后](issue-116-tauri-terminal-resized.png)。 |
| 桌面宽屏 Usage 拖动、缩放、浮窗化 | **PASS** | 同一 Release Tauri；Usage 实际从停靠页拖为浮窗并从 `920 × 680` 缩放到 `838 × 620`。 |
| 空间不足时覆盖与返回 | **PASS** | 真实 Tauri 浮窗覆盖主内容仍可见，并实际点击“返回看板”；Terminal/Inspector 也保持可达。 |
| 小屏/窄桌面无强制横向滚动 | **PASS** | 880px Tauri 窗口覆盖路径；专项 Browser E2E 以 760px 检查页面横向溢出为 0，390×844 移动路径由既有 board E2E 覆盖。 |
| 按 Client 保存且浏览器/Tauri 不互相覆盖 | **PASS** | `panel-layout.mjs` 覆盖两个 Browser Client、Tauri Client；布局 key 含 Client 身份，浏览器调整后 Tauri 仍保持独立布局。 |
| 重启恢复最近布局 | **PASS** | 关闭并重新打开同一 Release Tauri；Usage 恢复为 `838 × 620` 浮窗，Inspector 恢复为 `488 × 623` 停靠布局；Terminal 浮窗位置/尺寸也恢复。截图：[重启后 Run](issue-116-tauri-terminal-restored.png)。 |
| 切换/收起不丢 Issue、输入、Run、滚动 | **PASS** | `panel-layout.mjs` 断言 Terminal 输入草稿、Run ID、Issue 评论草稿和 Frontier 滚动位置均保持。 |

自动化验证：`npm --prefix apps/desktop run build`、`cargo check --workspace --all-targets`、`cargo test -p host-kernel --test board`（27/27）；专项 E2E 另覆盖 760px Run 单面板、Usage 纵向滚动、同类型 Browser Client 隔离和过期历史布局实例清理。布局 registry 以活动 Client 心跳保留当前实例，启动时清除超过 7 天未活跃的旧实例，避免每次新开 Client 都永久增加 `localStorage` 条目。

## 真实 Release Tauri 复核

- 验收包：`target/release/bundle/macos/Agent Taskboard.app`。
- 构建命令：`npm --prefix apps/desktop run tauri build -- --bundles app --config '{"bundle":{"createUpdaterArtifacts":false}}'`（exit 0）。
- 完整性：`codesign --verify --deep --strict 'target/release/bundle/macos/Agent Taskboard.app'`（通过）。
- 从该 bundle 启动独立 Tauri 进程，在真实 WebView 中完成 Terminal 拖动、缩放、浮窗化与关闭/重启恢复；上方截图为该进程的窗口采集。Browser E2E 的 `window.open` 场景还验证 opener 克隆的 `sessionStorage` 会分配新的 Client ID。
