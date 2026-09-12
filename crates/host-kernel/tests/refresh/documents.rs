use super::*;

#[test]
fn slow_remote_refresh_does_not_block_local_clients_or_restore_a_previous_host() {
    let remote_tmp = tempfile::tempdir().unwrap();
    let tracker = Arc::new(SeamTracker::new());
    tracker.add_issue(IssueRecord::open("you/remote", 1, "remote issue"));
    let mut remote = boot_seam(remote_tmp.path(), Arc::clone(&tracker));
    let remote_project = register(
        &mut remote,
        "remote",
        &make_dir(remote_tmp.path(), "work"),
        "you/remote",
    );
    let remote = Arc::new(Mutex::new(remote));
    let remote_server = LoopbackServer::attach_without_host_tick(
        Arc::clone(&remote),
        0,
        host_kernel::LoopbackAssets::Builtin,
        |_| {},
    )
    .unwrap();
    let offer = remote.lock().unwrap().handle(serde_json::json!({ "op": "beginPairingOffer", "address": remote_server.protocol_url() })).unwrap().snapshot.pairing_offer.unwrap();

    let local_tmp = tempfile::tempdir().unwrap();
    let mut local = boot(local_tmp.path(), Arc::new(MemoryTracker::new()));
    let local_project = register(
        &mut local,
        "local project",
        &make_dir(local_tmp.path(), "work"),
        "you/local",
    );
    local.handle(serde_json::json!({ "op": "pairRemoteHost", "address": remote_server.protocol_url(), "code": offer.code })).unwrap();
    let remote_id = local
        .snapshot()
        .hosts
        .iter()
        .find(|host| !host.local)
        .unwrap()
        .id
        .clone();
    let local = Arc::new(Mutex::new(local));
    let server = LoopbackServer::attach_without_host_tick(
        local,
        0,
        host_kernel::LoopbackAssets::Builtin,
        |_| {},
    )
    .unwrap();
    let protocol = server.protocol_url().to_string();
    post_rpc(
        &protocol,
        serde_json::json!({ "op": "focusHost", "hostId": remote_id, "clientInstanceId": "browser" }),
    );
    post_rpc(
        &protocol,
        serde_json::json!({ "op": "focusHost", "hostId": "local", "clientInstanceId": "desktop" }),
    );
    let baseline = tracker.read_start_count("you/remote");
    tracker.set_read_delay_ms(1000);
    let refreshing = std::thread::spawn({
        let protocol = protocol.clone();
        move || {
            post_rpc(
                &protocol,
                serde_json::json!({ "op": "refresh", "projectId": remote_project, "clientInstanceId": "browser" }),
            )
        }
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    while tracker.read_start_count("you/remote") == baseline {
        assert!(Instant::now() < deadline, "remote refresh did not start");
        std::thread::sleep(Duration::from_millis(5));
    }
    let started = Instant::now();
    let snapshot = post_rpc(
        &protocol,
        serde_json::json!({ "op": "snapshot", "clientInstanceId": "desktop" }),
    );
    let latency = started.elapsed();
    post_rpc(
        &protocol,
        serde_json::json!({ "op": "focusHost", "hostId": "local", "clientInstanceId": "browser" }),
    );
    let completed = refreshing.join().unwrap();
    assert!(
        latency < Duration::from_millis(250),
        "local Snapshot waited for remote Tracker: {latency:?}"
    );
    assert_eq!(snapshot["snapshot"]["focusedProjectId"], local_project);
    assert_eq!(
        completed["snapshot"]["focusedHostId"], "local",
        "the earlier remote response must respect the newer Host selection"
    );
}

#[test]
fn older_issue_document_cannot_undo_a_successful_issue_write() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(SeamTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "original title"));
    tracker.set_issue_body("you/garden#1", "original body");
    let mut kernel = boot_seam(tmp.path(), Arc::clone(&tracker));
    register(&mut kernel, "garden", &dir, "you/garden");
    let kernel = Arc::new(Mutex::new(kernel));
    let server = LoopbackServer::attach_without_host_tick(
        Arc::clone(&kernel),
        0,
        host_kernel::LoopbackAssets::Builtin,
        |_| {},
    )
    .unwrap();
    let protocol = server.protocol_url().to_string();
    post_rpc(
        &protocol,
        serde_json::json!({ "op": "focusIssue", "issueId": "you/garden#1" }),
    );

    for write in [
        serde_json::json!({ "op": "setIssueOpen", "issueId": "you/garden#1", "open": false }),
        serde_json::json!({ "op": "updateIssue", "issueId": "you/garden#1", "title": "new title", "body": "new body" }),
    ] {
        let (captured, release) = tracker.hold_next_document_response();
        let loading = std::thread::spawn({
            let protocol = protocol.clone();
            move || {
                post_rpc(
                    &protocol,
                    serde_json::json!({ "op": "loadIssueDocument", "issueId": "you/garden#1" }),
                )
            }
        });
        captured.recv_timeout(Duration::from_secs(5)).unwrap();
        let written = post_rpc(&protocol, write);
        release.send(()).unwrap();
        loading.join().unwrap();
        let final_state = post_rpc(&protocol, serde_json::json!({ "op": "snapshot" }));
        let selected = &final_state["snapshot"]["board"]["selected"];
        assert_eq!(
            selected["open"], false,
            "an old document must not reopen the Issue"
        );
        assert_eq!(
            selected["title"],
            written["snapshot"]["board"]["selected"]["title"]
        );
        assert_ne!(
            selected["document"]["kind"], "loading",
            "a discarded read must not leave the document stuck loading"
        );
        if selected["title"] == "new title" {
            assert_eq!(selected["document"]["body"], "new body");
        }
    }
}

#[test]
fn slow_issue_document_read_does_not_block_switching_to_another_issue() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(SeamTracker::new());
    tracker.set_issues(
        "you/garden",
        vec![
            IssueRecord::open("you/garden", 1, "slow detail"),
            IssueRecord::open("you/garden", 2, "fast detail"),
        ],
    );
    tracker.set_issue_body("you/garden#1", "slow body");
    tracker.set_issue_body("you/garden#2", "fast body");
    tracker.set_read_document_delay_ms(350);
    let project_dir = dir.clone();
    let mut kernel = boot_seam(tmp.path(), Arc::clone(&tracker));
    register(&mut kernel, "garden", &project_dir, "you/garden");
    let kernel = Arc::new(Mutex::new(kernel));
    let server = LoopbackServer::attach_without_host_tick(
        Arc::clone(&kernel),
        0,
        host_kernel::LoopbackAssets::Builtin,
        |_| {},
    )
    .unwrap();
    let protocol = server.protocol_url().to_string();

    post_rpc(
        &protocol,
        serde_json::json!({ "op": "focusIssue", "issueId": "you/garden#1" }),
    );
    let loading = std::thread::spawn({
        let protocol = protocol.clone();
        move || {
            post_rpc(
                &protocol,
                serde_json::json!({
                    "op": "loadIssueDocument",
                    "issueId": "you/garden#1",
                }),
            )
        }
    });
    std::thread::sleep(Duration::from_millis(40));
    let started = Instant::now();
    post_rpc(
        &protocol,
        serde_json::json!({ "op": "focusIssue", "issueId": "you/garden#2" }),
    );
    assert!(
        started.elapsed() < Duration::from_millis(250),
        "switching Issue was blocked by the slow document read: {:?}",
        started.elapsed()
    );
    loading.join().unwrap();

    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let selected = kernel
            .lock()
            .unwrap()
            .snapshot()
            .board
            .unwrap()
            .selected
            .unwrap();
        if selected.id == "you/garden#2" {
            assert!(
                !matches!(selected.document, host_kernel::IssueDocumentState::Ready { ref body, .. } if body == "slow body"),
                "a stale Issue document overwrote the current Issue"
            );
            break;
        }
        assert!(
            Instant::now() < deadline,
            "current Issue selection was lost"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn slow_issue_creation_does_not_hold_the_kernel_lock_for_other_navigation() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(SeamTracker::new());
    tracker.set_issues(
        "you/garden",
        vec![IssueRecord::open("you/garden", 1, "existing issue")],
    );
    tracker.set_write_delay_ms(350);
    let mut kernel = boot_seam(tmp.path(), Arc::clone(&tracker));
    let project_id = register(&mut kernel, "garden", &dir, "you/garden");
    let kernel = Arc::new(Mutex::new(kernel));
    let server = LoopbackServer::attach_without_host_tick(
        Arc::clone(&kernel),
        0,
        host_kernel::LoopbackAssets::Builtin,
        |_| {},
    )
    .unwrap();
    let protocol = server.protocol_url().to_string();

    post_rpc(
        &protocol,
        serde_json::json!({ "op": "focusIssue", "issueId": "you/garden#1" }),
    );
    let creating = std::thread::spawn({
        let protocol = protocol.clone();
        move || {
            post_rpc(
                &protocol,
                serde_json::json!({
                    "op": "createIssue",
                    "projectId": project_id,
                    "title": "created asynchronously",
                    "body": "created body",
                }),
            )
        }
    });
    std::thread::sleep(Duration::from_millis(40));
    let started = Instant::now();
    post_rpc(
        &protocol,
        serde_json::json!({ "op": "focusIssue", "issueId": "you/garden#1" }),
    );
    assert!(
        started.elapsed() < Duration::from_millis(250),
        "navigation was blocked by the slow Issue creation: {:?}",
        started.elapsed()
    );
    creating.join().unwrap();
    let issues = kernel
        .lock()
        .unwrap()
        .snapshot()
        .board
        .unwrap()
        .columns
        .unwrap()
        .frontier;
    assert!(
        issues
            .iter()
            .any(|issue| issue.title == "created asynchronously"),
        "the completed create response must merge the new Issue into the board"
    );
}

#[test]
fn slow_refresh_does_not_hold_the_kernel_lock_against_other_client_rpc() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(SeamTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut kernel = boot_seam(tmp.path(), Arc::clone(&tracker));
    let project_id = register(&mut kernel, "garden", &dir, "you/garden");
    let host = Arc::new(Mutex::new(kernel));
    let server = LoopbackServer::attach_client_transport(Arc::clone(&host), |_| {}).unwrap();
    let protocol_url = server.protocol_url().to_string();
    let baseline = tracker.read_start_count("you/garden");
    tracker.set_read_delay_ms(1_000);

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

    let started = Instant::now();
    let snapshot = post_rpc(
        &protocol_url,
        serde_json::json!({
            "op": "focusIssue",
            "clientInstanceId": "browser",
            "issueId": "you/garden#1",
        }),
    );
    assert!(
        started.elapsed() < Duration::from_millis(300),
        "navigation RPC waited behind the slow Tracker read: {:?}",
        started.elapsed()
    );
    assert_eq!(snapshot["snapshot"]["focusedProjectId"], project_id);
    assert_eq!(
        snapshot["snapshot"]["board"]["selected"]["id"],
        "you/garden#1"
    );
    assert_eq!(
        snapshot["snapshot"]["board"]["refresh"]["kind"],
        "refreshing"
    );

    let refreshed = refresh.join().unwrap();
    assert_eq!(refreshed["snapshot"]["board"]["refresh"]["kind"], "ready");
}

#[test]
fn issue_conflict_uses_a_structured_http_409_response() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(SeamTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "original title"));
    tracker.set_issue_body("you/garden#1", "original body");
    let mut kernel = boot_seam(tmp.path(), Arc::clone(&tracker));
    register(&mut kernel, "garden", &dir, "you/garden");
    tracker.set_issues(
        "you/garden",
        vec![IssueRecord::open("you/garden", 1, "remote title")],
    );
    let host = Arc::new(Mutex::new(kernel));
    let server = LoopbackServer::attach_client_transport(host, |_| {}).unwrap();

    let (status, body) = post_rpc_response(
        server.protocol_url(),
        serde_json::json!({
            "op": "updateIssue",
            "clientInstanceId": "desktop",
            "issueId": "you/garden#1",
            "title": "local title",
            "body": "original body",
            "base": { "title": "original title", "body": "original body" },
        }),
    );

    assert_eq!(status, 409);
    assert_eq!(body["error"], "issue-conflict");
    assert_eq!(body["issueId"], "you/garden#1");
    assert_eq!(body["fields"], serde_json::json!(["title"]));
    assert_eq!(body["latest"]["title"], "remote title");
}
