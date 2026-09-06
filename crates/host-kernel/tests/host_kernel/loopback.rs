use super::*;

#[test]
fn client_only_cold_start_can_use_the_saved_remote_host() {
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
    drop(client);

    let mut client = HostKernel::boot_client_only(boot_req(client_dir.path())).unwrap();
    let snap = client.snapshot();
    assert_eq!(snap.host_mode, HostMode::ClientOnly);
    assert_eq!(snap.hosts.len(), 1);
    assert_eq!(snap.hosts[0].display_name, "Mini");
    assert_eq!(snap.focused_host_id, snap.hosts[0].id);
    assert!(snap.hosts.iter().all(|host| !host.local));
    assert_eq!(
        client
            .handle(serde_json::json!({ "op": "snapshot" }))
            .unwrap()
            .process,
        ProcessIntent::KeepRunning
    );
}

#[test]
fn client_only_cold_start_preserves_existing_host_data() {
    let tmp = tempfile::tempdir().unwrap();
    let host = HostKernel::boot(boot_req(tmp.path())).unwrap();
    let settings_path = host.snapshot().data.host_settings_path;
    let before = std::fs::read(&settings_path).unwrap();
    drop(host);

    let client = HostKernel::boot_client_only(boot_req(tmp.path())).unwrap();
    assert!(client.snapshot().projects.is_empty());
    drop(client);
    assert_eq!(std::fs::read(&settings_path).unwrap(), before);
}

#[test]
fn client_only_cold_start_has_no_local_host_or_loopback_page() {
    let tmp = tempfile::tempdir().unwrap();
    let host = HostKernel::boot_client_only(boot_req(tmp.path())).unwrap();
    let snap = host.snapshot();

    assert!(!snap.running);
    assert!(snap.window_visible);
    assert_eq!(snap.host_mode, HostMode::ClientOnly);
    assert!(snap.hosts.iter().all(|host| !host.local));
    assert!(snap.projects.is_empty());
    assert!(snap.runs.is_empty());
    assert!(snap.data.host_dir.is_dir());
    assert!(snap.data.desktop_client_dir.is_dir());

    let kernel = Arc::new(Mutex::new(host));
    let server = LoopbackServer::attach(Arc::clone(&kernel), 0, |_| {}).unwrap();
    assert_eq!(server.protocol_url(), "");
    assert!(matches!(
        kernel.lock().unwrap().snapshot().loopback_page,
        LoopbackPage::HostNotRunning { .. }
    ));
}

#[test]
fn client_only_desktop_transport_is_private_and_keeps_the_client_process_alive() {
    let tmp = tempfile::tempdir().unwrap();
    let kernel = Arc::new(Mutex::new(
        HostKernel::boot_client_only(boot_req(tmp.path())).unwrap(),
    ));
    let server = LoopbackServer::attach_client_transport(Arc::clone(&kernel), |_| {}).unwrap();
    let addr: SocketAddr = server
        .protocol_url()
        .trim_start_matches("http://")
        .parse()
        .unwrap();

    assert!(addr.ip().is_loopback());
    assert_ne!(addr.port(), LOCAL_RPC_PORT);
    let (status, body) = http_post(addr, "tauri://localhost", r#"{"op":"snapshot"}"#);
    assert_eq!(status, 200);
    let value: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(value["process"], "keep-running");
    assert_eq!(value["snapshot"]["running"], false);
    assert_eq!(value["snapshot"]["hostMode"], "client-only");
    let (status, _, _) = http_get(addr, Some("tauri://localhost"), "/");
    assert_eq!(status, 404);
}

#[test]
fn client_only_window_can_hide_and_reopen_without_starting_a_host() {
    let tmp = tempfile::tempdir().unwrap();
    let mut client = HostKernel::boot_client_only(boot_req(tmp.path())).unwrap();

    let hidden = client.dispatch(Command::HideWindow).unwrap();
    assert!(!hidden.snapshot.window_visible);
    assert_eq!(hidden.process, ProcessIntent::KeepRunning);
    let reopened = client.dispatch(Command::ShowWindow).unwrap();
    assert!(reopened.snapshot.window_visible);
    assert!(!reopened.snapshot.running);
    assert_eq!(reopened.snapshot.host_mode, HostMode::ClientOnly);
    assert_eq!(reopened.process, ProcessIntent::KeepRunning);
}

#[test]
fn empty_host_offers_register_and_pair_and_focuses_one_host() {
    let tmp = tempfile::tempdir().unwrap();
    let host = HostKernel::boot(boot_req(tmp.path())).unwrap();
    let snap = host.snapshot();

    assert!(snap.projects.is_empty());
    assert_eq!(snap.hosts.len(), 1);
    assert_eq!(snap.focused_host_id, "local");
    assert_eq!(snap.hosts[0].id, "local");
    assert!(snap.hosts[0].local);
    assert_eq!(snap.hosts[0].display_name, "Studio");
    assert_eq!(
        snap.empty_actions,
        vec![
            EmptyAction::RegisterFirstProject,
            EmptyAction::PairAnotherHost
        ]
    );
    assert_eq!(snap.copy.register_first_project, "登记第一个 Project");
    assert_eq!(snap.copy.pair_another_host, "配对另一个 Host");
}

#[test]
fn desktop_and_loopback_origins_can_call_the_local_host() {
    assert!(!local_client_origin_allowed(None));
    assert!(local_client_origin_allowed(Some("http://localhost:1420")));
    assert!(local_client_origin_allowed(Some("http://127.0.0.1:1420")));
    assert!(local_client_origin_allowed(Some("http://127.0.0.1:10529")));
    assert!(local_client_origin_allowed(Some("https://tauri.localhost")));
    assert!(local_client_origin_allowed(Some("http://tauri.localhost")));
    assert!(local_client_origin_allowed(Some("tauri://localhost")));
    assert!(!local_client_origin_allowed(Some("http://127.0.0.1:10528")));
    assert!(!local_client_origin_allowed(Some("http://localhost:10529")));
    assert!(!local_client_origin_allowed(Some("null")));
}

#[test]
fn remote_origins_cannot_call_the_local_host() {
    assert!(!local_client_origin_allowed(Some(
        "http://100.64.1.2:10529"
    )));
    assert!(!local_client_origin_allowed(Some(
        "http://192.168.1.8:10529"
    )));
    assert!(!local_client_origin_allowed(Some("https://evil.example")));
}

#[test]
fn local_rpc_answers_json_on_loopback() {
    let tmp = tempfile::tempdir().unwrap();
    let kernel = Arc::new(Mutex::new(HostKernel::boot(boot_req(tmp.path())).unwrap()));
    let (listener, url) = bind_local_rpc(0).unwrap();
    assert!(url.starts_with("http://127.0.0.1:"));
    let addr: SocketAddr = url.trim_start_matches("http://").parse().unwrap();
    spawn_local_rpc(listener, kernel, |_| {});

    let (status, body) = http_post(addr, "http://127.0.0.1:1420", r#"{"op":"snapshot"}"#);
    assert_eq!(status, 200);
    let value: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(value["process"], "keep-running");
    assert_eq!(value["snapshot"]["running"], true);
    assert_eq!(value["snapshot"]["copy"]["quitHost"], "退出 Host");

    let (status, _) = http_post(addr, "http://127.0.0.1:1421", r#"{"op":"snapshot"}"#);
    assert_eq!(status, 403);
    let (status, _) = http_post(addr, "http://localhost:1421", r#"{"op":"snapshot"}"#);
    assert_eq!(status, 403);
    let (status, _) = http_post_without_origin(addr, r#"{"op":"snapshot"}"#);
    assert_eq!(status, 403);
}

#[test]
fn loopback_page_is_served_without_pairing() {
    let tmp = tempfile::tempdir().unwrap();
    let kernel = Arc::new(Mutex::new(HostKernel::boot(boot_req(tmp.path())).unwrap()));
    let (listener, url) = bind_local_rpc(0).unwrap();
    let addr: SocketAddr = url.trim_start_matches("http://").parse().unwrap();
    spawn_local_rpc(listener, kernel.clone(), |_| {});

    let origin = format!("http://127.0.0.1:{}", addr.port());
    let (status, headers, body) = http_get(addr, Some(&origin), "/");
    assert_eq!(status, 200);
    assert!(headers.to_ascii_lowercase().contains("text/html"));
    assert!(!body.contains("配对码"));
    assert!(!body.to_ascii_lowercase().contains("pairing code"));

    let (status, rpc) = http_post(addr, &origin, r#"{"op":"snapshot"}"#);
    assert_eq!(status, 200);
    let value: serde_json::Value = serde_json::from_str(&rpc).unwrap();
    assert_eq!(value["snapshot"]["running"], true);
    assert_eq!(
        value["snapshot"]["emptyActions"],
        serde_json::json!(["register-first-project", "pair-another-host"])
    );
    assert_eq!(
        value["snapshot"]["copy"]["registerFirstProject"],
        "登记第一个 Project"
    );
}

#[test]
fn non_loopback_access_is_not_pairing_exempt() {
    let tmp = tempfile::tempdir().unwrap();
    let kernel = Arc::new(Mutex::new(HostKernel::boot(boot_req(tmp.path())).unwrap()));
    let (listener, url) = bind_local_rpc(0).unwrap();
    let addr: SocketAddr = url.trim_start_matches("http://").parse().unwrap();
    spawn_local_rpc(listener, kernel, |_| {});

    let (status, body) = http_post(addr, "https://evil.example", r#"{"op":"snapshot"}"#);
    assert_eq!(status, 403);
    assert!(body.contains("pairing required"));
    assert!(body.contains("长期令牌"));

    let (status, _, body) = http_get(addr, Some("http://100.64.1.2:10529"), "/");
    assert_eq!(status, 403);
    assert!(body.contains("pairing required"));
    assert!(body.contains("长期令牌"));
}

#[test]
fn loopback_page_port_is_10529() {
    assert_eq!(LOCAL_RPC_PORT, 10529);
}

#[test]
fn occupied_loopback_port_explains_and_keeps_desktop_protocol() {
    let occupier = TcpListener::bind("0.0.0.0:0").unwrap();
    let port = occupier.local_addr().unwrap().port();
    let tmp = tempfile::tempdir().unwrap();
    let kernel = Arc::new(Mutex::new(HostKernel::boot(boot_req(tmp.path())).unwrap()));
    let client = LoopbackServer::attach(Arc::clone(&kernel), port, |_| {}).unwrap();

    let snap = kernel.lock().unwrap().snapshot();
    match snap.loopback_page {
        LoopbackPage::Occupied { url, reason } => {
            assert_eq!(url, format!("http://127.0.0.1:{port}/"));
            assert!(reason.contains(&port.to_string()));
            assert!(reason.contains("占用"));
            assert!(reason.contains("桌面窗口"));
        }
        other => panic!("expected occupied, got {other:?}"),
    }
    assert!(snap.running);

    let protocol: SocketAddr = client
        .protocol_url()
        .trim_start_matches("http://")
        .parse()
        .unwrap();
    assert_ne!(protocol.port(), port);
    let (status, body) = http_post(protocol, "http://127.0.0.1:1420", r#"{"op":"snapshot"}"#);
    assert_eq!(status, 200);
    let value: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(value["snapshot"]["running"], true);
    assert_eq!(value["snapshot"]["loopbackPage"]["status"], "occupied");
}

#[test]
fn loopback_page_is_absent_when_host_is_not_running() {
    let tmp = tempfile::tempdir().unwrap();
    let mut host = HostKernel::boot(boot_req(tmp.path())).unwrap();
    host.dispatch(Command::QuitHost).unwrap();
    let kernel = Arc::new(Mutex::new(host));
    let client = LoopbackServer::attach(Arc::clone(&kernel), 0, |_| {}).unwrap();

    assert_eq!(client.protocol_url(), "");
    let page = kernel.lock().unwrap().snapshot().loopback_page;
    match page {
        LoopbackPage::HostNotRunning { url, reason } => {
            assert_eq!(url, "http://127.0.0.1:10529/");
            assert!(reason.contains("没有这份回环页") || reason.contains("没有在跑"));
        }
        other => panic!("expected host-not-running, got {other:?}"),
    }
}

#[test]
fn quitting_the_host_stops_the_loopback_page() {
    let tmp = tempfile::tempdir().unwrap();
    let kernel = Arc::new(Mutex::new(HostKernel::boot(boot_req(tmp.path())).unwrap()));
    let client = LoopbackServer::attach(Arc::clone(&kernel), 0, |_| {}).unwrap();
    let addr: SocketAddr = client
        .protocol_url()
        .trim_start_matches("http://")
        .parse()
        .unwrap();

    let (status, _, _) = http_get(
        addr,
        Some(&format!("http://127.0.0.1:{}", addr.port())),
        "/",
    );
    assert_eq!(status, 200);

    let (status, body) = http_post(
        addr,
        &format!("http://127.0.0.1:{}", addr.port()),
        r#"{"op":"quitHost"}"#,
    );
    assert_eq!(status, 200);
    let value: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(value["process"], "exit");
    assert_eq!(
        value["snapshot"]["loopbackPage"]["status"],
        "host-not-running"
    );

    let mut last_ok = true;
    for _ in 0..50 {
        match TcpStream::connect_timeout(&addr, Duration::from_millis(50)) {
            Ok(_) => {
                last_ok = true;
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => {
                last_ok = false;
                break;
            }
        }
    }
    assert!(
        !last_ok,
        "loopback page should be gone after the Host stops"
    );
}

#[test]
fn loopback_page_serves_the_client_shell_files() {
    let tmp = tempfile::tempdir().unwrap();
    let web = tmp.path().join("web");
    std::fs::create_dir(&web).unwrap();
    std::fs::write(
        web.join("index.html"),
        "<!doctype html><title>same-empty-shell</title><div id=\"app\"></div>",
    )
    .unwrap();
    std::fs::create_dir(web.join("assets")).unwrap();
    std::fs::write(web.join("assets").join("shell.css"), "/* shell */").unwrap();

    let kernel = Arc::new(Mutex::new(HostKernel::boot(boot_req(tmp.path())).unwrap()));
    let client = LoopbackServer::attach_with(
        Arc::clone(&kernel),
        0,
        LoopbackAssets::Directory(web),
        |_| {},
    )
    .unwrap();
    let addr: SocketAddr = client
        .protocol_url()
        .trim_start_matches("http://")
        .parse()
        .unwrap();
    let origin = format!("http://127.0.0.1:{}", addr.port());

    let (status, headers, body) = http_get(addr, Some(&origin), "/");
    assert_eq!(status, 200);
    assert!(headers.to_ascii_lowercase().contains("text/html"));
    assert!(body.contains("same-empty-shell"));
    assert!(body.contains("id=\"app\""));

    let (status, headers, body) = http_get(addr, Some(&origin), "/assets/shell.css");
    assert_eq!(status, 200);
    assert!(headers.to_ascii_lowercase().contains("text/css"));
    assert!(body.contains("shell"));

    let (status, _, body) = http_get(addr, Some(&origin), "/../secrets.json");
    assert_eq!(status, 404);
    assert!(!body.contains("same-empty-shell"));
}

#[test]
fn loopback_page_can_proxy_the_dev_client() {
    let upstream = TcpListener::bind("127.0.0.1:0").unwrap();
    let up_addr = upstream.local_addr().unwrap();
    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = upstream.accept() {
            stream
                .set_read_timeout(Some(Duration::from_millis(100)))
                .unwrap();
            let mut request = Vec::new();
            let mut buf = [0u8; 2048];
            while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                let read = stream.read(&mut buf).unwrap();
                request.extend_from_slice(&buf[..read]);
            }
            let mut extra = [0u8; 1];
            if matches!(stream.read(&mut extra), Ok(0)) {
                return;
            }
            let body = b"<title>dev-empty-shell</title>";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.write_all(body);
        }
    });

    let tmp = tempfile::tempdir().unwrap();
    let kernel = Arc::new(Mutex::new(HostKernel::boot(boot_req(tmp.path())).unwrap()));
    let client = LoopbackServer::attach_with(
        Arc::clone(&kernel),
        0,
        LoopbackAssets::DevProxy {
            origin: format!("http://127.0.0.1:{}", up_addr.port()),
        },
        |_| {},
    )
    .unwrap();
    let addr: SocketAddr = client
        .protocol_url()
        .trim_start_matches("http://")
        .parse()
        .unwrap();
    let (status, _, body) = http_get(
        addr,
        Some(&format!("http://127.0.0.1:{}", addr.port())),
        "/",
    );
    assert_eq!(status, 200);
    assert!(body.contains("dev-empty-shell"));
}
