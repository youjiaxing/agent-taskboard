use std::path::Path;
use std::sync::Arc;

use host_kernel::{
    BootRequest, HostEvent, HostKernel, IssueRecord, KernelError, KernelPorts, MemoryAgent,
    MemoryLaunchEnv, MemorySessionFactory, MemoryTracker, RunStatus, SystemAppearance,
    PENDING_CONFIRM_MS,
};

const T0: u64 = 1_000_000;

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

fn harness(root: &Path, issues: Vec<IssueRecord>) -> Harness {
    let tracker = Arc::new(MemoryTracker::new());
    for issue in issues {
        tracker.add_issue(issue);
    }
    let agent = Arc::new(MemoryAgent::installed_grok());
    let sessions = MemorySessionFactory::new();
    let mut host = HostKernel::boot_with_ports(
        boot_req(root),
        KernelPorts {
            tracker: Arc::clone(&tracker) as _,
            agents: vec![Arc::clone(&agent) as _],
            launch_env: Arc::new(MemoryLaunchEnv::with_path("/mem/bin")) as _,
            sessions: Arc::clone(&sessions) as _,
        },
    )
    .unwrap();
    host.handle(serde_json::json!({ "op": "tick", "nowMs": T0 }))
        .unwrap();
    Harness {
        host,
        tracker,
        agent,
        sessions,
    }
}

fn ready(number: u64, title: &str) -> IssueRecord {
    IssueRecord::open("you/garden", number, title).label("ready-for-agent")
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

fn enable_both(host: &mut HostKernel, project_id: &str) {
    host.handle(serde_json::json!({
        "op": "setHostAutoAdvance",
        "enabled": true,
    }))
    .unwrap();
    host.handle(serde_json::json!({
        "op": "setProjectAutoAdvance",
        "projectId": project_id,
        "enabled": true,
    }))
    .unwrap();
}

fn finish_normal(h: &mut Harness, number: u64) {
    h.sessions.last_session().unwrap().set_session_end(true);
    h.tracker.close_issue("you/garden", number);
    h.sessions.last_session().unwrap().finish(0);
    h.host
        .handle(serde_json::json!({ "op": "snapshot" }))
        .unwrap();
}

fn claimed(host: &HostKernel, issue_id: &str) -> Vec<String> {
    host.snapshot()
        .board
        .as_ref()
        .and_then(|board| board.selected.as_ref().filter(|item| item.id == issue_id))
        .map(|item| item.claimed_by.clone())
        .unwrap_or_else(|| {
            host.snapshot()
                .board
                .as_ref()
                .and_then(|board| board.columns.as_ref())
                .and_then(|columns| {
                    columns
                        .in_progress
                        .iter()
                        .chain(columns.frontier.iter())
                        .chain(columns.recently_completed.iter())
                        .find(|card| card.id == issue_id)
                        .map(|card| card.claimed_by.clone())
                })
                .unwrap_or_default()
        })
}

fn active_issue_ids(host: &HostKernel) -> Vec<String> {
    host.snapshot()
        .runs
        .iter()
        .filter(|run| run.status == RunStatus::Running)
        .filter_map(|run| run.issue_id.clone())
        .collect()
}

#[test]
fn both_switches_must_be_on_before_pending_and_claim() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path(), vec![ready(1, "first"), ready(2, "second")]);
    let project_id = register(&mut h.host, &dir);
    assert!(!h.host.snapshot().auto_advance);
    assert!(!h.host.snapshot().projects[0].auto_advance);

    start_bound(&mut h.host, &project_id, "you/garden#1");
    finish_normal(&mut h, 1);
    assert!(h.host.snapshot().pending_confirmation.is_none());
    assert_eq!(h.sessions.spawn_count(), 1);
    assert!(claimed(&h.host, "you/garden#2").is_empty());

    h.host
        .handle(serde_json::json!({
            "op": "setHostAutoAdvance",
            "enabled": true,
        }))
        .unwrap();
    let mut only_host = harness(
        &tmp.path().join("only-host"),
        vec![ready(1, "first"), ready(2, "second")],
    );
    let only_host_dir = make_dir(tmp.path(), "work/only-host");
    let only_host_id = register(&mut only_host.host, &only_host_dir);
    only_host
        .host
        .handle(serde_json::json!({
            "op": "setHostAutoAdvance",
            "enabled": true,
        }))
        .unwrap();
    start_bound(&mut only_host.host, &only_host_id, "you/garden#1");
    finish_normal(&mut only_host, 1);
    assert!(only_host.host.snapshot().pending_confirmation.is_none());
    assert_eq!(only_host.sessions.spawn_count(), 1);

    let mut only_project = harness(
        &tmp.path().join("only-project"),
        vec![ready(1, "first"), ready(2, "second")],
    );
    let only_project_dir = make_dir(tmp.path(), "work/only-project");
    let only_project_id = register(&mut only_project.host, &only_project_dir);
    only_project
        .host
        .handle(serde_json::json!({
            "op": "setProjectAutoAdvance",
            "projectId": only_project_id,
            "enabled": true,
        }))
        .unwrap();
    start_bound(&mut only_project.host, &only_project_id, "you/garden#1");
    finish_normal(&mut only_project, 1);
    assert!(only_project.host.snapshot().pending_confirmation.is_none());
    assert_eq!(only_project.sessions.spawn_count(), 1);

    let mut both = harness(
        &tmp.path().join("both"),
        vec![ready(1, "first"), ready(2, "second")],
    );
    let both_dir = make_dir(tmp.path(), "work/both");
    let both_id = register(&mut both.host, &both_dir);
    enable_both(&mut both.host, &both_id);
    assert!(both.host.snapshot().auto_advance);
    assert!(both.host.snapshot().projects[0].auto_advance);
    start_bound(&mut both.host, &both_id, "you/garden#1");
    finish_normal(&mut both, 1);
    let pending = both
        .host
        .snapshot()
        .pending_confirmation
        .expect("pending confirmation");
    assert_eq!(pending.issue_id, "you/garden#1");
    assert_eq!(pending.remaining_ms, PENDING_CONFIRM_MS);
    assert_eq!(both.sessions.spawn_count(), 1);
    assert!(claimed(&both.host, "you/garden#2").is_empty());

    both.host
        .handle(serde_json::json!({
            "op": "tick",
            "nowMs": T0 + PENDING_CONFIRM_MS,
        }))
        .unwrap();
    assert!(both.host.snapshot().pending_confirmation.is_none());
    assert_eq!(both.sessions.spawn_count(), 2);
    assert_eq!(claimed(&both.host, "you/garden#2"), vec!["me"]);
    assert!(active_issue_ids(&both.host).contains(&"you/garden#2".into()));
}

#[test]
fn pending_confirmation_veto_does_not_claim_next() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path(), vec![ready(1, "first"), ready(2, "second")]);
    let project_id = register(&mut h.host, &dir);
    enable_both(&mut h.host, &project_id);
    start_bound(&mut h.host, &project_id, "you/garden#1");
    finish_normal(&mut h, 1);
    assert!(h.host.snapshot().pending_confirmation.is_some());

    let out = h
        .host
        .handle(serde_json::json!({
            "op": "vetoPendingConfirmation",
            "projectId": project_id,
        }))
        .unwrap();
    assert!(out.snapshot.pending_confirmation.is_none());
    assert!(out.events.iter().any(|event| matches!(
        event,
        HostEvent::PendingConfirmationEnded {
            advanced: false,
            ..
        }
    )));

    h.host
        .handle(serde_json::json!({
            "op": "tick",
            "nowMs": T0 + PENDING_CONFIRM_MS,
        }))
        .unwrap();
    assert_eq!(h.sessions.spawn_count(), 1);
    assert!(claimed(&h.host, "you/garden#2").is_empty());
}

#[test]
fn auto_pool_skips_grilling_prototype_and_triage_roles() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(
        tmp.path(),
        vec![
            ready(1, "done"),
            ready(2, "grill").label("grilling"),
            ready(3, "proto").label("prototype"),
            IssueRecord::open("you/garden", 4, "info")
                .label("ready-for-agent")
                .label("needs-info"),
            IssueRecord::open("you/garden", 5, "human").label("ready-for-human"),
            IssueRecord::open("you/garden", 6, "triage").label("needs-triage"),
            ready(7, "next agent"),
        ],
    );
    let project_id = register(&mut h.host, &dir);
    enable_both(&mut h.host, &project_id);
    start_bound(&mut h.host, &project_id, "you/garden#1");
    finish_normal(&mut h, 1);
    h.host
        .handle(serde_json::json!({
            "op": "tick",
            "nowMs": T0 + PENDING_CONFIRM_MS,
        }))
        .unwrap();
    assert_eq!(h.sessions.spawn_count(), 2);
    assert_eq!(claimed(&h.host, "you/garden#7"), vec!["me"]);
    assert!(claimed(&h.host, "you/garden#2").is_empty());
    assert!(claimed(&h.host, "you/garden#3").is_empty());
    assert!(claimed(&h.host, "you/garden#4").is_empty());
    assert!(claimed(&h.host, "you/garden#5").is_empty());
    assert!(claimed(&h.host, "you/garden#6").is_empty());
    assert!(active_issue_ids(&h.host).contains(&"you/garden#7".into()));
}

#[test]
fn self_check_stops_when_still_open_or_abnormal() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path(), vec![ready(1, "open work"), ready(2, "next")]);
    let project_id = register(&mut h.host, &dir);
    enable_both(&mut h.host, &project_id);
    start_bound(&mut h.host, &project_id, "you/garden#1");
    h.sessions.last_session().unwrap().set_session_end(true);
    h.sessions.last_session().unwrap().finish(0);
    h.host
        .handle(serde_json::json!({ "op": "snapshot" }))
        .unwrap();
    assert_eq!(h.sessions.spawn_count(), 2);
    assert!(active_issue_ids(&h.host).contains(&"you/garden#1".into()));
    assert!(h.host.snapshot().pending_confirmation.is_none());

    h.sessions.last_session().unwrap().set_session_end(true);
    h.sessions.last_session().unwrap().finish(0);
    h.host
        .handle(serde_json::json!({ "op": "snapshot" }))
        .unwrap();
    assert_eq!(h.sessions.spawn_count(), 2);
    assert!(!active_issue_ids(&h.host).contains(&"you/garden#1".into()));
    assert!(h.host.snapshot().pending_confirmation.is_none());
    assert!(claimed(&h.host, "you/garden#2").is_empty());
}

#[test]
fn read_failure_stops_before_claim_or_advance() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path(), vec![ready(1, "first"), ready(2, "second")]);
    let project_id = register(&mut h.host, &dir);
    enable_both(&mut h.host, &project_id);
    start_bound(&mut h.host, &project_id, "you/garden#1");
    finish_normal(&mut h, 1);
    assert!(h.host.snapshot().pending_confirmation.is_some());
    h.tracker.fail_read("you/garden");

    h.host
        .handle(serde_json::json!({
            "op": "tick",
            "nowMs": T0 + PENDING_CONFIRM_MS,
        }))
        .unwrap();
    assert_eq!(h.sessions.spawn_count(), 1);
    assert!(claimed(&h.host, "you/garden#2").is_empty());

    let err = h
        .host
        .handle(serde_json::json!({
            "op": "autoAdvance",
            "projectId": project_id,
        }))
        .unwrap_err();
    assert!(matches!(err, KernelError::Denied(_)));
    assert_eq!(h.sessions.spawn_count(), 1);
}

#[test]
fn missing_hooks_or_user_stop_does_not_advance() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path(), vec![ready(1, "first"), ready(2, "second")]);
    h.agent.fail_attach_hooks();
    let project_id = register(&mut h.host, &dir);
    enable_both(&mut h.host, &project_id);
    start_bound(&mut h.host, &project_id, "you/garden#1");
    finish_normal(&mut h, 1);
    assert!(h.host.snapshot().pending_confirmation.is_none());
    assert_eq!(h.sessions.spawn_count(), 1);

    let mut stopped = harness(
        &tmp.path().join("stopped"),
        vec![ready(1, "first"), ready(2, "second")],
    );
    let stopped_dir = make_dir(tmp.path(), "work/stopped");
    let stopped_id = register(&mut stopped.host, &stopped_dir);
    enable_both(&mut stopped.host, &stopped_id);
    let run_id = start_bound(&mut stopped.host, &stopped_id, "you/garden#1");
    stopped
        .host
        .handle(serde_json::json!({
            "op": "stopRun",
            "runId": run_id,
        }))
        .unwrap();
    assert!(stopped.host.snapshot().pending_confirmation.is_none());
    assert_eq!(stopped.sessions.spawn_count(), 1);
}

#[test]
fn closed_with_hook_abnormal_opens_view_changes_and_does_not_reopen() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path(), vec![ready(1, "first"), ready(2, "second")]);
    let project_id = register(&mut h.host, &dir);
    enable_both(&mut h.host, &project_id);
    start_bound(&mut h.host, &project_id, "you/garden#1");
    h.sessions.last_session().unwrap().set_stop_failure(true);
    h.tracker.close_issue("you/garden", 1);
    h.sessions.last_session().unwrap().finish(0);
    let out = h
        .host
        .handle(serde_json::json!({ "op": "snapshot" }))
        .unwrap();
    assert!(out.view_changes.is_some());
    assert!(out.snapshot.pending_confirmation.is_none());
    assert_eq!(h.sessions.spawn_count(), 1);
    assert!(claimed(&h.host, "you/garden#2").is_empty());
    h.host
        .handle(serde_json::json!({
            "op": "focusIssue",
            "issueId": "you/garden#1",
        }))
        .unwrap();
    assert!(!h.host.snapshot().board.unwrap().selected.unwrap().open);
}

#[test]
fn stop_failure_while_running_injects_self_check() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path(), vec![ready(1, "open work")]);
    let project_id = register(&mut h.host, &dir);
    enable_both(&mut h.host, &project_id);
    let run_id = start_bound(&mut h.host, &project_id, "you/garden#1");
    h.sessions.last_session().unwrap().set_stop_failure(true);
    h.host
        .handle(serde_json::json!({ "op": "snapshot" }))
        .unwrap();
    assert_eq!(h.sessions.spawn_count(), 1);
    let chunk = h
        .host
        .pty_output(&run_id, 0, std::time::Duration::from_millis(20))
        .unwrap();
    let text = String::from_utf8_lossy(&chunk.data);
    assert!(text.contains("请检查当前工作"), "{text}");
}

#[test]
fn switches_survive_reboot_but_cold_start_waits_restore() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path(), vec![ready(1, "first"), ready(2, "second")]);
    let project_id = register(&mut h.host, &dir);
    enable_both(&mut h.host, &project_id);
    h.host
        .handle(serde_json::json!({
            "op": "setProjectRestoreAutoAdvance",
            "projectId": project_id,
            "enabled": true,
        }))
        .unwrap();
    drop(h.host);

    let mut host = HostKernel::boot_with_ports(
        boot_req(tmp.path()),
        KernelPorts {
            tracker: Arc::clone(&h.tracker) as _,
            agents: vec![Arc::clone(&h.agent) as _],
            launch_env: Arc::new(MemoryLaunchEnv::with_path("/mem/bin")) as _,
            sessions: Arc::clone(&h.sessions) as _,
        },
    )
    .unwrap();
    assert!(host.snapshot().auto_advance);
    assert!(host.snapshot().projects[0].auto_advance);
    assert!(host.snapshot().projects[0].restore_auto_advance);
    let boot_ms = match host.snapshot().board.unwrap().refresh {
        host_kernel::RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
        other => panic!("expected ready after reboot, got {other:?}"),
    };

    start_bound(&mut host, &project_id, "you/garden#1");
    h.sessions.last_session().unwrap().set_session_end(true);
    h.tracker.close_issue("you/garden", 1);
    h.sessions.last_session().unwrap().finish(0);
    host.handle(serde_json::json!({ "op": "snapshot" }))
        .unwrap();
    assert!(host.snapshot().pending_confirmation.is_none());
    assert_eq!(h.sessions.spawn_count(), 1);

    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": boot_ms + 60_000,
    }))
    .unwrap();
    let run_id = start_bound(&mut host, &project_id, "you/garden#2");
    h.sessions.last_session().unwrap().set_session_end(true);
    h.tracker.close_issue("you/garden", 2);
    h.sessions.last_session().unwrap().finish(0);
    host.handle(serde_json::json!({ "op": "snapshot" }))
        .unwrap();
    let pending = host
        .snapshot()
        .pending_confirmation
        .expect("pending after restore delay");
    assert_eq!(pending.run_id, run_id);
}
