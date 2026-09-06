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
fn refresh_interval_has_a_five_minute_default_and_no_artificial_maximum() {
    assert_eq!(DEFAULT_REFRESH_INTERVAL_MS, 300_000);

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
fn claim_without_last_data_still_live_reads_the_focused_project() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.fail_read("you/garden");
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, "garden", &dir, "you/garden");
    let err = host
        .handle(serde_json::json!({
            "op": "claimIssue",
            "issueId": "you/garden#1",
        }))
        .unwrap_err();
    assert!(matches!(err, KernelError::Denied(message) if message.contains("never-fetched")));
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
