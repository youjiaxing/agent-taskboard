use std::collections::BTreeMap;
use std::path::PathBuf;

use host_kernel::{AgentPort, CodexAdapter, Language, CODEX_BIN, CODEX_ID, CODEX_NAME};

#[test]
fn codex_adapter_declares_interactive_tui_contract() {
    let adapter = CodexAdapter;
    assert_eq!(adapter.id(), CODEX_ID);
    assert_eq!(adapter.name(), CODEX_NAME);
    assert_eq!(adapter.bin(), CODEX_BIN);
    assert!(!adapter.native_isolation());
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
fn codex_attach_hooks_uses_per_run_config_not_home() {
    let tmp = tempfile::tempdir().unwrap();
    let sink = tmp.path().join("sink");
    let project = tmp.path().join("proj");
    std::fs::create_dir_all(&project).unwrap();
    assert!(CodexAdapter.completion_hooks_supported());
    let plan = CodexAdapter
        .attach_completion_hooks(&sink, &project)
        .unwrap();
    assert!(plan.extra_argv.iter().any(|arg| arg == "-c"));
    assert!(plan.extra_argv.iter().any(|arg| arg.contains("SessionEnd")));
    assert!(!project.join(".codex").exists());
}
