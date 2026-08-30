use std::collections::BTreeMap;
use std::path::PathBuf;

use host_kernel::{
    AgentPort, ClaudeAdapter, LaunchEnvironment, CLAUDE_BIN, CLAUDE_CODE_ID, CLAUDE_CODE_NAME,
};

fn make_discoverable_claude(dir: &std::path::Path) -> PathBuf {
    let path = dir.join("claude");
    std::fs::write(
        &path,
        r#"#!/bin/sh
if [ "$1" = "--help" ]; then
  printf '%s\n' "  --effort <level> Effort level (low, medium, high, xhigh, max)" "  --model <model> aliases (e.g. 'fable', 'opus', or 'sonnet')" '  --permission-mode <mode> (choices: "acceptEdits", "auto", "bypassPermissions", "manual", "dontAsk", "plan")'
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
fn claude_adapter_discovers_documented_aliases_and_enums_from_help() {
    let tmp = tempfile::tempdir().unwrap();
    let executable = make_discoverable_claude(tmp.path());
    let env = LaunchEnvironment::from_vars(
        tmp.path().to_path_buf(),
        BTreeMap::from([("PATH".into(), tmp.path().to_string_lossy().into_owned())]),
    );
    let discovery = ClaudeAdapter
        .discover_config(&executable, &env)
        .expect("Claude CLI discovery");
    let field = |id: &str| {
        discovery
            .fields
            .iter()
            .find(|field| field.id == id)
            .unwrap()
    };
    assert_eq!(field("model").options, vec!["fable", "opus", "sonnet"]);
    assert_eq!(
        field("effort").options,
        vec!["low", "medium", "high", "xhigh", "max"]
    );
    assert_eq!(
        field("permission-mode").options,
        vec![
            "acceptEdits",
            "auto",
            "bypassPermissions",
            "manual",
            "dontAsk",
            "plan"
        ]
    );
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

#[test]
fn claude_attach_hooks_passes_settings_inside_sink() {
    let tmp = tempfile::tempdir().unwrap();
    let sink = tmp.path().join("sink");
    let project = tmp.path().join("proj");
    std::fs::create_dir_all(&project).unwrap();
    assert!(ClaudeAdapter.completion_hooks_supported());
    let plan = ClaudeAdapter
        .attach_completion_hooks(&sink, &project)
        .unwrap();
    assert!(plan.extra_argv.windows(2).any(|pair| {
        pair[0] == "--settings" && pair[1].starts_with(&sink.to_string_lossy().into_owned())
    }));
    assert!(sink.join("claude-settings.json").is_file());
    assert!(!project.join(".claude").exists());
}
