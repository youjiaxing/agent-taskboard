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

#[cfg(unix)]
#[test]
fn manual_refresh_replaces_the_cached_snapshot_and_failure_keeps_the_last_good_one() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();
    let value = tmp.path().join("value");
    std::fs::write(&value, "before").unwrap();
    let shell = tmp.path().join("fake-shell");
    std::fs::write(
        &shell,
        format!(
            r#"#!/bin/sh
if [ -f "{failed}" ]; then exit 1; fi
printf 'AGENT_TASKBOARD_ENV_BEGIN\0VALUE='
cat "{value}"
printf '\0PATH=/bin\0'
"#,
            failed = tmp.path().join("failed").display(),
            value = value.display(),
        ),
    )
    .unwrap();
    std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o755)).unwrap();
    let env = ShellLaunchEnv::with_shell(shell);

    assert_eq!(env.capture(tmp.path()).unwrap().vars["VALUE"], "before");
    std::fs::write(&value, "after").unwrap();
    assert_eq!(env.capture(tmp.path()).unwrap().vars["VALUE"], "before");

    assert_eq!(env.refresh(tmp.path()).unwrap().vars["VALUE"], "after");
    std::fs::write(tmp.path().join("failed"), "yes").unwrap();
    assert!(env.refresh(tmp.path()).is_err());
    assert_eq!(env.capture(tmp.path()).unwrap().vars["VALUE"], "after");
}
