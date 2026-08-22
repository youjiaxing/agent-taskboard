use std::path::PathBuf;

use host_kernel::{
    builtin_agents, AgentPort, AntigravityAdapter, ANTIGRAVITY_BIN, ANTIGRAVITY_ID,
    ANTIGRAVITY_NAME,
};

#[test]
fn antigravity_adapter_only_uses_agy() {
    let adapter = AntigravityAdapter;
    assert_eq!(adapter.id(), ANTIGRAVITY_ID);
    assert_eq!(adapter.name(), ANTIGRAVITY_NAME);
    assert_eq!(adapter.bin(), ANTIGRAVITY_BIN);
    assert_eq!(adapter.bin(), "agy");
    assert!(!adapter.native_isolation());
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
