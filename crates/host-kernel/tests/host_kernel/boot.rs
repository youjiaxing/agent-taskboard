use super::*;

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
fn host_autonomous_tick_does_not_drive_shell_callbacks() {
    let tmp = tempfile::tempdir().unwrap();
    let kernel = Arc::new(Mutex::new(HostKernel::boot(boot_req(tmp.path())).unwrap()));
    let calls = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&calls);
    let server = LoopbackServer::attach(Arc::clone(&kernel), 0, move |_| {
        seen.fetch_add(1, Ordering::SeqCst);
    })
    .unwrap();
    std::thread::sleep(Duration::from_millis(1500));
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "Host tick must not rebuild the desktop shell from a background thread"
    );
    drop(server);
}
