use std::path::Path;
use std::sync::Arc;

use host_kernel::{
    BoardEmptyReason, BootRequest, HostEvent, HostKernel, IssueRecord, KernelError, MemoryTracker,
    RefreshStatus, SystemAppearance, DEFAULT_REFRESH_INTERVAL_MS,
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

fn boot(root: &Path, tracker: Arc<MemoryTracker>) -> HostKernel {
    HostKernel::boot_with(boot_req(root), tracker).unwrap()
}

fn register(host: &mut HostKernel, dir: &Path, name: &str, repository: &str) -> String {
    host.handle(serde_json::json!({
        "op": "registerProject",
        "name": name,
        "localPath": dir,
        "repository": repository,
    }))
    .unwrap()
    .snapshot
    .focused_project_id
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

fn refresh_status(host: &HostKernel) -> RefreshStatus {
    host.snapshot().board.unwrap().refresh
}

fn snapshot_path(host: &HostKernel, project_id: &str) -> std::path::PathBuf {
    host.snapshot()
        .data
        .host_dir
        .join("projects")
        .join(project_id)
        .join("tracker-snapshot")
}

#[test]
fn never_fetched_project_does_not_draw_four_columns() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.fail_read("you/garden");
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, &dir, "garden", "you/garden");
    let board = host.snapshot().board.unwrap();
    assert_eq!(board.empty, Some(BoardEmptyReason::NoData));
    assert!(board.columns.is_none());
    assert_eq!(board.refresh, RefreshStatus::NeverFetched);
}

#[test]
fn successful_refresh_persists_last_data_and_keeps_it_when_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    let project_id = register(&mut host, &dir, "garden", "you/garden");

    let path = snapshot_path(&host, &project_id);
    assert!(path.is_file());
    match refresh_status(&host) {
        RefreshStatus::Ready { fetched_at_ms, .. } => assert!(fetched_at_ms > 0),
        other => panic!("expected ready, got {other:?}"),
    }
    assert_eq!(frontier_ids(&host), vec!["you/garden#1"]);

    let before = std::fs::read(&path).unwrap();
    tracker.fail_read("you/garden");
    host.handle(serde_json::json!({ "op": "refresh" })).unwrap();
    match refresh_status(&host) {
        RefreshStatus::Offline { fetched_at_ms, .. } => assert!(fetched_at_ms > 0),
        other => panic!("expected offline, got {other:?}"),
    }
    assert_eq!(frontier_ids(&host), vec!["you/garden#1"]);
    assert_eq!(std::fs::read(&path).unwrap(), before);

    drop(host);
    let host = boot(tmp.path(), tracker);
    assert_eq!(frontier_ids(&host), vec!["you/garden#1"]);
    match refresh_status(&host) {
        RefreshStatus::Offline { .. } | RefreshStatus::Ready { .. } => {}
        other => panic!("expected last data after reboot, got {other:?}"),
    }
}

#[test]
fn refresh_emits_refreshing_then_a_terminal_status() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, &dir, "garden", "you/garden");
    let out = host.handle(serde_json::json!({ "op": "refresh" })).unwrap();
    let kinds: Vec<_> = out
        .events
        .iter()
        .filter_map(|event| match event {
            HostEvent::RefreshStatusChanged { status, .. } => Some(status.kind()),
            _ => None,
        })
        .collect();
    assert_eq!(kinds.first().copied(), Some("refreshing"));
    assert_eq!(kinds.last().copied(), Some("ready"));
}

#[test]
fn opening_focusing_foreground_and_manual_refresh_pull_immediately() {
    let tmp = tempfile::tempdir().unwrap();
    let garden = make_dir(tmp.path(), "work/garden");
    let notes = make_dir(tmp.path(), "work/notes");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    tracker.add_issue(IssueRecord::open("you/notes", 2, "note"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    let garden_id = register(&mut host, &garden, "garden", "you/garden");
    let notes_id = host
        .handle(serde_json::json!({
            "op": "registerProject",
            "name": "notes",
            "localPath": notes,
            "repository": "you/notes",
        }))
        .unwrap()
        .snapshot
        .projects
        .iter()
        .find(|project| project.name == "notes")
        .unwrap()
        .id
        .clone();

    let after_register_garden = tracker.read_count("you/garden");
    let after_register_notes = tracker.read_count("you/notes");
    assert!(after_register_garden >= 1);
    assert!(after_register_notes >= 1);

    host.handle(serde_json::json!({
        "op": "focusProject",
        "projectId": garden_id,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), after_register_garden + 1);

    host.handle(serde_json::json!({ "op": "hideWindow" }))
        .unwrap();
    host.handle(serde_json::json!({ "op": "showWindow" }))
        .unwrap();
    assert_eq!(tracker.read_count("you/garden"), after_register_garden + 2);

    host.handle(serde_json::json!({
        "op": "refresh",
        "projectId": notes_id,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/notes"), after_register_notes + 1);
    assert_eq!(tracker.read_count("you/garden"), after_register_garden + 2);
}

#[test]
fn visible_project_polls_every_sixty_seconds_and_hidden_does_not() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    register(&mut host, &dir, "garden", "you/garden");
    let fetched = match refresh_status(&host) {
        RefreshStatus::Ready {
            fetched_at_ms,
            next_refresh_in_ms,
        } => {
            assert_eq!(next_refresh_in_ms, Some(DEFAULT_REFRESH_INTERVAL_MS));
            fetched_at_ms
        }
        other => panic!("expected ready, got {other:?}"),
    };
    let baseline = tracker.read_count("you/garden");

    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + 30_000,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), baseline);
    match refresh_status(&host) {
        RefreshStatus::Ready {
            next_refresh_in_ms, ..
        } => assert_eq!(next_refresh_in_ms, Some(30_000)),
        other => panic!("expected countdown, got {other:?}"),
    }

    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + DEFAULT_REFRESH_INTERVAL_MS,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), baseline + 1);

    host.handle(serde_json::json!({ "op": "hideWindow" }))
        .unwrap();
    let hidden_at = match refresh_status(&host) {
        RefreshStatus::Ready {
            fetched_at_ms,
            next_refresh_in_ms,
        } => {
            assert!(next_refresh_in_ms.is_none());
            fetched_at_ms
        }
        other => panic!("expected ready without countdown, got {other:?}"),
    };
    let after_hide = tracker.read_count("you/garden");
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": hidden_at + DEFAULT_REFRESH_INTERVAL_MS,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), after_hide);
}

#[test]
fn another_visible_client_can_keep_polling_when_window_is_hidden() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    let project_id = register(&mut host, &dir, "garden", "you/garden");
    host.handle(serde_json::json!({ "op": "hideWindow" }))
        .unwrap();
    host.handle(serde_json::json!({
        "op": "setClientView",
        "clientId": "phone",
        "projectId": project_id,
        "visible": true,
    }))
    .unwrap();
    let fetched = match refresh_status(&host) {
        RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
        other => panic!("expected ready, got {other:?}"),
    };
    let baseline = tracker.read_count("you/garden");
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + DEFAULT_REFRESH_INTERVAL_MS,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), baseline + 1);
}

#[test]
fn run_end_refreshes_even_when_nobody_is_watching() {
    let tmp = tempfile::tempdir().unwrap();
    let garden = make_dir(tmp.path(), "work/garden");
    let notes = make_dir(tmp.path(), "work/notes");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    tracker.add_issue(IssueRecord::open("you/notes", 2, "note"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    let garden_id = register(&mut host, &garden, "garden", "you/garden");
    host.handle(serde_json::json!({
        "op": "registerProject",
        "name": "notes",
        "localPath": notes,
        "repository": "you/notes",
    }))
    .unwrap();
    host.handle(serde_json::json!({ "op": "hideWindow" }))
        .unwrap();
    let garden_reads = tracker.read_count("you/garden");
    let notes_reads = tracker.read_count("you/notes");
    host.handle(serde_json::json!({
        "op": "noteRunEnded",
        "projectId": garden_id,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), garden_reads + 1);
    assert_eq!(tracker.read_count("you/notes"), notes_reads);
}

#[test]
fn claim_close_check_and_auto_advance_only_refresh_the_involved_project() {
    let tmp = tempfile::tempdir().unwrap();
    let garden = make_dir(tmp.path(), "work/garden");
    let notes = make_dir(tmp.path(), "work/notes");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    tracker.add_issue(IssueRecord::open("you/notes", 2, "note"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    let garden_id = register(&mut host, &garden, "garden", "you/garden");
    host.handle(serde_json::json!({
        "op": "registerProject",
        "name": "notes",
        "localPath": notes,
        "repository": "you/notes",
    }))
    .unwrap();
    host.handle(serde_json::json!({
        "op": "focusProject",
        "projectId": garden_id,
    }))
    .unwrap();
    let garden_reads = tracker.read_count("you/garden");
    let notes_reads = tracker.read_count("you/notes");

    host.handle(serde_json::json!({
        "op": "claimIssue",
        "issueId": "you/garden#1",
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), garden_reads + 1);
    assert_eq!(tracker.read_count("you/notes"), notes_reads);
    assert!(frontier_ids(&host).is_empty());

    host.handle(serde_json::json!({
        "op": "checkIssueClosed",
        "issueId": "you/garden#1",
    }))
    .unwrap();
    host.handle(serde_json::json!({
        "op": "autoAdvance",
        "projectId": garden_id,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), garden_reads + 3);
    assert_eq!(tracker.read_count("you/notes"), notes_reads);
}

#[test]
fn last_data_is_not_used_to_claim_or_advance_when_read_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    let project_id = register(&mut host, &dir, "garden", "you/garden");
    let path = snapshot_path(&host, &project_id);
    let before = std::fs::read(&path).unwrap();
    tracker.fail_read("you/garden");

    let claim = host
        .handle(serde_json::json!({
            "op": "claimIssue",
            "issueId": "you/garden#1",
        }))
        .unwrap_err();
    assert!(matches!(claim, KernelError::Denied(message) if message.contains("offline")));
    let release = host
        .handle(serde_json::json!({
            "op": "releaseIssue",
            "issueId": "you/garden#1",
        }))
        .unwrap_err();
    assert!(matches!(release, KernelError::Denied(_)));
    let advance = host
        .handle(serde_json::json!({
            "op": "autoAdvance",
            "projectId": project_id,
        }))
        .unwrap_err();
    assert!(matches!(advance, KernelError::Denied(_)));
    let bound = host
        .handle(serde_json::json!({
            "op": "startBoundRun",
            "issueId": "you/garden#1",
        }))
        .unwrap_err();
    assert!(matches!(bound, KernelError::Denied(_)));

    let started = host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
        }))
        .unwrap();
    let run = started.snapshot.runs.first().expect("offline unbound Run");
    assert!(run.unbound);
    let run_id = run.id.clone();
    host.handle(serde_json::json!({
        "op": "injectRunInput",
        "runId": run_id,
        "text": "yes",
    }))
    .unwrap();
    host.handle(serde_json::json!({
        "op": "stopRun",
        "runId": run_id,
    }))
    .unwrap();

    assert_eq!(frontier_ids(&host), vec!["you/garden#1"]);
    assert_eq!(std::fs::read(&path).unwrap(), before);

    tracker.clear_read_script("you/garden");
    host.handle(serde_json::json!({ "op": "refresh" })).unwrap();
    assert_eq!(frontier_ids(&host), vec!["you/garden#1"]);
}

#[test]
fn rate_limit_pauses_auto_refresh_and_is_not_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    register(&mut host, &dir, "garden", "you/garden");
    let fetched = match refresh_status(&host) {
        RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
        other => panic!("expected ready, got {other:?}"),
    };
    tracker.fail_rate_limited("you/garden", Some(120_000));
    host.handle(serde_json::json!({ "op": "refresh" })).unwrap();
    match refresh_status(&host) {
        RefreshStatus::RateLimited {
            fetched_at_ms,
            retry_at_ms,
        } => {
            assert_eq!(fetched_at_ms, Some(fetched));
            assert_eq!(retry_at_ms, Some(fetched + 120_000));
        }
        other => panic!("expected rate-limited, got {other:?}"),
    }
    assert_eq!(frontier_ids(&host), vec!["you/garden#1"]);
    let claim = host
        .handle(serde_json::json!({
            "op": "claimIssue",
            "issueId": "you/garden#1",
        }))
        .unwrap_err();
    assert!(matches!(claim, KernelError::Denied(message) if message.contains("rate-limited")));
    let after_limit = tracker.read_count("you/garden");
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + DEFAULT_REFRESH_INTERVAL_MS,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), after_limit);

    tracker.clear_read_script("you/garden");
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + 120_000,
    }))
    .unwrap();
    match refresh_status(&host) {
        RefreshStatus::Ready { .. } => {}
        other => panic!("expected ready after retry-after, got {other:?}"),
    }
}

#[test]
fn rate_limit_without_retry_after_stays_paused_until_manual_success() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    register(&mut host, &dir, "garden", "you/garden");
    let fetched = match refresh_status(&host) {
        RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
        other => panic!("expected ready, got {other:?}"),
    };
    tracker.fail_rate_limited("you/garden", None);
    host.handle(serde_json::json!({ "op": "refresh" })).unwrap();
    let after_limit = tracker.read_count("you/garden");
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + DEFAULT_REFRESH_INTERVAL_MS * 3,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), after_limit);
    tracker.clear_read_script("you/garden");
    host.handle(serde_json::json!({ "op": "refresh" })).unwrap();
    match refresh_status(&host) {
        RefreshStatus::Ready { .. } => {}
        other => panic!("expected manual recovery, got {other:?}"),
    }
}

#[test]
fn auth_failure_is_project_degraded_not_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    register(&mut host, &dir, "garden", "you/garden");
    tracker.fail_auth("you/garden");
    host.handle(serde_json::json!({ "op": "refresh" })).unwrap();
    assert!(matches!(
        refresh_status(&host),
        RefreshStatus::AuthFailed { .. }
    ));
    assert!(matches!(
        host.snapshot().projects[0].connection,
        host_kernel::ProjectConnection::AuthFailed { .. }
    ));
    assert_eq!(frontier_ids(&host), vec!["you/garden#1"]);
    let claim = host
        .handle(serde_json::json!({
            "op": "claimIssue",
            "issueId": "you/garden#1",
        }))
        .unwrap_err();
    assert!(matches!(claim, KernelError::Denied(message) if message.contains("auth")));
}

#[test]
fn check_issue_closed_uses_live_read_not_last_data() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    register(&mut host, &dir, "garden", "you/garden");
    assert_eq!(frontier_ids(&host), vec!["you/garden#1"]);

    tracker.set_issues(
        "you/garden",
        vec![IssueRecord::open("you/garden", 1, "ready").closed_at("2026-08-22T12:00:00Z")],
    );
    host.handle(serde_json::json!({
        "op": "checkIssueClosed",
        "issueId": "you/garden#1",
    }))
    .unwrap();
    let columns = host.snapshot().board.unwrap().columns.unwrap();
    assert!(columns.frontier.is_empty());
    assert_eq!(columns.recently_completed[0].id, "you/garden#1");
}

#[test]
fn refresh_interval_is_configurable_and_survives_reboot() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    register(&mut host, &dir, "garden", "you/garden");
    host.handle(serde_json::json!({
        "op": "setRefreshInterval",
        "intervalMs": 15_000,
    }))
    .unwrap();
    let fetched = match refresh_status(&host) {
        RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
        other => panic!("expected ready, got {other:?}"),
    };
    let baseline = tracker.read_count("you/garden");
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + 15_000,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), baseline + 1);

    drop(host);
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    match refresh_status(&host) {
        RefreshStatus::Ready {
            next_refresh_in_ms, ..
        } => assert_eq!(next_refresh_in_ms, Some(15_000)),
        other => panic!("expected persisted 15s interval, got {other:?}"),
    }
    let fetched = match refresh_status(&host) {
        RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
        other => panic!("expected ready, got {other:?}"),
    };
    let baseline = tracker.read_count("you/garden");
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + 15_000,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), baseline + 1);
}

#[test]
fn claim_without_last_data_still_live_reads_the_focused_project() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.fail_read("you/garden");
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, &dir, "garden", "you/garden");
    let err = host
        .handle(serde_json::json!({
            "op": "claimIssue",
            "issueId": "you/garden#1",
        }))
        .unwrap_err();
    assert!(matches!(err, KernelError::Denied(message) if message.contains("never-fetched")));
}
