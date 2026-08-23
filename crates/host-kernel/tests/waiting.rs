use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use host_kernel::{
    BootRequest, HostEvent, HostKernel, IssueActivity, IssueRecord, KernelError, KernelPorts,
    MemoryAgent, MemoryLaunchEnv, MemorySessionFactory, MemoryTracker, NotificationKind,
    RunEndedReason, RunStatus, SystemAppearance,
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
    tracker: Arc<MemoryTracker>,
    agent: Arc<MemoryAgent>,
    sessions: Arc<MemorySessionFactory>,
}

fn harness(root: &Path) -> Harness {
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready work"));
    tracker.add_issue(IssueRecord::open("you/garden", 2, "second"));
    tracker.add_issue(IssueRecord::open("you/garden", 3, "third"));
    let agent = Arc::new(MemoryAgent::installed_grok());
    let sessions = MemorySessionFactory::new();
    let host = HostKernel::boot_with_ports(
        boot_req(root),
        KernelPorts {
            tracker: Arc::clone(&tracker) as _,
            agents: vec![Arc::clone(&agent) as _],
            launch_env: Arc::new(MemoryLaunchEnv::with_path("/mem/bin")) as _,
            sessions: Arc::clone(&sessions) as _,
        },
    )
    .unwrap();
    Harness {
        host,
        tracker,
        agent,
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

fn grok_values() -> serde_json::Value {
    serde_json::json!({
        "model": "grok-4.6",
        "effort": "high",
        "permission-mode": "normal",
        "always-approve": "false",
        "sandbox": "off",
        "initial-instruction": "",
        "additional-args": ""
    })
}

fn start_bound(host: &mut HostKernel, project_id: &str, issue_id: &str) -> String {
    host.handle(serde_json::json!({
        "op": "startUnboundRun",
        "projectId": project_id,
        "issueId": issue_id,
        "agentId": "grok-build",
        "values": grok_values(),
        "openingText": issue_id,
    }))
    .unwrap()
    .snapshot
    .runs
    .iter()
    .rev()
    .find(|run| run.issue_id.as_deref() == Some(issue_id) && run.status == RunStatus::Running)
    .unwrap()
    .id
    .clone()
}

fn snapshot(host: &mut HostKernel) -> host_kernel::CommandOutcome {
    host.handle(serde_json::json!({ "op": "snapshot" }))
        .unwrap()
}

fn in_progress(host: &HostKernel) -> Vec<(String, Option<IssueActivity>)> {
    host.snapshot()
        .board
        .unwrap()
        .columns
        .unwrap()
        .in_progress
        .iter()
        .map(|card| (card.id.clone(), card.activity))
        .collect()
}

fn frontier_ids(host: &HostKernel) -> Vec<String> {
    host.snapshot()
        .board
        .unwrap()
        .columns
        .unwrap()
        .frontier
        .iter()
        .map(|card| card.id.clone())
        .collect()
}

fn blocked_ids(host: &HostKernel) -> Vec<String> {
    host.snapshot()
        .board
        .unwrap()
        .columns
        .unwrap()
        .blocked
        .iter()
        .map(|card| card.id.clone())
        .collect()
}

fn notification_kinds(events: &[HostEvent]) -> Vec<NotificationKind> {
    events
        .iter()
        .filter_map(|event| match event {
            HostEvent::Notification { kind, .. } => Some(*kind),
            _ => None,
        })
        .collect()
}

fn run_named<'a>(
    snapshot: &'a host_kernel::HostSnapshot,
    run_id: &str,
) -> &'a host_kernel::RunSummary {
    snapshot
        .runs
        .iter()
        .find(|run| run.id == run_id)
        .expect("run")
}

#[test]
fn waiting_stays_an_active_run_not_execution_stopped_or_blocked() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    h.tracker.add_issue(
        IssueRecord::open("you/garden", 9, "has a blocker").blocked_by(
            "you/garden",
            8,
            "gate",
            true,
        ),
    );
    h.tracker
        .add_issue(IssueRecord::open("you/garden", 8, "gate"));
    let project_id = register(&mut h.host, &dir);
    let run_id = start_bound(&mut h.host, &project_id, "you/garden#1");
    h.sessions.last_session().unwrap().set_waiting(true);

    let out = snapshot(&mut h.host);
    let run = run_named(&out.snapshot, &run_id);
    assert_eq!(run.status, RunStatus::Running);
    assert!(run.waiting_for_user);
    assert!(run.is_active());
    assert!(run.ended_reason.is_none());

    h.host
        .handle(serde_json::json!({
            "op": "focusIssue",
            "issueId": "you/garden#1",
        }))
        .unwrap();
    let selected = h.host.snapshot().board.unwrap().selected.unwrap();
    assert!(selected.waiting_for_user);
    assert!(!selected.execution_stopped);
    assert_eq!(selected.active_run_id.as_deref(), Some(run_id.as_str()));
    assert!(!frontier_ids(&h.host).contains(&"you/garden#1".into()));
    assert!(!blocked_ids(&h.host).contains(&"you/garden#1".into()));
    assert!(in_progress(&h.host)
        .iter()
        .any(|(id, activity)| id == "you/garden#1" && *activity == Some(IssueActivity::Waiting)));
    assert!(blocked_ids(&h.host).contains(&"you/garden#9".into()));
    assert!(out
        .events
        .iter()
        .any(|event| matches!(event, HostEvent::Waiting { run_id: id } if id == &run_id)));
    assert!(notification_kinds(&out.events).contains(&NotificationKind::Waiting));
    assert_eq!(h.sessions.spawn_count(), 1);
}

#[test]
fn inject_writes_a_line_into_a_waiting_run() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let run_id = start_bound(&mut h.host, &project_id, "you/garden#1");
    h.sessions.last_session().unwrap().set_waiting(true);
    snapshot(&mut h.host);

    h.host
        .handle(serde_json::json!({
            "op": "injectRunInput",
            "runId": run_id,
            "text": "allow",
        }))
        .unwrap();

    let chunk = h
        .host
        .pty_output(&run_id, 0, Duration::from_millis(20))
        .unwrap();
    let text = String::from_utf8_lossy(&chunk.data);
    assert!(text.contains("allow\n"), "{text:?}");
    let snap = h.host.snapshot();
    let run = run_named(&snap, &run_id);
    assert_eq!(run.status, RunStatus::Running);
    assert!(run.waiting_for_user);
    assert_eq!(h.sessions.spawn_count(), 1);
}

#[test]
fn inject_into_ended_run_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let run_id = start_bound(&mut h.host, &project_id, "you/garden#1");
    h.host
        .handle(serde_json::json!({
            "op": "stopRun",
            "runId": run_id,
        }))
        .unwrap();

    let err = h
        .host
        .handle(serde_json::json!({
            "op": "injectRunInput",
            "runId": run_id,
            "text": "allow",
        }))
        .unwrap_err();
    assert!(
        matches!(err, KernelError::Protocol(ref message) if message.contains("unknown run")),
        "{err}"
    );
}

#[test]
fn memory_session_can_fail_inject() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let run_id = start_bound(&mut h.host, &project_id, "you/garden#1");
    h.sessions
        .last_session()
        .unwrap()
        .fail_next_write("pty refused");

    let err = h
        .host
        .handle(serde_json::json!({
            "op": "injectRunInput",
            "runId": run_id,
            "text": "allow",
        }))
        .unwrap_err();
    assert!(err.to_string().contains("pty refused"), "{err}");
}

#[test]
fn in_progress_list_has_running_waiting_and_execution_stopped() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    start_bound(&mut h.host, &project_id, "you/garden#1");
    start_bound(&mut h.host, &project_id, "you/garden#2");
    h.sessions.last_session().unwrap().set_waiting(true);
    let stopped_id = start_bound(&mut h.host, &project_id, "you/garden#3");
    h.host
        .handle(serde_json::json!({
            "op": "stopRun",
            "runId": stopped_id,
        }))
        .unwrap();
    snapshot(&mut h.host);

    let activities = in_progress(&h.host);
    assert_eq!(
        activities,
        vec![
            ("you/garden#1".into(), Some(IssueActivity::Running)),
            ("you/garden#2".into(), Some(IssueActivity::Waiting)),
            ("you/garden#3".into(), Some(IssueActivity::ExecutionStopped)),
        ]
    );
}

#[test]
fn normal_stop_and_waiting_do_not_start_self_check() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let run_id = start_bound(&mut h.host, &project_id, "you/garden#1");
    h.sessions.last_session().unwrap().set_waiting(true);
    snapshot(&mut h.host);
    assert_eq!(h.sessions.spawn_count(), 1);
    h.host
        .handle(serde_json::json!({
            "op": "injectRunInput",
            "runId": run_id,
            "text": "allow",
        }))
        .unwrap();
    assert_eq!(h.sessions.spawn_count(), 1);
    h.sessions.last_session().unwrap().set_waiting(false);
    h.sessions.last_session().unwrap().finish(0);
    snapshot(&mut h.host);
    assert_eq!(h.host.snapshot().runs.len(), 1);
    assert_eq!(
        h.host.snapshot().runs[0].ended_reason,
        Some(RunEndedReason::Exited)
    );
    assert_eq!(h.sessions.spawn_count(), 1);

    let run_id = start_bound(&mut h.host, &project_id, "you/garden#2");
    h.host
        .handle(serde_json::json!({
            "op": "stopRun",
            "runId": run_id,
        }))
        .unwrap();
    snapshot(&mut h.host);
    assert_eq!(h.sessions.spawn_count(), 2);
    assert_eq!(h.host.snapshot().runs.len(), 2);
}

#[test]
fn four_notification_kinds_include_jump_targets() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);

    let waiting_id = start_bound(&mut h.host, &project_id, "you/garden#1");
    h.sessions.last_session().unwrap().set_waiting(true);
    let waiting_out = snapshot(&mut h.host);
    let waiting = waiting_out
        .events
        .iter()
        .find_map(|event| match event {
            HostEvent::Notification {
                kind: NotificationKind::Waiting,
                run_id,
                issue_id,
                project_id: notified_project,
            } => Some((run_id.clone(), issue_id.clone(), notified_project.clone())),
            _ => None,
        })
        .expect("waiting notification");
    assert_eq!(waiting.0, waiting_id);
    assert_eq!(waiting.1.as_deref(), Some("you/garden#1"));
    assert_eq!(waiting.2, project_id);

    let completed_id = start_bound(&mut h.host, &project_id, "you/garden#2");
    h.sessions.last_session().unwrap().finish(0);
    let completed_out = snapshot(&mut h.host);
    assert!(notification_kinds(&completed_out.events).contains(&NotificationKind::Completed));
    assert!(completed_out.events.iter().any(|event| matches!(
        event,
        HostEvent::Notification {
            kind: NotificationKind::Completed,
            run_id,
            issue_id,
            ..
        } if run_id == &completed_id && issue_id.as_deref() == Some("you/garden#2")
    )));

    let stopped_id = start_bound(&mut h.host, &project_id, "you/garden#3");
    h.sessions.last_session().unwrap().finish(1);
    let stopped_out = snapshot(&mut h.host);
    assert!(notification_kinds(&stopped_out.events).contains(&NotificationKind::AbnormalStop));
    assert!(stopped_out.events.iter().any(|event| matches!(
        event,
        HostEvent::ExecutionStopped { issue_id, run_id }
            if issue_id == "you/garden#3" && run_id == &stopped_id
    )));
    assert!(stopped_out.events.iter().any(|event| matches!(
        event,
        HostEvent::Notification {
            kind: NotificationKind::AbnormalStop,
            run_id,
            issue_id,
            ..
        } if run_id == &stopped_id && issue_id.as_deref() == Some("you/garden#3")
    )));

    drop(h.host);
    let host = HostKernel::boot_with_ports(
        boot_req(tmp.path()),
        KernelPorts {
            tracker: Arc::clone(&h.tracker) as _,
            agents: vec![Arc::clone(&h.agent) as _],
            launch_env: Arc::new(MemoryLaunchEnv::with_path("/mem/bin")) as _,
            sessions: MemorySessionFactory::new(),
        },
    )
    .unwrap();
    let mut host = host;
    let crash_out = snapshot(&mut host);
    assert!(notification_kinds(&crash_out.events).contains(&NotificationKind::CrashRecovered));
    assert!(crash_out.events.iter().any(|event| matches!(
        event,
        HostEvent::HostCrashedRecovered { run_ids } if !run_ids.is_empty()
    )));
}

#[test]
fn user_stop_is_execution_stopped_without_abnormal_notification() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let run_id = start_bound(&mut h.host, &project_id, "you/garden#1");
    let out = h
        .host
        .handle(serde_json::json!({
            "op": "stopRun",
            "runId": run_id,
        }))
        .unwrap();
    assert!(out.events.iter().any(|event| matches!(
        event,
        HostEvent::ExecutionStopped { issue_id, run_id: id }
            if issue_id == "you/garden#1" && id == &run_id
    )));
    assert!(!notification_kinds(&out.events).contains(&NotificationKind::AbnormalStop));
    assert!(!notification_kinds(&out.events).contains(&NotificationKind::Completed));
}

#[test]
fn hidden_window_still_notifies_the_connected_client() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    h.host.dispatch(host_kernel::Command::HideWindow).unwrap();
    let run_id = start_bound(&mut h.host, &project_id, "you/garden#1");
    h.sessions.last_session().unwrap().set_waiting(true);
    let out = snapshot(&mut h.host);
    assert!(out
        .events
        .iter()
        .any(|event| matches!(event, HostEvent::Waiting { run_id: id } if id == &run_id)));
    assert!(notification_kinds(&out.events).contains(&NotificationKind::Waiting));
}

#[test]
fn notification_switches_persist_on_this_client() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    register(&mut h.host, &dir);
    assert!(h.host.snapshot().notify_desktop);
    assert!(h.host.snapshot().notify_sound);
    h.host
        .handle(serde_json::json!({
            "op": "setNotificationPrefs",
            "desktop": false,
            "sound": true,
        }))
        .unwrap();
    assert!(!h.host.snapshot().notify_desktop);
    assert!(h.host.snapshot().notify_sound);
    drop(h.host);

    let host = HostKernel::boot_with_ports(
        boot_req(tmp.path()),
        KernelPorts {
            tracker: Arc::clone(&h.tracker) as _,
            agents: vec![Arc::clone(&h.agent) as _],
            launch_env: Arc::new(MemoryLaunchEnv::with_path("/mem/bin")) as _,
            sessions: MemorySessionFactory::new(),
        },
    )
    .unwrap();
    assert!(!host.snapshot().notify_desktop);
    assert!(host.snapshot().notify_sound);
    assert_eq!(host.snapshot().copy.waiting, "等待操作");
    assert_eq!(host.snapshot().copy.running, "运行中");
}
