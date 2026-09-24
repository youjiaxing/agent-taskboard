use super::*;

#[test]
fn pairing_offer_qr_and_copy_share_the_same_payload() {
    let tmp = tempfile::tempdir().unwrap();
    let mut host = HostKernel::boot(boot_req(tmp.path())).unwrap();
    let address = "http://100.64.1.2:10529";

    let out = host
        .handle(serde_json::json!({
            "op": "beginPairingOffer",
            "address": address,
        }))
        .unwrap();

    let offer = out.snapshot.pairing_offer.expect("pairing offer");
    assert_eq!(offer.qr_text, offer.text);
    assert!(offer.text.contains(address));
    assert!(offer.text.contains(&offer.code));
    assert!(!offer.code.is_empty());
    assert_ne!(offer.code, address);
    assert!(offer.qr_svg.contains("<svg"));
}

#[test]
fn pairing_with_the_one_time_code_issues_a_long_term_token() {
    let tmp = tempfile::tempdir().unwrap();
    let mut host = HostKernel::boot(boot_req(tmp.path())).unwrap();
    let offer = host
        .handle(serde_json::json!({
            "op": "beginPairingOffer",
            "address": "http://100.64.1.2:10529",
        }))
        .unwrap()
        .snapshot
        .pairing_offer
        .unwrap();

    let out = host
        .handle(serde_json::json!({
            "op": "redeemPairing",
            "code": offer.code,
            "clientName": "Studio laptop",
        }))
        .unwrap();

    let pairing = out.pairing.expect("issued pairing");
    assert!(!pairing.token.is_empty());
    assert_ne!(pairing.token, offer.code);
    assert_eq!(pairing.display_name, "Studio");
    assert!(!pairing.host_id.is_empty());
    assert_ne!(pairing.host_id, "local");
    assert!(out.snapshot.pairing_offer.is_none());
    assert_eq!(out.snapshot.paired_clients.len(), 1);
    assert_eq!(out.snapshot.paired_clients[0].name, "Studio laptop");
    assert!(!out.snapshot.paired_clients[0].id.is_empty());
    let dump = serde_json::to_string(&out.snapshot).unwrap();
    assert!(!dump.contains(&pairing.token));
}

#[test]
fn wrong_pairing_code_does_not_issue_a_token() {
    let tmp = tempfile::tempdir().unwrap();
    let mut host = HostKernel::boot(boot_req(tmp.path())).unwrap();
    let offer = host
        .handle(serde_json::json!({
            "op": "beginPairingOffer",
            "address": "http://100.64.1.2:10529",
        }))
        .unwrap()
        .snapshot
        .pairing_offer
        .unwrap();

    let err = host
        .handle(serde_json::json!({
            "op": "redeemPairing",
            "code": "NOPE-NOPE",
            "clientName": "Studio laptop",
        }))
        .unwrap_err();

    assert!(err.to_string().contains("invalid pairing code"));
    let snap = host.snapshot();
    assert_eq!(
        snap.pairing_offer.as_ref().map(|value| value.code.as_str()),
        Some(offer.code.as_str())
    );
    assert!(snap.paired_clients.is_empty());
}

#[test]
fn used_pairing_code_cannot_be_reused() {
    let tmp = tempfile::tempdir().unwrap();
    let mut host = HostKernel::boot(boot_req(tmp.path())).unwrap();
    let offer = host
        .handle(serde_json::json!({
            "op": "beginPairingOffer",
            "address": "http://100.64.1.2:10529",
        }))
        .unwrap()
        .snapshot
        .pairing_offer
        .unwrap();
    host.handle(serde_json::json!({
        "op": "redeemPairing",
        "code": offer.code,
        "clientName": "Studio laptop",
    }))
    .unwrap();

    let err = host
        .handle(serde_json::json!({
            "op": "redeemPairing",
            "code": offer.code,
            "clientName": "Phone",
        }))
        .unwrap_err();

    assert!(err.to_string().contains("invalid pairing code"));
    assert_eq!(host.snapshot().paired_clients.len(), 1);
}

#[test]
fn remote_access_with_a_token_can_call_the_host() {
    let tmp = tempfile::tempdir().unwrap();
    let mut host = HostKernel::boot(boot_req(tmp.path())).unwrap();
    let offer = host
        .handle(serde_json::json!({
            "op": "beginPairingOffer",
            "address": "http://100.64.1.2:10529",
        }))
        .unwrap()
        .snapshot
        .pairing_offer
        .unwrap();
    let token = host
        .handle(serde_json::json!({
            "op": "redeemPairing",
            "code": offer.code,
            "clientName": "Studio laptop",
        }))
        .unwrap()
        .pairing
        .unwrap()
        .token;
    let kernel = Arc::new(Mutex::new(host));
    let (listener, url) = bind_local_rpc(0).unwrap();
    let addr: SocketAddr = url.trim_start_matches("http://").parse().unwrap();
    spawn_local_rpc(listener, kernel, |_| {});

    let (status, body) = http_rpc(
        addr,
        "http://100.64.1.2:10529",
        "100.64.1.2:10529",
        Some(&token),
        r#"{"op":"snapshot"}"#,
    );
    assert_eq!(status, 200);
    let value: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(value["snapshot"]["running"], true);
    assert_eq!(
        value["snapshot"]["pairedClients"].as_array().unwrap().len(),
        1
    );

    let request = format!(
        "GET / HTTP/1.1\r\nHost: 100.64.1.2:10529\r\nOrigin: http://100.64.1.2:10529\r\nAuthorization: Bearer {token}\r\nConnection: close\r\n\r\n"
    );
    let (status, headers, body) = http_exchange(addr, &request);
    assert_eq!(status, 200);
    assert!(headers.to_ascii_lowercase().contains("text/html"));
    assert!(body.contains("<div"));
}

#[test]
fn revoking_a_client_makes_its_token_unusable_immediately() {
    let tmp = tempfile::tempdir().unwrap();
    let mut host = HostKernel::boot(boot_req(tmp.path())).unwrap();
    let offer = host
        .handle(serde_json::json!({
            "op": "beginPairingOffer",
            "address": "http://100.64.1.2:10529",
        }))
        .unwrap()
        .snapshot
        .pairing_offer
        .unwrap();
    let issued = host
        .handle(serde_json::json!({
            "op": "redeemPairing",
            "code": offer.code,
            "clientName": "Studio laptop",
        }))
        .unwrap();
    let token = issued.pairing.unwrap().token;
    let client_id = issued.snapshot.paired_clients[0].id.clone();
    let kernel = Arc::new(Mutex::new(host));
    let (listener, url) = bind_local_rpc(0).unwrap();
    let addr: SocketAddr = url.trim_start_matches("http://").parse().unwrap();
    spawn_local_rpc(listener, Arc::clone(&kernel), |_| {});

    let (status, _) = http_rpc(
        addr,
        "http://100.64.1.2:10529",
        "100.64.1.2:10529",
        Some(&token),
        r#"{"op":"snapshot"}"#,
    );
    assert_eq!(status, 200);

    let revoked = kernel
        .lock()
        .unwrap()
        .handle(serde_json::json!({
            "op": "revokeClient",
            "clientId": client_id,
        }))
        .unwrap();
    assert!(revoked.snapshot.paired_clients.is_empty());

    let (status, body) = http_rpc(
        addr,
        "http://100.64.1.2:10529",
        "100.64.1.2:10529",
        Some(&token),
        r#"{"op":"snapshot"}"#,
    );
    assert_eq!(status, 403);
    assert!(body.contains("pairing required") || body.contains("invalid pairing"));
}

#[test]
fn a_client_window_can_switch_among_local_and_paired_hosts() {
    let host_dir = tempfile::tempdir().unwrap();
    let client_dir = tempfile::tempdir().unwrap();
    let mut host_req = boot_req(host_dir.path());
    host_req.host_display_name = "Mini".into();
    let host = Arc::new(Mutex::new(HostKernel::boot(host_req).unwrap()));
    let server = LoopbackServer::attach(Arc::clone(&host), 0, |_| {}).unwrap();
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
    let paired = client
        .handle(serde_json::json!({
            "op": "pairRemoteHost",
            "address": address,
            "code": code,
        }))
        .unwrap();

    assert_eq!(paired.snapshot.hosts.len(), 2);
    assert_eq!(paired.snapshot.focused_host_id, "local");
    let local = paired
        .snapshot
        .hosts
        .iter()
        .find(|item| item.local)
        .unwrap();
    let remote = paired
        .snapshot
        .hosts
        .iter()
        .find(|item| !item.local)
        .unwrap();
    assert_eq!(local.id, "local");
    assert_eq!(local.display_name, "Studio");
    assert_eq!(remote.display_name, "Mini");
    assert_ne!(remote.id, "local");

    let focused = client
        .handle(serde_json::json!({
            "op": "focusHost",
            "hostId": remote.id,
        }))
        .unwrap();
    assert_eq!(focused.snapshot.focused_host_id, remote.id);
    assert_eq!(focused.snapshot.hosts.len(), 2);
    assert_eq!(
        focused
            .snapshot
            .hosts
            .iter()
            .filter(|item| item.id == focused.snapshot.focused_host_id)
            .count(),
        1
    );
    assert_eq!(
        focused.snapshot.empty_actions,
        vec![
            EmptyAction::RegisterFirstProject,
            EmptyAction::PairAnotherHost
        ]
    );
    assert!(focused.snapshot.projects.is_empty());

    let remote_error = client
        .handle(serde_json::json!({
            "op": "registerProject",
            "name": "missing",
            "localPath": "/definitely/missing/remote-project",
            "repository": "you/missing",
        }))
        .unwrap_err();
    assert!(remote_error
        .to_string()
        .contains("local directory does not exist"));
    assert!(!remote_error.to_string().contains("not reachable"));

    let back = client
        .handle(serde_json::json!({
            "op": "focusHost",
            "hostId": "local",
        }))
        .unwrap();
    assert_eq!(back.snapshot.focused_host_id, "local");
    assert_eq!(back.snapshot.hosts.len(), 2);

    drop(client);
    assert!(host.lock().unwrap().snapshot().running);

    let mut client = HostKernel::boot(boot_req(client_dir.path())).unwrap();
    assert_eq!(client.snapshot().hosts.len(), 2);
    let err = client
        .handle(serde_json::json!({
            "op": "pairRemoteHost",
            "address": address,
            "code": "NOPE-NOPE",
        }))
        .unwrap_err();
    assert!(err.to_string().contains("invalid pairing code"));
    assert_eq!(client.snapshot().hosts.len(), 2);

    host.lock().unwrap().dispatch(Command::QuitHost).unwrap();
    let err = client
        .handle(serde_json::json!({
            "op": "focusHost",
            "hostId": remote.id,
        }))
        .unwrap_err();
    assert!(
        err.to_string().contains("not reachable")
            || err.to_string().contains("pairing failed")
            || err.to_string().contains("invalid pairing")
    );
}

#[test]
fn remote_client_projects_run_recovery_and_can_forward_the_retry() {
    let host_dir = tempfile::tempdir().unwrap();
    let client_dir = tempfile::tempdir().unwrap();
    let mut host_req = boot_req(host_dir.path());
    host_req.host_display_name = "Mini".into();
    let initial = HostKernel::boot(host_req.clone()).unwrap();
    let runs_path = initial.snapshot().data.host_dir.join("runs.json");
    drop(initial);
    std::fs::write(&runs_path, br#"[{"id":"truncated""#).unwrap();

    let host = Arc::new(Mutex::new(HostKernel::boot(host_req).unwrap()));
    let server = LoopbackServer::attach(Arc::clone(&host), 0, |_| {}).unwrap();
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
    let paired = client
        .handle(serde_json::json!({
            "op": "pairRemoteHost",
            "address": address,
            "code": code,
        }))
        .unwrap();
    let remote_id = paired
        .snapshot
        .hosts
        .iter()
        .find(|item| !item.local)
        .unwrap()
        .id
        .clone();
    let focused = client
        .handle(serde_json::json!({ "op": "focusHost", "hostId": remote_id }))
        .unwrap();
    let recovery = focused.snapshot.run_persistence_recovery.unwrap();
    assert_eq!(recovery.kind, RunPersistenceFailureKind::InvalidJson);
    assert!(!focused.snapshot.capabilities.run_persistence_writes);
    assert!(focused.snapshot.capabilities.run_persistence_recovery);

    let still_broken = client
        .handle(serde_json::json!({ "op": "retryRunPersistenceLoad" }))
        .unwrap();
    assert!(still_broken.snapshot.run_persistence_recovery.is_some());
    assert!(!still_broken.snapshot.capabilities.run_persistence_writes);

    std::fs::write(
        &runs_path,
        br#"[{"id":"run-remote","projectId":"project-remote","agentId":"codex","agentName":"Codex","unbound":true,"status":"ended"}]"#,
    )
    .unwrap();
    let recovered = client
        .handle(serde_json::json!({ "op": "retryRunPersistenceLoad" }))
        .unwrap();
    assert!(recovered.snapshot.run_persistence_recovery.is_none());
    assert!(recovered.snapshot.capabilities.run_persistence_writes);
    assert_eq!(recovered.snapshot.runs.len(), 1);
    assert_eq!(recovered.snapshot.runs[0].id, "run-remote");
}

#[test]
fn dependency_graph_centering_and_expansion_are_forwarded_to_a_remote_host() {
    let host_dir = tempfile::tempdir().unwrap();
    let client_dir = tempfile::tempdir().unwrap();
    let project_dir = host_dir.path().join("work/garden");
    std::fs::create_dir_all(&project_dir).unwrap();
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "center").blocking(
        "you/garden",
        2,
        "next",
    ));
    tracker.add_issue(IssueRecord::open("you/garden", 2, "next").blocking("you/garden", 3, "last"));
    tracker.add_issue(IssueRecord::open("you/garden", 3, "last"));
    let mut host_req = boot_req(host_dir.path());
    host_req.host_display_name = "Mini".into();
    let mut host_kernel = HostKernel::boot_with(host_req, tracker).unwrap();
    host_kernel
        .handle(serde_json::json!({
            "op": "registerProject",
            "name": "garden",
            "localPath": project_dir,
            "repository": "you/garden",
        }))
        .unwrap();
    let host = Arc::new(Mutex::new(host_kernel));
    let server = LoopbackServer::attach(Arc::clone(&host), 0, |_| {}).unwrap();
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
    let paired = client
        .handle(serde_json::json!({
            "op": "pairRemoteHost",
            "address": address,
            "code": code,
        }))
        .unwrap();
    let remote_id = paired
        .snapshot
        .hosts
        .iter()
        .find(|host| !host.local)
        .unwrap()
        .id
        .clone();
    client
        .handle(serde_json::json!({ "op": "focusHost", "hostId": remote_id }))
        .unwrap();
    client
        .handle(serde_json::json!({
            "op": "centerDependencyGraph",
            "issueId": "you/garden#1",
        }))
        .unwrap();
    let expanded = client
        .handle(serde_json::json!({
            "op": "setDependencyGraphComplete",
            "complete": true,
        }))
        .unwrap();
    let graph = expanded.snapshot.board.unwrap().graph.unwrap();
    assert_eq!(graph.center_id.as_deref(), Some("you/garden#1"));
    assert!(graph.complete);
    assert_eq!(graph.total_count, 3);
    assert_eq!(graph.nodes.len(), 3);

    let overview = client
        .handle(serde_json::json!({
            "op": "setCenterView",
            "view": "graph",
        }))
        .unwrap();
    assert_eq!(
        overview.snapshot.center_view,
        host_kernel::CenterView::Graph
    );
    let graph = overview.snapshot.board.unwrap().graph.unwrap();
    assert_eq!(graph.mode, host_kernel::DependencyGraphMode::Overview);
    assert_eq!(graph.center_id, None);
    assert!(!graph.complete);
    assert_eq!(graph.nodes.len(), 3);
}

#[test]
fn remote_host_snapshots_honor_each_clients_explicit_issue_navigation() {
    let host_dir = tempfile::tempdir().unwrap();
    let client_dir = tempfile::tempdir().unwrap();
    let project_dir = host_dir.path().join("work/garden");
    std::fs::create_dir_all(&project_dir).unwrap();
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "first remote issue"));
    tracker.add_issue(IssueRecord::open("you/garden", 2, "second remote issue"));
    let mut host_req = boot_req(host_dir.path());
    host_req.host_display_name = "Mini".into();
    let mut host_kernel = HostKernel::boot_with(host_req, tracker).unwrap();
    let _project_id = host_kernel
        .handle(serde_json::json!({
            "op": "registerProject",
            "name": "garden",
            "localPath": project_dir,
            "repository": "you/garden",
        }))
        .unwrap()
        .snapshot
        .focused_project_id;
    let host = Arc::new(Mutex::new(host_kernel));
    let server = LoopbackServer::attach(Arc::clone(&host), 0, |_| {}).unwrap();
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
    let paired = client
        .handle(serde_json::json!({
            "op": "pairRemoteHost",
            "address": address,
            "code": code,
        }))
        .unwrap();
    let remote_id = paired
        .snapshot
        .hosts
        .iter()
        .find(|host| !host.local)
        .unwrap()
        .id
        .clone();

    let snapshot_for = |client: &mut HostKernel, client_id: &str, issue_id: &str| {
        client
            .handle(serde_json::json!({
                "op": "focusHost",
                "hostId": remote_id,
                "clientInstanceId": client_id,
            }))
            .unwrap();
        client
            .handle(serde_json::json!({
                "op": "focusIssue",
                "issueId": issue_id,
                "clientInstanceId": client_id,
            }))
            .unwrap()
            .snapshot
    };

    let first = snapshot_for(&mut client, "client-a", "you/garden#1");
    assert_eq!(first.focused_host_id, remote_id);
    assert_eq!(
        first.board.as_ref().unwrap().selected.as_ref().unwrap().id,
        "you/garden#1"
    );

    let second = snapshot_for(&mut client, "client-b", "you/garden#2");
    assert_eq!(
        second.board.as_ref().unwrap().selected.as_ref().unwrap().id,
        "you/garden#2"
    );

    let first_again = client
        .handle(serde_json::json!({
            "op": "snapshot",
            "clientInstanceId": "client-a",
        }))
        .unwrap()
        .snapshot;
    assert_eq!(
        first_again
            .board
            .as_ref()
            .unwrap()
            .selected
            .as_ref()
            .unwrap()
            .id,
        "you/garden#1"
    );
}

#[test]
fn closing_the_client_does_not_stop_the_remote_host() {
    let host_dir = tempfile::tempdir().unwrap();
    let client_dir = tempfile::tempdir().unwrap();
    let mut host_req = boot_req(host_dir.path());
    host_req.host_display_name = "Mini".into();
    let host = Arc::new(Mutex::new(HostKernel::boot(host_req).unwrap()));
    let server = LoopbackServer::attach(Arc::clone(&host), 0, |_| {}).unwrap();
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
    client
        .handle(serde_json::json!({
            "op": "pairRemoteHost",
            "address": address,
            "code": code,
        }))
        .unwrap();
    client.dispatch(Command::HideWindow).unwrap();
    drop(client);

    let snap = host.lock().unwrap().snapshot();
    assert!(snap.running);
    assert_eq!(snap.paired_clients.len(), 1);
}
