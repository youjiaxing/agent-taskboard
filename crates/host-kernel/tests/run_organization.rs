use std::path::Path;
use std::sync::{Arc, Barrier, Mutex};

use host_kernel::{
    BootRequest, GitHubTracker, HostKernel, IssueRecord, KernelPorts, MemoryAgent, MemoryLaunchEnv,
    MemorySessionFactory, MemoryTracker, ProjectConnection, RunRestoreConflictReason,
    RunRestoreResult, RunStatus, ScriptedGitHub, SystemAppearance, TelemetryLane, TelemetrySample,
    TokenCounts, WorkspaceView,
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
    agent: Arc<MemoryAgent>,
    sessions: Arc<MemorySessionFactory>,
}

fn harness(root: &Path) -> Harness {
    let agent = Arc::new(MemoryAgent::installed_grok());
    let sessions = MemorySessionFactory::new();
    let mut host = HostKernel::boot_with_ports(
        boot_req(root),
        KernelPorts {
            tracker: Arc::new(MemoryTracker::new()),
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

fn values() -> serde_json::Value {
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

fn start_run(host: &mut HostKernel, project_id: &str, client_id: Option<&str>) -> String {
    let mut request = serde_json::json!({
        "op": "startUnboundRun",
        "projectId": project_id,
        "agentId": "grok-build",
        "values": values(),
        "openingText": "organize run",
    });
    if let Some(client_id) = client_id {
        request["clientInstanceId"] = client_id.into();
    }
    host.handle(request).unwrap().snapshot.focused_run_id
}

fn stop_run(host: &mut HostKernel, run_id: &str) {
    host.handle(serde_json::json!({
        "op": "stopRun",
        "runId": run_id,
    }))
    .unwrap();
}

fn set_time(host: &mut HostKernel, now_ms: u64) {
    host.handle(serde_json::json!({ "op": "tick", "nowMs": now_ms }))
        .unwrap();
}

fn block_runs_file(path: &Path) -> Vec<u8> {
    let previous = std::fs::read(path).unwrap();
    std::fs::remove_file(path).unwrap();
    std::fs::create_dir(path).unwrap();
    previous
}

fn restore_runs_file(path: &Path, previous: &[u8]) {
    std::fs::remove_dir(path).unwrap();
    std::fs::write(path, previous).unwrap();
}

fn archive_and_remove(host: &mut HostKernel, dir: &Path) -> (String, String, String) {
    let project_id = register(host, dir);
    let run_id = start_run(host, &project_id, None);
    stop_run(host, &run_id);
    host.handle(serde_json::json!({ "op": "archiveRun", "runId": run_id }))
        .unwrap();
    host.handle(serde_json::json!({ "op": "removeProject", "projectId": project_id }))
        .unwrap();
    let prepared = host
        .handle(serde_json::json!({ "op": "prepareRestoreRun", "runId": run_id }))
        .unwrap()
        .run_restore
        .unwrap();
    let RunRestoreResult::ProjectRecreateRequired { tombstone, .. } = prepared else {
        panic!("expected project recreation preflight");
    };
    (project_id, run_id, tombstone.revision)
}

#[test]
fn legacy_runs_default_to_unpinned_and_unarchived() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let run_id = start_run(&mut h.host, &project_id, None);
    stop_run(&mut h.host, &run_id);
    let runs_path = h.host.snapshot().data.host_dir.join("runs.json");
    drop(h.host);

    let mut stored: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&runs_path).unwrap()).unwrap();
    let run = stored.as_array_mut().unwrap().first_mut().unwrap();
    run.as_object_mut().unwrap().remove("pinnedAtMs");
    run.as_object_mut().unwrap().remove("archivedAtMs");
    std::fs::write(&runs_path, serde_json::to_vec_pretty(&stored).unwrap()).unwrap();

    let mut host = HostKernel::boot_with_ports(
        boot_req(tmp.path()),
        KernelPorts {
            tracker: Arc::new(MemoryTracker::new()),
            agents: vec![Arc::clone(&h.agent) as _],
            launch_env: Arc::new(MemoryLaunchEnv::with_path("/mem/bin")) as _,
            sessions: Arc::clone(&h.sessions) as _,
        },
    )
    .unwrap();
    let run = &host.snapshot().runs[0];
    assert!(run.pinned_at_ms.is_none());
    assert!(run.archived_at_ms.is_none());
    assert!(host.snapshot().capabilities.run_organization);

    set_time(&mut host, T0 + 10);
    host.handle(serde_json::json!({
        "op": "setRunPinned",
        "runId": run_id,
        "pinned": true,
    }))
    .unwrap();
    host.handle(serde_json::json!({
        "op": "setRunPinned",
        "runId": run_id,
        "pinned": false,
    }))
    .unwrap();
    let rewritten: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(runs_path).unwrap()).unwrap();
    assert!(rewritten[0].get("pinnedAtMs").is_none());
    assert!(rewritten[0].get("archivedAtMs").is_none());
}

#[test]
fn active_and_waiting_runs_reject_archive_without_stopping() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let run_id = start_run(&mut h.host, &project_id, None);
    h.sessions.last_session().unwrap().set_waiting(true);
    h.host
        .handle(serde_json::json!({ "op": "snapshot" }))
        .unwrap();

    let err = h
        .host
        .handle(serde_json::json!({ "op": "archiveRun", "runId": run_id }))
        .unwrap_err();
    assert!(err.to_string().contains("only ended"));
    let snapshot = h.host.snapshot();
    let run = snapshot.runs.iter().find(|run| run.id == run_id).unwrap();
    assert_eq!(run.status, RunStatus::Running);
    assert!(run.waiting_for_user);
    assert!(!h.sessions.last_session().unwrap().stopped());
    h.host.write_pty(&run_id, b"still live").unwrap();
}

#[test]
fn pin_archive_restore_are_absolute_idempotent_and_archives_are_on_demand() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let run_id = start_run(&mut h.host, &project_id, None);
    stop_run(&mut h.host, &run_id);

    set_time(&mut h.host, T0 + 10);
    let pinned = h
        .host
        .handle(serde_json::json!({
            "op": "setRunPinned",
            "runId": run_id,
            "pinned": true,
        }))
        .unwrap();
    assert_eq!(pinned.snapshot.runs[0].pinned_at_ms, Some(T0 + 10));
    set_time(&mut h.host, T0 + 20);
    let repeated = h
        .host
        .handle(serde_json::json!({
            "op": "setRunPinned",
            "runId": run_id,
            "pinned": true,
        }))
        .unwrap();
    assert_eq!(repeated.snapshot.runs[0].pinned_at_ms, Some(T0 + 10));

    set_time(&mut h.host, T0 + 30);
    let archived = h
        .host
        .handle(serde_json::json!({ "op": "archiveRun", "runId": run_id }))
        .unwrap();
    assert!(archived.snapshot.runs.is_empty());
    let listed = h
        .host
        .handle(serde_json::json!({ "op": "listArchivedRuns" }))
        .unwrap()
        .archived_runs
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, run_id);
    assert!(listed[0].pinned_at_ms.is_none());
    assert_eq!(listed[0].archived_at_ms, Some(T0 + 30));

    set_time(&mut h.host, T0 + 40);
    h.host
        .handle(serde_json::json!({ "op": "archiveRun", "runId": run_id }))
        .unwrap();
    let repeated = h
        .host
        .handle(serde_json::json!({
            "op": "listArchivedRuns",
            "projectId": project_id,
        }))
        .unwrap()
        .archived_runs
        .unwrap();
    assert_eq!(repeated[0].archived_at_ms, Some(T0 + 30));
    let err = h
        .host
        .handle(serde_json::json!({
            "op": "setRunPinned",
            "runId": run_id,
            "pinned": true,
        }))
        .unwrap_err();
    assert!(err.to_string().contains("cannot be pinned"));

    set_time(&mut h.host, T0 + 50);
    let restored = h
        .host
        .handle(serde_json::json!({ "op": "restoreRun", "runId": run_id }))
        .unwrap();
    let run = restored
        .snapshot
        .runs
        .iter()
        .find(|run| run.id == run_id)
        .unwrap();
    assert_eq!(run.status, RunStatus::Ended);
    assert!(run.archived_at_ms.is_none());
    assert!(run.pinned_at_ms.is_none());
    h.host
        .handle(serde_json::json!({ "op": "restoreRun", "runId": run_id }))
        .unwrap();
}

#[test]
fn organization_write_failure_keeps_memory_unchanged_and_retryable() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let run_id = start_run(&mut h.host, &project_id, None);
    stop_run(&mut h.host, &run_id);
    let runs_path = h.host.snapshot().data.host_dir.join("runs.json");

    let previous = block_runs_file(&runs_path);
    let err = h
        .host
        .handle(serde_json::json!({
            "op": "setRunPinned",
            "runId": run_id,
            "pinned": true,
        }))
        .unwrap_err();
    assert!(!err.to_string().is_empty());
    assert!(h.host.snapshot().runs[0].pinned_at_ms.is_none());
    restore_runs_file(&runs_path, &previous);

    h.host
        .handle(serde_json::json!({
            "op": "setRunPinned",
            "runId": run_id,
            "pinned": true,
        }))
        .unwrap();
    let previous = block_runs_file(&runs_path);
    h.host
        .handle(serde_json::json!({ "op": "archiveRun", "runId": run_id }))
        .unwrap_err();
    let run = &h.host.snapshot().runs[0];
    assert!(run.archived_at_ms.is_none());
    assert!(run.pinned_at_ms.is_some());
    restore_runs_file(&runs_path, &previous);

    h.host
        .handle(serde_json::json!({ "op": "archiveRun", "runId": run_id }))
        .unwrap();
    let previous = block_runs_file(&runs_path);
    h.host
        .handle(serde_json::json!({ "op": "restoreRun", "runId": run_id }))
        .unwrap_err();
    assert!(h.host.snapshot().runs.is_empty());
    assert_eq!(
        h.host
            .handle(serde_json::json!({ "op": "listArchivedRuns" }))
            .unwrap()
            .archived_runs
            .unwrap()[0]
            .id,
        run_id
    );
    restore_runs_file(&runs_path, &previous);

    let restored = h
        .host
        .handle(serde_json::json!({ "op": "restoreRun", "runId": run_id }))
        .unwrap();
    assert_eq!(restored.snapshot.runs.len(), 1);
}

#[test]
fn restore_preflight_and_confirmation_recreate_the_original_project_identity() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let run_id = start_run(&mut h.host, &project_id, None);
    stop_run(&mut h.host, &run_id);
    h.host
        .handle(serde_json::json!({ "op": "archiveRun", "runId": run_id }))
        .unwrap();
    assert!(matches!(
        h.host
            .handle(serde_json::json!({ "op": "prepareRestoreRun", "runId": run_id }))
            .unwrap()
            .run_restore,
        Some(RunRestoreResult::Ready { .. })
    ));

    h.host
        .handle(serde_json::json!({ "op": "removeProject", "projectId": project_id }))
        .unwrap();
    let prepared = h
        .host
        .handle(serde_json::json!({ "op": "prepareRestoreRun", "runId": run_id }))
        .unwrap()
        .run_restore
        .unwrap();
    let RunRestoreResult::ProjectRecreateRequired { tombstone, .. } = prepared else {
        panic!("expected project recreation preflight");
    };
    assert_eq!(tombstone.project_id, project_id);
    assert_eq!(tombstone.name, "garden");
    assert_eq!(tombstone.local_path, dir);
    assert_eq!(tombstone.repository, "you/garden");
    let revision = tombstone.revision.clone();

    let confirmation_required = h
        .host
        .handle(serde_json::json!({ "op": "restoreRun", "runId": run_id }))
        .unwrap()
        .run_restore
        .unwrap();
    assert!(matches!(
        confirmation_required,
        RunRestoreResult::ProjectRecreateRequired { .. }
    ));
    let stale = h
        .host
        .handle(serde_json::json!({
            "op": "restoreRun",
            "runId": run_id,
            "confirmProjectRecreate": true,
            "expectedTombstoneRevision": "stale",
        }))
        .unwrap()
        .run_restore
        .unwrap();
    assert!(matches!(
        stale,
        RunRestoreResult::Conflict {
            reason: RunRestoreConflictReason::TombstoneRevisionChanged,
            ..
        }
    ));

    let restored = h
        .host
        .handle(serde_json::json!({
            "op": "restoreRun",
            "runId": run_id,
            "confirmProjectRecreate": true,
            "expectedTombstoneRevision": revision.clone(),
        }))
        .unwrap();
    assert!(matches!(
        restored.run_restore,
        Some(RunRestoreResult::Restored {
            project_recreated: true,
            ..
        })
    ));
    assert_eq!(restored.snapshot.projects.len(), 1);
    assert_eq!(restored.snapshot.projects[0].id, project_id);
    assert_eq!(restored.snapshot.runs.len(), 1);
    assert_eq!(restored.snapshot.runs[0].id, run_id);
    assert_eq!(restored.snapshot.runs[0].project_id, project_id);

    let settings: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&restored.snapshot.data.host_settings_path).unwrap(),
    )
    .unwrap();
    assert_eq!(settings["projects"][0]["id"], project_id);
    assert!(settings.get("projectTombstones").is_none());

    h.host
        .handle(serde_json::json!({ "op": "archiveRun", "runId": run_id }))
        .unwrap();
    h.host
        .handle(serde_json::json!({ "op": "removeProject", "projectId": project_id }))
        .unwrap();
    let next = h
        .host
        .handle(serde_json::json!({ "op": "prepareRestoreRun", "runId": run_id }))
        .unwrap()
        .run_restore
        .unwrap();
    let RunRestoreResult::ProjectRecreateRequired {
        tombstone: next_tombstone,
        ..
    } = next
    else {
        panic!("expected second project recreation preflight");
    };
    assert_ne!(next_tombstone.revision, revision);
    assert!(matches!(
        h.host
            .handle(serde_json::json!({
                "op": "restoreRun",
                "runId": run_id,
                "confirmProjectRecreate": true,
                "expectedTombstoneRevision": revision,
            }))
            .unwrap()
            .run_restore,
        Some(RunRestoreResult::Conflict {
            reason: RunRestoreConflictReason::TombstoneRevisionChanged,
            ..
        })
    ));
}

#[test]
fn restore_retries_after_project_commit_when_runs_write_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let (project_id, run_id, revision) = archive_and_remove(&mut h.host, &dir);
    let runs_path = h.host.snapshot().data.host_dir.join("runs.json");
    let previous = block_runs_file(&runs_path);

    h.host
        .handle(serde_json::json!({
            "op": "restoreRun",
            "runId": run_id,
            "confirmProjectRecreate": true,
            "expectedTombstoneRevision": revision,
        }))
        .unwrap_err();
    assert_eq!(h.host.snapshot().projects[0].id, project_id);
    assert!(h.host.snapshot().runs.is_empty());
    let settings: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(h.host.snapshot().data.host_settings_path).unwrap(),
    )
    .unwrap();
    assert_eq!(settings["projects"][0]["id"], project_id);
    assert!(settings.get("projectTombstones").is_none());

    restore_runs_file(&runs_path, &previous);
    let retried = h
        .host
        .handle(serde_json::json!({ "op": "restoreRun", "runId": run_id }))
        .unwrap();
    assert_eq!(retried.snapshot.projects.len(), 1);
    assert_eq!(retried.snapshot.projects[0].id, project_id);
    assert_eq!(retried.snapshot.runs[0].id, run_id);
}

#[test]
fn simultaneous_client_confirmations_converge_idempotently() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let (project_id, run_id, revision) = archive_and_remove(&mut h.host, &dir);
    let host = Arc::new(Mutex::new(h.host));
    let barrier = Arc::new(Barrier::new(3));

    let results = std::thread::scope(|scope| {
        let handles = ["desktop", "browser"].map(|client_id| {
            let host = Arc::clone(&host);
            let barrier = Arc::clone(&barrier);
            let run_id = run_id.clone();
            let revision = revision.clone();
            scope.spawn(move || {
                barrier.wait();
                host.lock()
                    .unwrap()
                    .handle(serde_json::json!({
                        "op": "restoreRun",
                        "clientInstanceId": client_id,
                        "runId": run_id,
                        "confirmProjectRecreate": true,
                        "expectedTombstoneRevision": revision,
                    }))
                    .unwrap()
                    .run_restore
                    .unwrap()
            })
        });
        barrier.wait();
        handles.map(|handle| handle.join().unwrap())
    });

    assert!(results
        .iter()
        .all(|result| matches!(result, RunRestoreResult::Restored { .. })));
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(
                result,
                RunRestoreResult::Restored {
                    project_recreated: true,
                    ..
                }
            ))
            .count(),
        1
    );
    let snapshot = host.lock().unwrap().snapshot();
    assert_eq!(snapshot.projects.len(), 1);
    assert_eq!(snapshot.projects[0].id, project_id);
    assert_eq!(snapshot.runs.len(), 1);
    assert_eq!(snapshot.runs[0].project_id, project_id);
}

#[test]
fn restore_preflight_reports_missing_directory_conflict_and_legacy_gap() {
    let missing_dir_tmp = tempfile::tempdir().unwrap();
    let missing_dir = make_dir(missing_dir_tmp.path(), "work/garden");
    let mut missing_dir_host = harness(missing_dir_tmp.path());
    let (_, missing_dir_run, _) = archive_and_remove(&mut missing_dir_host.host, &missing_dir);
    std::fs::remove_dir_all(&missing_dir).unwrap();
    assert!(matches!(
        missing_dir_host
            .host
            .handle(serde_json::json!({ "op": "prepareRestoreRun", "runId": missing_dir_run }))
            .unwrap()
            .run_restore,
        Some(RunRestoreResult::Conflict {
            reason: RunRestoreConflictReason::DirectoryMissing,
            ..
        })
    ));

    let conflict_tmp = tempfile::tempdir().unwrap();
    let conflict_dir = make_dir(conflict_tmp.path(), "work/garden");
    let mut conflict_host = harness(conflict_tmp.path());
    let (old_project_id, conflict_run, _) =
        archive_and_remove(&mut conflict_host.host, &conflict_dir);
    let conflicting_project_id = register(&mut conflict_host.host, &conflict_dir);
    let conflict = conflict_host
        .host
        .handle(serde_json::json!({ "op": "prepareRestoreRun", "runId": conflict_run }))
        .unwrap()
        .run_restore
        .unwrap();
    assert!(matches!(
        conflict,
        RunRestoreResult::Conflict {
            reason: RunRestoreConflictReason::DirectoryInUse,
            conflicting_project_id: Some(ref id),
            ..
        } if id == &conflicting_project_id
    ));
    assert_ne!(old_project_id, conflicting_project_id);

    let legacy_tmp = tempfile::tempdir().unwrap();
    let legacy_dir = make_dir(legacy_tmp.path(), "work/garden");
    let mut legacy = harness(legacy_tmp.path());
    let (_, legacy_run, _) = archive_and_remove(&mut legacy.host, &legacy_dir);
    let settings_path = legacy.host.snapshot().data.host_settings_path.clone();
    let mut settings: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&settings_path).unwrap()).unwrap();
    settings["projectTombstones"] = serde_json::json!([]);
    std::fs::write(
        &settings_path,
        serde_json::to_vec_pretty(&settings).unwrap(),
    )
    .unwrap();
    let agent = Arc::clone(&legacy.agent);
    let sessions = Arc::clone(&legacy.sessions);
    drop(legacy.host);
    let mut rebooted = HostKernel::boot_with_ports(
        boot_req(legacy_tmp.path()),
        KernelPorts {
            tracker: Arc::new(MemoryTracker::new()),
            agents: vec![agent],
            launch_env: Arc::new(MemoryLaunchEnv::with_path("/mem/bin")) as _,
            sessions,
        },
    )
    .unwrap();
    assert!(matches!(
        rebooted
            .handle(serde_json::json!({ "op": "prepareRestoreRun", "runId": legacy_run }))
            .unwrap()
            .run_restore,
        Some(RunRestoreResult::Conflict {
            reason: RunRestoreConflictReason::TombstoneMissing,
            ..
        })
    ));
    assert!(rebooted.snapshot().projects.is_empty());
}

#[test]
fn active_project_tombstone_conflict_is_visible_and_blocks_restore() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let (project_id, run_id, revision) = archive_and_remove(&mut h.host, &dir);
    let settings_path = h.host.snapshot().data.host_settings_path.clone();
    let mut settings: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&settings_path).unwrap()).unwrap();
    let mut active = settings["projectTombstones"][0].clone();
    active.as_object_mut().unwrap().remove("revision");
    settings["projects"] = serde_json::json!([active]);
    settings["focusedProjectId"] = serde_json::json!(project_id);
    std::fs::write(
        &settings_path,
        serde_json::to_vec_pretty(&settings).unwrap(),
    )
    .unwrap();
    let bytes_before = std::fs::read(&settings_path).unwrap();
    let agent = Arc::clone(&h.agent);
    let sessions = Arc::clone(&h.sessions);
    drop(h.host);

    let mut host = HostKernel::boot_with_ports(
        boot_req(tmp.path()),
        KernelPorts {
            tracker: Arc::new(MemoryTracker::new()),
            agents: vec![agent],
            launch_env: Arc::new(MemoryLaunchEnv::with_path("/mem/bin")) as _,
            sessions,
        },
    )
    .unwrap();
    assert_eq!(host.snapshot().data_conflicts.len(), 1);
    assert!(matches!(
        host.handle(serde_json::json!({ "op": "prepareRestoreRun", "runId": run_id }))
            .unwrap()
            .run_restore,
        Some(RunRestoreResult::Conflict {
            reason: RunRestoreConflictReason::ActiveProjectTombstoneConflict,
            ..
        })
    ));
    assert!(matches!(
        host.handle(serde_json::json!({
            "op": "restoreRun",
            "runId": run_id,
            "confirmProjectRecreate": true,
            "expectedTombstoneRevision": revision,
        }))
        .unwrap()
        .run_restore,
        Some(RunRestoreResult::Conflict {
            reason: RunRestoreConflictReason::ActiveProjectTombstoneConflict,
            ..
        })
    ));
    assert_eq!(std::fs::read(settings_path).unwrap(), bytes_before);
    assert!(host.snapshot().runs.is_empty());
}

#[test]
fn tracker_unreachable_does_not_block_project_recreation() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let agent = Arc::new(MemoryAgent::installed_grok());
    let sessions = MemorySessionFactory::new();
    let tracker = Arc::new(GitHubTracker::scripted(ScriptedGitHub {
        env: [("GH_TOKEN".into(), "token".into())].into(),
        accept_tokens: ["token".into()].into(),
        unreachable: true,
        ..Default::default()
    }));
    let mut host = HostKernel::boot_with_ports(
        boot_req(tmp.path()),
        KernelPorts {
            tracker,
            agents: vec![agent],
            launch_env: Arc::new(MemoryLaunchEnv::with_path("/mem/bin")) as _,
            sessions,
        },
    )
    .unwrap();
    let (project_id, run_id, revision) = archive_and_remove(&mut host, &dir);
    let restored = host
        .handle(serde_json::json!({
            "op": "restoreRun",
            "runId": run_id,
            "confirmProjectRecreate": true,
            "expectedTombstoneRevision": revision,
        }))
        .unwrap();
    assert_eq!(restored.snapshot.projects[0].id, project_id);
    assert!(matches!(
        restored.snapshot.projects[0].connection,
        ProjectConnection::Unreachable { .. }
    ));
    assert_eq!(restored.snapshot.runs[0].id, run_id);
}

#[test]
fn archiving_clears_host_and_every_client_navigation() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let run_id = start_run(&mut h.host, &project_id, None);
    stop_run(&mut h.host, &run_id);
    h.host
        .handle(serde_json::json!({ "op": "focusRun", "runId": run_id }))
        .unwrap();
    for client_id in ["desktop", "browser"] {
        h.host
            .handle(serde_json::json!({
                "op": "focusRun",
                "clientInstanceId": client_id,
                "runId": run_id,
            }))
            .unwrap();
    }

    h.host
        .handle(serde_json::json!({
            "op": "archiveRun",
            "clientInstanceId": "desktop",
            "runId": run_id,
        }))
        .unwrap();

    let host = h.host.snapshot();
    assert!(host.focused_run_id.is_empty());
    assert_eq!(host.workspace_view, WorkspaceView::Project);
    for client_id in ["desktop", "browser"] {
        let snapshot = h
            .host
            .handle(serde_json::json!({
                "op": "snapshot",
                "clientInstanceId": client_id,
            }))
            .unwrap()
            .snapshot;
        assert!(snapshot.focused_run_id.is_empty());
        assert_eq!(snapshot.workspace_view, WorkspaceView::Project);
    }
}

#[test]
fn archived_runs_reject_old_navigation_and_control_without_moving_focus() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let archived_id = start_run(&mut h.host, &project_id, None);
    stop_run(&mut h.host, &archived_id);
    h.host
        .handle(serde_json::json!({ "op": "archiveRun", "runId": archived_id }))
        .unwrap();
    let live_id = start_run(&mut h.host, &project_id, None);
    h.host
        .handle(serde_json::json!({ "op": "focusRun", "runId": live_id }))
        .unwrap();

    for request in [
        serde_json::json!({ "op": "focusRun", "runId": archived_id }),
        serde_json::json!({ "op": "stopRun", "runId": archived_id }),
        serde_json::json!({ "op": "injectRunInput", "runId": archived_id, "text": "no" }),
        serde_json::json!({ "op": "openUsageForRun", "runId": archived_id }),
        serde_json::json!({ "op": "openRunFromUsage", "runId": archived_id }),
        serde_json::json!({ "op": "viewChanges", "runId": archived_id }),
    ] {
        let err = h.host.handle(request).unwrap_err();
        assert!(err.to_string().contains("archived"), "{err}");
        let snapshot = h.host.snapshot();
        assert_eq!(snapshot.focused_run_id, live_id);
        assert_eq!(snapshot.workspace_view, WorkspaceView::Run);
    }
}

#[test]
fn archived_run_samples_stay_in_totals_but_leave_usage_rows() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let run_id = start_run(&mut h.host, &project_id, None);
    h.agent.push_telemetry(TelemetrySample {
        run_id: run_id.clone(),
        project_id: String::new(),
        agent_id: String::new(),
        model: "grok-4.6".into(),
        lane: TelemetryLane::Main,
        tokens: TokenCounts {
            input: Some(7),
            output: Some(5),
            cache_read: None,
            cache_write: None,
            reasoning: None,
            total: Some(12),
        },
        ttft_ms: Some(100),
        tokens_per_sec: Some(50),
        at_ms: T0,
    });
    h.host
        .handle(serde_json::json!({ "op": "snapshot" }))
        .unwrap();
    stop_run(&mut h.host, &run_id);
    h.host
        .handle(serde_json::json!({ "op": "archiveRun", "runId": run_id }))
        .unwrap();

    let archived = h
        .host
        .handle(serde_json::json!({ "op": "listArchivedRuns" }))
        .unwrap()
        .archived_runs
        .unwrap();
    assert_eq!(archived[0].telemetry.len(), 1);

    let usage = h
        .host
        .handle(serde_json::json!({ "op": "openUsage" }))
        .unwrap()
        .snapshot
        .usage;
    assert!(usage.runs.is_empty());
    assert_eq!(usage.totals.input, Some(7));
    assert_eq!(usage.totals.total, Some(12));
}

#[test]
fn issue_links_hide_archived_runs_but_continue_keeps_the_history() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "organized work"));
    let agent = Arc::new(MemoryAgent::installed_grok());
    let sessions = MemorySessionFactory::new();
    let mut host = HostKernel::boot_with_ports(
        boot_req(tmp.path()),
        KernelPorts {
            tracker,
            agents: vec![agent],
            launch_env: Arc::new(MemoryLaunchEnv::with_path("/mem/bin")) as _,
            sessions,
        },
    )
    .unwrap();
    host.handle(serde_json::json!({ "op": "tick", "nowMs": T0 }))
        .unwrap();
    let project_id = register(&mut host, &dir);
    let started = host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
            "issueId": "you/garden#1",
            "agentId": "grok-build",
            "values": values(),
            "openingText": "bound work",
        }))
        .unwrap();
    let run_id = started.snapshot.focused_run_id.clone();
    stop_run(&mut host, &run_id);
    host.handle(serde_json::json!({
        "op": "focusIssue",
        "issueId": "you/garden#1",
    }))
    .unwrap();
    host.handle(serde_json::json!({ "op": "archiveRun", "runId": run_id }))
        .unwrap();

    let board = host.snapshot().board.unwrap();
    let selected = board.selected.unwrap();
    assert!(selected.active_run_id.is_none());
    let card = board
        .columns
        .unwrap()
        .in_progress
        .into_iter()
        .find(|card| card.id == "you/garden#1")
        .unwrap();
    assert!(card.run_id.is_none());
    assert!(selected.execution_stopped);

    let continued = host
        .handle(serde_json::json!({
            "op": "continueRun",
            "issueId": "you/garden#1",
        }))
        .unwrap();
    let next = continued
        .snapshot
        .runs
        .iter()
        .find(|run| run.status == RunStatus::Running)
        .unwrap();
    assert_eq!(next.previous_run_id.as_deref(), Some(run_id.as_str()));
}
