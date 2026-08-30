use std::path::Path;
use std::sync::Arc;

use host_kernel::{
    BootRequest, HostKernel, IssueRecord, KernelError, KernelPorts, MemoryAgent, MemoryLaunchEnv,
    MemorySessionFactory, MemoryTracker, RunEndedReason, RunStatus, SystemAppearance,
    WorkspaceView,
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
        "permission-mode": "default",
        "always-approve": "false",
        "sandbox": "off",
        "initial-instruction": "",
        "additional-args": ""
    })
}

fn start_bound_from_form(
    host: &mut HostKernel,
    project_id: &str,
    issue_id: &str,
) -> Result<host_kernel::CommandOutcome, KernelError> {
    host.handle(serde_json::json!({
        "op": "startUnboundRun",
        "projectId": project_id,
        "issueId": issue_id,
        "agentId": "grok-build",
        "values": grok_values(),
        "openingText": "ready work\nhttps://github.com/you/garden/issues/1",
    }))
}

fn claimed_by(host: &mut HostKernel, issue_id: &str) -> Vec<String> {
    host.handle(serde_json::json!({
        "op": "focusIssue",
        "issueId": issue_id,
    }))
    .unwrap();
    host.snapshot().board.unwrap().selected.unwrap().claimed_by
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

fn in_progress_ids(host: &HostKernel) -> Vec<String> {
    host.snapshot()
        .board
        .unwrap()
        .columns
        .unwrap()
        .in_progress
        .iter()
        .map(|card| card.id.clone())
        .collect()
}

#[test]
fn claim_failure_does_not_start_a_bound_run() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    h.tracker.fail_claim("you/garden");

    let err = start_bound_from_form(&mut h.host, &project_id, "you/garden#1").unwrap_err();
    assert!(
        matches!(&err, KernelError::Denied(message) if message.contains("cannot claim")),
        "{err}"
    );
    assert!(h.host.snapshot().runs.is_empty());
    assert_eq!(h.sessions.spawn_count(), 0);
    assert!(claimed_by(&mut h.host, "you/garden#1").is_empty());
    assert_eq!(frontier_ids(&h.host), vec!["you/garden#1"]);
}

#[test]
fn starting_a_bound_run_claims_and_leaves_frontier() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);

    let out = start_bound_from_form(&mut h.host, &project_id, "you/garden#1").unwrap();
    assert_eq!(out.snapshot.runs.len(), 1);
    let run = &out.snapshot.runs[0];
    assert_eq!(run.status, RunStatus::Running);
    assert_eq!(run.issue_id.as_deref(), Some("you/garden#1"));
    assert!(!run.unbound);
    assert_eq!(h.sessions.spawn_count(), 1);
    assert_eq!(claimed_by(&mut h.host, "you/garden#1"), vec!["me"]);
    assert!(frontier_ids(&h.host).is_empty());
    assert_eq!(in_progress_ids(&h.host), vec!["you/garden#1"]);
}

#[test]
fn start_bound_run_command_does_not_choose_an_agent_implicitly() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    register(&mut h.host, &dir);
    let err = h
        .host
        .handle(serde_json::json!({
            "op": "startBoundRun",
            "issueId": "you/garden#1",
        }))
        .unwrap_err();
    assert!(
        matches!(&err, KernelError::Denied(message) if message.contains("choose an Agent")),
        "{err}"
    );
    assert!(h.host.snapshot().runs.is_empty());
    assert_eq!(
        claimed_by(&mut h.host, "you/garden#1"),
        Vec::<String>::new()
    );
    assert_eq!(h.sessions.spawn_count(), 0);
}

#[test]
fn bound_opening_is_title_and_stable_url() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let form = h
        .host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
            "issueId": "you/garden#1",
            "agentId": "grok-build",
        }))
        .unwrap()
        .snapshot
        .launch_form
        .unwrap();
    assert_eq!(
        form.opening_text,
        "ready work\nhttps://github.com/you/garden/issues/1"
    );
    assert_eq!(
        form.values.get("initial-instruction").map(String::as_str),
        Some("ready work\nhttps://github.com/you/garden/issues/1")
    );
    assert!(!form.opening_text.contains("##"));
}

#[test]
fn one_active_run_per_issue() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    start_bound_from_form(&mut h.host, &project_id, "you/garden#1").unwrap();
    let err = start_bound_from_form(&mut h.host, &project_id, "you/garden#1").unwrap_err();
    assert!(
        matches!(&err, KernelError::Denied(message) if message.contains("active Run")),
        "{err}"
    );
    assert_eq!(h.host.snapshot().runs.len(), 1);
    assert_eq!(h.sessions.spawn_count(), 1);
}

#[test]
fn offline_form_start_does_not_claim_or_spawn() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    h.tracker.fail_read("you/garden");
    let err = start_bound_from_form(&mut h.host, &project_id, "you/garden#1").unwrap_err();
    assert!(
        matches!(&err, KernelError::Denied(message) if message.contains("offline")),
        "{err}"
    );
    assert!(h.host.snapshot().runs.is_empty());
    assert_eq!(h.sessions.spawn_count(), 0);
    h.tracker.clear_read_script("you/garden");
    assert!(claimed_by(&mut h.host, "you/garden#1").is_empty());
}

#[test]
fn abnormal_end_is_execution_stopped() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    start_bound_from_form(&mut h.host, &project_id, "you/garden#1").unwrap();
    h.sessions.last_session().unwrap().finish(1);
    h.host
        .handle(serde_json::json!({ "op": "snapshot" }))
        .unwrap();
    assert_eq!(
        h.host.snapshot().runs[0].ended_reason,
        Some(RunEndedReason::Abnormal)
    );
    h.host
        .handle(serde_json::json!({
            "op": "focusIssue",
            "issueId": "you/garden#1",
        }))
        .unwrap();
    assert!(
        h.host
            .snapshot()
            .board
            .unwrap()
            .selected
            .unwrap()
            .execution_stopped
    );
    assert!(frontier_ids(&h.host).is_empty());
}

#[test]
fn closing_an_issue_does_not_stop_the_run() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let run_id = start_bound_from_form(&mut h.host, &project_id, "you/garden#1")
        .unwrap()
        .snapshot
        .runs[0]
        .id
        .clone();
    h.tracker.close_issue("you/garden", 1);
    h.host
        .handle(serde_json::json!({ "op": "refresh" }))
        .unwrap();
    let run = h
        .host
        .snapshot()
        .runs
        .into_iter()
        .find(|run| run.id == run_id)
        .unwrap();
    assert_eq!(run.status, RunStatus::Running);
}

#[test]
fn focusing_an_issue_with_an_active_run_focuses_that_pty() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let run_id = start_bound_from_form(&mut h.host, &project_id, "you/garden#1")
        .unwrap()
        .snapshot
        .runs[0]
        .id
        .clone();
    let out = h
        .host
        .handle(serde_json::json!({
            "op": "focusIssue",
            "issueId": "you/garden#1",
        }))
        .unwrap();
    assert_eq!(out.snapshot.focused_run_id, run_id);
    assert_eq!(
        out.snapshot
            .board
            .unwrap()
            .selected
            .unwrap()
            .active_run_id
            .as_deref(),
        Some(run_id.as_str())
    );
}

#[test]
fn focus_run_command_focuses_the_bound_issue_and_pty() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let run_id = start_bound_from_form(&mut h.host, &project_id, "you/garden#1")
        .unwrap()
        .snapshot
        .runs[0]
        .id
        .clone();

    let out = h
        .host
        .handle(serde_json::json!({
            "op": "focusRun",
            "runId": run_id,
        }))
        .unwrap();

    assert_eq!(out.snapshot.focused_run_id, run_id);
    assert_eq!(out.snapshot.focused_project_id, project_id);
    assert_eq!(out.snapshot.workspace_view, WorkspaceView::Run);
    assert_eq!(
        out.snapshot.board.unwrap().selected.unwrap().id,
        "you/garden#1"
    );
}

#[test]
fn returning_to_board_keeps_the_issue_and_restores_its_pty() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    start_bound_from_form(&mut h.host, &project_id, "you/garden#1").unwrap();
    h.host
        .handle(serde_json::json!({
            "op": "focusIssue",
            "issueId": "you/garden#1",
        }))
        .unwrap();

    let out = h
        .host
        .handle(serde_json::json!({ "op": "returnToBoard" }))
        .unwrap();

    assert_eq!(out.snapshot.workspace_view, WorkspaceView::Project);
    assert!(!out.snapshot.focused_run_id.is_empty());
    assert_eq!(
        out.snapshot.board.unwrap().selected.unwrap().id,
        "you/garden#1"
    );
}

#[test]
fn focusing_an_issue_without_an_active_run_hides_the_pty() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    start_bound_from_form(&mut h.host, &project_id, "you/garden#1").unwrap();
    h.tracker
        .add_issue(IssueRecord::open("you/garden", 2, "other work"));
    h.host
        .handle(serde_json::json!({ "op": "refresh" }))
        .unwrap();

    let out = h
        .host
        .handle(serde_json::json!({
            "op": "focusIssue",
            "issueId": "you/garden#2",
        }))
        .unwrap();

    assert_eq!(out.snapshot.workspace_view, WorkspaceView::Project);
    assert!(out.snapshot.focused_run_id.is_empty());
    assert_eq!(
        out.snapshot.board.unwrap().selected.unwrap().id,
        "you/garden#2"
    );
}

#[test]
fn host_overview_is_a_host_view_and_keeps_all_project_runs() {
    let tmp = tempfile::tempdir().unwrap();
    let first_dir = make_dir(tmp.path(), "work/garden");
    let second_dir = make_dir(tmp.path(), "work/tools");
    let mut h = harness(tmp.path());
    let first_project = register(&mut h.host, &first_dir);
    start_bound_from_form(&mut h.host, &first_project, "you/garden#1").unwrap();
    h.tracker
        .add_issue(IssueRecord::open("you/tools", 1, "tool work"));
    let second_project = h
        .host
        .handle(serde_json::json!({
            "op": "registerProject",
            "name": "tools",
            "localPath": second_dir,
            "repository": "you/tools",
        }))
        .unwrap()
        .snapshot
        .projects
        .into_iter()
        .find(|project| project.repository == "you/tools")
        .unwrap()
        .id;
    start_bound_from_form(&mut h.host, &second_project, "you/tools#1").unwrap();

    let out = h
        .host
        .handle(serde_json::json!({ "op": "openHostOverview" }))
        .unwrap();

    assert_eq!(out.snapshot.workspace_view, WorkspaceView::HostOverview);
    assert_eq!(out.snapshot.runs.len(), 2);
    assert!(out.snapshot.focused_run_id.is_empty());
}

#[test]
fn continue_links_previous_run_and_resumes_native_session() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    h.agent.set_native_session_id(Some("sess-1".into()));
    let project_id = register(&mut h.host, &dir);
    let first = start_bound_from_form(&mut h.host, &project_id, "you/garden#1")
        .unwrap()
        .snapshot
        .runs[0]
        .clone();
    assert_eq!(first.native_session_id.as_deref(), Some("sess-1"));
    h.host
        .handle(serde_json::json!({
            "op": "stopRun",
            "runId": first.id,
        }))
        .unwrap();
    h.host
        .handle(serde_json::json!({
            "op": "focusIssue",
            "issueId": "you/garden#1",
        }))
        .unwrap();
    let selected = h.host.snapshot().board.unwrap().selected.unwrap();
    assert!(selected.execution_stopped);
    assert!(frontier_ids(&h.host).is_empty());

    let out = h
        .host
        .handle(serde_json::json!({
            "op": "continueRun",
            "issueId": "you/garden#1",
        }))
        .unwrap();
    assert_eq!(out.snapshot.runs.len(), 2);
    let continued = out
        .snapshot
        .runs
        .iter()
        .find(|run| run.id != first.id)
        .unwrap();
    assert_eq!(continued.status, RunStatus::Running);
    assert_eq!(
        continued.previous_run_id.as_deref(),
        Some(first.id.as_str())
    );
    assert_eq!(continued.issue_id.as_deref(), Some("you/garden#1"));
    let spawn = h.sessions.last_spawn().unwrap();
    assert!(spawn
        .argv
        .windows(2)
        .any(|pair| pair == ["--resume", "sess-1"]));
}

#[test]
fn execution_stopped_can_release_claim() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let run_id = start_bound_from_form(&mut h.host, &project_id, "you/garden#1")
        .unwrap()
        .snapshot
        .runs[0]
        .id
        .clone();
    h.host
        .handle(serde_json::json!({
            "op": "stopRun",
            "runId": run_id,
        }))
        .unwrap();
    assert_eq!(
        h.host.snapshot().runs[0].ended_reason,
        Some(RunEndedReason::Stopped)
    );
    h.host
        .handle(serde_json::json!({
            "op": "releaseIssue",
            "issueId": "you/garden#1",
        }))
        .unwrap();
    assert!(claimed_by(&mut h.host, "you/garden#1").is_empty());
    assert_eq!(frontier_ids(&h.host), vec!["you/garden#1"]);
    assert!(
        !h.host
            .snapshot()
            .board
            .unwrap()
            .selected
            .unwrap()
            .execution_stopped
    );
}

#[test]
fn remove_project_with_execution_stopped_keeps_tracker_claim() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let run_id = start_bound_from_form(&mut h.host, &project_id, "you/garden#1")
        .unwrap()
        .snapshot
        .runs[0]
        .id
        .clone();
    h.host
        .handle(serde_json::json!({
            "op": "stopRun",
            "runId": run_id,
        }))
        .unwrap();
    assert!(h.host.snapshot().projects[0].has_execution_stopped);
    h.host
        .handle(serde_json::json!({
            "op": "removeProject",
            "projectId": project_id,
        }))
        .unwrap();
    assert!(h.host.snapshot().projects.is_empty());
    assert_eq!(h.tracker.assignees("you/garden", 1), vec!["me"]);
}

#[test]
fn host_crash_marks_bound_run_execution_stopped() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    start_bound_from_form(&mut h.host, &project_id, "you/garden#1").unwrap();
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
    assert_eq!(host.snapshot().runs.len(), 1);
    let run = &host.snapshot().runs[0];
    assert_eq!(run.status, RunStatus::Ended);
    assert_eq!(run.ended_reason, Some(RunEndedReason::Crash));
    let mut host = host;
    host.handle(serde_json::json!({
        "op": "focusIssue",
        "issueId": "you/garden#1",
    }))
    .unwrap();
    assert!(
        host.snapshot()
            .board
            .unwrap()
            .selected
            .unwrap()
            .execution_stopped
    );
    assert!(host.snapshot().projects[0].has_execution_stopped);
}
