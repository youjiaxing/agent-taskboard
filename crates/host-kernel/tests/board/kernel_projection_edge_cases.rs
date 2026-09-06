use super::*;

#[test]
fn legacy_dependency_graph_payload_defaults_new_centering_fields() {
    let graph: DependencyGraph = serde_json::from_value(serde_json::json!({
        "nodes": [{
            "id": "you/garden#3",
            "repository": "you/garden",
            "number": 3,
            "title": "legacy node",
            "open": true,
            "rank": 0
        }],
        "edges": [],
        "closedCount": 0
    }))
    .unwrap();

    assert_eq!(graph.center_id, None);
    assert_eq!(graph.mode, DependencyGraphMode::Focused);
    assert_eq!(graph.total_count, 0);
    assert!(!graph.complete);
    assert_eq!(graph.max_distance, 0);
    assert!(!graph.truncated);
    assert_eq!(graph.nodes[0].distance, 0);
    assert_eq!(graph.nodes[0].relation, GraphRelation::Center);
}

#[test]
fn centered_dependency_graph_expands_the_complete_upstream_and_downstream_closure() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(
        IssueRecord::open("you/garden", 10, "center")
            .blocked_by("you/garden", 9, "upstream", true)
            .blocking("you/garden", 11, "downstream"),
    );
    tracker.add_issue(IssueRecord::open("you/garden", 9, "upstream").blocked_by(
        "you/garden",
        8,
        "closed origin",
        false,
    ));
    tracker.add_issue(
        IssueRecord::open("you/garden", 8, "closed origin").closed_at("2026-08-19T10:00:00Z"),
    );
    tracker.add_issue(IssueRecord::open("you/garden", 11, "downstream").blocking(
        "you/garden",
        12,
        "closed result",
    ));
    tracker.add_issue(
        IssueRecord::open("you/garden", 12, "closed result").closed_at("2026-08-20T10:00:00Z"),
    );
    tracker.add_issue(
        IssueRecord::open("you/garden", 99, "unrelated history").closed_at("2026-08-21T10:00:00Z"),
    );
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, "garden", &dir, "you/garden");
    host.handle(serde_json::json!({
        "op": "focusIssue",
        "issueId": "you/garden#10",
    }))
    .unwrap();
    host.handle(serde_json::json!({
        "op": "setCenterView",
        "view": "graph",
    }))
    .unwrap();
    host.handle(serde_json::json!({
        "op": "centerDependencyGraph",
        "issueId": "you/garden#10",
    }))
    .unwrap();
    let before = host.snapshot().board.unwrap();
    let graph = before.graph.expect("graph");
    assert_eq!(graph.center_id.as_deref(), Some("you/garden#10"));
    assert_eq!(graph.total_count, 5);
    assert!(!graph.complete);
    assert_eq!(
        node_ids(&graph),
        vec!["you/garden#9", "you/garden#10", "you/garden#11"]
    );
    assert_eq!(
        edge_pairs(&graph),
        vec![
            ("you/garden#10".into(), "you/garden#11".into()),
            ("you/garden#9".into(), "you/garden#10".into()),
        ]
    );

    host.handle(serde_json::json!({
        "op": "focusIssue",
        "issueId": "you/garden#9",
    }))
    .unwrap();
    let detail_only = host.snapshot().board.unwrap();
    assert_eq!(detail_only.selected.unwrap().id, "you/garden#9");
    assert_eq!(
        detail_only.graph.expect("graph").center_id.as_deref(),
        Some("you/garden#10")
    );

    host.handle(serde_json::json!({
        "op": "focusIssue",
        "issueId": "you/garden#10",
    }))
    .unwrap();
    host.handle(serde_json::json!({
        "op": "setDependencyGraphComplete",
        "complete": true,
    }))
    .unwrap();
    host.handle(serde_json::json!({
        "op": "centerDependencyGraph",
        "issueId": "you/garden#9",
    }))
    .unwrap();
    let recentered_board = host.snapshot().board.unwrap();
    assert_eq!(recentered_board.selected.unwrap().id, "you/garden#9");
    let recentered = recentered_board.graph.expect("graph");
    assert_eq!(recentered.center_id.as_deref(), Some("you/garden#9"));
    assert!(recentered.complete);
    assert_eq!(
        node_ids(&recentered),
        vec![
            "you/garden#8",
            "you/garden#9",
            "you/garden#10",
            "you/garden#11",
            "you/garden#12",
        ]
    );
    assert!(!recentered.nodes[0].open);

    let graph = host.snapshot().board.unwrap().graph.expect("graph");
    assert_eq!(graph.center_id.as_deref(), Some("you/garden#9"));
    assert!(graph.complete);
    assert_eq!(graph.total_count, 5);
    assert_eq!(
        node_ids(&graph),
        vec![
            "you/garden#8",
            "you/garden#9",
            "you/garden#10",
            "you/garden#11",
            "you/garden#12",
        ]
    );
    assert_eq!(
        edge_pairs(&graph),
        vec![
            ("you/garden#10".into(), "you/garden#11".into()),
            ("you/garden#11".into(), "you/garden#12".into()),
            ("you/garden#8".into(), "you/garden#9".into()),
            ("you/garden#9".into(), "you/garden#10".into()),
        ]
    );
    assert!(!node_ids(&graph).contains(&"you/garden#99".into()));
}

#[test]
fn center_view_defaults_to_board_and_is_remembered() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, "garden", &dir, "you/garden");
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
    register(&mut host, "garden", &dir, "you/garden");
    host.handle(serde_json::json!({
        "op": "setCenterView",
        "view": "graph",
    }))
    .unwrap();
    let center_before = host
        .snapshot()
        .board
        .unwrap()
        .graph
        .expect("graph")
        .center_id;
    host.handle(serde_json::json!({
        "op": "focusIssue",
        "issueId": "you/garden#9",
    }))
    .unwrap();
    let board = host.snapshot().board.unwrap();
    assert_eq!(host.snapshot().center_view, CenterView::Graph);
    assert_eq!(board.selected.unwrap().id, "you/garden#9");
    assert_eq!(board.graph.expect("graph").center_id, center_before);
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
    register(&mut host, "garden", &dir, "you/garden");
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
    register(&mut host, "garden", &dir, "you/garden");
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
