use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use host_kernel::{
    BootRequest, Command, HostKernel, KernelPorts, Language, MemoryAgent, MemoryLaunchEnv,
    MemorySessionFactory, MemoryTracker, ProcessIntent, RunStatus, SystemAppearance,
};

fn boot_req(root: &Path) -> BootRequest {
    BootRequest {
        app_local_data_dir: root.to_path_buf(),
        app_log_dir: root.join("logs"),
        system_locale: "zh-Hans-CN".into(),
        system_appearance: SystemAppearance::Light,
        host_display_name: "Studio".into(),
    }
}

fn make_dir(root: &Path, name: &str) -> std::path::PathBuf {
    let dir = root.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

struct Harness {
    host: HostKernel,
    agent: Arc<MemoryAgent>,
    launch_env: Arc<MemoryLaunchEnv>,
    sessions: Arc<MemorySessionFactory>,
}

fn harness(root: &Path, agent: MemoryAgent, path: &str) -> Harness {
    let agent = Arc::new(agent);
    let launch_env = Arc::new(MemoryLaunchEnv::with_path(path));
    let sessions = MemorySessionFactory::new();
    let host = HostKernel::boot_with_ports(
        boot_req(root),
        KernelPorts {
            tracker: Arc::new(MemoryTracker::new()),
            agents: vec![Arc::clone(&agent) as _],
            launch_env: Arc::clone(&launch_env) as _,
            sessions: Arc::clone(&sessions) as _,
        },
    )
    .unwrap();
    Harness {
        host,
        agent,
        launch_env,
        sessions,
    }
}

fn register(host: &mut HostKernel, dir: &Path) -> String {
    host.handle(serde_json::json!({
        "op": "registerProject",
        "name": "garden",
        "localPath": dir,
        "repository": "you/garden",
    }))
    .unwrap()
    .snapshot
    .projects[0]
        .id
        .clone()
}

#[test]
fn missing_grok_lists_command_path_and_known_locations() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(
        tmp.path(),
        MemoryAgent::missing_grok(),
        "/opt/empty:/usr/bin",
    );
    let project_id = register(&mut h.host, &dir);

    let out = h
        .host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
        }))
        .unwrap();

    assert_eq!(out.snapshot.runs.len(), 1);
    let run = &out.snapshot.runs[0];
    assert!(run.unbound);
    assert!(run.issue_id.is_none());
    assert_eq!(run.status, RunStatus::Ended);
    assert_eq!(run.agent_name, "Grok Build");
    let failure = run.failure.as_deref().unwrap();
    assert!(failure.contains("grok"), "{failure}");
    assert!(failure.contains("/opt/empty"), "{failure}");
    assert!(failure.contains("/mem/.grok/bin"), "{failure}");
    assert!(
        !failure.to_ascii_lowercase().contains("terminal app"),
        "{failure}"
    );
    assert!(!failure.contains("先开终端"), "{failure}");
    assert_eq!(h.sessions.spawn_count(), 0);
}

#[test]
fn new_unbound_run_does_not_claim_and_shows_grok() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path(), MemoryAgent::installed_grok(), "/mem/bin");
    let project_id = register(&mut h.host, &dir);

    let out = h
        .host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
        }))
        .unwrap();

    assert_eq!(out.snapshot.runs.len(), 1);
    let run = &out.snapshot.runs[0];
    assert!(run.unbound);
    assert!(run.issue_id.is_none());
    assert_eq!(run.status, RunStatus::Running);
    assert_eq!(run.agent_id, "grok-build");
    assert_eq!(run.agent_name, "Grok Build");
    assert!(run.recent_action.is_none());
    assert_eq!(out.snapshot.copy.new_run, "新建");
    assert_eq!(out.snapshot.copy.unbound_issue, "未绑定 Issue");
    assert!(out.snapshot.projects[0].has_active_run);
    assert_eq!(out.snapshot.focused_run_id, run.id);
    assert_eq!(h.launch_env.capture_count(), 1);
    assert_eq!(h.launch_env.captured_dirs(), vec![dir.clone()]);
}

#[test]
fn probe_and_start_share_one_launch_env_and_exec_absolute_path() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path(), MemoryAgent::installed_grok(), "/mem/bin");
    let project_id = register(&mut h.host, &dir);
    h.host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
        }))
        .unwrap();

    assert_eq!(h.launch_env.capture_count(), 1);
    let spawn = h.sessions.last_spawn().unwrap();
    assert_eq!(spawn.argv[0], "/mem/grok");
    assert!(spawn
        .argv
        .windows(2)
        .any(|pair| pair == ["--model", "grok-4.6"]));
    assert!(spawn
        .argv
        .windows(2)
        .any(|pair| pair == ["--effort", "high"]));
    assert!(spawn
        .argv
        .windows(2)
        .any(|pair| pair == ["--sandbox", "off"]));
    assert!(!spawn
        .argv
        .iter()
        .any(|arg| arg == "-p" || arg == "--single"));
    assert!(std::path::Path::new(&spawn.argv[0]).is_absolute());
    assert_eq!(spawn.cwd, dir);
    assert_eq!(
        spawn.env.get("TERM").map(String::as_str),
        Some("xterm-256color")
    );
    assert_eq!(
        spawn.env.get("COLORTERM").map(String::as_str),
        Some("truecolor")
    );
    assert!(spawn.env.get("PATH").unwrap().contains("/mem/.grok/bin"));
}

#[test]
fn launch_failure_leaves_a_record_and_does_not_retry() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path(), MemoryAgent::installed_grok(), "/mem/bin");
    let project_id = register(&mut h.host, &dir);
    h.sessions.fail_next("could not spawn grok");

    let out = h
        .host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
        }))
        .unwrap();

    assert_eq!(out.snapshot.runs.len(), 1);
    let run = &out.snapshot.runs[0];
    assert_eq!(run.status, RunStatus::Ended);
    assert_eq!(run.failure.as_deref(), Some("could not spawn grok"));
    assert_eq!(h.sessions.spawn_count(), 1);
    assert!(!out.snapshot.projects[0].has_active_run);
}

#[test]
fn stopping_a_run_ends_it() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path(), MemoryAgent::installed_grok(), "/mem/bin");
    let project_id = register(&mut h.host, &dir);
    let run_id = h
        .host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
        }))
        .unwrap()
        .snapshot
        .runs[0]
        .id
        .clone();

    let out = h
        .host
        .handle(serde_json::json!({
            "op": "stopRun",
            "runId": run_id,
        }))
        .unwrap();
    assert_eq!(out.snapshot.runs[0].status, RunStatus::Ended);
    assert!(!out.snapshot.projects[0].has_active_run);
    assert!(h.sessions.last_session().unwrap().stopped());
}

#[test]
fn unbound_runs_can_run_in_parallel() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path(), MemoryAgent::installed_grok(), "/mem/bin");
    let project_id = register(&mut h.host, &dir);
    h.host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
        }))
        .unwrap();
    let out = h
        .host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
        }))
        .unwrap();
    assert_eq!(out.snapshot.runs.len(), 2);
    assert!(out
        .snapshot
        .runs
        .iter()
        .all(|run| run.status == RunStatus::Running));
    assert_eq!(h.sessions.spawn_count(), 2);
}

#[test]
fn quitting_host_with_active_runs_requires_a_choice() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path(), MemoryAgent::installed_grok(), "/mem/bin");
    let project_id = register(&mut h.host, &dir);
    h.host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
        }))
        .unwrap();

    let out = h.host.dispatch(Command::QuitHost).unwrap();
    assert_eq!(out.process, ProcessIntent::KeepRunning);
    assert!(out.snapshot.running);
    assert_eq!(
        out.snapshot.quit_offer.as_ref().unwrap().active_run_count,
        1
    );
    assert_eq!(out.snapshot.runs[0].status, RunStatus::Running);

    let out = h.host.dispatch(Command::CancelQuit).unwrap();
    assert!(out.snapshot.quit_offer.is_none());
    assert!(out.snapshot.running);
    assert_eq!(out.snapshot.runs[0].status, RunStatus::Running);

    h.host.dispatch(Command::QuitHost).unwrap();
    let out = h.host.dispatch(Command::ConfirmQuitStopAll).unwrap();
    assert_eq!(out.process, ProcessIntent::Exit);
    assert!(!out.snapshot.running);
    assert_eq!(out.snapshot.runs[0].status, RunStatus::Ended);
    assert!(h.sessions.last_session().unwrap().stopped());
}

#[test]
fn recent_action_stays_empty_when_adapter_has_none() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path(), MemoryAgent::installed_grok(), "/mem/bin");
    h.agent.set_recent_action(None);
    let project_id = register(&mut h.host, &dir);
    let run = &h
        .host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
        }))
        .unwrap()
        .snapshot
        .runs[0];
    assert!(run.recent_action.is_none());
}

#[test]
fn pty_bytes_round_trip_through_host() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path(), MemoryAgent::installed_grok(), "/mem/bin");
    let project_id = register(&mut h.host, &dir);
    let run_id = h
        .host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
        }))
        .unwrap()
        .snapshot
        .runs[0]
        .id
        .clone();
    h.host.write_pty(&run_id, b"hi").unwrap();
    let chunk = h
        .host
        .pty_output(&run_id, 0, Duration::from_millis(50))
        .unwrap();
    assert_eq!(chunk.data, b"hi");
}

#[test]
fn english_copy_uses_new_for_the_plus_button() {
    let tmp = tempfile::tempdir().unwrap();
    let mut h = harness(tmp.path(), MemoryAgent::installed_grok(), "/mem/bin");
    h.host.dispatch(Command::SetLanguage(Language::En)).unwrap();
    let snap = h.host.snapshot();
    assert_eq!(snap.copy.new_run, "New");
    assert_eq!(snap.copy.unbound_issue, "Unbound Issue");
    assert_eq!(snap.copy.stop_run, "Stop");
}
