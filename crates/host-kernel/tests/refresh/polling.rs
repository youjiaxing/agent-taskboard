use super::*;

#[test]
fn refresh_interval_is_configurable_and_survives_reboot() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    register(&mut host, "garden", &dir, "you/garden");
    host.handle(serde_json::json!({
        "op": "setRefreshInterval",
        "intervalMs": 15_000,
    }))
    .unwrap();
    let fetched = match refresh_status(&host) {
        RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
        other => panic!("expected ready, got {other:?}"),
    };
    let baseline = tracker.read_count("you/garden");
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + 15_000,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), baseline + 1);

    drop(host);
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    match refresh_status(&host) {
        RefreshStatus::Ready {
            next_refresh_in_ms, ..
        } => assert_eq!(next_refresh_in_ms, Some(15_000)),
        other => panic!("expected persisted 15s interval, got {other:?}"),
    }
    let fetched = match refresh_status(&host) {
        RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
        other => panic!("expected ready, got {other:?}"),
    };
    let baseline = tracker.read_count("you/garden");
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + 15_000,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), baseline + 1);
}

#[test]
fn refresh_interval_has_a_one_minute_default_and_no_artificial_maximum() {
    assert_eq!(DEFAULT_REFRESH_INTERVAL_MS, 60_000);

    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, "garden", &dir, "you/garden");

    let one_day = 24 * 60 * 60 * 1_000;
    let outcome = host
        .handle(serde_json::json!({
            "op": "setRefreshInterval",
            "intervalMs": one_day,
        }))
        .unwrap();

    assert_eq!(outcome.snapshot.refresh_interval_ms, one_day);
}

#[test]
fn incomplete_read_persists_snapshot_and_board_across_reboot() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(SeamTracker::new());
    tracker.set_issues(
        "you/garden",
        vec![IssueRecord::open("you/garden", 1, "ready")],
    );
    tracker.set_read_mode(
        "you/garden",
        ReadMode::Incomplete("truncated at 500 issues".into()),
    );
    let mut host = boot_seam(tmp.path(), Arc::clone(&tracker));
    let project_id = register(&mut host, "garden", &dir, "you/garden");

    let board = host.snapshot().board.unwrap();
    assert_eq!(board.empty, Some(BoardEmptyReason::IncompleteRead));
    assert!(board.columns.is_none());
    let fetched = match board.refresh {
        RefreshStatus::Incomplete {
            fetched_at_ms: Some(fetched),
            next_refresh_in_ms,
            detail,
        } => {
            assert_eq!(next_refresh_in_ms, Some(DEFAULT_REFRESH_INTERVAL_MS));
            assert_eq!(detail.as_deref(), Some("truncated at 500 issues"));
            fetched
        }
        other => panic!("expected incomplete, got {other:?}"),
    };

    // 快照标记不完整并保留详情
    let path = snapshot_path(&host, &project_id);
    let stored: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(stored["complete"], false);
    assert_eq!(stored["detail"], "truncated at 500 issues");
    assert_eq!(stored["issues"][0]["number"], 1);

    // 到点自动重试，未完整前看板一直不画 Frontier
    let baseline = tracker.read_count("you/garden");
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + DEFAULT_REFRESH_INTERVAL_MS,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), baseline + 1);
    assert!(host.snapshot().board.unwrap().columns.is_none());

    // 重启后仍是不完整状态
    drop(host);
    let host = boot_seam(tmp.path(), Arc::clone(&tracker));
    let board = host.snapshot().board.unwrap();
    assert!(board.columns.is_none());
    match board.refresh {
        RefreshStatus::Incomplete { .. } => {}
        other => panic!("expected incomplete after reboot, got {other:?}"),
    }
}

#[test]
fn offline_after_incomplete_read_is_shown_as_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(SeamTracker::new());
    tracker.set_issues(
        "you/garden",
        vec![IssueRecord::open("you/garden", 1, "ready")],
    );
    tracker.set_read_mode(
        "you/garden",
        ReadMode::Incomplete("truncated at 500 issues".into()),
    );
    let mut host = boot_seam(tmp.path(), Arc::clone(&tracker));
    register(&mut host, "garden", &dir, "you/garden");
    match refresh_status(&host) {
        RefreshStatus::Incomplete { .. } => {}
        other => panic!("expected incomplete, got {other:?}"),
    }
    tracker.set_read_mode("you/garden", ReadMode::Offline);
    host.handle(serde_json::json!({ "op": "refresh" })).unwrap();
    match refresh_status(&host) {
        RefreshStatus::Offline { .. } => {}
        other => panic!("expected offline after a later failed read, got {other:?}"),
    }
}

#[test]
fn run_end_refreshes_even_when_rate_limited() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    let project_id = register(&mut host, "garden", &dir, "you/garden");
    tracker.fail_rate_limited("you/garden", Some(120_000));
    host.handle(serde_json::json!({ "op": "refresh" })).unwrap();
    let fetched = match refresh_status(&host) {
        RefreshStatus::RateLimited { fetched_at_ms, .. } => fetched_at_ms.expect("last data"),
        other => panic!("expected rate-limited, got {other:?}"),
    };
    let after_limit = tracker.read_count("you/garden");
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + 60_000,
    }))
    .unwrap();
    assert_eq!(
        tracker.read_count("you/garden"),
        after_limit,
        "interval auto-refresh must stay paused"
    );
    host.handle(serde_json::json!({
        "op": "noteRunEnded",
        "projectId": project_id,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), after_limit + 1);
}

#[test]
fn stale_client_view_without_heartbeat_stops_polling() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    let project_id = register(&mut host, "garden", &dir, "you/garden");
    host.handle(serde_json::json!({ "op": "hideWindow" }))
        .unwrap();
    host.handle(serde_json::json!({
        "op": "setClientView",
        "clientId": "phone",
        "projectId": project_id,
        "visible": true,
    }))
    .unwrap();
    let fetched = match refresh_status(&host) {
        RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
        other => panic!("expected ready, got {other:?}"),
    };
    let baseline = tracker.read_count("you/garden");
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + DEFAULT_REFRESH_INTERVAL_MS,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), baseline + 1);

    let after_poll = tracker.read_count("you/garden");
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + DEFAULT_REFRESH_INTERVAL_MS * 4,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), after_poll);
}

#[test]
fn tick_heartbeat_keeps_visible_client_past_ttl() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    let project_id = register(&mut host, "garden", &dir, "you/garden");
    host.handle(serde_json::json!({ "op": "hideWindow" }))
        .unwrap();
    host.handle(serde_json::json!({
        "op": "setClientView",
        "clientId": "phone",
        "projectId": project_id,
        "visible": true,
    }))
    .unwrap();
    let fetched = match refresh_status(&host) {
        RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
        other => panic!("expected ready, got {other:?}"),
    };
    let baseline = tracker.read_count("you/garden");
    let later = fetched + DEFAULT_REFRESH_INTERVAL_MS * 4;
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": later,
        "clientId": "phone",
        "projectId": project_id,
        "visible": true,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), baseline + 1);
}

#[test]
fn host_and_client_ticks_do_not_duplicate_interval_fetch() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    register(&mut host, "garden", &dir, "you/garden");
    let fetched = match refresh_status(&host) {
        RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
        other => panic!("expected ready, got {other:?}"),
    };
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + DEFAULT_REFRESH_INTERVAL_MS,
    }))
    .unwrap();
    let after_host_tick = tracker.read_count("you/garden");
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + DEFAULT_REFRESH_INTERVAL_MS + 500,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), after_host_tick);
}

#[test]
fn tauri_client_viewing_remote_host_keeps_that_project_watched() {
    let host_dir = tempfile::tempdir().unwrap();
    let client_dir = tempfile::tempdir().unwrap();
    let garden = make_dir(host_dir.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host_kernel = boot(host_dir.path(), Arc::clone(&tracker));
    let project_id = register(&mut host_kernel, "garden", &garden, "you/garden");
    host_kernel
        .handle(serde_json::json!({ "op": "hideWindow" }))
        .unwrap();
    let host = Arc::new(Mutex::new(host_kernel));
    let server = LoopbackServer::attach_client_transport(Arc::clone(&host), |_| {}).unwrap();
    let address = server.protocol_url().trim_end_matches('/').to_string();
    let code = host
        .lock()
        .unwrap()
        .handle(serde_json::json!({
            "op": "beginPairingOffer",
            "address": address,
        }))
        .unwrap()
        .snapshot
        .pairing_offer
        .unwrap()
        .code;

    let mut client = HostKernel::boot(boot_req(client_dir.path())).unwrap();
    let remote_id = client
        .handle(serde_json::json!({
            "op": "pairRemoteHost",
            "address": address,
            "code": code,
        }))
        .unwrap()
        .snapshot
        .hosts
        .iter()
        .find(|item| !item.local)
        .unwrap()
        .id
        .clone();
    client
        .handle(serde_json::json!({
            "op": "focusHost",
            "hostId": remote_id,
        }))
        .unwrap();

    let fetched = match host.lock().unwrap().snapshot().board.unwrap().refresh {
        RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
        other => panic!("expected ready, got {other:?}"),
    };
    let without_view = tracker.read_count("you/garden");
    host.lock()
        .unwrap()
        .handle(serde_json::json!({
            "op": "tick",
            "nowMs": fetched + DEFAULT_REFRESH_INTERVAL_MS,
        }))
        .unwrap();
    assert_eq!(
        tracker.read_count("you/garden"),
        without_view,
        "hidden Host window without Client heartbeat must not poll"
    );

    client
        .handle(serde_json::json!({
            "op": "setClientView",
            "clientId": "tauri-desktop",
            "projectId": project_id,
            "visible": true,
        }))
        .unwrap();
    let after_view = tracker.read_count("you/garden");
    assert!(
        after_view > without_view,
        "visible remote Client must refresh immediately"
    );
    let watched_at = match host.lock().unwrap().snapshot().board.unwrap().refresh {
        RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
        other => panic!("expected ready after remote Client view, got {other:?}"),
    };
    host.lock()
        .unwrap()
        .handle(serde_json::json!({
            "op": "tick",
            "nowMs": watched_at + DEFAULT_REFRESH_INTERVAL_MS,
        }))
        .unwrap();
    assert_eq!(tracker.read_count("you/garden"), after_view + 1);

    client
        .handle(serde_json::json!({
            "op": "setClientView",
            "clientId": "tauri-desktop",
            "projectId": "",
            "visible": false,
        }))
        .unwrap();
    let after_hide = tracker.read_count("you/garden");
    host.lock()
        .unwrap()
        .handle(serde_json::json!({
            "op": "tick",
            "nowMs": watched_at + DEFAULT_REFRESH_INTERVAL_MS * 2,
        }))
        .unwrap();
    assert_eq!(tracker.read_count("you/garden"), after_hide);
}

#[test]
fn project_focus_uses_fresh_cached_board_without_full_refresh() {
    let tmp = tempfile::tempdir().unwrap();
    let garden = make_dir(tmp.path(), "work/garden");
    let notes = make_dir(tmp.path(), "work/notes");
    let tracker = Arc::new(SeamTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "garden issue"));
    tracker.add_issue(IssueRecord::open("you/notes", 1, "notes issue"));
    let mut host = boot_seam(tmp.path(), Arc::clone(&tracker));
    let garden_id = register(&mut host, "garden", &garden, "you/garden");
    register(&mut host, "notes", &notes, "you/notes");
    let baseline = tracker.read_count("you/garden");

    let focused = host
        .handle(serde_json::json!({
            "op": "focusProject",
            "clientInstanceId": "desktop",
            "projectId": garden_id,
        }))
        .unwrap();
    assert_eq!(focused.snapshot.focused_project_id, garden_id);
    assert_eq!(
        focused.snapshot.board.unwrap().columns.unwrap().frontier[0].title,
        "garden issue"
    );
    host.handle(serde_json::json!({
        "op": "setClientView",
        "clientId": "browser",
        "projectId": garden_id,
        "visible": true,
    }))
    .unwrap();

    assert_eq!(
        tracker.read_count("you/garden"),
        baseline,
        "fresh cached Project focus and visibility must not read the full Tracker"
    );
}

#[test]
fn project_focus_does_not_retry_a_recent_failure_when_cached_board_exists() {
    let tmp = tempfile::tempdir().unwrap();
    let garden = make_dir(tmp.path(), "work/garden");
    let notes = make_dir(tmp.path(), "work/notes");
    let tracker = Arc::new(SeamTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "cached garden issue"));
    tracker.add_issue(IssueRecord::open("you/notes", 1, "notes issue"));
    let mut host = boot_seam(tmp.path(), Arc::clone(&tracker));
    let garden_id = register(&mut host, "garden", &garden, "you/garden");
    register(&mut host, "notes", &notes, "you/notes");

    tracker.set_read_mode("you/garden", ReadMode::Offline);
    host.handle(serde_json::json!({
        "op": "refresh",
        "projectId": garden_id,
    }))
    .unwrap();
    let baseline = tracker.read_count("you/garden");
    tracker.set_read_mode("you/garden", ReadMode::Complete);

    let focused = host
        .handle(serde_json::json!({
            "op": "focusProject",
            "clientInstanceId": "desktop",
            "projectId": garden_id,
        }))
        .unwrap();

    assert_eq!(tracker.read_count("you/garden"), baseline);
    assert_eq!(
        focused.snapshot.board.as_ref().unwrap().refresh.kind(),
        "offline"
    );
    assert_eq!(
        focused.snapshot.board.unwrap().columns.unwrap().frontier[0].title,
        "cached garden issue"
    );
}

#[test]
fn project_focus_returns_cached_board_before_one_background_refresh_finishes() {
    let tmp = tempfile::tempdir().unwrap();
    let garden = make_dir(tmp.path(), "work/garden");
    let notes = make_dir(tmp.path(), "work/notes");
    let tracker = Arc::new(SeamTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "garden issue"));
    tracker.add_issue(IssueRecord::open("you/notes", 1, "notes issue"));
    let mut kernel = boot_seam(tmp.path(), Arc::clone(&tracker));
    let garden_id = register(&mut kernel, "garden", &garden, "you/garden");
    let garden_fetched_at = match refresh_status(&kernel) {
        RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
        other => panic!("expected ready garden cache, got {other:?}"),
    };
    let notes_id = register(&mut kernel, "notes", &notes, "you/notes");
    kernel
        .handle(serde_json::json!({
            "op": "setClientView",
            "clientId": "desktop",
            "projectId": notes_id,
            "visible": true,
        }))
        .unwrap();
    kernel
        .handle(serde_json::json!({
            "op": "tick",
            "nowMs": garden_fetched_at + DEFAULT_REFRESH_INTERVAL_MS,
        }))
        .unwrap();

    let baseline = tracker.read_count("you/garden");
    tracker.set_read_delay_ms(750);
    let host = Arc::new(Mutex::new(kernel));
    let server = LoopbackServer::attach_client_transport(Arc::clone(&host), |_| {}).unwrap();

    let focused_at = Instant::now();
    let focused = post_rpc(
        server.protocol_url(),
        serde_json::json!({
            "op": "focusProject",
            "clientInstanceId": "desktop",
            "projectId": garden_id,
        }),
    );
    assert!(
        focused_at.elapsed() < Duration::from_millis(250),
        "Project focus waited for the Tracker: {:?}",
        focused_at.elapsed()
    );
    assert_eq!(focused["snapshot"]["focusedProjectId"], garden_id);
    assert_eq!(
        focused["snapshot"]["board"]["columns"]["frontier"][0]["title"],
        "garden issue"
    );

    let reported_at = Instant::now();
    post_rpc(
        server.protocol_url(),
        serde_json::json!({
            "op": "setClientView",
            "clientInstanceId": "desktop",
            "clientId": "desktop",
            "projectId": garden_id,
            "visible": true,
        }),
    );
    assert!(
        reported_at.elapsed() < Duration::from_millis(250),
        "Client view report waited for the Tracker: {:?}",
        reported_at.elapsed()
    );

    let deadline = Instant::now() + Duration::from_secs(2);
    while tracker.read_count("you/garden") == baseline && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(tracker.read_count("you/garden"), baseline + 1);
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(
        tracker.read_count("you/garden"),
        baseline + 1,
        "focusProject followed by setClientView must coalesce to one refresh"
    );
}

#[test]
fn client_tick_returns_before_background_refresh_finishes() {
    let tmp = tempfile::tempdir().unwrap();
    let garden = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(SeamTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "garden issue"));
    let mut kernel = boot_seam(tmp.path(), Arc::clone(&tracker));
    register(&mut kernel, "garden", &garden, "you/garden");
    let garden_fetched_at = match refresh_status(&kernel) {
        RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
        other => panic!("expected ready garden cache, got {other:?}"),
    };

    let baseline = tracker.read_count("you/garden");
    tracker.set_read_delay_ms(750);
    let host = Arc::new(Mutex::new(kernel));
    let server = LoopbackServer::attach_client_transport(Arc::clone(&host), |_| {}).unwrap();

    let ticked_at = Instant::now();
    post_rpc(
        server.protocol_url(),
        serde_json::json!({
            "op": "tick",
            "clientInstanceId": "desktop",
            "nowMs": garden_fetched_at + DEFAULT_REFRESH_INTERVAL_MS,
        }),
    );
    assert!(
        ticked_at.elapsed() < Duration::from_millis(250),
        "Client tick waited for the Tracker: {:?}",
        ticked_at.elapsed()
    );

    let deadline = Instant::now() + Duration::from_secs(2);
    while tracker.read_count("you/garden") == baseline && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(tracker.read_count("you/garden"), baseline + 1);
}

#[test]
fn rapid_project_focus_keeps_the_last_client_focus_after_older_refreshes_finish() {
    let tmp = tempfile::tempdir().unwrap();
    let tracker = Arc::new(SeamTracker::new());
    let mut kernel = boot_seam(tmp.path(), Arc::clone(&tracker));
    let mut projects = Vec::new();
    for name in ["alpha", "beta", "gamma"] {
        let repository = format!("you/{name}");
        tracker.add_issue(IssueRecord::open(&repository, 1, format!("{name} issue")));
        let directory = make_dir(tmp.path(), &format!("work/{name}"));
        let project_id = register(&mut kernel, name, &directory, &repository);
        projects.push((repository, project_id));
    }
    let fetched_at = match refresh_status(&kernel) {
        RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
        other => panic!("expected ready Project caches, got {other:?}"),
    };
    kernel
        .handle(serde_json::json!({ "op": "hideWindow" }))
        .unwrap();
    kernel
        .handle(serde_json::json!({
            "op": "tick",
            "nowMs": fetched_at + DEFAULT_REFRESH_INTERVAL_MS,
        }))
        .unwrap();
    let baseline: Vec<_> = projects
        .iter()
        .map(|(repository, _)| tracker.read_count(repository))
        .collect();
    tracker.set_read_delay_ms(300);
    let host = Arc::new(Mutex::new(kernel));
    let server = LoopbackServer::attach_client_transport(Arc::clone(&host), |_| {}).unwrap();

    for (_, project_id) in &projects {
        let started = Instant::now();
        let focused = post_rpc(
            server.protocol_url(),
            serde_json::json!({
                "op": "focusProject",
                "clientInstanceId": "desktop",
                "projectId": project_id,
            }),
        );
        assert!(started.elapsed() < Duration::from_millis(250));
        assert_eq!(focused["snapshot"]["focusedProjectId"], *project_id);
    }

    let deadline = Instant::now() + Duration::from_secs(2);
    while projects
        .iter()
        .zip(&baseline)
        .any(|((repository, _), count)| tracker.read_count(repository) == *count)
        && Instant::now() < deadline
    {
        std::thread::sleep(Duration::from_millis(10));
    }
    std::thread::sleep(Duration::from_millis(50));
    for ((repository, _), count) in projects.iter().zip(&baseline) {
        assert_eq!(
            tracker.read_count(repository),
            count + 1,
            "each stale Project switch must start exactly one refresh"
        );
    }
    let latest = post_rpc(
        server.protocol_url(),
        serde_json::json!({ "op": "snapshot", "clientInstanceId": "desktop" }),
    );
    assert_eq!(latest["snapshot"]["focusedProjectId"], projects[2].1);
    assert_eq!(
        latest["snapshot"]["board"]["columns"]["frontier"][0]["title"],
        "gamma issue"
    );
}

#[test]
fn project_focus_without_cached_data_returns_loading_before_the_tracker_finishes() {
    let tmp = tempfile::tempdir().unwrap();
    let tracker = Arc::new(SeamTracker::new());
    tracker.add_issue(IssueRecord::open("you/empty", 1, "loaded later"));
    tracker.set_read_mode("you/empty", ReadMode::Offline);
    let mut kernel = boot_seam(tmp.path(), Arc::clone(&tracker));
    let empty_dir = make_dir(tmp.path(), "work/empty");
    let empty_id = register(&mut kernel, "empty", &empty_dir, "you/empty");
    tracker.add_issue(IssueRecord::open("you/stable", 1, "stable issue"));
    let stable_dir = make_dir(tmp.path(), "work/stable");
    register(&mut kernel, "stable", &stable_dir, "you/stable");

    tracker.set_read_mode("you/empty", ReadMode::Complete);
    let baseline = tracker.read_count("you/empty");
    tracker.set_read_delay_ms(750);
    let host = Arc::new(Mutex::new(kernel));
    let server = LoopbackServer::attach_client_transport(host, |_| {}).unwrap();

    let started = Instant::now();
    let focused = post_rpc(
        server.protocol_url(),
        serde_json::json!({
            "op": "focusProject",
            "clientInstanceId": "desktop",
            "projectId": empty_id,
        }),
    );
    assert!(
        started.elapsed() < Duration::from_millis(250),
        "uncached Project focus waited for the Tracker: {:?}",
        started.elapsed()
    );
    assert_eq!(focused["snapshot"]["focusedProjectId"], empty_id);
    assert_eq!(focused["snapshot"]["board"]["empty"], "no-data");
    assert!(focused["snapshot"]["board"]["columns"].is_null());
    assert_eq!(
        focused["snapshot"]["board"]["refresh"]["kind"],
        "refreshing"
    );

    let deadline = Instant::now() + Duration::from_secs(2);
    while tracker.read_count("you/empty") == baseline && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    let loaded = post_rpc(
        server.protocol_url(),
        serde_json::json!({ "op": "snapshot", "clientInstanceId": "desktop" }),
    );
    assert_eq!(
        loaded["snapshot"]["board"]["columns"]["frontier"][0]["title"],
        "loaded later"
    );
}

#[test]
fn direct_claim_without_last_data_rejects_an_unknown_issue_without_refetching() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.fail_read("you/garden");
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    register(&mut host, "garden", &dir, "you/garden");
    let reads = tracker.read_count("you/garden");
    let err = host
        .handle(serde_json::json!({
            "op": "claimIssue",
            "issueId": "you/garden#1",
        }))
        .unwrap_err();
    assert!(matches!(err, KernelError::Protocol(message) if message.contains("unknown issue")));
    assert_eq!(tracker.read_count("you/garden"), reads);
}

#[test]
fn refresh_response_keeps_its_board_update_when_another_client_is_waiting() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(SeamTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut kernel = boot_seam(tmp.path(), Arc::clone(&tracker));
    let project_id = register(&mut kernel, "garden", &dir, "you/garden");
    let host = Arc::new(Mutex::new(kernel));
    let server = LoopbackServer::attach_client_transport(Arc::clone(&host), |_| {}).unwrap();
    let protocol_url = server.protocol_url().to_string();

    tracker.set_issues(
        "you/garden",
        vec![
            IssueRecord::open("you/garden", 1, "ready"),
            IssueRecord::open("you/garden", 2, "new from refresh"),
        ],
    );
    let baseline = tracker.read_start_count("you/garden");
    tracker.set_read_delay_ms(150);

    let refresh_url = protocol_url.clone();
    let refresh_project_id = project_id.clone();
    let refresh = std::thread::spawn(move || {
        post_rpc(
            &refresh_url,
            serde_json::json!({
                "op": "refresh",
                "clientInstanceId": "desktop",
                "projectId": refresh_project_id,
            }),
        )
    });

    let deadline = Instant::now() + Duration::from_secs(1);
    while tracker.read_start_count("you/garden") == baseline && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(tracker.read_start_count("you/garden") > baseline);

    let guard = host.lock().unwrap();
    std::thread::sleep(Duration::from_millis(200));
    let competing_url = protocol_url.clone();
    let competing = std::thread::spawn(move || {
        post_rpc(
            &competing_url,
            serde_json::json!({
                "op": "snapshot",
                "clientInstanceId": "browser",
            }),
        )
    });
    std::thread::sleep(Duration::from_millis(20));
    drop(guard);

    let refreshed = refresh.join().unwrap();
    let competing = competing.join().unwrap();
    assert!(
        refreshed["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["type"] == "board-updated"),
        "refresh requester lost board-updated to the competing Client: refreshed={refreshed}, competing={competing}"
    );
    assert_eq!(
        refreshed["snapshot"]["board"]["columns"]["frontier"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}
