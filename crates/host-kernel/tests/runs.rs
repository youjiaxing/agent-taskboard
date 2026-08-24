use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use host_kernel::{
    BootRequest, Command, HostKernel, KernelPorts, Language, LaunchEnvironment, MemoryAgent,
    MemoryLaunchEnv, MemorySessionFactory, MemoryTracker, ProcessIntent, RunStatus,
    SystemAppearance,
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
    assert_eq!(h.launch_env.capture_count(), 2);
    assert_eq!(h.launch_env.captured_dirs(), vec![dir.clone(), dir.clone()]);
}

#[test]
fn manual_environment_refresh_updates_later_agent_probes_and_run_spawns() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path(), MemoryAgent::installed_grok(), "/before/bin");
    let project_id = register(&mut h.host, &dir);
    h.launch_env.set(
        dir.clone(),
        LaunchEnvironment::from_vars(
            dir.clone(),
            BTreeMap::from([("PATH".into(), "/after/bin".into())]),
        ),
    );

    let refreshed = h
        .host
        .handle(serde_json::json!({ "op": "refreshLaunchEnvironment" }))
        .unwrap();
    let launch_environment = refreshed.launch_environment.unwrap();
    assert_eq!(launch_environment.status, "ready");
    assert_eq!(launch_environment.refreshed_directories, 1);

    let form = h
        .host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
        }))
        .unwrap()
        .snapshot
        .launch_form
        .unwrap();
    assert!(form.agents[0].installed);
    h.host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
        }))
        .unwrap();
    assert!(h.sessions.last_spawn().unwrap().env["PATH"].contains("/after/bin"));
}

#[test]
fn client_only_switch_reserves_the_process_against_new_runs() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path(), MemoryAgent::installed_grok(), "/mem/bin");
    let project_id = register(&mut h.host, &dir);

    let gate = h.host.begin_client_only_switch();
    assert!(gate.allowed);
    let err = h
        .host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
        }))
        .unwrap_err();
    assert!(err.to_string().contains("update install is starting"));
    assert_eq!(h.sessions.spawn_count(), 0);
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

    assert_eq!(h.launch_env.capture_count(), 2);
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
fn update_install_requires_every_run_to_end() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path(), MemoryAgent::installed_grok(), "/mem/bin");
    let project_id = register(&mut h.host, &dir);

    let idle = h
        .host
        .handle(serde_json::json!({ "op": "updateInstallGate" }))
        .unwrap()
        .update_install_gate
        .expect("update install gate");
    assert!(idle.allowed);
    assert_eq!(idle.active_run_count, 0);

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

    let busy = h
        .host
        .handle(serde_json::json!({ "op": "updateInstallGate" }))
        .unwrap()
        .update_install_gate
        .expect("update install gate");
    assert!(!busy.allowed);
    assert_eq!(busy.active_run_count, 1);

    h.host
        .handle(serde_json::json!({
            "op": "stopRun",
            "runId": run_id,
        }))
        .unwrap();
    assert!(
        h.host
            .handle(serde_json::json!({ "op": "beginUpdateInstall" }))
            .unwrap()
            .update_install_gate
            .expect("update install gate")
            .allowed
    );

    let err = h
        .host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
        }))
        .unwrap_err();
    assert!(err.to_string().contains("update install"));

    h.host
        .handle(serde_json::json!({ "op": "cancelUpdateInstall" }))
        .unwrap();
    let started = h
        .host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
        }))
        .unwrap();
    assert_eq!(
        started.snapshot.runs.last().unwrap().status,
        RunStatus::Running
    );
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
    assert_eq!(snap.copy.execute_run, "Run");
    assert_eq!(snap.copy.continue_run, "Continue");
    assert_eq!(snap.copy.release_claim, "Release claim");
    assert_eq!(snap.copy.execution_stopped, "Execution stopped");
    assert_eq!(snap.copy.waiting, "Waiting");
    assert_eq!(snap.copy.running, "Running");
    assert_eq!(snap.copy.inject_line, "Inject");
    assert_eq!(snap.copy.notify_desktop, "Desktop notifications");
    assert_eq!(snap.copy.notify_sound, "Notification sound");
    assert_eq!(snap.copy.unbound_issue, "Unbound Issue");
    assert_eq!(snap.copy.stop_run, "Stop");
}
