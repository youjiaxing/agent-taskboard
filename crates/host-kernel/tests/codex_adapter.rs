use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use host_kernel::{
    AgentFieldKind, AgentPort, AgentSession, BootRequest, CodexAdapter, HostKernel, KernelPorts,
    Language, LaunchEnvironment, MemoryLaunchEnv, MemorySessionFactory, MemoryTracker,
    SystemAppearance, CODEX_BIN, CODEX_ID, CODEX_NAME,
};

fn make_discoverable_codex(dir: &std::path::Path) -> PathBuf {
    let path = dir.join("codex");
    std::fs::write(
        &path,
        r#"#!/bin/sh
if [ "$1" = "--help" ]; then
  printf '%s\n' '  -s, --sandbox <SANDBOX_MODE>' '          [possible values: read-only, workspace-write, danger-full-access]' '  -a, --ask-for-approval <APPROVAL_POLICY>' '          Possible values:' '          - untrusted:' '          - on-request:' '          - never:'
  exit 0
fi
if [ "$1" = "app-server" ]; then
  read initialize
  printf '%s\n' '{"id":0,"result":{"userAgent":"fake"}}'
  read initialized
  case "$initialized" in
    *'"method":"initialized"'*) ;;
    *) exit 3 ;;
  esac
  read models
  printf '%s\n' '{"id":1,"result":{"data":[{"id":"gpt-fast","model":"gpt-fast","isDefault":true,"defaultReasoningEffort":"low","supportedReasoningEfforts":[{"reasoningEffort":"low"},{"reasoningEffort":"medium"}]},{"id":"gpt-deep","model":"gpt-deep","isDefault":false,"defaultReasoningEffort":"high","supportedReasoningEfforts":[{"reasoningEffort":"high"},{"reasoningEffort":"xhigh"},{"reasoningEffort":"max"}]}],"nextCursor":null}}'
  exit 0
fi
exit 2
"#,
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    path
}

#[test]
fn codex_adapter_declares_interactive_tui_contract() {
    let adapter = CodexAdapter;
    assert_eq!(adapter.id(), CODEX_ID);
    assert_eq!(adapter.name(), CODEX_NAME);
    assert_eq!(adapter.bin(), CODEX_BIN);
    assert!(!adapter.native_isolation());
    assert_eq!(adapter.skill_invocation("wayfinder"), "$wayfinder");
    assert_eq!(adapter.skill_invocation("implement"), "$implement");
    assert!(adapter
        .isolation_unavailable_reason(Language::ZhCn)
        .contains("--worktree"));
    let known = adapter.known_install_locations();
    assert!(
        known
            .iter()
            .any(|path| path.ends_with(std::path::Path::new(".local/bin"))),
        "{known:?}"
    );
}

#[test]
fn codex_adapter_declares_own_fields_not_permission_mode() {
    let fields = CodexAdapter.config_fields();
    let ids: Vec<_> = fields.iter().map(|field| field.id.as_str()).collect();
    assert!(ids.contains(&"model"));
    assert!(ids.contains(&"effort"));
    assert!(ids.contains(&"approval"));
    assert!(ids.contains(&"sandbox"));
    assert!(ids.contains(&"initial-instruction"));
    assert!(!ids.contains(&"permission-mode"));
    assert!(!ids.contains(&"execution-mode"));
    assert!(fields
        .iter()
        .any(|field| field.id == "profile" && field.folded));
    assert!(fields
        .iter()
        .any(|field| field.id == "additional-args" && field.folded));
}

#[test]
fn codex_adapter_discovers_models_and_model_specific_efforts_from_the_cli() {
    let tmp = tempfile::tempdir().unwrap();
    let executable = make_discoverable_codex(tmp.path());
    let env = LaunchEnvironment::from_vars(
        tmp.path().to_path_buf(),
        BTreeMap::from([("PATH".into(), tmp.path().to_string_lossy().into_owned())]),
    );

    let discovery = CodexAdapter
        .discover_config(&executable, &env)
        .expect("Codex CLI discovery");
    let model = discovery
        .fields
        .iter()
        .find(|field| field.id == "model")
        .unwrap();
    assert_eq!(model.options, vec!["gpt-fast", "gpt-deep"]);
    assert_eq!(model.kind, AgentFieldKind::Select);
    let effort = discovery
        .fields
        .iter()
        .find(|field| field.id == "effort")
        .unwrap();
    let filter = effort.option_filter.as_ref().unwrap();
    assert_eq!(filter.field_id, "model");
    assert_eq!(filter.options_by_value["gpt-fast"], vec!["low", "medium"]);
    assert_eq!(
        filter.options_by_value["gpt-deep"],
        vec!["high", "xhigh", "max"]
    );
    assert_eq!(filter.defaults_by_value["gpt-fast"], "low");
    assert_eq!(filter.defaults_by_value["gpt-deep"], "high");
    assert_eq!(discovery.seed["model"], "gpt-fast");
    assert_eq!(discovery.seed["effort"], "low");
    assert_eq!(
        discovery
            .fields
            .iter()
            .find(|field| field.id == "approval")
            .unwrap()
            .options,
        vec!["untrusted", "on-request", "never"]
    );
    assert_eq!(
        discovery
            .fields
            .iter()
            .find(|field| field.id == "sandbox")
            .unwrap()
            .options,
        vec!["read-only", "workspace-write", "danger-full-access"]
    );
}

#[cfg(unix)]
#[test]
fn codex_adapter_stops_app_server_when_model_discovery_times_out() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
    let executable = tmp.path().join("codex");
    std::fs::write(
        &executable,
        r#"#!/bin/sh
if [ "$1" = "app-server" ]; then
  printf '%s' "$$" > "$CODEX_TEST_PID_FILE"
  /bin/sleep 60 >/dev/null 2>&1 &
  printf '%s' "$!" > "$CODEX_TEST_DESCENDANT_PID_FILE"
  read initialize
  printf '%s\n' '{"id":0,"result":{"userAgent":"fake"}}'
  read initialized
  read models
  read forever
fi
exit 2
"#,
    )
    .unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
    let pid_file = tmp.path().join("app-server.pid");
    let descendant_pid_file = tmp.path().join("app-server-descendant.pid");
    let env = LaunchEnvironment::from_vars(
        tmp.path().to_path_buf(),
        BTreeMap::from([
            ("PATH".into(), tmp.path().to_string_lossy().into_owned()),
            (
                "CODEX_TEST_PID_FILE".into(),
                pid_file.to_string_lossy().into_owned(),
            ),
            (
                "CODEX_TEST_DESCENDANT_PID_FILE".into(),
                descendant_pid_file.to_string_lossy().into_owned(),
            ),
        ]),
    );

    let error = CodexAdapter
        .discover_config(&executable, &env)
        .expect_err("model discovery should time out");
    assert!(error.contains("timed out"), "{error}");
    let pids = [
        std::fs::read_to_string(&pid_file).unwrap(),
        std::fs::read_to_string(&descendant_pid_file).unwrap(),
    ];
    let mut leaked = Vec::new();
    for pid in pids {
        let running = std::process::Command::new("/bin/kill")
            .args(["-0", pid.trim()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        if running {
            leaked.push(pid.trim().to_string());
            let _ = std::process::Command::new("/bin/kill")
                .args(["-KILL", pid.trim()])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        }
    }
    assert!(
        leaked.is_empty(),
        "Codex app-server processes survived timeout: {leaked:?}"
    );
}

#[test]
fn codex_adapter_assembles_approval_sandbox_and_profile() {
    let executable = PathBuf::from("/opt/fake/codex");
    let mut values = CodexAdapter.seed_config();
    values.insert("model".into(), "gpt-5.1".into());
    values.insert("effort".into(), "high".into());
    values.insert("approval".into(), "on-request".into());
    values.insert("sandbox".into(), "workspace-write".into());
    values.insert("profile".into(), "work".into());
    values.insert("additional-args".into(), "--search".into());
    let argv = CodexAdapter.assemble_argv_for(&executable, &values);
    assert_eq!(argv[0], "/opt/fake/codex");
    assert!(argv.windows(2).any(|pair| pair == ["--model", "gpt-5.1"]));
    assert!(argv
        .windows(2)
        .any(|pair| pair == ["-c", "model_reasoning_effort=\"high\""]));
    assert!(argv
        .windows(2)
        .any(|pair| pair == ["--ask-for-approval", "on-request"]));
    assert!(argv
        .windows(2)
        .any(|pair| pair == ["--sandbox", "workspace-write"]));
    assert!(argv.windows(2).any(|pair| pair == ["--profile", "work"]));
    assert_eq!(argv.last().map(String::as_str), Some("--search"));
    assert!(!argv.iter().any(|arg| arg == "--permission-mode"));
    assert!(!argv.iter().any(|arg| arg == "exec"));
    assert!(!argv.iter().any(|arg| arg == "-p" || arg == "--single"));
    values.insert("isolation".into(), "true".into());
    let isolated = CodexAdapter.assemble_argv_for(&executable, &values);
    assert!(!isolated.iter().any(|arg| arg == "--worktree"));
}

#[test]
fn codex_adapter_omits_empty_profile_and_effort() {
    let executable = PathBuf::from("/opt/fake/codex");
    let values = BTreeMap::from([
        ("model".into(), "gpt-5.1".into()),
        ("approval".into(), "never".into()),
        ("sandbox".into(), "read-only".into()),
        ("effort".into(), String::new()),
        ("profile".into(), String::new()),
    ]);
    let argv = CodexAdapter.assemble_argv_for(&executable, &values);
    assert!(!argv
        .iter()
        .any(|arg| arg == "-c" || arg.contains("model_reasoning_effort")));
    assert!(!argv.iter().any(|arg| arg == "--profile"));
}

#[test]
fn codex_opening_prompt_is_submitted_as_a_launch_argument() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    make_discoverable_codex(tmp.path());
    let sessions = MemorySessionFactory::new();
    let path = tmp.path().to_string_lossy().into_owned();
    let launch_env = Arc::new(MemoryLaunchEnv::with_path(&path));
    let mut host = HostKernel::boot_with_ports(
        BootRequest {
            app_local_data_dir: tmp.path().join("data"),
            app_log_dir: tmp.path().join("logs"),
            system_locale: "zh-Hans-CN".into(),
            system_appearance: SystemAppearance::Light,
            host_display_name: "Studio".into(),
        },
        KernelPorts {
            tracker: Arc::new(MemoryTracker::new()),
            agents: vec![Arc::new(CodexAdapter)],
            launch_env,
            sessions: Arc::clone(&sessions) as _,
        },
    )
    .unwrap();
    let project_id = host
        .handle(serde_json::json!({
            "op": "registerProject",
            "name": "project",
            "localPath": project,
            "repository": "you/project",
        }))
        .unwrap()
        .snapshot
        .projects[0]
        .id
        .clone();
    let opening = "--help";

    host.handle(serde_json::json!({
        "op": "startUnboundRun",
        "projectId": project_id,
        "agentId": "codex",
        "values": {
            "model": "gpt-5.6-luna",
            "effort": "high",
            "approval": "never",
            "sandbox": "danger-full-access",
            "initial-instruction": opening,
            "profile": "",
            "additional-args": ""
        },
        "openingText": opening,
    }))
    .unwrap();

    let spawn = sessions.last_spawn().unwrap();
    assert_eq!(spawn.argv.last().map(String::as_str), Some(opening));
    assert_eq!(
        spawn
            .argv
            .get(spawn.argv.len().saturating_sub(2))
            .map(String::as_str),
        Some("--"),
        "a leading dash in the prompt must not be parsed as a Codex option"
    );
    assert!(
        sessions
            .last_session()
            .unwrap()
            .read_after(0, Duration::ZERO)
            .data
            .is_empty(),
        "Codex launch prompts must not race TUI startup through PTY input"
    );
}

#[test]
fn codex_attach_hooks_uses_per_run_config_not_home() {
    let tmp = tempfile::tempdir().unwrap();
    let sink = tmp.path().join("sink");
    let project = tmp.path().join("proj");
    std::fs::create_dir_all(&project).unwrap();
    assert!(CodexAdapter.completion_hooks_supported());
    let plan = CodexAdapter
        .attach_completion_hooks(&sink, &project)
        .unwrap();
    assert!(plan
        .extra_argv
        .iter()
        .any(|arg| arg == "--dangerously-bypass-hook-trust"));
    assert!(plan.extra_argv.iter().any(|arg| arg == "-c"));
    assert!(plan
        .extra_argv
        .iter()
        .any(|arg| arg.contains("PermissionRequest")));
    assert!(plan
        .extra_argv
        .iter()
        .any(|arg| arg.contains("UserPromptSubmit")));
    assert!(plan.extra_argv.iter().any(|arg| arg.contains("SessionEnd")));
    assert!(!project.join(".codex").exists());
}
