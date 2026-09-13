# v1 验收矩阵：启动、Run、终端、改动与自动推进（H–L）

这是 Issue #101 / Agent Taskboard v1 最终验收矩阵的分册，索引见 [`../v1.md`](../v1.md)。

### H. 启动配置表单与 Run 启动（59–77）

- **#59** 「执行」先进启动配置表，点启动才建 Run 并认领：`launch_form.rs` 全组（prepareRunLaunch 返回表单、start 才创建）；`bound_runs.rs::starting_a_bound_run_claims_and_leaves_frontier`。状态：通过。
- **#60** 已有活跃 Run 点卡片直接抬终端：`bound_runs.rs::focusing_an_issue_with_an_active_run_focuses_that_pty`；e2e。状态：通过。
- **#61** 行上「新建」（New）进同一张表、不认领：`runs.rs::new_unbound_run_does_not_claim_and_shows_grok`、`english_copy_uses_new_for_the_plus_button`；e2e 断言 aria-label＝新建/New。状态：通过。
- **#62** 「继续」走继续入口：`bound_runs.rs::continue_links_previous_run_and_resumes_native_session`。状态：通过。
- **#63** Agent 默认顺序 Grok Build → Codex → Claude Code → Antigravity CLI：`launch_form.rs::default_agent_is_first_installed_in_builtin_order`、`antigravity_adapter.rs::builtin_agents_follow_v1_priority_without_gemini`。状态：通过。
- **#64** 按 Project 记上次成功用哪家、可换一家：`launch_form.rs::last_agent_skips_picker_until_switch`。状态：通过。
- **#65** 找不到可执行文件不能启动并说明找过哪些位置：`runs.rs::missing_grok_lists_command_path_and_known_locations`、`launch_form.rs::uninstalled_agent_cannot_start_from_form`、`grok_adapter.rs::grok_probe_missing_keeps_command_path_and_known_location`。状态：通过。
- **#66** 未登录仍能点启动：Adapter 不读登录态（合同），探测只查可执行文件；`launch_form.rs::uninstalled_agent_cannot_start_from_form`（唯一门槛是可执行文件）。状态：通过（合同设计）。
- **#67** 切 Agent 后表单换那家字段、都是具体值：`launch_form.rs::switching_agent_replaces_fields_and_isolation_reason`、`prepare_form_uses_cli_seed_concrete_values`。状态：通过。
- **#68** 预填来源写清楚（本 Project 上次/其它 Project/CLI 种子）且可改：`launch_form.rs::prepare_form_uses_cli_seed_concrete_values`（CliSeed）、`memory_prefers_current_project_then_other_project`（CurrentProject/OtherProject）；表单可编辑（壳）。状态：通过。
- **#69** 隔离是单次显式选择、默认关、不记住：`isolation.rs::isolation_is_off_by_default_when_adapter_can_build_a_tree`、`isolation_stays_off_for_a_bound_issue_and_is_not_remembered`。状态：通过。
- **#70** 绑票初始指令只带票名和稳定地址：`bound_runs.rs::bound_opening_is_title_and_stable_url`。状态：通过。
- **#71** 游离 Run 初始指令默认空、占位「要 Agent 做什么」：`launch_form.rs::prepare_form_uses_cli_seed_concrete_values`（opening_placeholder 断言）。状态：通过。
- **#72** Run 意图生成可见可编辑前缀：`launch_form.rs::intent_only_changes_opening_text`、`english_intent_prefix_uses_client_language`；e2e 断言点意图后前缀可见。状态：通过。
- **#73** 意图每次默认不选、不记忆：`launch_form.rs::isolation_intent_and_instruction_are_not_remembered`（打开新表单 opening 为空）。状态：通过。
- **#74** 改过前缀显示「自定义」、不是第五种意图：e2e 断言（`board.mjs`：点意图后编辑前缀，intent-custom 按钮 active 且文案为 自定义/Custom）；`main.ts` 只局部更新意图按钮，逐字输入时保留文本框与焦点，复跑通过。状态：通过。
- **#75** 成功后清掉待发送开场白/备注，失败保留：`view_changes.rs::notes_go_into_the_next_opening_not_the_current_run_or_tracker`（备注只进下一次且随后清空）、`launch_form.rs::spawn_failure_keeps_form_and_does_not_overwrite_memory`（失败保留）。状态：通过。
- **#76** 必填拦住、未知枚举只警告：`launch_form.rs::required_instruction_keeps_form_and_creates_no_run`、`unknown_enum_warns_but_does_not_block`。状态：通过。
- **#77** 启动失败留 Run 记录、可改再开、不自动重试：`runs.rs::launch_failure_leaves_a_record_and_does_not_retry`、`launch_form.rs::spawn_failure_keeps_form_and_does_not_overwrite_memory`。状态：通过。

### I. Run 观察与生命周期（78–86）

- **#78** 进行中卡片只露 Agent 名加一行最近动作：`runs.rs::recent_action_uses_only_the_adapter_observation`、`recent_action_stays_empty_when_adapter_has_none`。状态：通过。
- **#79** 那行只写 Adapter 能看见的最近一步、不编：`recent_action_stays_empty_when_adapter_has_none`、`recent_action_uses_only_the_adapter_observation`。状态：通过。
- **#80** 同一 Issue 最多一条活跃 Run：`bound_runs.rs::one_active_run_per_issue`。状态：通过。
- **#81** Run 生命周期与 Issue 开关独立：`bound_runs.rs::closing_an_issue_does_not_stop_the_run`；Run 结束不表示票完成（自动推进需票已关，见 L 域）。状态：通过。
- **#82** 全局并行 Run 不设上限：`runs.rs::unbound_runs_can_run_in_parallel`。状态：通过。
- **#83** Adapter 声明原生建树时由官方 CLI 自己建 worktree：`isolation.rs::isolated_run_passes_worktree_and_does_not_add_a_tree`。状态：通过。
- **#84** Codex/Antigravity v1 不能开隔离并看到原因：`isolation.rs::isolation_is_disabled_without_native_capability`、`codex_adapter.rs`/`antigravity_adapter.rs`（不声明 worktree）。状态：通过。
- **#85** 继续沿用上次隔离目录但不传建树开关、树没了回主目录并说明：`isolation.rs::continue_reuses_the_recorded_directory_without_worktree`、`continue_falls_back_to_the_project_directory_when_the_tree_is_gone`。状态：通过。
- **#86** 端口占用、本地锁文件只警告不禁止：`isolation.rs::lock_files_and_sibling_runs_warn_but_do_not_block_launch`（锁文件/并发）；端口占用提示在壳层。状态：通过（锁文件机制）；端口占用提示 部分。

### J. Embedded Terminal（87–91）

- **#87** PTY 在 Host 上、Client 只画和回传按键：`runs.rs::pty_bytes_round_trip_through_host`（内核 PTY 转发）。状态：通过。
- **#88** 不嵌系统终端 App：架构为 Host 内 portable-pty + 壳层 xterm.js 绘制，无终端 App 嵌入。状态：通过（设计）。
- **#89** 等待操作仍算活跃：`waiting.rs::waiting_stays_an_active_run_not_execution_stopped_or_blocked`。状态：通过。
- **#90** 向活跃 Run 注入一行：`waiting.rs::inject_writes_a_line_into_a_waiting_run`、`inject_into_ended_run_fails`。状态：通过。
- **#91** 停止一条 Run：`runs.rs::stopping_a_run_ends_it`。状态：通过。

### K. 查看改动（92–98）

- **#92** 干活中/结束后/票关后都能看改动：`view_changes.rs::view_changes_stays_available_while_running_after_end_and_after_close`。状态：通过。
- **#93** 默认相对启动时 commit 现算、可切未提交：`view_changes.rs::this_round_includes_committed_and_uncommitted_uncommitted_scope_does_not`。状态：通过。
- **#94** 含有限几层独立子仓库、跳过 node_modules 等：`view_changes.rs::missing_nested_repo_does_not_compare_against_the_parent`（子仓库边界）、`src/changes.rs::SKIP_DIRS` + 测试断言路径不含 node_modules。状态：通过。
- **#95** 隔离树没了提示看不了、不拿主目录比：`view_changes.rs::isolated_tree_gone_does_not_fall_back_to_project_directory`。状态：通过。
- **#96** 每行写改动备注、只留看板：`view_changes.rs::notes_go_into_the_next_opening_not_the_current_run_or_tracker`。状态：通过。
- **#97** 下次生成开场白后备注从待送出清掉：同上测试（下一次 opening 已不含备注文本）。状态：通过。
- **#98** 手机不要完整查看改动：壳层手机视图（board/issue/run）无改动视图入口；`main.ts` 手机分支。状态：通过（壳设计）。

### L. 自动推进（99–109）

- **#99** 默认关、Project+Host 双开关：`advance.rs::both_switches_must_be_on_before_pending_and_claim`。状态：通过。
- **#100** 冷启动不推进、恢复默认关、到点等 N 秒：`advance.rs::switches_survive_reboot_but_cold_start_waits_restore`。状态：通过。
- **#101** 正常态进 60 秒待确认、无人否决才领下一张：`advance.rs::pending_confirmation_veto_does_not_claim_next`、`both_switches_must_be_on_before_pending_and_claim`。状态：通过（计时参数在 `src/advance.rs`；测试用可等待的确认窗口）。
- **#102** grilling/prototype/needs-info/ready-for-human/needs-triage 不进自动池：`advance.rs::auto_pool_skips_grilling_prototype_and_triage_roles`。状态：通过。
- **#103** 票没关/异常时同 Agent 自检、不多开复查 Run：`advance.rs::self_check_stops_when_still_open_or_abnormal`、`waiting.rs::normal_stop_and_waiting_do_not_start_self_check`。状态：通过。
- **#104** StopFailure 时先注入同一条 TUI 自检句：`advance.rs::stop_failure_while_running_injects_self_check`。状态：通过。
- **#105** 自检后仍异常就停下：`advance.rs::self_check_stops_when_still_open_or_abnormal`。状态：通过。
- **#106** 票已关但 hook 异常只开查看改动、不 reopen 不领下一张：`advance.rs::closed_with_hook_abnormal_opens_view_changes_and_does_not_reopen`。状态：通过。
- **#107** 看板不代关票：壳层无关票按钮；`board.rs::unknown_move_op_does_not_change_tracker_state`；Agent 在终端里用 `gh` 自己关。状态：通过。
- **#108** 待确认只否决「领不领下一张」：`advance.rs::pending_confirmation_veto_does_not_claim_next`（否决不触发其它动作）。状态：通过。
- **#109** 只读 hook 挂不上则不推进、不改长期 CLI 配置：`advance.rs::missing_hooks_or_user_stop_does_not_advance` + `grok_attach_hooks_stays_inside_sink_and_sets_grok_home`、`codex_attach_hooks_uses_per_run_config_not_home`、`claude_attach_hooks_passes_settings_inside_sink`。状态：通过。
