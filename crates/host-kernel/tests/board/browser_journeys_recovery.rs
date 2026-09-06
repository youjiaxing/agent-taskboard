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
    tracker.add_issue(IssueRecord::open("you/garden", 2, "child ready").parent(
        "you/garden",
        1,
        "parent",
    ));
    tracker.add_issue(
        IssueRecord::open("you/garden", 3, "child blocked")
            .parent("you/garden", 1, "parent")
            .blocked_by("you/garden", 9, "blocker", true)
            .blocking("you/garden", 5, "waiting on history")
            .blocking("you/garden", 10, "active work"),
    );
    tracker.add_issue(IssueRecord::open("you/garden", 4, "unparented ready"));
    tracker.add_issue(IssueRecord::open("you/garden", 10, "active work"));
    tracker.add_issue(IssueRecord::open("you/garden", 9, "blocker").blocked_by(
        "you/garden",
        6,
        "old gate",
        false,
    ));
    tracker.set_issue_body(
        "you/garden#2",
        "# Question\n\nCan the operator read **every constraint** beside the official TUI?\n\n## Constraints\n\n- Keep Tracker markdown unchanged\n- Render `inline code` clearly\n- Keep [the GitHub Issue](https://github.com/you/garden/issues/2) available\n- Reject [dangerous links](javascript:alert(1))\n\n<script>window.__ISSUE_HTML_EXECUTED__ = true</script>\n\n## Long document\n\nParagraph one explains why the Issue document remains the source material while the board stays read-only.\n\nParagraph two is intentionally long enough to require scrolling in the inspector at 1440 by 900.\n\nParagraph three keeps family and Dependency sections below the complete document.\n\nParagraph four verifies that the title and primary actions remain available while this content scrolls.\n\nParagraph five provides enough vertical depth for the mobile Issue view at 390 by 844.\n\nParagraph six confirms that entering a Run must retain this same complete Issue document.",
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
    let mut remote_host = HostKernel::boot(remote_req).unwrap();
    pin_board_test_time(&mut remote_host);
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
