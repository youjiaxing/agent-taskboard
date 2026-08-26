mod common;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

use common::{ReadMode, SeamTracker};
use host_kernel::{
    BoardEmptyReason, BootRequest, CenterView, FrontierEmptyReason, HostKernel, IssueRecord,
    LoopbackAssets, LoopbackServer, MemoryTracker, SystemAppearance, TriageRole,
    DEFAULT_RECENT_LIMIT,
};

fn boot_req(root: &Path) -> BootRequest {
    BootRequest {
        app_local_data_dir: root.to_path_buf(),
        app_log_dir: root.join("logs"),
        system_locale: "zh-Hans-CN".into(),
        system_appearance: SystemAppearance::Light,
        host_display_name: "Studio".into(),
    }
}

fn make_dir(root: &Path, name: &str) -> std::path::PathBuf {
    let dir = root.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn boot(root: &Path, tracker: Arc<MemoryTracker>) -> HostKernel {
    HostKernel::boot_with(boot_req(root), tracker).unwrap()
}

fn boot_seam(root: &Path, tracker: Arc<SeamTracker>) -> HostKernel {
    HostKernel::boot_with(boot_req(root), tracker).unwrap()
}

fn register(host: &mut HostKernel, dir: &Path, repository: &str) -> String {
    host.handle(serde_json::json!({
        "op": "registerProject",
        "name": "garden",
        "localPath": dir,
        "repository": repository,
    }))
    .unwrap()
    .snapshot
    .focused_project_id
}

fn run_browser_e2e(host: HostKernel, script_name: &str, envs: &[(&str, &Path)]) {
    let kernel = Arc::new(Mutex::new(host));
    let dist = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/desktop/dist")
        .canonicalize()
        .expect("built desktop client");
    let client = LoopbackServer::attach_with(
        Arc::clone(&kernel),
        0,
        LoopbackAssets::Directory(dist),
        |_| {},
    )
    .unwrap();
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut command = Command::new("node");
    command
        .arg(repo.join("apps/desktop/e2e").join(script_name))
        .current_dir(&repo)
        .env("BOARD_URL", client.protocol_url().to_string());
    for (name, value) in envs {
        command.env(name, value);
    }
    let output = command.output().expect("playwright");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{script_name} browser e2e failed\nstdout:{stdout}\nstderr:{stderr}"
    );
}

fn ids(cards: &[host_kernel::IssueCard]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn node_ids(graph: &host_kernel::DependencyGraph) -> Vec<String> {
    graph.nodes.iter().map(|node| node.id.clone()).collect()
}

fn edge_pairs(graph: &host_kernel::DependencyGraph) -> Vec<(String, String)> {
    graph
        .edges
        .iter()
        .map(|edge| (edge.from.clone(), edge.to.clone()))
        .collect()
}

fn node_rank(graph: &host_kernel::DependencyGraph, id: &str) -> u32 {
    graph
        .nodes
        .iter()
        .find(|node| node.id == id)
        .expect("node")
        .rank
}

#[test]
fn selecting_a_github_project_projects_four_columns_left_to_right() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(
        IssueRecord::open("you/garden", 2, "blocked work")
            .label("ready-for-agent")
            .blocked_by("you/garden", 9, "blocker", true),
    );
    tracker.add_issue(IssueRecord::open("you/garden", 3, "ready work").label("needs-triage"));
    tracker.add_issue(
        IssueRecord::open("you/garden", 4, "claimed work")
            .label("ready-for-agent")
            .assignee("ada"),
    );
    tracker.add_issue(
        IssueRecord::open("you/garden", 8, "done later").closed_at("2026-08-20T10:00:00Z"),
    );
    tracker.add_issue(
        IssueRecord::open("you/garden", 7, "just closed").closed_at("2026-08-22T10:00:00Z"),
    );
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, &dir, "you/garden");
    let board = host.snapshot().board.expect("board");
    let columns = board.columns.expect("columns");

    assert_eq!(ids(&columns.blocked), vec!["you/garden#2"]);
    assert_eq!(ids(&columns.frontier), vec!["you/garden#3"]);
    assert_eq!(ids(&columns.in_progress), vec!["you/garden#4"]);
    assert_eq!(
        ids(&columns.recently_completed),
        vec!["you/garden#7", "you/garden#8"]
    );
    assert_eq!(columns.in_progress[0].claimed_by, vec!["ada"]);
    assert_eq!(
        columns.frontier[0].triage_role,
        Some(TriageRole::NeedsTriage)
    );
    assert_eq!(board.recent_limit, DEFAULT_RECENT_LIMIT);
    assert!(board.empty.is_none());
}

#[test]
fn title_search_stacks_with_triage_and_open_closed_filters() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(
        IssueRecord::open("you/garden", 1, "Fix search keyboard").label("ready-for-agent"),
    );
    tracker.add_issue(IssueRecord::open("you/garden", 2, "Fix search result").label("needs-info"));
    tracker.add_issue(
        IssueRecord::open("you/garden", 3, "Fix search history")
            .label("ready-for-agent")
            .closed_at("2026-08-01T10:00:00Z"),
    );
    tracker.add_issue(
        IssueRecord::open("you/garden", 4, "Unrelated keyboard").label("ready-for-agent"),
    );
    let mut host = boot(tmp.path(), tracker);
    let project_id = register(&mut host, &dir, "you/garden");

    let open = host
        .handle(serde_json::json!({
            "op": "searchIssues",
            "projectId": project_id,
            "title": "search",
            "triageRole": "ready-for-agent",
            "state": "open",
        }))
        .unwrap()
        .snapshot
        .board
        .unwrap();
    assert_eq!(ids(&open.columns.unwrap().frontier), vec!["you/garden#1"]);
    assert_eq!(open.search.title, "search");

    let closed = host
        .handle(serde_json::json!({
            "op": "searchIssues",
            "projectId": project_id,
            "title": "SEARCH",
            "triageRole": "ready-for-agent",
            "state": "closed",
        }))
        .unwrap()
        .snapshot
        .board
        .unwrap();
    let columns = closed.columns.unwrap();
    assert!(columns.frontier.is_empty());
    assert_eq!(ids(&columns.recently_completed), vec!["you/garden#3"]);
}

#[test]
fn triage_role_does_not_decide_frontier_membership() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "needs triage").label("needs-triage"));
    tracker.add_issue(IssueRecord::open("you/garden", 2, "needs info").label("needs-info"));
    tracker.add_issue(IssueRecord::open("you/garden", 3, "human").label("ready-for-human"));
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, &dir, "you/garden");
    let columns = host.snapshot().board.unwrap().columns.unwrap();
    assert_eq!(
        ids(&columns.frontier),
        vec!["you/garden#1", "you/garden#2", "you/garden#3"]
    );
    assert!(columns.blocked.is_empty());
}

#[test]
fn closed_blocker_does_not_latch_frontier() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 50, "unlocked").blocked_by(
        "you/garden",
        49,
        "already closed",
        false,
    ));
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, &dir, "you/garden");
    let columns = host.snapshot().board.unwrap().columns.unwrap();
    assert_eq!(ids(&columns.frontier), vec!["you/garden#50"]);
}

#[test]
fn parent_filter_is_not_a_second_frontier() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(
        IssueRecord::open("you/garden", 1, "parent")
            .child("you/garden", 2, "child ready")
            .child("you/garden", 3, "child blocked"),
    );
    tracker.add_issue(IssueRecord::open("you/garden", 2, "child ready").parent(
        "you/garden",
        1,
        "parent",
    ));
    tracker.add_issue(
        IssueRecord::open("you/garden", 3, "child blocked")
            .parent("you/garden", 1, "parent")
            .blocked_by("you/garden", 9, "blocker", true),
    );
    tracker.add_issue(IssueRecord::open("you/garden", 4, "unparented ready"));
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, &dir, "you/garden");

    let before = host.snapshot().board.unwrap().columns.unwrap();
    assert_eq!(
        ids(&before.frontier),
        vec!["you/garden#1", "you/garden#2", "you/garden#4"]
    );

    host.handle(serde_json::json!({
        "op": "filterParent",
        "issueId": "you/garden#1",
    }))
    .unwrap();
    let filtered = host.snapshot().board.unwrap();
    let columns = filtered.columns.unwrap();
    assert_eq!(ids(&columns.frontier), vec!["you/garden#2"]);
    assert_eq!(ids(&columns.blocked), vec!["you/garden#3"]);
    assert!(!ids(&columns.frontier).contains(&"you/garden#4".into()));
    assert_eq!(filtered.parent_filter.unwrap().id, "you/garden#1");

    host.handle(serde_json::json!({ "op": "clearParentFilter" }))
        .unwrap();
    let restored = host.snapshot().board.unwrap().columns.unwrap();
    assert_eq!(
        ids(&restored.frontier),
        vec!["you/garden#1", "you/garden#2", "you/garden#4"]
    );
}

#[test]
fn parent_filter_does_not_shrink_the_dependency_graph() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(
        IssueRecord::open("you/garden", 1, "parent")
            .child("you/garden", 2, "child ready")
            .child("you/garden", 3, "child blocked"),
    );
    tracker.add_issue(IssueRecord::open("you/garden", 2, "child ready").parent(
        "you/garden",
        1,
        "parent",
    ));
    tracker.add_issue(
        IssueRecord::open("you/garden", 3, "child blocked")
            .parent("you/garden", 1, "parent")
            .blocked_by("you/garden", 9, "blocker", true),
    );
    tracker.add_issue(IssueRecord::open("you/garden", 4, "unparented ready"));
    tracker.add_issue(IssueRecord::open("you/garden", 9, "blocker"));
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, &dir, "you/garden");
    host.handle(serde_json::json!({
        "op": "filterParent",
        "issueId": "you/garden#1",
    }))
    .unwrap();
    let board = host.snapshot().board.unwrap();
    assert_eq!(ids(&board.columns.unwrap().frontier), vec!["you/garden#2"]);
    let graph = board.graph.expect("graph");
    assert!(node_ids(&graph).contains(&"you/garden#4".into()));
    assert!(node_ids(&graph).contains(&"you/garden#9".into()));
    assert_eq!(
        edge_pairs(&graph),
        vec![("you/garden#9".into(), "you/garden#3".into())]
    );
}

#[test]
fn focusing_a_relation_only_changes_details() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(
        IssueRecord::open("you/garden", 10, "child")
            .parent("you/garden", 1, "parent")
            .blocked_by("you/garden", 9, "blocker", true)
            .blocking("you/garden", 11, "downstream")
            .assignee("ada"),
    );
    tracker.add_issue(IssueRecord::open("you/garden", 4, "unparented"));
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, &dir, "you/garden");
    host.handle(serde_json::json!({
        "op": "focusIssue",
        "issueId": "you/garden#10",
    }))
    .unwrap();
    let board = host.snapshot().board.unwrap();
    let detail = board.selected.expect("detail");
    assert_eq!(detail.id, "you/garden#10");
    assert_eq!(detail.claimed_by, vec!["ada"]);
    assert_eq!(detail.parent.unwrap().id, "you/garden#1");
    assert_eq!(detail.blocked_by[0].id, "you/garden#9");
    assert_eq!(detail.blocking[0].id, "you/garden#11");
    assert!(board.parent_filter.is_none());
    assert!(ids(&board.columns.unwrap().frontier).contains(&"you/garden#4".into()));

    host.handle(serde_json::json!({
        "op": "focusIssue",
        "issueId": "you/garden#1",
    }))
    .unwrap();
    let after = host.snapshot().board.unwrap();
    assert_eq!(after.selected.unwrap().id, "you/garden#1");
    assert!(after.parent_filter.is_none());
}

#[test]
fn unclear_cross_project_blocker_keeps_issue_off_frontier() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(
        IssueRecord::open("you/garden", 12, "waiting on other repo")
            .blocked_by_unclear("acme/notes", 3),
    );
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, &dir, "you/garden");
    let board = host.snapshot().board.unwrap();
    let columns = board.columns.unwrap();
    assert_eq!(ids(&columns.blocked), vec!["you/garden#12"]);
    assert!(columns.frontier.is_empty());
    host.handle(serde_json::json!({
        "op": "focusIssue",
        "issueId": "you/garden#12",
    }))
    .unwrap();
    let blocked_by = host.snapshot().board.unwrap().selected.unwrap().blocked_by;
    assert_eq!(blocked_by.len(), 1);
    assert!(!blocked_by[0].visible);
}

#[test]
fn recently_completed_uses_this_client_limit() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    for number in 1..=6 {
        tracker.add_issue(
            IssueRecord::open("you/garden", number, format!("closed {number}"))
                .closed_at(format!("2026-08-2{number}T10:00:00Z")),
        );
    }
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, &dir, "you/garden");
    let first = host.snapshot().board.unwrap().columns.unwrap();
    assert_eq!(first.recently_completed.len(), 5);

    host.handle(serde_json::json!({
        "op": "setRecentCompletedLimit",
        "limit": 2,
    }))
    .unwrap();
    let limited = host.snapshot().board.unwrap();
    assert_eq!(limited.recent_limit, 2);
    assert_eq!(limited.columns.unwrap().recently_completed.len(), 2);

    drop(host);
    let host = HostKernel::boot_with(boot_req(tmp.path()), Arc::new(MemoryTracker::new())).unwrap();
    assert_eq!(host.snapshot().board.unwrap().recent_limit, 2);
}

#[test]
fn no_official_labels_means_an_ordinary_board() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "bug").label("bug"));
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, &dir, "you/garden");
    let board = host.snapshot().board.unwrap();
    assert!(!board.label_mapping_active);
    assert!(board.columns.unwrap().frontier[0].triage_role.is_none());
}

#[test]
fn three_empty_states_are_distinct() {
    let missing_tmp = tempfile::tempdir().unwrap();
    let missing_dir = make_dir(missing_tmp.path(), "work/garden");
    let unread = Arc::new(MemoryTracker::new());
    unread.fail_read("you/garden");
    let mut host = boot(missing_tmp.path(), unread);
    register(&mut host, &missing_dir, "you/garden");
    let missing = host.snapshot().board.unwrap();
    assert_eq!(missing.empty, Some(BoardEmptyReason::NoData));
    assert!(missing.columns.is_none());

    let blocked_tmp = tempfile::tempdir().unwrap();
    let blocked_dir = make_dir(blocked_tmp.path(), "work/garden");
    let blocked = Arc::new(MemoryTracker::new());
    blocked.add_issue(IssueRecord::open("you/garden", 1, "stuck").blocked_by(
        "you/garden",
        2,
        "gate",
        true,
    ));
    let mut host = boot(blocked_tmp.path(), blocked);
    register(&mut host, &blocked_dir, "you/garden");
    let all_blocked = host.snapshot().board.unwrap();
    assert!(all_blocked.columns.is_some());
    assert_eq!(
        all_blocked.frontier_empty,
        Some(FrontierEmptyReason::AllBlocked)
    );

    let claimed_tmp = tempfile::tempdir().unwrap();
    let claimed_dir = make_dir(claimed_tmp.path(), "work/garden");
    let claimed = Arc::new(MemoryTracker::new());
    claimed.add_issue(IssueRecord::open("you/garden", 1, "mine").assignee("ada"));
    let mut host = boot(claimed_tmp.path(), claimed);
    register(&mut host, &claimed_dir, "you/garden");
    let all_claimed = host.snapshot().board.unwrap();
    assert_eq!(
        all_claimed.frontier_empty,
        Some(FrontierEmptyReason::AllClaimed)
    );
}

#[test]
fn dependency_graph_contains_only_dependency_edges() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(
        IssueRecord::open("you/garden", 1, "parent")
            .child("you/garden", 2, "child ready")
            .child("you/garden", 3, "child blocked"),
    );
    tracker.add_issue(IssueRecord::open("you/garden", 2, "child ready").parent(
        "you/garden",
        1,
        "parent",
    ));
    tracker.add_issue(
        IssueRecord::open("you/garden", 3, "child blocked")
            .parent("you/garden", 1, "parent")
            .blocked_by("you/garden", 9, "blocker", true),
    );
    tracker.add_issue(IssueRecord::open("you/garden", 9, "blocker"));
    tracker.add_issue(IssueRecord::open("you/garden", 4, "unparented ready"));
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, &dir, "you/garden");
    let board = host.snapshot().board.unwrap();
    let graph = board.graph.expect("graph");

    assert_eq!(
        node_ids(&graph),
        vec![
            "you/garden#1",
            "you/garden#2",
            "you/garden#4",
            "you/garden#9",
            "you/garden#3",
        ]
    );
    assert_eq!(
        edge_pairs(&graph),
        vec![("you/garden#9".into(), "you/garden#3".into())]
    );
    assert!(node_rank(&graph, "you/garden#9") < node_rank(&graph, "you/garden#3"));
    assert!(!board.show_closed_graph_context);
}

#[test]
fn closed_context_toggle_only_adds_nodes() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(
        IssueRecord::open("you/garden", 10, "waiting")
            .parent("you/garden", 1, "parent")
            .blocked_by("you/garden", 9, "already closed", false),
    );
    tracker.add_issue(IssueRecord::open("you/garden", 1, "parent").child(
        "you/garden",
        10,
        "waiting",
    ));
    tracker.add_issue(
        IssueRecord::open("you/garden", 9, "already closed").closed_at("2026-08-20T10:00:00Z"),
    );
    tracker.add_issue(
        IssueRecord::open("you/garden", 8, "unrelated closed").closed_at("2026-08-19T10:00:00Z"),
    );
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, &dir, "you/garden");
    let before = host.snapshot().board.unwrap();
    let graph = before.graph.expect("graph");
    assert!(!before.show_closed_graph_context);
    assert_eq!(node_ids(&graph), vec!["you/garden#1", "you/garden#10"]);
    assert!(edge_pairs(&graph).is_empty());

    host.handle(serde_json::json!({
        "op": "setShowClosedGraphContext",
        "show": true,
    }))
    .unwrap();
    let after = host.snapshot().board.unwrap();
    let graph = after.graph.expect("graph");
    assert!(after.show_closed_graph_context);
    assert_eq!(
        node_ids(&graph),
        vec!["you/garden#1", "you/garden#9", "you/garden#10"]
    );
    assert_eq!(
        edge_pairs(&graph),
        vec![("you/garden#9".into(), "you/garden#10".into())]
    );
    assert!(!node_ids(&graph).contains(&"you/garden#8".into()));
    assert!(node_rank(&graph, "you/garden#9") < node_rank(&graph, "you/garden#10"));
}

#[test]
fn center_view_defaults_to_board_and_is_remembered() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, &dir, "you/garden");
    assert_eq!(host.snapshot().center_view, CenterView::Board);

    host.handle(serde_json::json!({
        "op": "setCenterView",
        "view": "graph",
    }))
    .unwrap();
    assert_eq!(host.snapshot().center_view, CenterView::Graph);

    drop(host);
    let host = HostKernel::boot_with(boot_req(tmp.path()), Arc::new(MemoryTracker::new())).unwrap();
    assert_eq!(host.snapshot().center_view, CenterView::Graph);
}

#[test]
fn focusing_a_graph_node_only_changes_details() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(
        IssueRecord::open("you/garden", 3, "child blocked")
            .parent("you/garden", 1, "parent")
            .blocked_by("you/garden", 9, "blocker", true),
    );
    tracker.add_issue(IssueRecord::open("you/garden", 9, "blocker"));
    tracker.add_issue(IssueRecord::open("you/garden", 4, "unparented ready"));
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, &dir, "you/garden");
    host.handle(serde_json::json!({
        "op": "setCenterView",
        "view": "graph",
    }))
    .unwrap();
    host.handle(serde_json::json!({
        "op": "focusIssue",
        "issueId": "you/garden#9",
    }))
    .unwrap();
    let board = host.snapshot().board.unwrap();
    assert_eq!(host.snapshot().center_view, CenterView::Graph);
    assert_eq!(board.selected.unwrap().id, "you/garden#9");
    assert!(board.parent_filter.is_none());
    assert!(ids(&board.columns.unwrap().frontier).contains(&"you/garden#4".into()));
}

#[test]
fn incomplete_read_does_not_compute_frontier_or_dependency_graph() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(SeamTracker::new());
    tracker.set_issues(
        "you/garden",
        vec![
            IssueRecord::open("you/garden", 3, "ready work").label("ready-for-agent"),
            IssueRecord::open("you/garden", 4, "claimed work")
                .label("ready-for-agent")
                .assignee("ada"),
            IssueRecord::open("you/garden", 7, "closed").closed_at("2026-08-22T10:00:00Z"),
        ],
    );
    tracker.set_read_mode("you/garden", ReadMode::Incomplete("page cut off".into()));
    let mut host = boot_seam(tmp.path(), tracker);
    register(&mut host, &dir, "you/garden");
    let board = host.snapshot().board.unwrap();
    assert_eq!(
        board.empty,
        Some(host_kernel::BoardEmptyReason::IncompleteRead)
    );
    assert!(board.columns.is_none());
    assert!(board.graph.is_none());
    assert!(board.frontier_empty.is_none());
    assert!(matches!(
        board.refresh,
        host_kernel::RefreshStatus::Incomplete { .. }
    ));
}

#[test]
fn unknown_move_op_does_not_change_tracker_state() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, &dir, "you/garden");
    let err = host
        .handle(serde_json::json!({
            "op": "moveIssue",
            "issueId": "you/garden#1",
            "column": "recentlyCompleted",
        }))
        .unwrap_err();
    assert!(err.to_string().contains("unknown op"));
    let columns = host.snapshot().board.unwrap().columns.unwrap();
    assert_eq!(ids(&columns.frontier), vec!["you/garden#1"]);
    assert!(columns.recently_completed.is_empty());
}

#[test]
fn browser_renders_incomplete_state_then_recovers_all_board_flows() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(SeamTracker::new());
    tracker.add_issue(
        IssueRecord::open("you/garden", 1, "parent")
            .child("you/garden", 2, "child ready")
            .child("you/garden", 3, "child blocked"),
    );
    tracker.add_issue(IssueRecord::open("you/garden", 2, "child ready").parent(
        "you/garden",
        1,
        "parent",
    ));
    tracker.add_issue(
        IssueRecord::open("you/garden", 3, "child blocked")
            .parent("you/garden", 1, "parent")
            .blocked_by("you/garden", 9, "blocker", true),
    );
    tracker.add_issue(IssueRecord::open("you/garden", 4, "unparented ready"));
    tracker.add_issue(IssueRecord::open("you/garden", 10, "active work"));
    tracker.add_issue(IssueRecord::open("you/garden", 9, "blocker"));
    tracker.set_issue_body(
        "you/garden#2",
        "# Question\n\nCan the operator read **every constraint** beside the official TUI?\n\n## Constraints\n\n- Keep Tracker markdown unchanged\n- Render `inline code` clearly\n- Keep [the GitHub Issue](https://github.com/you/garden/issues/2) available\n- Reject [dangerous links](javascript:alert(1))\n\n<script>window.__ISSUE_HTML_EXECUTED__ = true</script>\n\n## Long document\n\nParagraph one explains why the Issue document remains the source material while the board stays read-only.\n\nParagraph two is intentionally long enough to require scrolling in the inspector at 1440 by 900.\n\nParagraph three keeps family and Dependency sections below the complete document.\n\nParagraph four verifies that the title and primary actions remain available while this content scrolls.\n\nParagraph five provides enough vertical depth for the mobile Issue view at 390 by 844.\n\nParagraph six confirms that entering a Run must retain this same complete Issue document.",
    );
    tracker.set_issue_body(
        "you/garden#10",
        "# Active Run Question\n\nKeep this **same complete Issue** visible after entering the Run.\n\n- terminal stays primary\n- document stays readable\n- browser link still points to Tracker",
    );
    tracker.add_issue(
        IssueRecord::open("you/garden", 5, "waiting on history").blocked_by(
            "you/garden",
            6,
            "old gate",
            false,
        ),
    );
    tracker.add_issue(
        IssueRecord::open("you/garden", 6, "old gate").closed_at("2026-08-18T10:00:00Z"),
    );
    tracker.add_issue(
        IssueRecord::open("you/garden", 8, "older closed").closed_at("2026-08-20T10:00:00Z"),
    );
    tracker.add_issue(
        IssueRecord::open("you/garden", 7, "just closed").closed_at("2026-08-22T10:00:00Z"),
    );
    tracker.set_read_mode(
        "you/garden",
        ReadMode::Incomplete("pagination stopped early".into()),
    );
    let sessions = host_kernel::MemorySessionFactory::new();
    let agent = Arc::new(host_kernel::MemoryAgent::installed_grok());
    let mut host = HostKernel::boot_with_ports(
        boot_req(tmp.path()),
        host_kernel::KernelPorts {
            tracker: Arc::clone(&tracker) as _,
            agents: vec![Arc::clone(&agent) as _],
            launch_env: Arc::new(host_kernel::MemoryLaunchEnv::with_path("/mem/bin")) as _,
            sessions: Arc::clone(&sessions) as _,
        },
    )
    .unwrap();
    let garden_project_id = register(&mut host, &dir, "you/garden");
    assert_eq!(
        host.snapshot().board.unwrap().empty,
        Some(BoardEmptyReason::IncompleteRead)
    );
    tracker.set_read_mode("you/garden", ReadMode::Complete);
    host.handle(serde_json::json!({ "op": "refresh" })).unwrap();
    host.handle(serde_json::json!({
        "op": "startBoundRun",
        "issueId": "you/garden#10",
    }))
    .unwrap();
    sessions
        .last_session()
        .expect("active Run session")
        .push_output(b"mobile recent output\n");
    let active_run_id = host.snapshot().focused_run_id;
    agent.push_telemetry(host_kernel::TelemetrySample {
        run_id: active_run_id,
        project_id: String::new(),
        agent_id: String::new(),
        model: "grok-4.6".into(),
        lane: host_kernel::TelemetryLane::Main,
        tokens: host_kernel::TokenCounts {
            input: Some(12),
            output: Some(8),
            cache_read: Some(4),
            cache_write: Some(0),
            reasoning: Some(2),
            total: Some(26),
        },
        ttft_ms: Some(180),
        tokens_per_sec: Some(40),
        at_ms: 1_787_486_400_000,
    });
    host.handle(serde_json::json!({
        "op": "snapshot",
    }))
    .unwrap();
    let stopped_run_id = host
        .handle(serde_json::json!({
            "op": "startBoundRun",
            "issueId": "you/garden#7",
        }))
        .unwrap()
        .snapshot
        .focused_run_id;
    sessions
        .last_session()
        .expect("stopped Run session")
        .push_output(b"ended recent output\n");
    let stopped = host
        .handle(serde_json::json!({
            "op": "stopRun",
            "runId": stopped_run_id,
        }))
        .unwrap();
    assert!(stopped
        .snapshot
        .runs
        .iter()
        .find(|run| run.id == stopped_run_id)
        .expect("stopped Run")
        .recent_output
        .ends_with("ended recent output\n"));
    let tools_dir = make_dir(tmp.path(), "work/tools");
    tracker.add_issue(IssueRecord::open("you/tools", 1, "tool ready"));
    host.handle(serde_json::json!({
        "op": "registerProject",
        "name": "tools",
        "localPath": tools_dir,
        "repository": "you/tools",
    }))
    .unwrap();
    host.handle(serde_json::json!({
        "op": "startUnboundRun",
        "projectId": host.snapshot().focused_project_id,
    }))
    .unwrap();
    host.handle(serde_json::json!({
        "op": "focusProject",
        "projectId": garden_project_id,
    }))
    .unwrap();
    let remote_tmp = tempfile::tempdir().unwrap();
    let mut remote_req = boot_req(remote_tmp.path());
    remote_req.host_display_name = "Mini".into();
    let remote = Arc::new(Mutex::new(HostKernel::boot(remote_req).unwrap()));
    let _remote_server = LoopbackServer::attach(Arc::clone(&remote), 0, |_| {}).unwrap();
    let remote_address = _remote_server
        .protocol_url()
        .trim_end_matches('/')
        .to_string();
    let remote_code = remote
        .lock()
        .unwrap()
        .handle(serde_json::json!({
            "op": "beginPairingOffer",
            "address": remote_address,
        }))
        .unwrap()
        .snapshot
        .pairing_offer
        .unwrap()
        .code;
    host.handle(serde_json::json!({
        "op": "pairRemoteHost",
        "address": remote_address,
        "code": remote_code,
    }))
    .unwrap();
    assert_eq!(host.snapshot().hosts.len(), 2);
    run_browser_e2e(host, "board.mjs", &[]);
}

fn run_degraded_shell_edge_state(
    state: &str,
    repository: &str,
    inject_failure: impl FnOnce(&MemoryTracker, &str),
) {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), &format!("work/{state}"));
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open(repository, 1, "cached"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    register(&mut host, &project_dir, repository);
    inject_failure(&tracker, repository);
    host.handle(serde_json::json!({ "op": "refresh" })).unwrap();
    run_browser_e2e(
        host,
        "shell-edge-state.mjs",
        &[("SHELL_EDGE_STATE", Path::new(state))],
    );
}

#[test]
fn browser_renders_shell_edge_state_fixtures() {
    let empty_tmp = tempfile::tempdir().unwrap();
    run_browser_e2e(
        boot(empty_tmp.path(), Arc::new(MemoryTracker::new())),
        "shell-edge-state.mjs",
        &[("SHELL_EDGE_STATE", Path::new("empty-host"))],
    );

    let single_tmp = tempfile::tempdir().unwrap();
    let single_dir = make_dir(single_tmp.path(), "work/single");
    let single_tracker = Arc::new(MemoryTracker::new());
    single_tracker.add_issue(IssueRecord::open("you/single", 1, "ready"));
    let mut single = boot(single_tmp.path(), single_tracker);
    register(&mut single, &single_dir, "you/single");
    run_browser_e2e(
        single,
        "shell-edge-state.mjs",
        &[("SHELL_EDGE_STATE", Path::new("single-project"))],
    );

    let frontier_tmp = tempfile::tempdir().unwrap();
    let frontier_dir = make_dir(frontier_tmp.path(), "work/frontier");
    let frontier_tracker = Arc::new(MemoryTracker::new());
    frontier_tracker.add_issue(IssueRecord::open("you/frontier", 1, "claimed").assignee("ada"));
    let mut frontier = boot(frontier_tmp.path(), frontier_tracker);
    register(&mut frontier, &frontier_dir, "you/frontier");
    run_browser_e2e(
        frontier,
        "shell-edge-state.mjs",
        &[("SHELL_EDGE_STATE", Path::new("frontier-empty"))],
    );

    run_degraded_shell_edge_state("offline", "you/offline", |tracker, repository| {
        tracker.fail_read(repository);
    });
    run_degraded_shell_edge_state("rate-limited", "you/rate", |tracker, repository| {
        tracker.fail_rate_limited(repository, Some(120_000));
    });
    run_degraded_shell_edge_state("auth-failed", "you/auth", |tracker, repository| {
        tracker.fail_auth(repository);
    });
}

#[test]
fn browser_registers_the_first_project_from_an_empty_host_and_retries_failures() {
    let tmp = tempfile::tempdir().unwrap();
    let first = make_dir(tmp.path(), "work/first");
    let stale = make_dir(tmp.path(), "work/stale");
    let retry = make_dir(tmp.path(), "work/retry");
    for (dir, repository) in [(&first, "you/first"), (&stale, "you/stale")] {
        std::fs::create_dir(dir.join(".git")).unwrap();
        std::fs::write(
            dir.join(".git/config"),
            format!("[remote \"origin\"]\n\turl = git@github.com:{repository}.git\n"),
        )
        .unwrap();
    }
    let missing = tmp.path().join("work/missing");
    let tracker = Arc::new(SeamTracker::new());
    tracker.add_issue(IssueRecord::open("you/first", 1, "first tracker issue"));
    tracker.add_issue(IssueRecord::open("manual/retry", 1, "retry tracker issue"));
    let host = HostKernel::boot_with_ports(
        boot_req(tmp.path()),
        host_kernel::KernelPorts {
            tracker: Arc::clone(&tracker) as _,
            agents: vec![Arc::new(host_kernel::MemoryAgent::installed_grok()) as _],
            launch_env: Arc::new(host_kernel::MemoryLaunchEnv::with_path("/mem/bin")) as _,
            sessions: host_kernel::MemorySessionFactory::new() as _,
        },
    )
    .unwrap();
    assert!(host.snapshot().projects.is_empty());
    run_browser_e2e(
        host,
        "project-registration.mjs",
        &[
            ("FIRST_PROJECT_DIR", first.as_path()),
            ("STALE_PROJECT_DIR", stale.as_path()),
            ("MISSING_PROJECT_DIR", missing.as_path()),
            ("RETRY_PROJECT_DIR", retry.as_path()),
        ],
    );
}
