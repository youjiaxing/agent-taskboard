use std::collections::BTreeMap;
use std::path::PathBuf;

use host_kernel::{
    probe_binary, AgentPort, GrokAdapter, LaunchEnvironment, ProbeResult, GROK_BIN, GROK_BUILD_ID,
    GROK_BUILD_NAME,
};

fn make_grok(dir: &std::path::Path) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join("grok");
    std::fs::write(&path, "#!/bin/sh\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    path
}

#[test]
fn grok_adapter_declares_interactive_tui_contract() {
    let adapter = GrokAdapter;
    assert_eq!(adapter.id(), GROK_BUILD_ID);
    assert_eq!(adapter.name(), GROK_BUILD_NAME);
    assert_eq!(adapter.bin(), GROK_BIN);
    let known = adapter.known_install_locations();
    assert!(
        known
            .iter()
            .any(|path| path.ends_with(std::path::Path::new(".grok/bin"))),
        "{known:?}"
    );
}

#[test]
fn grok_adapter_declares_first_layer_fields() {
    let fields = GrokAdapter.config_fields();
    let ids: Vec<_> = fields.iter().map(|field| field.id.as_str()).collect();
    assert!(ids.contains(&"model"));
    assert!(ids.contains(&"effort"));
    assert!(ids.contains(&"permission-mode"));
    assert!(ids.contains(&"always-approve"));
    assert!(ids.contains(&"sandbox"));
    assert!(ids.contains(&"initial-instruction"));
    assert!(fields
        .iter()
        .any(|field| field.id == "additional-args" && field.folded));
}

#[test]
fn grok_adapter_assembles_form_values_without_prompt_flag() {
    let executable = PathBuf::from("/opt/fake/grok");
    let mut values = GrokAdapter.seed_config();
    values.insert("model".into(), "grok-4.6".into());
    values.insert("always-approve".into(), "true".into());
    values.insert("additional-args".into(), "--no-subagents".into());
    let argv = GrokAdapter.assemble_argv_for(&executable, &values);
    assert_eq!(argv[0], "/opt/fake/grok");
    assert!(argv.windows(2).any(|pair| pair == ["--model", "grok-4.6"]));
    assert!(argv.iter().any(|arg| arg == "--always-approve"));
    assert!(argv.iter().any(|arg| arg == "--no-subagents"));
    assert!(!argv.iter().any(|arg| arg == "-p" || arg == "--single"));
}

#[test]
fn grok_adapter_assembles_bare_interactive_tui_argv() {
    let executable = PathBuf::from("/opt/fake/grok");
    assert_eq!(
        GrokAdapter.assemble_argv(&executable),
        vec!["/opt/fake/grok"]
    );
    let argv = GrokAdapter.assemble_argv(&executable).join(" ");
    assert!(!argv.contains(" -p"), "{argv}");
    assert!(!argv.contains("--single"), "{argv}");
}

#[test]
fn grok_probe_finds_binary_on_path_when_known_location_is_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let bin_dir = tmp.path().join("bin");
    let grok = make_grok(&bin_dir);
    let env = LaunchEnvironment::from_vars(
        tmp.path().to_path_buf(),
        BTreeMap::from([("PATH".into(), bin_dir.to_string_lossy().into_owned())]),
    );
    match probe_binary("grok", &env, &[]) {
        ProbeResult::Found { executable } => assert_eq!(executable, grok),
        other => panic!("expected found, got {other:?}"),
    }
}

#[test]
fn grok_probe_missing_keeps_command_path_and_known_location() {
    let env = LaunchEnvironment::from_vars(
        PathBuf::from("/tmp"),
        BTreeMap::from([("PATH".into(), "/opt/empty".into())]),
    );
    match probe_binary("grok", &env, &[PathBuf::from("/mem/.grok/bin")]) {
        ProbeResult::Missing {
            command,
            searched_path,
            known_locations,
        } => {
            assert_eq!(command, "grok");
            assert_eq!(searched_path, "/opt/empty");
            assert_eq!(known_locations, vec![PathBuf::from("/mem/.grok/bin")]);
        }
        other => panic!("expected missing, got {other:?}"),
    }
}

#[test]
fn probe_binary_prefers_known_install_location() {
    let tmp = tempfile::tempdir().unwrap();
    let known = tmp.path().join("known");
    let path_dir = tmp.path().join("path");
    let grok = make_grok(&known);
    make_grok(&path_dir);
    let env = LaunchEnvironment::from_vars(
        tmp.path().to_path_buf(),
        BTreeMap::from([("PATH".into(), path_dir.to_string_lossy().into_owned())]),
    );
    match probe_binary("grok", &env, &[known]) {
        ProbeResult::Found { executable } => assert_eq!(executable, grok),
        other => panic!("expected found, got {other:?}"),
    }
}
