# v1 验收矩阵：刷新、用量、Client 与发布（M–T）

这是 Issue #101 / Agent Taskboard v1 最终验收矩阵的分册，索引见 [`../v1.md`](../v1.md)。

### M. 刷新、上次数据、离线与写边界（110–124）

- **#110** 打开/回前台/手动刷新立即拉；切 Project 时先显示上次数据，最近一次刷新尝试仍在刷新周期内则不重复读取，超过周期或从未取得 Tracker 读取结果才后台刷新；限流等到 `retry_at`，鉴权失败等待凭据修复或手动动作：`refresh.rs::opening_foreground_and_manual_refresh_pull_while_fresh_focus_reuses_cache`、`polling::project_focus_uses_fresh_cached_board_without_full_refresh`、`polling::project_focus_does_not_retry_a_recent_failure_when_cached_board_exists`、`polling::project_focus_returns_cached_board_before_one_background_refresh_finishes`、`polling::project_focus_without_cached_data_returns_loading_before_the_tracker_finishes`、`refresh.rs::rate_limit_pauses_auto_refresh_and_is_not_offline`、`refresh.rs::auth_failure_is_project_degraded_not_offline`。状态：通过。
- **#111** 可见 Client 时默认每 60 秒刷新并见倒计时：`refresh.rs::visible_project_polls_every_minute_and_hidden_does_not`；倒计时在壳层刷新状态栏（e2e 断言状态栏存在）。状态：通过。
- **#112** 无人看时不轮询：同上（hidden 不轮询）。状态：通过。
- **#113** 绑该 Project 的 Run 结束立即刷新：`refresh.rs::run_end_refreshes_even_when_nobody_is_watching`。状态：通过。
- **#114** 认领/判断关票/自动推进只刷新涉及 Project：`refresh.rs::claim_close_check_and_auto_advance_only_refresh_the_involved_project`。状态：通过。
- **#115** 上次数据只用于先画板和离线查看、标明截至时间：`refresh.rs::successful_refresh_persists_last_data_and_keeps_it_when_offline`、`incomplete_read_persists_snapshot_and_board_across_reboot`；壳层标注数据截至。状态：通过。
- **#116** 判断已关/认领/推进前必须先成功读取：`refresh.rs::last_data_is_not_used_to_claim_or_advance_when_read_fails`、`check_issue_closed_uses_live_read_not_last_data`、`advance.rs::read_failure_stops_before_claim_or_advance`。状态：通过。
- **#117** 离线且有上次数据继续展示、标明离线、可手动再试：`successful_refresh_persists_last_data_and_keeps_it_when_offline`（保持展示）+ 壳层离线状态。状态：通过。
- **#118** 从未成功不画假四列：`refresh.rs::never_fetched_project_does_not_draw_four_columns`、`board.rs::incomplete_read_does_not_compute_frontier_or_dependency_graph`。状态：通过。
- **#119** 限流只暂停该 Project、按 Retry-After 恢复：`refresh.rs::rate_limit_pauses_auto_refresh_and_is_not_offline`、`rate_limit_without_retry_after_stays_paused_until_manual_success`、`github_adapter.rs::github_adapter_maps_rate_limit_with_retry_after`。状态：通过。
- **#120** 鉴权失败单独写清并指向修复入口：`refresh.rs::auth_failure_is_project_degraded_not_offline`、`github_adapter.rs::github_adapter_graphql_auth_error_maps_to_auth_with_detail`。状态：通过。
- **#121** 离线写边界：仍可停 Run、注入、启动游离 Run；不能认领/放领、自动领下一张或启动需要先认领的绑票 Run：`refresh.rs::last_data_is_not_used_to_claim_or_advance_when_read_fails`、`bound_runs.rs::offline_form_start_does_not_claim_or_spawn`。状态：通过。
- **#122** 写操作不排队、不先改上次数据：内核写操作直接走 seam、失败即报（`tracker_seam.rs::failed_writes_are_denied_and_keep_tracker_details`、`writes_blocked_by_read_errors_keep_consistent_reasons`）；前端 rpc 队列只保序不重放。状态：通过。
- **#123** 看板只写认领和放领：壳层无评论/改正文/关票入口；渲染/拖拽不改 Tracker（`unknown_move_op_does_not_change_tracker_state`）；内核 seam 保留合同写能力（`tracker_seam.rs::host_commands_cover_create_update_open_comment_parent_and_dependency`，未在壳暴露）。状态：通过（壳不暴露）。
- **#124** 刷新状态栏同一位置：壳层 `.refresh-bar`（e2e 断言状态栏与数据截至）。状态：通过（壳 e2e）。

### N. 用量与遥测（125–132）

- **#125** 从左侧 Host 区进独立用量页、按 Host 全部 Project：`usage.rs::usage_opens_from_host_as_independent_page_defaulting_to_today`。状态：通过。
- **#126** 24h/今天（默认）/7 天/30 天/自定义筛：`usage.rs::usage_filters_and_time_windows_do_not_mix_old_samples`。状态：通过。
- **#127** 按 Project/Agent/模型筛 + 六字段、缺字段是 `—`：`usage.rs::missing_token_fields_stay_absent_not_zero`、`bucket_total_is_missing_when_any_run_row_is_missing_a_field`。状态：通过。
- **#128** 分时段趋势看首字/速率、偏慢标红、声明不管理节点：`usage.rs::spike_uses_the_model_recent_median_not_a_fixed_threshold`；`board-desktop-run.mjs` 断言两条趋势与观察用途声明，壳层 `.slow` 呈现偏慢。状态：通过。
- **#129** 终端顶栏分轨胶囊、点击展开：`usage.rs::run_telemetry_keeps_models_on_separate_lanes`；`board-desktop-run.mjs` 实际点击分轨胶囊并断言各模型卡片、首字/速率和观察声明。状态：通过。
- **#130** 各模型独立六字段、缺失 `—`、绝不跨模型加总：`missing_token_fields_stay_absent_not_zero`、`run_telemetry_keeps_models_on_separate_lanes`。状态：通过。
- **#131** 用该模型最近 10 次中位数判突刺：`usage.rs::spike_uses_the_model_recent_median_not_a_fixed_threshold`、`each_model_keeps_only_the_last_ten_calls`。状态：通过。
- **#132** 观察条与用量页互跳并高亮：`usage.rs::usage_and_run_observation_jump_both_ways`。状态：通过。

### O. 列表动作、键盘、通知与快速入口（133–138）

- **#133** Frontier 可认领并启动/进行中可聚焦停止查看改动/最近完成可查看改动：e2e（`board.mjs`：frontier execute-run、recently completed view-changes、focus-run）+ 内核动作命令。状态：通过。
- **#134** 键盘流 + `?` 帮助、不抢 TUI 按键：e2e（`?` 帮助、j/Enter 聚焦）；终端为独立 xterm 区域不受应用键控。状态：通过。
- **#135** 按标题搜索（回车才查）、可叠 triage/open/closed：`board.rs::title_search_stacks_with_triage_and_open_closed_filters`；e2e。状态：通过。
- **#136** 四类通知（等待/正常完成/异常停止/崩溃捡回）、点击跳转、桌面与声音分开关：`waiting.rs::four_notification_kinds_include_jump_targets`、`jump` 目标、`notification_switches_persist_on_this_client`、`hidden_window_still_notifies_the_connected_client`（内核事件+壳）；v0.1.0/0.1.1 原生安装包运行时已观察到系统通知横幅，设置中“桌面通知”和“通知声音”可独立持久化。状态：通过。
- **#137** 列表上看得出运行中/等待操作/执行已停三态：`waiting.rs::in_progress_list_has_running_waiting_and_execution_stopped`。状态：通过。
- **#138** 详情和最近完成能浏览器打开该 Issue：e2e（点开 GitHub Issue URL 断言）。状态：通过。

### P. 界面语言与主题（139–143）

- **#139** 简体中文/英语、中文为源文案、不译 Issue/TUI/报错：`host_kernel.rs::language_and_theme_catalogs_have_no_follow_system`（双语目录）+ 壳层文案（e2e 断言中英文文案）。状态：通过。
- **#140** 语言记在 Client、具体值、无跟随系统：`host_kernel.rs::language_and_theme_catalogs_have_no_follow_system`、`first_launch_matches_system_then_writes_concrete_values`。状态：通过。
- **#141** 本机窗口与托盘共用一份语言：`host_kernel.rs::window_and_tray_share_the_client_language_and_theme`。状态：通过。
- **#142** 主题清单暖纸（仅白天）/素纸/素纸夜间、无暖纸夜间：`host_kernel.rs::language_and_theme_catalogs_have_no_follow_system`（themes 断言三套）、`chinese_locale_picks_simplified_chinese_and_light_picks_warm_paper`。状态：通过。
- **#143** 主题记在 Client、设置完整清单、顶栏浅/深快切：`window_and_tray_share_the_client_language_and_theme` + `board-desktop-shell.mjs` 逐项切换三套主题，`board-mobile.mjs` 验证 Client 级持久与 Host 状态隔离。状态：通过。

### Q. 发布与更新（144–146）

- **#144** GitHub Releases 安装：macOS 两份 dmg（AS/Intel、ad-hoc）+ Windows 一份 NSIS：v0.1.1 Release 矩阵成功产出并发布 `Agent.Taskboard_0.1.1_darwin_aarch64.dmg`、`Agent.Taskboard_0.1.1_darwin_x64.dmg`、`Agent.Taskboard_0.1.1_windows_x64-setup.exe`，并上传对应 updater tar/sig 与 `latest.json`；macOS AS/Intel 与 Windows 安装启动门均通过。状态：通过。
- **#145** 更新先告知、确认后下载安装、活跃 Run 不让装、装完按原模式拉起：`runs.rs::update_install_requires_every_run_to_end`（运行门）+ 壳层 `updateDialog` 确认流 + 原生 v0.1.0→v0.1.1 下载、替换并重拉（更新后继续监听 10529）。状态：通过。
- **#146** 更新失败或跨大版本不动 Host 数据和 Client 设置：updater 只替换应用包、数据在 `appLocalDataDir`（`host_data_and_desktop_client_settings_are_two_trees`）；原生更新后 `host/secrets.json`、`desktop-client/settings.json`、`desktop-client/secrets.json` 内容与更新前保持一致，Host 仍在 10529 提供服务。状态：通过。

### R. 手机 Client（147–150）

- **#147** 突出当前 Project/刷新状态/进行中/Frontier、清单收进切换面板、底栏「看板 | 票 | Run」：e2e（mobile-nav 顺序断言、范围移出主布局、刷新状态先于看板、进行中先于 Frontier）。状态：通过。
- **#148** 看态势、开停 Run、只读最近输出、注入一行、活终端仅逃生口：e2e（mobile stop-run、read-only recent output、活终端非默认）+ 壳层 inject-row。状态：通过。
- **#149** 手机用量只留当前合计和按 Project 几行、遥测只留主模型胶囊：`board-mobile.mjs` 在 390×844 断言主模型胶囊、精简多模型清单、当前合计、1–3 行 Project，并确认完整筛选/趋势/明细隐藏。状态：通过。
- **#150** 在切换范围里登记/编辑/移除 Project：e2e（mobile-scope-sheet 暴露 register/edit-project/remove-project 且可交互）。状态：通过。

### S. 内置 Agent 与启动环境（151–154）

- **#151** Antigravity 第四家、只探测启动 `agy`、不支持不提示 Gemini：`antigravity_adapter.rs::antigravity_adapter_only_uses_agy`、`builtin_agents_follow_v1_priority_without_gemini`。状态：通过。
- **#152** Antigravity 首层 model/effort/执行模式/跳过权限确认/sandbox/初始指令、子 Agent 与额外目录折叠：`antigravity_adapter.rs::antigravity_adapter_declares_execution_mode_not_permission_axis`、`antigravity_adapter_assembles_mode_not_permission_flag`。状态：通过。
- **#153** 在目标目录用日常默认壳拍启动环境、绝对路径 exec：`runs.rs::probe_and_start_share_one_launch_env_and_exec_absolute_path`、`launch_env.rs::shell_capture_reads_vars_after_shell_noise`、`grok_probe_finds_binary_on_path_when_known_location_is_empty`。状态：通过。
- **#154** 环境快照只内存不落盘、设置里能重读：`launch_env.rs::manual_refresh_replaces_the_cached_snapshot_and_failure_keeps_the_last_good_one`、`runs.rs::manual_environment_refresh_updates_later_agent_probes_and_run_spawns`；无快照持久化代码。状态：通过。

### T. 数据存放与设置（155–156）

- **#155** Host 数据与 Client 设置分树、系统用户本地目录、秘密为当前用户可读 JSON 不用钥匙串：`host_kernel.rs::host_data_and_desktop_client_settings_are_two_trees`、`secrets_are_user_readable_json_not_keychain`。状态：通过。
- **#156** v1 没有备份按钮、搬家就是复制目录：壳层无备份入口（无该 UI/命令）。状态：通过（设计+无入口）。
