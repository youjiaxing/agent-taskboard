# v1 验收矩阵：看板、主壳与 skills（E–G）

这是 Issue #101 / Agent Taskboard v1 最终验收矩阵的分册，索引见 [`../v1.md`](../v1.md)。

### E. 四列看板与 Frontier（34–43）

- **#34** 打开 Project 即四列：`board.rs::selecting_a_github_project_projects_four_columns_left_to_right`；e2e 断言四列标题顺序。状态：通过。
- **#35** Frontier = 未关闭 ∧ 无未完成阻塞 ∧ 未被认领：`board.rs::closed_blocker_does_not_latch_frontier`、`bound_runs.rs::starting_a_bound_run_claims_and_leaves_frontier`。状态：通过。
- **#36** Triage Role 只筛选分组、不决定 Frontier：`board.rs::triage_role_does_not_decide_frontier_membership`。状态：通过。
- **#37** 点父 Issue 只是过滤、不是第二种 Frontier：`board.rs::parent_filter_is_not_a_second_frontier`。状态：通过。
- **#38** 跨 Project 阻塞不清则不进 Frontier：`board.rs::unclear_cross_project_blocker_keeps_issue_off_frontier`。状态：通过。
- **#39** Dependency 与父子两套关系分开：`board.rs::dependency_graph_contains_only_dependency_edges`、`github_adapter.rs::github_adapter_does_not_treat_relates_to_or_body_as_dependency`。状态：通过。
- **#40** 拖进列不关票/不改 Tracker：`board.rs::unknown_move_op_does_not_change_tracker_state`（未知操作被忽略，Tracker 状态不变）。状态：通过。
- **#41** 最近完成只展示最近关闭的 N（默认 5，可改）：`board.rs::recently_completed_uses_this_client_limit`。状态：通过。
- **#42** 详情点父/子/挡住只换详情：`board.rs::focusing_a_relation_only_changes_details`。状态：通过。
- **#43** 三种空状态分开：`board.rs::three_empty_states_are_distinct`。状态：通过。

### F. 主壳布局、总览与折叠（44–55）

- **#44** 侧栏 = 当前 Host + Project + 其下进行中/执行已停 Run：`waiting.rs::in_progress_list_has_running_waiting_and_execution_stopped`（三态列表数据）+ 壳层侧栏（e2e 侧栏开关）。状态：通过。
- **#45** 中间跟当前 Project，看板/依赖图切换：`board.rs::center_view_defaults_to_board_and_is_remembered`；e2e 依赖图交互。状态：通过。
- **#46** 默认看板并记住上次：`board.rs::center_view_defaults_to_board_and_is_remembered`。状态：通过。
- **#47** 依赖图只画 Dependency 不画父子：`board.rs::dependency_graph_contains_only_dependency_edges`、`parent_filter_does_not_shrink_the_dependency_graph`。状态：通过。
- **#48** 可开「已关闭上下文」，默认关、只加点：`board.rs::closed_context_toggle_only_adds_nodes`。状态：通过。
- **#49** 点图节点只换详情：`board.rs::focusing_a_graph_node_only_changes_details`。状态：通过。
- **#50** 跟 Host 走的总览：按终端状态分组、按 Project 过滤：`bound_runs.rs::host_overview_is_a_host_view_and_keeps_all_project_runs` + `board-desktop-run.mjs` 的空 Run、Project 指标、返回看板与 ended 过滤验收。状态：通过。
- **#51** 总览不是跨 Project Frontier 聚合、也不是跨 Host：`bound_runs.rs::host_overview_is_a_host_view_and_keeps_all_project_runs`（只跟一个 Host、只列 Run）。状态：通过。
- **#52** 底栏跟当前 Issue：有活跃 Run 出终端、无则收起：`bound_runs.rs::focusing_an_issue_with_an_active_run_focuses_that_pty`、`focusing_an_issue_without_an_active_run_hides_the_pty`；e2e（无活跃 Run 移除终端 dock）。状态：通过。
- **#53** 点已有 Run 的票终端抬到中间、自动收起左侧、右侧留 Issue：e2e（lift 后侧栏移除、运行聚焦）；内核 `focus_run_command_focuses_the_bound_issue_and_pty`。状态：通过（壳 e2e + 内核）。
- **#54** 「返回看板」恢复布局：`bound_runs.rs::returning_to_board_keeps_the_issue_and_restores_its_pty`、`focus_run_command_focuses_the_bound_issue_and_pty`。状态：通过。
- **#55** 折叠只是从布局拿掉、不留占位条、开关在顶栏：e2e（侧栏开关后侧栏从布局移除）。状态：通过（壳 e2e）。
- （注：#55 与 #53 的「收起」为同一条交互链，见上。）

### G. skills 只读透镜与执行已停（56–58）

- **#56** skills 只读透镜：通用父 Issue + 标签 + Label Mapping，对不上就普通板：`board.rs::no_official_labels_means_an_ordinary_board`、`triage_role_does_not_decide_frontier_membership`、`parent_filter_is_not_a_second_frontier`。状态：通过。
- **#57** 看板不解析 map 正文结构：实现无正文解析路径；`github_adapter.rs::github_adapter_does_not_treat_relates_to_or_body_as_dependency` 证明正文不被当作关系。状态：通过。
- **#58** 执行已停不算 Frontier、能继续（尽量恢复）或释放认领：`bound_runs.rs::abnormal_end_is_execution_stopped`、`execution_stopped_can_release_claim`、`continue_links_previous_run_and_resumes_native_session`、`host_crash_marks_bound_run_execution_stopped`。状态：通过。
