use std::path::PathBuf;

use host_kernel::{LaunchEnvPort, ShellLaunchEnv};

#[cfg(unix)]
#[test]
fn shell_capture_reads_vars_after_shell_noise() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();
    let shell = tmp.path().join("fake-shell");
    std::fs::write(
        &shell,
        r#"#!/bin/sh
cmd="$1"
while [ "$#" -gt 0 ]; do
  cmd="$1"
  shift
done
echo "welcome to the fake shell"
eval "$cmd"
"#,
    )
    .unwrap();
    std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o755)).unwrap();

    let captured = ShellLaunchEnv::with_shell(shell)
        .capture(tmp.path())
        .expect("capture");
    assert_eq!(captured.cwd, tmp.path());
    assert!(
        captured.vars.contains_key("PATH") || captured.vars.contains_key("HOME"),
        "missing env vars: {:?}",
        captured.vars.keys().collect::<Vec<_>>()
    );
    assert!(!captured.vars.contains_key("welcome to the fake shell"));
}

#[test]
fn grok_known_location_is_under_home_grok_bin() {
    let known = host_kernel::GrokAdapter::known_location().expect("home");
    assert!(known.ends_with(PathBuf::from(".grok/bin")));
}
