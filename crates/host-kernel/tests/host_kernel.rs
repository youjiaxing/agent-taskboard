use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use host_kernel::{
    bind_local_rpc, local_client_origin_allowed, spawn_local_rpc, BootRequest, Command,
    EmptyAction, HostKernel, HostMode, Language, LoopbackAssets, LoopbackPage, LoopbackServer,
    ProcessIntent, SystemAppearance, Theme, LOCAL_RPC_PORT,
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

#[test]
fn opening_the_desktop_app_starts_the_local_host() {
    let tmp = tempfile::tempdir().unwrap();
    let host = HostKernel::boot(boot_req(tmp.path())).unwrap();
    let snap = host.snapshot();
    assert!(snap.running);
    assert!(snap.window_visible);
}

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
fn hiding_the_window_does_not_stop_the_host() {
    let tmp = tempfile::tempdir().unwrap();
    let mut host = HostKernel::boot(boot_req(tmp.path())).unwrap();

    let out = host.dispatch(Command::HideWindow).unwrap();

    assert!(out.snapshot.running);
    assert!(!out.snapshot.window_visible);
    assert_eq!(out.process, ProcessIntent::KeepRunning);
}

#[test]
fn only_quit_host_stops_the_process() {
    let tmp = tempfile::tempdir().unwrap();
    let mut host = HostKernel::boot(boot_req(tmp.path())).unwrap();
    host.dispatch(Command::HideWindow).unwrap();

    let out = host.dispatch(Command::QuitHost).unwrap();

    assert!(!out.snapshot.running);
    assert_eq!(out.process, ProcessIntent::Exit);
}

#[test]
fn host_data_and_desktop_client_settings_are_two_trees() {
    let tmp = tempfile::tempdir().unwrap();
    let host = HostKernel::boot(boot_req(tmp.path())).unwrap();
    let snap = host.snapshot();

    assert_eq!(snap.data.host_dir, tmp.path().join("host"));
    assert_eq!(
        snap.data.desktop_client_dir,
        tmp.path().join("desktop-client")
    );
    assert!(snap.data.host_dir.is_dir());
    assert!(snap.data.desktop_client_dir.is_dir());
    assert!(snap.data.log_dir.is_dir());
    assert!(snap.data.host_settings_path.is_file());
    assert!(snap.data.desktop_client_settings_path.is_file());
    assert_ne!(
        snap.data.host_settings_path,
        snap.data.desktop_client_settings_path
    );
}

#[test]
fn secrets_are_user_readable_json_not_keychain() {
    let tmp = tempfile::tempdir().unwrap();
    let host = HostKernel::boot(boot_req(tmp.path())).unwrap();
    let snap = host.snapshot();

    let secrets = std::fs::read_to_string(&snap.data.host_secrets_path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&secrets).unwrap();
    assert_eq!(value, serde_json::json!({}));
    assert!(snap.data.host_secrets_path.starts_with(&snap.data.host_dir));
    assert!(snap
        .data
        .host_secrets_path
        .ends_with(Path::new("host/secrets.json")));
    assert!(!snap
        .data
        .host_secrets_path
        .starts_with(&snap.data.desktop_client_dir));

    let settings = std::fs::read_to_string(&snap.data.host_settings_path).unwrap();
    let settings_value: serde_json::Value = serde_json::from_str(&settings).unwrap();
    assert!(settings_value.is_object());
    assert_ne!(snap.data.host_secrets_path, snap.data.host_settings_path);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&snap.data.host_secrets_path)
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[cfg(windows)]
    {
        let output = std::process::Command::new("icacls")
            .arg(&snap.data.host_secrets_path)
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
        assert!(!text.contains("everyone"));
        assert!(!text.contains("builtin\\users"));
    }
}

#[test]
fn language_and_theme_catalogs_have_no_follow_system() {
    let tmp = tempfile::tempdir().unwrap();
    let host = HostKernel::boot(boot_req(tmp.path())).unwrap();
    let snap = host.snapshot();

    assert_eq!(
        snap.appearance.languages,
        vec![Language::ZhCn, Language::En]
    );
    assert_eq!(
        snap.appearance.themes,
        vec![Theme::WarmPaper, Theme::PlainPaper, Theme::PlainNight]
    );
    let appearance = serde_json::to_value(&snap.appearance).unwrap();
    assert!(appearance.get("followSystem").is_none());
    assert!(appearance.get("follow_system").is_none());
    let dump = format!(
        "{}{}",
        appearance,
        serde_json::to_value(&snap.copy).unwrap()
    )
    .to_ascii_lowercase();
    assert!(!dump.contains("follow"));
}

#[test]
fn first_launch_matches_system_then_writes_concrete_values() {
    let tmp = tempfile::tempdir().unwrap();
    let mut req = boot_req(tmp.path());
    req.system_locale = "it-IT".into();
    req.system_appearance = SystemAppearance::Dark;

    let host = HostKernel::boot(req.clone()).unwrap();
    let snap = host.snapshot();
    assert_eq!(snap.appearance.language, Language::En);
    assert_eq!(snap.appearance.theme, Theme::PlainNight);
    drop(host);

    req.system_locale = "zh-CN".into();
    req.system_appearance = SystemAppearance::Light;
    let host = HostKernel::boot(req).unwrap();
    let snap = host.snapshot();
    assert_eq!(snap.appearance.language, Language::En);
    assert_eq!(snap.appearance.theme, Theme::PlainNight);
}

#[test]
fn chinese_locale_picks_simplified_chinese_and_light_picks_warm_paper() {
    let tmp = tempfile::tempdir().unwrap();
    let host = HostKernel::boot(boot_req(tmp.path())).unwrap();
    let snap = host.snapshot();
    assert_eq!(snap.appearance.language, Language::ZhCn);
    assert_eq!(snap.appearance.theme, Theme::WarmPaper);
    assert_eq!(snap.copy.quit_host, "退出 Host");
}

#[test]
fn window_and_tray_share_the_client_language_and_theme() {
    let tmp = tempfile::tempdir().unwrap();
    let mut host = HostKernel::boot(boot_req(tmp.path())).unwrap();

    host.dispatch(Command::SetLanguage(Language::En)).unwrap();
    host.dispatch(Command::SetTheme(Theme::PlainPaper)).unwrap();
    let snap = host.snapshot();
    assert_eq!(snap.appearance.language, Language::En);
    assert_eq!(snap.appearance.theme, Theme::PlainPaper);
    assert_eq!(snap.copy.quit_host, "Quit Host");
    assert_eq!(snap.copy.show_window, "Open window");
    drop(host);

    let host = HostKernel::boot(boot_req(tmp.path())).unwrap();
    let snap = host.snapshot();
    assert_eq!(snap.appearance.language, Language::En);
    assert_eq!(snap.appearance.theme, Theme::PlainPaper);
    assert_eq!(snap.copy.quit_host, "Quit Host");
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
    assert!(local_client_origin_allowed(None));
    assert!(local_client_origin_allowed(Some("http://localhost:1420")));
    assert!(local_client_origin_allowed(Some("http://127.0.0.1:10529")));
    assert!(local_client_origin_allowed(Some("https://tauri.localhost")));
    assert!(local_client_origin_allowed(Some("http://tauri.localhost")));
    assert!(local_client_origin_allowed(Some("tauri://localhost")));
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
fn closing_a_browser_client_does_not_stop_the_host() {
    let tmp = tempfile::tempdir().unwrap();
    let kernel = Arc::new(Mutex::new(HostKernel::boot(boot_req(tmp.path())).unwrap()));
    let client = LoopbackServer::attach(Arc::clone(&kernel), 0, |_| {}).unwrap();
    let addr: SocketAddr = client
        .protocol_url()
        .trim_start_matches("http://")
        .parse()
        .unwrap();
    let origin = format!("http://127.0.0.1:{}", addr.port());

    let (status, _, _) = http_get(addr, Some(&origin), "/");
    assert_eq!(status, 200);
    let (status, body) = http_post(addr, &origin, r#"{"op":"snapshot"}"#);
    assert_eq!(status, 200);
    let value: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(value["snapshot"]["running"], true);

    let (status, _, _) = http_get(addr, Some(&origin), "/");
    assert_eq!(status, 200);
    assert!(kernel.lock().unwrap().snapshot().running);
    assert!(matches!(
        kernel.lock().unwrap().snapshot().loopback_page,
        LoopbackPage::Serving { .. }
    ));
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
            let mut buf = [0u8; 2048];
            let _ = stream.read(&mut buf);
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

fn http_get(addr: SocketAddr, origin: Option<&str>, path: &str) -> (u16, String, String) {
    let origin_header = origin
        .map(|origin| format!("Origin: {origin}\r\n"))
        .unwrap_or_default();
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n{origin_header}Connection: close\r\n\r\n",
        addr.port()
    );
    let (status, headers, body) = http_exchange(addr, &request);
    (status, headers, body)
}

fn http_post(addr: SocketAddr, origin: &str, body: &str) -> (u16, String) {
    http_rpc(
        addr,
        origin,
        &format!("127.0.0.1:{}", addr.port()),
        None,
        body,
    )
}

fn http_rpc(
    addr: SocketAddr,
    origin: &str,
    host: &str,
    token: Option<&str>,
    body: &str,
) -> (u16, String) {
    let auth = token
        .map(|token| format!("Authorization: Bearer {token}\r\n"))
        .unwrap_or_default();
    let request = format!(
        "POST /rpc HTTP/1.1\r\nHost: {host}\r\nOrigin: {origin}\r\n{auth}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let (status, _, body) = http_exchange(addr, &request);
    (status, body)
}

fn http_exchange(addr: SocketAddr, request: &str) -> (u16, String, String) {
    let mut last_err = None;
    for _ in 0..50 {
        match TcpStream::connect_timeout(&addr, Duration::from_millis(200)) {
            Ok(mut stream) => {
                stream.write_all(request.as_bytes()).unwrap();
                let _ = stream.shutdown(std::net::Shutdown::Write);
                let mut buf = String::new();
                stream.read_to_string(&mut buf).unwrap();
                let (head, body) = buf.split_once("\r\n\r\n").unwrap_or((buf.as_str(), ""));
                let status = head
                    .lines()
                    .next()
                    .unwrap_or("")
                    .split_whitespace()
                    .nth(1)
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(0);
                return (status, head.to_string(), body.to_string());
            }
            Err(err) => {
                last_err = Some(err);
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
    panic!("connect {addr} failed: {last_err:?}");
}
