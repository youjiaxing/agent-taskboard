use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use host_kernel::{
    bind_local_rpc, local_client_origin_allowed, spawn_local_rpc, BootRequest, Command,
    EmptyAction, HostKernel, Language, ProcessIntent, SystemAppearance, Theme,
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
fn local_rpc_rejects_a_remote_origin() {
    let tmp = tempfile::tempdir().unwrap();
    let kernel = Arc::new(Mutex::new(HostKernel::boot(boot_req(tmp.path())).unwrap()));
    let (listener, url) = bind_local_rpc(0).unwrap();
    let addr: SocketAddr = url.trim_start_matches("http://").parse().unwrap();
    spawn_local_rpc(listener, kernel, |_| {});

    let (status, body) = http_post(addr, "https://evil.example", r#"{"op":"snapshot"}"#);
    assert_eq!(status, 403);
    assert!(body.contains("origin not allowed"));
}

fn http_post(addr: SocketAddr, origin: &str, body: &str) -> (u16, String) {
    let request = format!(
        "POST /rpc HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nOrigin: {origin}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        addr.port(),
        body.len()
    );
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
                return (status, body.to_string());
            }
            Err(err) => {
                last_err = Some(err);
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
    panic!("connect {addr} failed: {last_err:?}");
}
