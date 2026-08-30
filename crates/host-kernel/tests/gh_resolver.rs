use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use host_kernel::{
    gh_known_install_locations, probe_binary, resolve_gh, AuthFailureKind, BootRequest,
    CredentialSource, GitHubTracker, HostKernel, KernelPorts, LaunchEnvPort, LaunchEnvironment,
    MemoryLaunchEnv, ProbeContext, ProbeOutcome, ProbeResult, ProjectConnection, ScriptedGitHub,
    SystemAppearance, TrackerPort,
};

fn make_unix_gh(dir: &Path, token: &str) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join("gh");
    let body = format!(
        "#!/bin/sh\ncase \"$1\" in\n--version) echo gh; exit 0;;\nauth) echo {token}; exit 0;;\nesac\nexit 1\n"
    );
    std::fs::write(&path, body).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    path
}

fn env_with_path(cwd: &Path, path: &Path) -> LaunchEnvironment {
    LaunchEnvironment::from_vars(
        cwd.to_path_buf(),
        BTreeMap::from([("PATH".into(), path.to_string_lossy().into_owned())]),
    )
}

fn probe_ctx<'a>(secrets_path: &'a Path) -> ProbeContext<'a> {
    ProbeContext {
        tracker: host_kernel::TrackerKind::Github,
        github_host: "github.com",
        repository: "you/garden",
        secrets_pat: None,
        secrets_path,
    }
}

fn boot_req(root: &Path) -> BootRequest {
    BootRequest {
        app_local_data_dir: root.to_path_buf(),
        app_log_dir: root.join("logs"),
        system_locale: "zh-Hans-CN".into(),
        system_appearance: SystemAppearance::Light,
        host_display_name: "Studio".into(),
    }
}

fn test_ports(
    tracker: Arc<dyn host_kernel::TrackerSeam>,
    launch_env: Arc<dyn LaunchEnvPort>,
) -> KernelPorts {
    KernelPorts {
        tracker,
        agents: vec![Arc::new(host_kernel::MemoryAgent::installed_grok())],
        launch_env,
        sessions: host_kernel::MemorySessionFactory::new(),
    }
}

fn register_garden(host: &mut HostKernel, dir: &Path) -> ProjectConnection {
    host.handle(serde_json::json!({
        "op": "registerProject",
        "name": "garden",
        "localPath": dir,
        "repository": "you/garden",
    }))
    .unwrap()
    .snapshot
    .projects
    .pop()
    .unwrap()
    .connection
}

#[test]
fn gh_known_install_locations_include_homebrew_and_windows_cli_dirs() {
    let rendered: Vec<_> = gh_known_install_locations()
        .iter()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .collect();
    assert!(
        rendered
            .iter()
            .any(|path| path.ends_with("/opt/homebrew/bin")),
        "{rendered:?}"
    );
    assert!(
        rendered.iter().any(|path| path.ends_with("/usr/local/bin")),
        "{rendered:?}"
    );
    assert!(
        rendered
            .iter()
            .any(|path| path.contains("Program Files") && path.contains("GitHub CLI")),
        "{rendered:?}"
    );
    assert!(
        rendered
            .iter()
            .any(|path| path.contains("AppData/Local/Programs") && path.contains("GitHub CLI")),
        "{rendered:?}"
    );
}

#[test]
fn probe_binary_finds_gh_on_macos_homebrew_layout() {
    let tmp = tempfile::tempdir().unwrap();
    let brew = tmp.path().join("opt/homebrew/bin");
    let gh = make_unix_gh(&brew, "unused");
    let env = env_with_path(tmp.path(), &tmp.path().join("empty-path"));
    match probe_binary("gh", &env, &[brew]) {
        ProbeResult::Found { executable } => assert_eq!(executable, gh),
        other => panic!("expected found, got {other:?}"),
    }
}

#[test]
fn probe_binary_finds_windows_gh_exe() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("GitHub CLI");
    std::fs::create_dir_all(&dir).unwrap();
    let exe = dir.join("gh.exe");
    std::fs::write(&exe, b"MZ").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let env = env_with_path(tmp.path(), &tmp.path().join("empty-path"));
    match probe_binary("gh", &env, &[dir]) {
        ProbeResult::Found { executable } => assert_eq!(executable, exe),
        other => panic!("expected found, got {other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn resolve_gh_uses_launch_env_path() {
    let tmp = tempfile::tempdir().unwrap();
    let path_dir = tmp.path().join("bin");
    let gh = make_unix_gh(&path_dir, "cli-token");
    let launch = MemoryLaunchEnv::new();
    launch.set(
        tmp.path().to_path_buf(),
        env_with_path(tmp.path(), &path_dir),
    );
    let resolved = resolve_gh(Arc::new(launch), tmp.path(), &[]).expect("resolved");
    assert_eq!(resolved, gh);
}

#[test]
fn system_path_still_finds_real_gh_via_known_locations() {
    let launch = MemoryLaunchEnv::with_path("/usr/bin:/bin");
    let known = gh_known_install_locations();
    let expected = known.iter().find_map(|dir| {
        ["gh", "gh.exe"]
            .into_iter()
            .map(|name| dir.join(name))
            .find(|path| path.is_file())
    });
    let Some(expected) = expected else {
        return;
    };
    let resolved = resolve_gh(Arc::new(launch), Path::new("/"), &known).expect("resolved");
    assert_eq!(resolved, expected);
}

#[cfg(unix)]
#[test]
fn resolve_gh_uses_known_location_when_capture_fails() {
    struct FailEnv;
    impl LaunchEnvPort for FailEnv {
        fn capture(&self, _cwd: &Path) -> Result<LaunchEnvironment, String> {
            Err("capture failed".into())
        }
    }
    let tmp = tempfile::tempdir().unwrap();
    let known = tmp.path().join("opt/homebrew/bin");
    let gh = make_unix_gh(&known, "cli-token");
    let resolved = resolve_gh(Arc::new(FailEnv), tmp.path(), &[known]).expect("resolved");
    assert_eq!(resolved, gh);
}

#[test]
fn live_tracker_detects_homebrew_gh_when_process_path_is_system_only() {
    let brew = PathBuf::from("/opt/homebrew/bin/gh");
    if !brew.is_file() {
        return;
    }
    let launch = MemoryLaunchEnv::with_path("/usr/bin:/bin");
    let tracker = GitHubTracker::live_with_script(
        Arc::new(launch),
        PathBuf::from("/"),
        gh_known_install_locations(),
        ScriptedGitHub::default(),
    );
    match tracker.probe(&probe_ctx(Path::new("/tmp/host-secrets.json"))) {
        ProbeOutcome::Ready {
            source: CredentialSource::Cli,
        }
        | ProbeOutcome::Failed {
            cli_detected: true, ..
        } => {}
        other => panic!("expected Homebrew gh to be detected, got {other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn live_tracker_reads_token_from_resolved_gh_when_process_path_is_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let path_dir = tmp.path().join("bin");
    make_unix_gh(&path_dir, "cli-token");
    let launch = MemoryLaunchEnv::new();
    launch.set(
        tmp.path().to_path_buf(),
        env_with_path(tmp.path(), &path_dir),
    );
    let tracker = GitHubTracker::live_with_script(
        Arc::new(launch),
        tmp.path().to_path_buf(),
        vec![tmp.path().join("unused-known")],
        ScriptedGitHub {
            accept_tokens: ["cli-token".into()].into(),
            ..Default::default()
        },
    );
    match tracker.probe(&probe_ctx(&tmp.path().join("secrets.json"))) {
        ProbeOutcome::Ready {
            source: CredentialSource::Cli,
        } => {}
        other => panic!("expected cli token to be used, got {other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn live_tracker_does_not_see_gh_when_path_and_known_locations_are_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let launch = MemoryLaunchEnv::with_path(tmp.path().join("empty-path").to_str().unwrap());
    let tracker = GitHubTracker::live_with_script(
        Arc::new(launch),
        tmp.path().to_path_buf(),
        vec![tmp.path().join("missing")],
        ScriptedGitHub::default(),
    );
    match tracker.probe(&probe_ctx(&tmp.path().join("secrets.json"))) {
        ProbeOutcome::Failed {
            kind: AuthFailureKind::MissingCredentials,
            cli_detected: false,
            source: None,
            ..
        } => {}
        other => panic!("expected missing credentials without gh, got {other:?}"),
    }
}

#[test]
fn packaged_host_connects_with_secrets_pat_when_gh_is_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("work/garden");
    std::fs::create_dir_all(&dir).unwrap();
    let secrets = tmp.path().join("host").join("secrets.json");
    std::fs::create_dir_all(secrets.parent().unwrap()).unwrap();
    std::fs::write(
        &secrets,
        serde_json::json!({ "githubPats": { "github.com": "secret-token" } }).to_string(),
    )
    .unwrap();
    let launch = MemoryLaunchEnv::with_path(tmp.path().join("empty-path").to_str().unwrap());
    let tracker = GitHubTracker::scripted(ScriptedGitHub {
        gh_detected: false,
        accept_tokens: ["secret-token".into()].into(),
        ..Default::default()
    });
    let mut host = HostKernel::boot_with_ports(
        boot_req(tmp.path()),
        test_ports(Arc::new(tracker), Arc::new(launch)),
    )
    .unwrap();
    match register_garden(&mut host, &dir) {
        ProjectConnection::Ready {
            source: CredentialSource::SecretsFile,
        } => {}
        other => panic!("expected secrets connection, got {other:?}"),
    }
}

#[test]
fn packaged_host_reports_missing_gh_when_no_credentials_exist() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("work/garden");
    std::fs::create_dir_all(&dir).unwrap();
    let empty = tmp.path().join("empty-path").to_string_lossy().into_owned();
    let tracker = GitHubTracker::live_with_script(
        Arc::new(MemoryLaunchEnv::with_path(&empty)),
        tmp.path().to_path_buf(),
        vec![tmp.path().join("missing")],
        ScriptedGitHub::default(),
    );
    let mut host = HostKernel::boot_with_ports(
        boot_req(tmp.path()),
        test_ports(
            Arc::new(tracker),
            Arc::new(MemoryLaunchEnv::with_path(&empty)),
        ),
    )
    .unwrap();
    match register_garden(&mut host, &dir) {
        ProjectConnection::AuthFailed {
            kind: AuthFailureKind::MissingCredentials,
            repair,
            message,
            ..
        } => {
            assert!(!repair.cli_detected);
            assert!(
                message.contains("没有可用的 GitHub 凭据")
                    || message.contains("No GitHub credentials"),
                "{message}"
            );
            assert_eq!(
                host.snapshot().copy.no_gh_detected,
                "这台电脑上没检测到 gh。"
            );
        }
        other => panic!("expected missing credentials, got {other:?}"),
    }
}
