# Issue #114 Client 状态隔离、刷新策略与响应性能验收

验收日期：2026-08-30

## 结论

Issue #114：**PASS**。

| 必验场景 | 结果 | 证据边界 |
| --- | --- | --- |
| 真实 Browser Client + 真实 Release Tauri 双 Client | **PASS** | 同一 Host 上同时操作两个真实 Client；各自 Project、Issue、Run、视图和 Inspector 不互相抢占。 |
| 可观测慢 Tracker 与恢复 | **PASS** | 最终签名 Release `.app` 的真实 GitHub 读取约 4 秒；期间另一 Browser Client 的 Project 看板 RPC 仍在 307ms 返回，桌面只显示局部刷新状态且输入可继续编辑。 |
| 刷新配置 | **PASS** | 真实 Host 保留已有 60 秒配置；专项 Browser E2E 通过真实设置表单依次保存 15、999999，并恢复原值。 |

真实体验使用：

- 最终源码重新构建并签名校验通过的 `target/release/bundle/macos/Agent Taskboard.app`；
- Host loopback 提供的真实 Browser Client；
- live GitHub Tracker 与本仓实际 Issue，而不是 fixture Tracker。

RPC、fixture、Node bridge 和内核测试只计自动化证据，不替代上述真实双 Client 与慢 Tracker PASS。

## 双 Client 隔离：PASS

两个 Client 同时连接同一 Host：

- Browser Client 保持在 `issue110-local-tauri-fixed` Project、Issue #1 和 `Grok Build Run`；
- Release Tauri 保持在 `agent-taskboard` Project、依赖图、中心 Issue #114 和其 Inspector；
- 在 Browser Client 切换 Project、Issue、Run 与页面后，Tauri 没有被强制切走；
- 在 Tauri 切换看板/依赖图、中心 Issue 与 Inspector 后，Browser Client 的选择和 Run 仍保持原状。

内核回归另外覆盖两个独立 `clientInstanceId` 的 Project、Issue、Run、当前视图、Inspector、筛选、Launch Form、Usage 等 Client 状态。

截图：[刷新前的 Tauri 依赖图与 #114 Inspector](issue-114-tauri-before-refresh.png)。

## 慢 Tracker、局部刷新与恢复：PASS

在最终 Release Tauri 上触发 live GitHub Tracker 刷新：

1. 一次直接请求 `api.github.com` 从约 17:51:27 持续到 17:51:31。运行日志确认请求为直接连接；尝试设置的代理环境没有被当前 `ureq` 构建采用，因此不把代理当作慢场景来源。
2. Tracker 请求进行时，Browser Client 请求另一 Project 看板并在 307ms 返回；慢读取没有持有 Kernel mutex 阻塞其他 Client RPC。
3. Tauri 只在工作区顶部显示“正在刷新”，没有整页 loading、空白或重置。
4. 刷新期间创建 Issue 表单中的未提交标题 `draft survives final release refresh` 与正文 `body remains editable while GitHub Tracker is slow` 仍可编辑。
5. 刷新完成后，看板内容局部恢复；#114 Inspector、中心 Issue 与未提交输入仍保留，没有关闭 Inspector 或跳回页面顶部。

截图：

- [Tracker 读取期间的局部刷新与未提交输入](issue-114-tauri-refreshing.png)
- [刷新完成后看板恢复且输入保留](issue-114-tauri-after-refresh.png)
- [刷新完成后 #114 Inspector 与中心 Issue 保留](issue-114-tauri-ready-preserved.png)

自动化慢读取回归使用受控 Tracker 明确阻塞 refresh，同时要求另一 Client RPC 在 300ms 内返回；generation 检查还防止较旧的 deferred refresh 结果覆盖更新的 Action refresh。

## 刷新配置：PASS

- 真实 Client 启动时显示并保留已有的 60 秒明确配置，没有被新的 300 秒默认覆盖。
- 默认值为 300 秒，仅用于未配置 Client。
- 最低允许值为 15 秒；没有人为最大值。
- 专项 Browser E2E 使用真实设置输入框执行 `fill + blur`，先保存 15，再保存 999999，并从 Host snapshot 验证生效；结束时恢复原值 60。

截图：[Release Tauri 显示保留的 60 秒设置](issue-114-tauri-settings.png)。

## Acceptance matrix

| Acceptance criterion | 结果 | 证据 |
| --- | --- | --- |
| Browser 与 Tauri 的 Project、Issue、Run、视图、选中 Issue 互不抢占 | **PASS** | 真实双 Client 同时操作；`client_state` 与专项 Browser E2E 回归 |
| 看板/依赖图切换不做无必要全量刷新并保留交互状态 | **PASS** | 真实 Tauri 切换；board E2E 的 DOM/滚动/输入保持断言 |
| 慢/暂不可用 Tracker 时 UI 可操作且只显示局部状态 | **PASS** | live GitHub 约 4 秒读取；另一 Browser RPC 307ms；受控慢 Tracker 并发测试 |
| 默认 300 秒、最低 15 秒、无上限、保留已有配置 | **PASS** | HostKernel 配置测试；设置表单 15/999999 E2E；真实 60 秒配置保留 |
| 无变化刷新不重置 Inspector、中心 Issue 或滚动 | **PASS** | 最终 Release 刷新前/中/后截图；board E2E 与 refresh 回归 |
| 真实 Browser + Tauri + 可观测慢 Tracker | **PASS** | 最终 Release `.app`、真实 Browser Client、live GitHub Tracker |

## 自动化与构建验证

以下门禁均通过：

```sh
npm --prefix apps/desktop run build
RELEASE_TAG=v0.1.0 npm --prefix apps/desktop run verify:release
cargo test --workspace --all-targets
cargo test -p host-kernel --test refresh
cargo test -p host-kernel --test host_kernel
cargo test -p host-kernel --test board
cargo test -p host-kernel --test client_state
git diff --check
```

专项 `client-isolation-refresh.mjs` 由 `board` E2E 启动，覆盖双 Client 身份隔离、互不抢占、慢 Tracker 时的可响应性、刷新状态、无变化更新以及配置输入路径。最终 `.app` 另行执行 `codesign --verify --deep --strict`，结果通过。

仓库全量 `cargo fmt --all -- --check` 仍会被本次未修改的 `crates/host-kernel/tests/usage.rs` 既有格式差异阻塞；本次修改的 Rust 文件已单独通过 `rustfmt --check`。
