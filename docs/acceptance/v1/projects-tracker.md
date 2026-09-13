# v1 验收矩阵：Project 与 Tracker（C–D）

这是 Issue #101 / Agent Taskboard v1 最终验收矩阵的分册，索引见 [`../v1.md`](../v1.md)。

### C. Project 登记、编辑与移除（18–28）

- **#18** 空 Host 上能登记第一个 Project：`host_kernel.rs::empty_host_offers_register_and_pair_and_focuses_one_host`、`projects.rs::registering_a_github_project_lists_it_and_makes_it_current`。状态：通过。
- **#19** PROJECT 标题旁加号登记：壳层（`main.ts` 注册入口）+ e2e `apps/desktop/e2e/board.mjs`（register 流程）。状态：通过（壳 e2e）。
- **#20** 行尾菜单编辑/移除：`browser_manages_projects_from_the_desktop_sidebar_without_losing_context` 覆盖桌面行尾菜单的编辑、活跃 Run 拦截、执行已停确认、移除与相邻回退；#100 原生验收又复跑同路径。状态：通过。
- **#21** 配对入口在当前 Host 行旁：`empty_host_offers_register_and_pair_and_focuses_one_host`（内核配对入口）+ 壳层 Host 行按钮。状态：通过。
- **#22** 有活跃 Run 的 Project 不能移除：`projects.rs::an_active_run_blocks_remove`。状态：通过。
- **#23** 执行已停仍可移除，但确认 Tracker 认领不自动释放：`bound_runs.rs::remove_project_with_execution_stopped_keeps_tracker_claim`。状态：通过。
- **#24** 从未成功拉过 Tracker 仍可移除：`projects.rs::a_project_that_never_synced_the_tracker_can_still_be_removed`。状态：通过。
- **#25** 移除只取消登记，不删目录/git/远端：`projects.rs::remove_only_unregisters_and_falls_back_to_the_neighbor`、`removing_a_project_preserves_ended_run_history`。状态：通过。
- **#26** 移除后回退相邻 Project，无则空 Host：`projects.rs::remove_only_unregisters_and_falls_back_to_the_neighbor`、`removing_the_last_project_returns_to_an_empty_host`。状态：通过。
- **#27** 推断只是候选，必须确认：`projects.rs::inference_is_only_a_candidate_until_register`；e2e 中浏览器端只给手动路径输入。状态：通过。
- **#28** 登记时手动给目录、Tracker 类型、连接信息：`projects.rs::registering_a_github_project_lists_it_and_makes_it_current`、`project_persists_tracker_kind_and_old_settings_default_to_github`；e2e 手动表单。状态：通过。

### D. Tracker 与凭据（29–33）

- **#29** v1 完整打通 GitHub：`github_adapter.rs` 全组 22 测试（原生 blocked_by / sub-issues / assignee、分页到 500+、创建/编辑/开关/评论、写子树与依赖边、限流 Retry-After、401/403、离线详情、自托管 host）。状态：通过。
- **#30** GitLab 与本地 markdown 不要求同版完整：GitLab 保留 `TrackerSeam` 合同口；Local Markdown 已通过 `projects.rs::local_markdown::*` 及 `browser_covers_local_markdown_issue_111_write_forms` 实现读取、写入、关系、状态、监听刷新和失败关闭。状态：通过。
- **#31** 凭据顺序「应用专用环境变量 → 秘密文件显式 PAT → 本机 CLI → 通用环境变量」：`projects.rs::credentials_prefer_app_env_then_secrets_then_cli_then_generic_env`。状态：通过。
- **#32** Tracker 鉴权与 Agent 登录态分开：Agent Adapter 合同不碰登录态（`grok_adapter.rs`/`codex_adapter.rs`/`claude_adapter.rs`/`antigravity_adapter.rs` 只探测可执行文件与声明字段）；看板读取不依赖任何 Agent 登录。状态：通过（合同设计，无专门登录测试——因产品本就不读登录态）。
- **#33** 单 Project 凭据失败只降级该 Project：`projects.rs::one_project_auth_failure_does_not_degrade_the_others`、`an_unreachable_host_is_not_reported_as_auth_failure`。状态：通过。
