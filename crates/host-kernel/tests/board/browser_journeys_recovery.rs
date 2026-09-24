use super::*;

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
    tracker.add_issue(
        IssueRecord::open("you/garden", 2, "child ready")
            .parent("you/garden", 1, "parent")
            .label("ready-for-agent"),
    );
    tracker.add_issue(
        IssueRecord::open("you/garden", 3, "child blocked")
            .parent("you/garden", 1, "parent")
            .blocked_by("you/garden", 9, "blocker", true)
            .blocking("you/garden", 5, "waiting on history")
            .blocking("you/garden", 10, "active work"),
    );
    tracker
        .add_issue(IssueRecord::open("you/garden", 4, "unparented ready").label("ready-for-agent"));
    tracker.add_issue(IssueRecord::open("you/garden", 10, "active work"));
    tracker.add_issue(IssueRecord::open("you/garden", 9, "blocker").blocked_by(
        "you/garden",
        6,
        "old gate",
        false,
    ));
    tracker.set_issue_body(
        "you/garden#2",
        r##"# Question

Can the operator read **every constraint** beside the official TUI?

> Tracker Markdown stays the source of truth while the client renders a safe document.

## Constraints

- Keep Tracker markdown unchanged
  - Preserve nested unordered context
    1. Preserve nested ordered steps
- Render `inline code` clearly
- Keep [the GitHub Issue](https://github.com/you/garden/issues/2) available
- Reject [dangerous links](javascript:alert(1)) and [data links](data:text/plain;base64,SGVsbG8=)
- Keep ![the **Tracker image** label](https://example.com/image.png) inert

- [x] Render completed tasks
- [ ] Render pending tasks

| Structure | Expected result |
| --- | --- |
| Table | Semantic rows and cells |
| Long value | WrapWithoutCreatingHorizontalPageOverflowAtMobileWidth |

```ts
const trackerBody = "<script>code stays inert</script>";
```

<script>window.__ISSUE_HTML_EXECUTED__ = true</script>
<button onclick="window.__ISSUE_HTML_EXECUTED__ = true">unsafe button</button>

## Long document

Paragraph one explains why the Issue document remains the source material while the board stays read-only.

Paragraph two is intentionally long enough to require scrolling in the inspector at 1440 by 900.

Paragraph three keeps family and Dependency sections below the complete document.

Paragraph four verifies that the title and primary actions remain available while this content scrolls.

Paragraph five provides enough vertical depth for the mobile Issue view at 390 by 844.

Paragraph six confirms that entering a Run must retain this same complete Issue document."##,
    );
    tracker.set_issue_body(
        "you/garden#10",
        "# Active Run Question\n\nKeep this **same complete Issue** visible after entering the Run.\n\n- terminal stays primary\n- document stays readable\n- browser link still points to Tracker",
    );
    tracker.add_issue(
        IssueRecord::open("you/garden", 5, "waiting on history")
            .blocked_by("you/garden", 6, "old gate", false)
            .blocking("you/garden", 7, "just closed"),
    );
    tracker.add_issue(
        IssueRecord::open("you/garden", 6, "old gate")
            .blocked_by("you/garden", 100, "history 100", false)
            .closed_at("2026-08-18T10:00:00Z"),
    );
    for number in 100..=154 {
        let issue = IssueRecord::open("you/garden", number, format!("history {number}"))
            .closed_at("2020-01-01T00:00:00Z");
        tracker.add_issue(if number < 154 {
            issue.blocked_by(
                "you/garden",
                number + 1,
                format!("history {}", number + 1),
                false,
            )
        } else {
            issue
        });
    }
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
    pin_board_test_time(&mut host);
    let garden_project_id = register(&mut host, "garden", &dir, "you/garden");
    assert_eq!(
        host.snapshot().board.unwrap().empty,
        Some(BoardEmptyReason::IncompleteRead)
    );
    tracker.set_read_mode("you/garden", ReadMode::Complete);
    host.handle(serde_json::json!({ "op": "refresh" })).unwrap();
    start_bound_grok(&mut host, &garden_project_id, "you/garden#10");
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
    let stopped_run_id = start_bound_grok(&mut host, &garden_project_id, "you/garden#7")
        .snapshot
        .focused_run_id;
    sessions
        .last_session()
        .expect("stopped Run session")
        .push_output(b"\x1b[2J\x1b[Hended recent output\r\n");
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
        .ends_with("ended recent output"));
    let garden_unbound_run_id = host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": garden_project_id,
            "agentId": "grok-build",
            "values": {
                "model": "grok-4.6",
                "effort": "high",
                "permission-mode": "default",
                "always-approve": "false",
                "sandbox": "off",
                "initial-instruction": "",
                "additional-args": ""
            },
            "openingText": "garden unbound history integration",
        }))
        .unwrap()
        .snapshot
        .focused_run_id;
    sessions
        .last_session()
        .expect("garden unbound Run session")
        .push_output(b"garden unbound history output\n");
    host.handle(serde_json::json!({
        "op": "stopRun",
        "runId": garden_unbound_run_id,
    }))
    .unwrap();
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
        "agentId": "grok-build",
        "values": {
            "model": "grok-4.6",
            "effort": "high",
            "permission-mode": "default",
            "always-approve": "false",
            "sandbox": "off",
            "initial-instruction": "",
            "additional-args": ""
        },
        "openingText": "browser board unbound integration",
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
    let remote_tracker = Arc::new(MemoryTracker::new());
    remote_tracker.add_issue(IssueRecord::open("you/ledger", 1, "ledger ready"));
    let mut remote_host =
        HostKernel::boot_with(remote_req, Arc::clone(&remote_tracker) as _).unwrap();
    pin_board_test_time(&mut remote_host);
    let ledger_dir = make_dir(remote_tmp.path(), "work/ledger");
    register(&mut remote_host, "ledger", &ledger_dir, "you/ledger");
    let remote = Arc::new(Mutex::new(remote_host));
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
    host.handle(serde_json::json!({
        "op": "setRecentCompletedLimit",
        "limit": 3,
    }))
    .unwrap();
    run_browser_e2e(host, "board.mjs", &[]);
}

#[test]
fn browser_delivers_desktop_run_organization_workflows() {
    let tmp = tempfile::tempdir().unwrap();
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(
        IssueRecord::open("you/garden", 1, "finish and confirm").label("ready-for-agent"),
    );
    tracker.add_issue(IssueRecord::open("you/garden", 2, "next ready").label("ready-for-agent"));
    tracker.add_issue(IssueRecord::open("you/tools", 1, "tools ready"));
    tracker.add_issue(IssueRecord::open("you/legacy", 1, "legacy history"));
    let agent = Arc::new(MemoryAgent::installed_grok());
    let sessions = MemorySessionFactory::new();
    let mut host = HostKernel::boot_with_ports(
        boot_req(tmp.path()),
        host_kernel::KernelPorts {
            tracker: Arc::clone(&tracker) as _,
            agents: vec![Arc::clone(&agent) as _],
            launch_env: Arc::new(MemoryLaunchEnv::with_path("/mem/bin")) as _,
            sessions: Arc::clone(&sessions) as _,
        },
    )
    .unwrap();
    pin_board_test_time(&mut host);
    let garden_dir = make_dir(tmp.path(), "work/garden");
    let garden_project_id = register(&mut host, "garden", &garden_dir, "you/garden");
    host.handle(serde_json::json!({ "op": "setHostAutoAdvance", "enabled": true }))
        .unwrap();
    host.handle(serde_json::json!({
        "op": "setProjectAutoAdvance",
        "projectId": garden_project_id,
        "enabled": true,
    }))
    .unwrap();
    let pending_run_id = start_bound_grok(&mut host, &garden_project_id, "you/garden#1")
        .snapshot
        .focused_run_id;
    sessions
        .last_session()
        .unwrap()
        .push_output(b"pending confirmation output\n");
    sessions.last_session().unwrap().set_session_end(true);
    tracker.close_issue("you/garden", 1);
    sessions.last_session().unwrap().finish(0);
    host.handle(serde_json::json!({ "op": "snapshot" }))
        .unwrap();
    assert!(host.snapshot().pending_confirmation.is_some());
    host.handle(serde_json::json!({ "op": "tick", "nowMs": BOARD_TEST_NOW_MS + 10 }))
        .unwrap();
    host.handle(
        serde_json::json!({ "op": "setRunPinned", "runId": pending_run_id, "pinned": true }),
    )
    .unwrap();

    let archived_run_id = host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": garden_project_id,
            "agentId": "grok-build",
            "values": {
                "model": "grok-4.6",
                "effort": "high",
                "permission-mode": "default",
                "always-approve": "false",
                "sandbox": "off",
                "initial-instruction": "",
                "additional-args": ""
            },
            "openingText": "archived garden Run",
        }))
        .unwrap()
        .snapshot
        .focused_run_id;
    sessions
        .last_session()
        .unwrap()
        .push_output(b"garden archived recent output\n");
    host.handle(serde_json::json!({ "op": "stopRun", "runId": archived_run_id }))
        .unwrap();
    host.handle(serde_json::json!({ "op": "tick", "nowMs": BOARD_TEST_NOW_MS + 20 }))
        .unwrap();
    host.handle(serde_json::json!({ "op": "archiveRun", "runId": archived_run_id }))
        .unwrap();

    let tools_dir = make_dir(tmp.path(), "work/tools");
    let tools_project_id = register(&mut host, "tools", &tools_dir, "you/tools");
    let active_run_id = host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": tools_project_id,
            "agentId": "grok-build",
            "values": {
                "model": "grok-4.6",
                "effort": "high",
                "permission-mode": "default",
                "always-approve": "false",
                "sandbox": "off",
                "initial-instruction": "",
                "additional-args": ""
            },
            "openingText": "active pinned tools Run",
        }))
        .unwrap()
        .snapshot
        .focused_run_id;
    sessions
        .last_session()
        .unwrap()
        .push_output(b"active tools output\n");
    host.handle(serde_json::json!({ "op": "tick", "nowMs": BOARD_TEST_NOW_MS + 30 }))
        .unwrap();
    host.handle(
        serde_json::json!({ "op": "setRunPinned", "runId": active_run_id, "pinned": true }),
    )
    .unwrap();

    let legacy_dir = make_dir(tmp.path(), "work/legacy");
    let legacy_project_id = register(&mut host, "legacy", &legacy_dir, "you/legacy");
    let removed_run_id = host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": legacy_project_id,
            "agentId": "grok-build",
            "values": {
                "model": "grok-4.6",
                "effort": "high",
                "permission-mode": "default",
                "always-approve": "false",
                "sandbox": "off",
                "initial-instruction": "",
                "additional-args": ""
            },
            "openingText": "removed Project archived Run",
        }))
        .unwrap()
        .snapshot
        .focused_run_id;
    sessions
        .last_session()
        .unwrap()
        .push_output(b"legacy archived output\n");
    host.handle(serde_json::json!({ "op": "stopRun", "runId": removed_run_id }))
        .unwrap();
    host.handle(serde_json::json!({ "op": "tick", "nowMs": BOARD_TEST_NOW_MS + 40 }))
        .unwrap();
    host.handle(serde_json::json!({ "op": "archiveRun", "runId": removed_run_id }))
        .unwrap();
    host.handle(serde_json::json!({ "op": "removeProject", "projectId": legacy_project_id }))
        .unwrap();

    let remote_tmp = tempfile::tempdir().unwrap();
    let remote_tracker = Arc::new(MemoryTracker::new());
    remote_tracker.add_issue(IssueRecord::open("you/remote", 1, "remote ready"));
    let remote_agent = Arc::new(MemoryAgent::installed_grok());
    let remote_sessions = MemorySessionFactory::new();
    let mut remote_req = boot_req(remote_tmp.path());
    remote_req.host_display_name = "Remote Studio".into();
    let mut remote_host = HostKernel::boot_with_ports(
        remote_req,
        host_kernel::KernelPorts {
            tracker: Arc::clone(&remote_tracker) as _,
            agents: vec![Arc::clone(&remote_agent) as _],
            launch_env: Arc::new(MemoryLaunchEnv::with_path("/mem/bin")) as _,
            sessions: Arc::clone(&remote_sessions) as _,
        },
    )
    .unwrap();
    pin_board_test_time(&mut remote_host);
    let remote_dir = make_dir(remote_tmp.path(), "work/remote");
    let remote_project_id = register(&mut remote_host, "remote", &remote_dir, "you/remote");
    let remote_run_id = remote_host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": remote_project_id,
            "agentId": "grok-build",
            "values": {
                "model": "grok-4.6",
                "effort": "high",
                "permission-mode": "default",
                "always-approve": "false",
                "sandbox": "off",
                "initial-instruction": "",
                "additional-args": ""
            },
            "openingText": "remote pinned Run",
        }))
        .unwrap()
        .snapshot
        .focused_run_id;
    remote_host
        .handle(serde_json::json!({ "op": "setRunPinned", "runId": remote_run_id, "pinned": true }))
        .unwrap();
    let remote = Arc::new(Mutex::new(remote_host));
    let remote_server = LoopbackServer::attach(Arc::clone(&remote), 0, |_| {}).unwrap();
    let remote_address = remote_server
        .protocol_url()
        .trim_end_matches('/')
        .to_string();
    let remote_code = remote
        .lock()
        .unwrap()
        .handle(serde_json::json!({ "op": "beginPairingOffer", "address": remote_address }))
        .unwrap()
        .snapshot
        .pairing_offer
        .unwrap()
        .code;
    host.handle(serde_json::json!({ "op": "pairRemoteHost", "address": remote_address, "code": remote_code }))
        .unwrap();
    let remote_host_id = host
        .snapshot()
        .hosts
        .iter()
        .find(|candidate| !candidate.local)
        .unwrap()
        .id
        .clone();
    host.handle(serde_json::json!({ "op": "focusProject", "projectId": garden_project_id }))
        .unwrap();
    host.handle(serde_json::json!({ "op": "returnToBoard" }))
        .unwrap();

    run_browser_e2e(
        host,
        "run-organization.mjs",
        &[
            ("PENDING_RUN_ID", Path::new(&pending_run_id)),
            ("ACTIVE_RUN_ID", Path::new(&active_run_id)),
            ("ARCHIVED_RUN_ID", Path::new(&archived_run_id)),
            ("REMOVED_RUN_ID", Path::new(&removed_run_id)),
            ("REMOVED_PROJECT_ID", Path::new(&legacy_project_id)),
            ("REMOTE_HOST_ID", Path::new(&remote_host_id)),
            ("REMOTE_RUN_ID", Path::new(&remote_run_id)),
        ],
    );
}

#[test]
fn browser_clients_keep_navigation_and_forms_responsive_during_a_slow_tracker_read() {
    let tmp = tempfile::tempdir().unwrap();
    let garden_dir = make_dir(tmp.path(), "work/garden");
    let notes_dir = make_dir(tmp.path(), "work/notes");
    let tracker = Arc::new(SeamTracker::new());
    for number in 1..=18 {
        tracker.add_issue(IssueRecord::open(
            "you/garden",
            number,
            format!("garden issue {number}"),
        ));
    }
    tracker.add_issue(IssueRecord::open("you/notes", 1, "notes issue"));
    let mut host = boot_seam(tmp.path(), Arc::clone(&tracker));
    let garden_id = register(&mut host, "garden", &garden_dir, "you/garden");
    let notes_id = host
        .handle(serde_json::json!({
            "op": "registerProject",
            "name": "notes",
            "localPath": notes_dir,
            "repository": "you/notes",
        }))
        .unwrap()
        .snapshot
        .focused_project_id
        .clone();
    tracker.set_read_delay_ms(1_200);

    run_browser_e2e(
        host,
        "client-isolation-refresh.mjs",
        &[
            ("GARDEN_PROJECT_ID", Path::new(&garden_id)),
            ("NOTES_PROJECT_ID", Path::new(&notes_id)),
        ],
    );
}

#[test]
fn browser_keeps_navigation_responsive_during_a_slow_issue_write() {
    let tmp = tempfile::tempdir().unwrap();
    let garden_dir = make_dir(tmp.path(), "work/garden");
    let notes_dir = make_dir(tmp.path(), "work/notes");
    let tracker = Arc::new(SeamTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "garden issue"));
    tracker.set_issue_body("you/garden#1", "garden body");
    tracker.add_issue(IssueRecord::open("you/notes", 1, "notes issue"));
    tracker.set_issue_body("you/notes#1", "notes body");
    let mut host = boot_seam(tmp.path(), Arc::clone(&tracker));
    let garden_id = register(&mut host, "garden", &garden_dir, "you/garden");
    let notes_id = host
        .handle(serde_json::json!({
            "op": "registerProject",
            "name": "notes",
            "localPath": notes_dir,
            "repository": "you/notes",
        }))
        .unwrap()
        .snapshot
        .focused_project_id
        .clone();
    tracker.set_write_delay_ms(1_200);

    run_browser_e2e(
        host,
        "slow-issue-write-navigation.mjs",
        &[
            ("GARDEN_PROJECT_ID", Path::new(&garden_id)),
            ("NOTES_PROJECT_ID", Path::new(&notes_id)),
        ],
    );
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
    register(&mut host, "garden", &project_dir, repository);
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
    register(&mut single, "garden", &single_dir, "you/single");
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
    register(&mut frontier, "garden", &frontier_dir, "you/frontier");
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

    let truth_tmp = tempfile::tempdir().unwrap();
    let garden_dir = make_dir(truth_tmp.path(), "work/garden");
    let tools_dir = make_dir(truth_tmp.path(), "work/tools");
    let truth_tracker = Arc::new(MemoryTracker::new());
    for number in 1..=4 {
        truth_tracker.add_issue(IssueRecord::open(
            "you/garden",
            number,
            format!("garden run {number}"),
        ));
    }
    for number in 1..=2 {
        truth_tracker.add_issue(IssueRecord::open(
            "you/tools",
            number,
            format!("tools run {number}"),
        ));
    }
    let sessions = MemorySessionFactory::new();
    let agent = Arc::new(MemoryAgent::installed_grok());
    let mut truth_host = HostKernel::boot_with_ports(
        boot_req(truth_tmp.path()),
        host_kernel::KernelPorts {
            tracker: truth_tracker,
            agents: vec![Arc::clone(&agent) as _],
            launch_env: Arc::new(MemoryLaunchEnv::with_path("/mem/bin")) as _,
            sessions: Arc::clone(&sessions) as _,
        },
    )
    .unwrap();
    pin_board_test_time(&mut truth_host);
    let garden_id = register(&mut truth_host, "garden", &garden_dir, "you/garden");
    let waiting_run_id = start_bound_grok(&mut truth_host, &garden_id, "you/garden#1")
        .snapshot
        .focused_run_id;
    sessions.last_session().unwrap().set_waiting(true);
    let running_run_id = start_bound_grok(&mut truth_host, &garden_id, "you/garden#2")
        .snapshot
        .focused_run_id;
    let stopped_run_id = start_bound_grok(&mut truth_host, &garden_id, "you/garden#3")
        .snapshot
        .focused_run_id;
    truth_host
        .handle(serde_json::json!({
            "op": "stopRun",
            "runId": stopped_run_id,
        }))
        .unwrap();
    start_bound_grok(&mut truth_host, &garden_id, "you/garden#4");
    sessions.last_session().unwrap().finish(0);
    truth_host
        .handle(serde_json::json!({ "op": "snapshot" }))
        .unwrap();

    let tools_id = register(&mut truth_host, "tools", &tools_dir, "you/tools");
    start_bound_grok(&mut truth_host, &tools_id, "you/tools#1");
    start_bound_grok(&mut truth_host, &tools_id, "you/tools#2");
    sessions.last_session().unwrap().finish(0);
    agent.push_telemetry(host_kernel::TelemetrySample {
        run_id: waiting_run_id,
        project_id: String::new(),
        agent_id: String::new(),
        model: "grok-4.6".into(),
        lane: host_kernel::TelemetryLane::Main,
        tokens: host_kernel::TokenCounts {
            input: Some(4),
            output: Some(2),
            cache_read: None,
            cache_write: None,
            reasoning: None,
            total: Some(6),
        },
        ttft_ms: None,
        tokens_per_sec: None,
        at_ms: BOARD_TEST_NOW_MS - 3_600_000,
    });
    agent.push_telemetry(host_kernel::TelemetrySample {
        run_id: running_run_id,
        project_id: String::new(),
        agent_id: String::new(),
        model: "grok-4.6".into(),
        lane: host_kernel::TelemetryLane::Main,
        tokens: host_kernel::TokenCounts {
            input: Some(8),
            output: Some(4),
            cache_read: None,
            cache_write: None,
            reasoning: None,
            total: Some(12),
        },
        ttft_ms: Some(180),
        tokens_per_sec: None,
        at_ms: BOARD_TEST_NOW_MS,
    });
    truth_host
        .handle(serde_json::json!({ "op": "snapshot" }))
        .unwrap();
    run_browser_e2e(
        truth_host,
        "shell-edge-state.mjs",
        &[
            ("SHELL_EDGE_STATE", Path::new("overview-usage-truth")),
            ("GARDEN_PROJECT_ID", Path::new(&garden_id)),
        ],
    );
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
    let mut host = HostKernel::boot_with_ports(
        boot_req(tmp.path()),
        host_kernel::KernelPorts {
            tracker: Arc::clone(&tracker) as _,
            agents: vec![Arc::new(host_kernel::MemoryAgent::installed_grok()) as _],
            launch_env: Arc::new(host_kernel::MemoryLaunchEnv::with_path("/mem/bin")) as _,
            sessions: host_kernel::MemorySessionFactory::new() as _,
        },
    )
    .unwrap();
    pin_board_test_time(&mut host);
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

#[test]
fn browser_registers_local_markdown_and_self_hosted_github_projects_through_the_form() {
    let tmp = tempfile::tempdir().unwrap();
    let local = make_dir(tmp.path(), "work/local-tracker");
    let remote = make_dir(tmp.path(), "work/enterprise-project");
    let fallback = make_dir(tmp.path(), "work/fallback-project");
    for (dir, number, title) in [(&local, 1, "local issue"), (&fallback, 2, "fallback issue")] {
        let issue_dir = dir.join(".scratch/feature/issues");
        std::fs::create_dir_all(&issue_dir).unwrap();
        std::fs::write(
            issue_dir.join(format!("{number:02}-issue.md")),
            format!("# {number} — {title}\n\nStatus: ready-for-agent\n\nBlocked by: None\n"),
        )
        .unwrap();
    }
    std::fs::create_dir(remote.join(".git")).unwrap();
    std::fs::write(
        remote.join(".git/config"),
        "[remote \"origin\"]\n\turl = https://github.enterprise.example.com/acme/garden.git\n",
    )
    .unwrap();

    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("acme/garden", 3, "self-hosted issue"));
    let mut host = HostKernel::boot_with_ports(
        boot_req(tmp.path()),
        host_kernel::KernelPorts {
            tracker: Arc::new(TrackerRouter::new(Arc::clone(&tracker) as _)) as _,
            agents: vec![Arc::new(host_kernel::MemoryAgent::installed_grok()) as _],
            launch_env: Arc::new(host_kernel::MemoryLaunchEnv::with_path("/mem/bin")) as _,
            sessions: host_kernel::MemorySessionFactory::new() as _,
        },
    )
    .unwrap();
    pin_board_test_time(&mut host);
    run_browser_e2e(
        host,
        "project-tracker-lifecycle.mjs",
        &[
            ("LOCAL_PROJECT_DIR", local.as_path()),
            ("REMOTE_PROJECT_DIR", remote.as_path()),
            ("FALLBACK_PROJECT_DIR", fallback.as_path()),
        ],
    );
}

#[test]
fn browser_covers_local_markdown_issue_111_write_forms() {
    let tmp = tempfile::tempdir().unwrap();
    let local = make_dir(tmp.path(), "work/issue-111-ui");
    let issue_dir = local.join(".scratch/feature/issues");
    std::fs::create_dir_all(&issue_dir).unwrap();
    std::fs::write(
        issue_dir.join("01-parent.md"),
        "# 01 — Parent\n\nStatus: ready-for-agent\nType: task\n\nparent body\n",
    )
    .unwrap();
    std::fs::write(
        issue_dir.join("02-child.md"),
        "# 02 — Child\n\nStatus: ready-for-agent\nType: task\n\nchild body\n",
    )
    .unwrap();
    let host = HostKernel::boot_with_ports(
        boot_req(tmp.path()),
        host_kernel::KernelPorts {
            tracker: Arc::new(TrackerRouter::new(Arc::new(MemoryTracker::new()))) as _,
            agents: vec![Arc::new(host_kernel::MemoryAgent::installed_grok()) as _],
            launch_env: Arc::new(host_kernel::MemoryLaunchEnv::with_path("/mem/bin")) as _,
            sessions: host_kernel::MemorySessionFactory::new() as _,
        },
    )
    .unwrap();
    run_browser_e2e(
        host,
        "issue-111-ui.mjs",
        &[("LOCAL_PROJECT_DIR", local.as_path())],
    );
}
