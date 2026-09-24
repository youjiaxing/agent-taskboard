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
fn invalid_run_organization_metadata_falls_back_without_losing_the_run() {
    let tmp = tempfile::tempdir().unwrap();
    let host = HostKernel::boot(boot_req(tmp.path())).unwrap();
    let runs_path = host.snapshot().data.host_dir.join("runs.json");
    drop(host);
    std::fs::write(
        &runs_path,
        br#"[{"id":"run-1","projectId":"project-1","agentId":"codex","agentName":"Codex","unbound":true,"status":"ended","pinnedAtMs":"bad","archivedAtMs":-1}]"#,
    )
    .unwrap();

    let host = HostKernel::boot(boot_req(tmp.path())).unwrap();
    let snapshot = host.snapshot();

    assert!(snapshot.run_persistence_recovery.is_none());
    assert!(snapshot.capabilities.run_persistence_writes);
    assert_eq!(snapshot.runs.len(), 1);
    assert_eq!(snapshot.runs[0].id, "run-1");
    assert!(snapshot.runs[0].pinned_at_ms.is_none());
    assert!(snapshot.runs[0].archived_at_ms.is_none());
}

#[test]
fn unreliable_runs_files_are_preserved_and_exposed_as_recovery() {
    let cases: Vec<(&str, Vec<u8>, RunPersistenceFailureKind)> = vec![
        (
            "truncated-json",
            br#"[{"id":"run-1""#.to_vec(),
            RunPersistenceFailureKind::InvalidJson,
        ),
        (
            "invalid-base-field",
            br#"[{"id":7,"projectId":"project-1","agentId":"codex","agentName":"Codex","unbound":true,"status":"ended"}]"#.to_vec(),
            RunPersistenceFailureKind::InvalidRunRecord,
        ),
        (
            "unreadable-utf8",
            vec![0xff, 0xfe, 0xfd],
            RunPersistenceFailureKind::Unreadable,
        ),
    ];

    for (name, original, expected_kind) in cases {
        let tmp = tempfile::tempdir().unwrap();
        let host = HostKernel::boot(boot_req(tmp.path())).unwrap();
        let runs_path = host.snapshot().data.host_dir.join("runs.json");
        drop(host);
        std::fs::write(&runs_path, &original).unwrap();

        let host = HostKernel::boot(boot_req(tmp.path())).unwrap();
        let snapshot = host.snapshot();
        let recovery = snapshot
            .run_persistence_recovery
            .as_ref()
            .unwrap_or_else(|| panic!("{name}: recovery state missing"));

        assert_eq!(recovery.kind, expected_kind, "{name}");
        assert_eq!(
            recovery.retry_operation, "retryRunPersistenceLoad",
            "{name}"
        );
        assert!(recovery.writes_blocked, "{name}");
        assert!(!snapshot.capabilities.run_persistence_writes, "{name}");
        assert!(snapshot.capabilities.run_persistence_recovery, "{name}");
        assert!(snapshot.runs.is_empty(), "{name}");
        assert_eq!(std::fs::read(&runs_path).unwrap(), original, "{name}");
        assert!(!snapshot.copy.run_persistence_recovery_body.is_empty());
        assert!(!snapshot.copy.run_persistence_recovery_retry.is_empty());
        assert!(!snapshot.copy.run_persistence_write_failed.is_empty());
        assert!(!snapshot.copy_catalog[&Language::En]
            .run_persistence_recovery_body
            .is_empty());
    }
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
fn appearance_state_exposes_the_preference_catalog_without_legacy_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let host = HostKernel::boot(boot_req(tmp.path())).unwrap();
    let snap = host.snapshot();

    assert_eq!(snap.appearance.language, Language::ZhCn);
    assert_eq!(
        snap.appearance.appearance_preference,
        AppearancePreference::System
    );
    assert_eq!(
        snap.appearance.appearance_preferences,
        vec![
            AppearancePreference::System,
            AppearancePreference::Light,
            AppearancePreference::Dark,
            AppearancePreference::Warm,
        ]
    );

    let appearance = serde_json::to_value(&snap.appearance).unwrap();
    assert_eq!(appearance["appearancePreference"], "system");
    assert_eq!(
        appearance["appearancePreferences"],
        serde_json::json!(["system", "light", "dark", "warm"])
    );
    for legacy in ["theme", "lastLightTheme", "themes", "languages"] {
        assert!(
            appearance.get(legacy).is_none(),
            "legacy field {legacy} leaked"
        );
    }
}

#[test]
fn first_launch_defaults_to_system_regardless_of_system_appearance() {
    let tmp = tempfile::tempdir().unwrap();
    let mut req = boot_req(tmp.path());
    req.system_locale = "it-IT".into();
    req.system_appearance = SystemAppearance::Dark;

    let host = HostKernel::boot(req.clone()).unwrap();
    let snap = host.snapshot();
    assert_eq!(snap.appearance.language, Language::En);
    assert_eq!(
        snap.appearance.appearance_preference,
        AppearancePreference::System
    );
    let settings = std::fs::read_to_string(&snap.data.desktop_client_settings_path).unwrap();
    let settings: serde_json::Value = serde_json::from_str(&settings).unwrap();
    assert_eq!(settings["appearance_preference"], "system");
    drop(host);

    req.system_locale = "zh-CN".into();
    req.system_appearance = SystemAppearance::Light;
    let host = HostKernel::boot(req).unwrap();
    let snap = host.snapshot();
    assert_eq!(snap.appearance.language, Language::En);
    assert_eq!(
        snap.appearance.appearance_preference,
        AppearancePreference::System
    );
}

#[test]
fn chinese_locale_picks_simplified_chinese() {
    let tmp = tempfile::tempdir().unwrap();
    let host = HostKernel::boot(boot_req(tmp.path())).unwrap();
    let snap = host.snapshot();
    assert_eq!(snap.appearance.language, Language::ZhCn);
    assert_eq!(
        snap.appearance.appearance_preference,
        AppearancePreference::System
    );
    assert_eq!(snap.copy.quit_host, "退出 Host");
    assert_eq!(snap.copy.open_web_page, "打开网页");
}

#[test]
fn window_and_tray_share_the_client_language_and_appearance_preference() {
    let tmp = tempfile::tempdir().unwrap();
    let mut host = HostKernel::boot(boot_req(tmp.path())).unwrap();

    host.dispatch(Command::SetLanguage(Language::En)).unwrap();
    host.dispatch(Command::SetAppearancePreference(AppearancePreference::Warm))
        .unwrap();
    let snap = host.snapshot();
    assert_eq!(snap.appearance.language, Language::En);
    assert_eq!(
        snap.appearance.appearance_preference,
        AppearancePreference::Warm
    );
    assert_eq!(snap.copy.quit_host, "Quit Host");
    assert_eq!(snap.copy.show_window, "Open window");
    assert_eq!(snap.copy.open_web_page, "Open web page");
    assert_eq!(snap.copy.window_menu, "Window");
    assert_eq!(snap.copy.help_menu, "Help");
    assert_eq!(snap.copy.edit_undo, "Undo typing");
    assert_eq!(snap.copy.usage_guide, "User guide");
    drop(host);

    let host = HostKernel::boot(boot_req(tmp.path())).unwrap();
    let snap = host.snapshot();
    assert_eq!(snap.appearance.language, Language::En);
    assert_eq!(
        snap.appearance.appearance_preference,
        AppearancePreference::Warm
    );
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

#[test]
fn legacy_theme_settings_are_ignored_and_reset_to_system() {
    let tmp = tempfile::tempdir().unwrap();
    let host = HostKernel::boot(boot_req(tmp.path())).unwrap();
    let path = host.snapshot().data.desktop_client_settings_path.clone();
    drop(host);
    std::fs::write(
        &path,
        r#"{"language":"en","theme":"plain-night","lastLightTheme":"warm-paper"}"#,
    )
    .unwrap();

    let host = HostKernel::boot(boot_req(tmp.path())).unwrap();
    let snap = host.snapshot();
    assert_eq!(snap.appearance.language, Language::En);
    assert_eq!(
        snap.appearance.appearance_preference,
        AppearancePreference::System
    );
}

#[test]
#[cfg(unix)]
fn repeating_the_same_appearance_preference_does_not_persist_again() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
    let mut host = HostKernel::boot(boot_req(tmp.path())).unwrap();
    let client_dir = host.snapshot().data.desktop_client_dir.clone();

    host.dispatch(Command::SetAppearancePreference(AppearancePreference::Dark))
        .unwrap();

    let writable = std::fs::metadata(&client_dir).unwrap().permissions();
    let mut read_only = writable.clone();
    read_only.set_mode(0o555);
    std::fs::set_permissions(&client_dir, read_only).unwrap();

    // 同值写入是空操作，只读目录下也不会尝试落盘。
    host.dispatch(Command::SetAppearancePreference(AppearancePreference::Dark))
        .unwrap();
    assert_eq!(
        host.snapshot().appearance.appearance_preference,
        AppearancePreference::Dark
    );

    // 换值仍会落盘，只读目录让这次写入以错误返回。
    assert!(host
        .dispatch(Command::SetAppearancePreference(
            AppearancePreference::Light
        ))
        .is_err());

    std::fs::set_permissions(&client_dir, writable).unwrap();
}

#[test]
fn set_appearance_preference_op_replaces_set_theme() {
    let tmp = tempfile::tempdir().unwrap();
    let mut host = HostKernel::boot(boot_req(tmp.path())).unwrap();

    let out = host
        .handle(serde_json::json!({
            "op": "setAppearancePreference",
            "appearancePreference": "warm",
        }))
        .unwrap();
    assert_eq!(
        out.snapshot.appearance.appearance_preference,
        AppearancePreference::Warm
    );

    assert!(host
        .handle(serde_json::json!({ "op": "setTheme", "theme": "plain-paper" }))
        .is_err());
}
