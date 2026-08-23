use std::path::PathBuf;

use host_kernel::{AgentPort, ClaudeAdapter, CLAUDE_BIN, CLAUDE_CODE_ID, CLAUDE_CODE_NAME};

#[test]
fn claude_adapter_declares_interactive_tui_contract() {
    let adapter = ClaudeAdapter;
    assert_eq!(adapter.id(), CLAUDE_CODE_ID);
    assert_eq!(adapter.name(), CLAUDE_CODE_NAME);
    assert_eq!(adapter.bin(), CLAUDE_BIN);
    assert!(adapter.native_isolation());
}

#[test]
fn claude_adapter_declares_permission_mode_on_first_layer() {
    let fields = ClaudeAdapter.config_fields();
    let ids: Vec<_> = fields.iter().map(|field| field.id.as_str()).collect();
    assert!(ids.contains(&"model"));
    assert!(ids.contains(&"effort"));
    assert!(ids.contains(&"permission-mode"));
    assert!(ids.contains(&"initial-instruction"));
    assert!(!ids.contains(&"approval"));
    assert!(!ids.contains(&"execution-mode"));
    let permission = fields
        .iter()
        .find(|field| field.id == "permission-mode")
        .unwrap();
    assert!(!permission.folded);
    assert!(fields
        .iter()
        .any(|field| field.id == "additional-args" && field.folded));
}

#[test]
fn claude_adapter_assembles_permission_mode_and_effort() {
    let executable = PathBuf::from("/opt/fake/claude");
    let mut values = ClaudeAdapter.seed_config();
    values.insert("model".into(), "sonnet".into());
    values.insert("effort".into(), "high".into());
    values.insert("permission-mode".into(), "plan".into());
    values.insert("additional-args".into(), "--verbose".into());
    let argv = ClaudeAdapter.assemble_argv_for(&executable, &values);
    assert_eq!(argv[0], "/opt/fake/claude");
    assert!(argv.windows(2).any(|pair| pair == ["--model", "sonnet"]));
    assert!(argv.windows(2).any(|pair| pair == ["--effort", "high"]));
    assert!(argv
        .windows(2)
        .any(|pair| pair == ["--permission-mode", "plan"]));
    assert_eq!(argv.last().map(String::as_str), Some("--verbose"));
    assert!(!argv.iter().any(|arg| arg == "--ask-for-approval"));
    assert!(!argv.iter().any(|arg| arg == "-p" || arg == "--print"));
    assert!(!argv.iter().any(|arg| arg == "--worktree"));
}

#[test]
fn claude_adapter_passes_worktree_without_inventing_a_name() {
    let executable = PathBuf::from("/opt/fake/claude");
    let mut values = ClaudeAdapter.seed_config();
    values.insert("isolation".into(), "true".into());
    let argv = ClaudeAdapter.assemble_argv_for(&executable, &values);
    assert!(argv.iter().any(|arg| arg == "--worktree"));
    let flag = argv.iter().position(|arg| arg == "--worktree").unwrap();
    if let Some(next) = argv.get(flag + 1) {
        assert!(next.starts_with('-'), "{argv:?}");
    }
}
