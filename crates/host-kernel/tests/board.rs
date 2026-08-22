use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

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
fn browser_renders_four_columns_and_keeps_filter_separate_from_details() {
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
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    register(&mut host, &dir, "you/garden");
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
    let url = client.protocol_url().to_string();
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script = repo.join("apps/desktop/e2e/board.mjs");
    let output = Command::new("node")
        .arg(&script)
        .current_dir(&repo)
        .env("BOARD_URL", &url)
        .output()
        .expect("playwright");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "browser e2e failed\nstdout:{stdout}\nstderr:{stderr}"
    );
}
