use std::collections::BTreeMap;
use std::path::PathBuf;

use host_kernel::{
    builtin_agents, AgentPort, AntigravityAdapter, Language, LaunchEnvironment, ANTIGRAVITY_BIN,
    ANTIGRAVITY_ID, ANTIGRAVITY_NAME,
};

fn make_discoverable_agy(dir: &std::path::Path) -> PathBuf {
    let path = dir.join("agy");
    std::fs::write(
        &path,
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' 'gemini-fast-low\tGemini Fast Low' 'gemini-fast-high\tGemini Fast High' 'claude-thinking\tClaude Thinking'
  exit 0
fi
if [ "$1" = "--help" ]; then
  printf '%s\n' '  --effort Reasoning effort (low|medium|high)' '  --mode execution mode (accept-edits, plan)'
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
fn antigravity_adapter_only_uses_agy() {
    let adapter = AntigravityAdapter;
    assert_eq!(adapter.id(), ANTIGRAVITY_ID);
    assert_eq!(adapter.name(), ANTIGRAVITY_NAME);
    assert_eq!(adapter.bin(), ANTIGRAVITY_BIN);
    assert_eq!(adapter.bin(), "agy");
    assert!(!adapter.native_isolation());
    assert!(adapter
        .isolation_unavailable_reason(Language::ZhCn)
        .contains("--worktree"));
    assert!(!adapter.name().to_ascii_lowercase().contains("gemini"));
    assert!(!adapter
        .known_install_locations()
        .iter()
        .any(|path| path.to_string_lossy().contains("gemini")));
}

#[test]
fn builtin_agents_follow_v1_priority_without_gemini() {
    let agents = builtin_agents();
    let ids: Vec<_> = agents.iter().map(|agent| agent.id()).collect();
    let names: Vec<_> = agents.iter().map(|agent| agent.name()).collect();
    assert_eq!(
        ids,
        vec!["grok-build", "codex", "claude-code", "antigravity-cli"]
    );
    assert_eq!(
        names,
        vec!["Grok Build", "Codex", "Claude Code", "Antigravity CLI"]
    );
    assert!(!agents.iter().any(|agent| agent.bin() == "gemini"
        || agent.id().contains("gemini")
        || agent.name().to_ascii_lowercase().contains("gemini")));
}

#[test]
fn antigravity_adapter_declares_execution_mode_not_permission_axis() {
    let fields = AntigravityAdapter.config_fields();
    let first: Vec<_> = fields
        .iter()
        .filter(|field| !field.folded)
        .map(|field| field.id.as_str())
        .collect();
    let folded: Vec<_> = fields
        .iter()
        .filter(|field| field.folded)
        .map(|field| field.id.as_str())
        .collect();
    assert_eq!(
        first,
        vec![
            "model",
            "effort",
            "execution-mode",
            "skip-permissions",
            "sandbox",
            "initial-instruction"
        ]
    );
    assert!(folded.contains(&"agent"));
    assert!(folded.contains(&"add-dir"));
    assert!(folded.contains(&"additional-args"));
    assert!(!fields.iter().any(|field| field.id == "permission-mode"));
    assert!(!fields.iter().any(|field| field.id == "approval"));
}

#[test]
fn antigravity_adapter_discovers_models_and_filters_encoded_efforts() {
    let tmp = tempfile::tempdir().unwrap();
    let executable = make_discoverable_agy(tmp.path());
    let env = LaunchEnvironment::from_vars(
        tmp.path().to_path_buf(),
        BTreeMap::from([("PATH".into(), tmp.path().to_string_lossy().into_owned())]),
    );
    let discovery = AntigravityAdapter
        .discover_config(&executable, &env)
        .expect("Antigravity CLI discovery");
    let field = |id: &str| {
        discovery
            .fields
            .iter()
            .find(|field| field.id == id)
            .unwrap()
    };
    assert_eq!(
        field("model").options,
        vec!["gemini-fast-low", "gemini-fast-high", "claude-thinking"]
    );
    assert_eq!(field("effort").options, vec!["low", "medium", "high"]);
    assert_eq!(
        field("execution-mode").options,
        vec!["accept-edits", "plan"]
    );
    let filter = field("effort").option_filter.as_ref().unwrap();
    assert_eq!(filter.options_by_value["gemini-fast-low"], vec!["low"]);
    assert_eq!(filter.options_by_value["gemini-fast-high"], vec!["high"]);
    assert_eq!(
        filter.options_by_value["claude-thinking"],
        vec!["low", "medium", "high"]
    );
}

#[test]
fn antigravity_adapter_assembles_mode_not_permission_flag() {
    let executable = PathBuf::from("/opt/fake/agy");
    let mut values = AntigravityAdapter.seed_config();
    values.insert("model".into(), "gemini-3-flash".into());
    values.insert("effort".into(), "high".into());
    values.insert("execution-mode".into(), "plan".into());
    values.insert("skip-permissions".into(), "true".into());
    values.insert("sandbox".into(), "true".into());
    values.insert("agent".into(), "reviewer".into());
    values.insert("add-dir".into(), "/tmp/extra /tmp/docs".into());
    values.insert("additional-args".into(), "--continue".into());
    let argv = AntigravityAdapter.assemble_argv_for(&executable, &values);
    assert_eq!(argv[0], "/opt/fake/agy");
    assert!(argv
        .windows(2)
        .any(|pair| pair == ["--model", "gemini-3-flash"]));
    assert!(argv.windows(2).any(|pair| pair == ["--effort", "high"]));
    assert!(argv.windows(2).any(|pair| pair == ["--mode", "plan"]));
    assert!(argv
        .iter()
        .any(|arg| arg == "--dangerously-skip-permissions"));
    assert!(argv.iter().any(|arg| arg == "--sandbox"));
    assert!(argv.windows(2).any(|pair| pair == ["--agent", "reviewer"]));
    assert!(argv
        .windows(2)
        .any(|pair| pair == ["--add-dir", "/tmp/extra"]));
    assert!(argv
        .windows(2)
        .any(|pair| pair == ["--add-dir", "/tmp/docs"]));
    assert_eq!(argv.last().map(String::as_str), Some("--continue"));
    assert!(!argv.iter().any(|arg| arg == "--permission-mode"));
    assert!(!argv.iter().any(|arg| arg == "--ask-for-approval"));
    assert!(!argv.iter().any(|arg| arg == "-p" || arg == "--print"));
    assert!(!argv.iter().any(|arg| arg == "gemini"));
}

#[test]
fn antigravity_does_not_claim_per_run_completion_hooks() {
    assert!(!AntigravityAdapter.completion_hooks_supported());
    let tmp = tempfile::tempdir().unwrap();
    let err = AntigravityAdapter
        .attach_completion_hooks(&tmp.path().join("sink"), tmp.path())
        .unwrap_err();
    assert!(!err.is_empty());
}
