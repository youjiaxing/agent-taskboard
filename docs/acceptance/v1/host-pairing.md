# v1 验收矩阵：安装、配对与回环（A–B）

这是 Issue #101 / Agent Taskboard v1 最终验收矩阵的分册，索引见 [`../v1.md`](../v1.md)。

### A. 安装、Host 常驻与退出模式（1–8、16、17）

- **#1** 本机安装并打开即有一份 Host 跑本机 Project：`host_kernel.rs::opening_the_desktop_app_starts_the_local_host`；安装包层面见 §3「v0.1.0 资产」。状态：通过。
- **#2** 关窗口后 Host 仍在托盘跑：`host_kernel.rs::hiding_the_window_does_not_stop_the_host`（内核）+ `src-tauri/src/lib.rs` 关窗只 hide、托盘菜单 show/quit（壳）。真实托盘交互待真机。状态：通过（内核）；真实托盘菜单行为 待真实平台。
- **#3** 只有「退出 Host」才停进程：`host_kernel.rs::only_quit_host_stops_the_process`、`closing_a_browser_client_does_not_stop_the_host`。状态：通过。
- **#4** 退出时若有活跃 Run 必须选择返回或停掉：`runs.rs::quitting_host_with_active_runs_requires_a_choice`（内核）+ 壳层退出对话框 `quitOfferDialog`。状态：通过。
- **#5** 开机默认不自启、设置里可开：`src-tauri` 用 plugin-autostart，`main.ts::setStartAtLogin`（enable/disable），默认不调用 enable；Windows x64 Release 安装冒烟通过 UI Automation 实际切换并核对 HKCU `Software\\Microsoft\\Windows\\CurrentVersion\\Run`，随后关闭开关并确认条目移除。状态：通过。
- **#6** 崩溃后遗留 Run 标为意外中断且不自动拉起：`bound_runs.rs::host_crash_marks_bound_run_execution_stopped`；无自动拉起代码路径。状态：通过。
- **#7** 桌面默认拉起本机 Host 且免配对：`host_kernel.rs::opening_the_desktop_app_starts_the_local_host`；默认 `HostMode::HostAndClient`（`startup.rs::first_launch_defaults_to_host_and_client_and_persisted_client_only_survives_relogin`）。状态：通过。
- **#8** 本机窗口可只当 Client：`host_kernel.rs::client_only_cold_start_has_no_local_host_or_loopback_page`、`client_only_desktop_transport_is_private_and_keeps_the_client_process_alive`、`client_only_cold_start_can_use_the_saved_remote_host`。状态：通过。
- **#16** 10529 被占用时网页入口起不来并说明、桌面仍可用：`host_kernel.rs::occupied_loopback_port_explains_and_keeps_desktop_protocol`。状态：通过。
- **#17** 本机不起 Host 时没有回环页：`host_kernel.rs::loopback_page_is_absent_when_host_is_not_running`、`client_only_cold_start_has_no_local_host_or_loopback_page`。状态：通过。

### B. 配对、网络边界与回环（9–15）

- **#9** 一个 Client 同时连本机与已配对远程 Host：`host_kernel.rs::a_client_window_can_switch_among_local_and_paired_hosts`。状态：通过。
- **#10** 地址 + 一次性配对码 → 长期令牌：`host_kernel.rs::pairing_with_the_one_time_code_issues_a_long_term_token`、`wrong_pairing_code_does_not_issue_a_token`、`used_pairing_code_cannot_be_reused`。状态：通过。
- **#11** 二维码与复制文本同一份信息：`host_kernel.rs::pairing_offer_qr_and_copy_share_the_same_payload`。状态：通过。
- **#12** 撤销某个已配对 Client：`host_kernel.rs::revoking_a_client_makes_its_token_unusable_immediately`。状态：通过。
- **#13** 走自己的 Tailscale/VPN/局域网，产品无中继：架构无任何中继/账号代码；`host_kernel.rs::remote_access_with_a_token_can_call_the_host` 证明远程访问只凭 Token。状态：通过（设计+测试）。
- **#14** 本机浏览器 127.0.0.1:10529 免配对：`host_kernel.rs::loopback_page_is_served_without_pairing`、`loopback_page_port_is_10529`。状态：通过。
- **#15** 远程地址访问必须持长期令牌：`host_kernel.rs::non_loopback_access_is_not_pairing_exempt`、`remote_origins_cannot_call_the_local_host`。状态：通过。
