# Issue #110 Project / Tracker 生命周期验收

验收日期：2026-08-29。

## 结论

自动化结果：**PASS**。Project / Tracker 公共 RPC、Client focus 回退、Local Markdown、单一合法 Git remote、多 remote 候选和 self-hosted GitLab nested namespace 均有回归证据。

真实 Tauri 桌面壳结果：**PASS**。使用源码构建的唯一 bundle 完成了 Local Markdown 与 self-hosted Git remote 的登记、编辑、活跃 Run 移除防护、停止 Run、移除登记和当前 Project 回退；未删除本地目录。虚构 GitLab host 的真实远端同步因没有凭据保持 BLOCKED，不影响本地登记与生命周期验收。

Codex Desktop 内置 Browser 结果：**PASS（生命周期）/ PARTIAL（桌面专属能力）**。Local Markdown、self-hosted Git remote、编辑、活跃 Run 移除防护、停止 Run、移除当前 Project 后回退均已通过内置 Browser；目录选择器不可由 Browser Client 操作，self-hosted 虚构 host 的真实同步因没有凭据保持 BLOCKED，自动推断由共享 Client E2E 与 HostKernel 测试覆盖。

## 验收矩阵

| Acceptance criterion | 状态 | 证据 |
| --- | --- | --- |
| 在真实 Tauri App 添加、编辑、移除 Project，并给出明确确认反馈 | PASS | 源码 bundle 实际登记 `issue110-local-tauri-fixed` 与 `issue110-gitlab-tauri`，编辑 self-hosted Project 后显示名称更新；“只移除登记”确认后从侧栏消失并回退到 Local Markdown |
| 活跃 Run 时安全拒绝移除 | PASS | 真实 Tauri 与内置 Browser 均显示“这个 Project 有活跃 Run。先停止或结束 Run，再回来移除。”；`projects.rs::an_active_run_blocks_remove`；`project-management.mjs` |
| 移除当前 Project 后回退，其他 Client 的 Project / Issue focus 不被抢占 | PASS | `board.rs::removing_a_middle_project_uses_the_host_fallback_for_the_removing_client`；`board.rs::removing_a_project_keeps_another_clients_project_and_issue_focus` |
| Local Markdown 不依赖 GitHub Host 或 credential | PASS | 真实 Tauri / 内置 Browser 均读取 `/tmp/l110/.scratch/feature/issues/01-local.md`，显示 `#1 Local desktop issue`；`project-tracker-lifecycle.mjs` + `projects.rs` |
| 单一合法 remote 自动填充，多 remote 才候选，手填值不覆盖 | PASS | `project-registration.mjs`；`projects.rs` inference tests |
| HTTPS、SSH、自建 GitLab / 其他 Git remote 形式可保留完整仓库身份 | PASS | `projects.rs::inference_accepts_a_non_github_git_remote`；`projects.rs::inference_preserves_nested_self_hosted_git_namespaces` |
| 真实桌面壳实际登记 Local Markdown、自建 Git remote，编辑、移除、活跃 Run 防护、回退 | PASS | 源码 Tauri bundle 已实际完成全部生命周期场景；self-hosted 虚构 host 的真实远端同步单独为 BLOCKED，未填写 credential |
| Codex Desktop 内置 Browser 实际登记 Local Markdown、自建 Git remote，编辑、移除、活跃 Run 防护、回退 | PASS（生命周期）/ PARTIAL（桌面专属能力） | 内置 Browser 完成登记、完整 namespace、编辑、active Run 防护、停止和当前 Project 回退；目录选择器与真实远端同步按边界记录为 BLOCKED |
| 每个场景记录 PASS / FAIL / BLOCKED 与实际操作结果 | PASS | 本文件逐项记录真实 Tauri、内置 Browser、自动化和外部远端同步边界；未覆盖项不计 PASS |

## 自动化证据

- `apps/desktop/e2e/project-tracker-lifecycle.mjs` 从产品 Project 登记入口开始，覆盖 Local Markdown 登记、self-hosted GitLab remote 自动推断、nested namespace、编辑、移除当前 Project、邻近 Project 回退和旧 Issue 不残留。
- `crates/host-kernel/tests/projects.rs` 覆盖 tracker 类型、持久化、重复目录、活跃 Run、Local Markdown 和 remote inference。
- `crates/host-kernel/tests/board.rs` 覆盖带 `clientView` 的删除回退和其他 Client 的 Issue focus。
- 自动化使用真实生产 `dist`、真实 Client 代码、真实 `HostKernel`、真实 loopback RPC 和临时文件系统；Tracker 读写使用 fixture，不作为真实 GitHub 写操作证据。

## 真实 / fixture 边界

- Local Markdown 场景读取真实临时目录中的 Markdown Issue 文件，不需要 GitHub credential。
- self-hosted Git remote 场景使用真实 `.git/config` 形式验证登记字段；自动化 Tracker Issue 使用 `MemoryTracker` fixture。
- 自动化不能替代真实 Tauri WebView、系统目录选择器和真实 Agent CLI；这些结果单独记录为 PASS / FAIL / BLOCKED。
- 不填写 credential，不调用真实 GitHub 写接口，不删除用户已有 Project 目录；Project 移除只取消登记，不删除本地目录。

## 真实 Tauri 桌面验收日志

| 场景 | 状态 | 实际操作结果 |
| --- | --- | --- |
| Local Markdown 登记并看到 Issue | PASS | 源码 bundle 登记 `issue110-local-tauri-fixed`，侧栏显示 `Local Markdown`，看板显示 `#1 Local desktop issue` |
| self-hosted Git remote 完整 namespace | PASS（登记） / BLOCKED（真实同步） | 源码 bundle 登记 `issue110-gitlab-tauri`，页面显示 `gitlab.example.com/acme/platform/garden`；虚构 host 无凭据，真实 Tracker 同步未执行 |
| 编辑 Project | PASS | 将 `issue110-gitlab-tauri` 改为 `issue110-gitlab-tauri-edited`，本地目录和 `gitlab.example.com/acme/platform/garden` 保持不变 |
| 移除当前 Project、回退、旧 Issue 清除 | PASS | 点击“只移除登记”后 self-hosted Project 从侧栏消失，当前 Project 回退到 `issue110-local-tauri-fixed`，Local Markdown Issue 仍可见 |
| 活跃 Run 阻止移除 | PASS | 真实 Tauri 启动临时 `Grok Build` Run；移除入口明确拒绝；停止后状态为“执行已停” |
| 原生菜单跨刷新周期保持展开 | PASS | `Agent Taskboard` 与“编辑”菜单均打开后等待 2.5 秒仍保持展开 |

真实 Tauri 边界：debug bundle 的 10529 loopback 页面通过本地 Vite `127.0.0.1:1420` 代理提供；Tauri WebView 本身和原生菜单均为真实 bundle。self-hosted host 为临时 fixture，未填写 credential、未执行 GitHub 写操作，也没有删除任何本地目录。

## Codex Desktop 内置 Browser 验收日志

| 场景 | 状态 | 实际操作结果 |
| --- | --- | --- |
| Local Markdown 登记并看到 Issue | PASS | 在 `http://127.0.0.1:10529/` 使用内置 Browser 登记并读取临时 Markdown Issue；侧栏显示 `Local Markdown`，看板显示 `#1 Local desktop issue` |
| self-hosted Git remote 完整 namespace 登记 | PASS（登记） / BLOCKED（真实同步） | 内置 Browser 登记 `issue110-gitlab-browser-continued` 后页面显示 `gitlab.example.com/acme/platform/garden`；虚构 host 无凭据，未填写 credential |
| 编辑 Project | PASS | 将 self-hosted Project 名称改为 `issue110-gitlab-browser-continued-edited`，remote identity 保持不变 |
| 活跃 Run 阻止移除 | PASS | 从 Local Markdown Project 启动未绑定 Issue 的临时 `Grok Build` Run；移除入口显示“这个 Project 有活跃 Run。先停止或结束 Run”，随后停止临时 Run，状态为“执行已停” |
| 移除当前 Project、回退 | PASS | 在确认后点击“只移除登记”移除当前 self-hosted Project；它从侧栏消失，当前 Project 回退到 `issue110-local-tauri-fixed`，Local Markdown Issue 仍可见 |
| 目录选择与自动推断 | PARTIAL | Browser Client 的“选择目录”明确提示仅桌面窗口可用；本轮使用手动路径和 host/repository 字段完成登记，自动推断由 `project-tracker-lifecycle.mjs` 和 HostKernel 测试覆盖 |

Browser 证据边界：使用 Codex Desktop 内置 Browser 连接本机 loopback Client，所有 UI 操作通过 Browser 的可见 DOM / Playwright / CUA 完成。未调用真实 GitHub 写接口、未填写 credential；临时 fixture 目录和 Markdown Issue 文件已确认保留。为保持测试环境可复用，`issue110-local-tauri-fixed` 及其已停止 Run 仍保留在本机 Host 登记中。

## 验证命令

```sh
npm --prefix apps/desktop run build
cargo fmt --check -- crates/host-kernel/src/lib.rs crates/host-kernel/src/project.rs crates/host-kernel/tests/projects.rs crates/host-kernel/tests/board.rs
cargo test -p host-kernel --test projects -- --nocapture
cargo test -p host-kernel --test board -- --nocapture
cargo test --workspace --all-targets
npm --prefix apps/desktop run verify:release
```
