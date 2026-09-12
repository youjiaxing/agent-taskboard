use std::path::{Path, PathBuf};
use std::sync::Arc;

use host_kernel::{
    BootRequest, HostKernel, IssueRecord, KernelPorts, Language, MemoryAgent, MemoryLaunchEnv,
    MemorySessionFactory, MemoryTracker, RunStatus, SystemAppearance, CODEX_BIN, CODEX_ID,
    CODEX_NAME,
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

fn make_dir(root: &Path, name: &str) -> PathBuf {
    let dir = root.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn mark_git(dir: &Path) {
    std::fs::create_dir_all(dir.join(".git")).unwrap();
}

fn init_git(dir: &Path) {
    let status = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir)
        .status()
        .unwrap();
    assert!(status.success());
}

fn git_worktree_count(dir: &Path) -> usize {
    let output = std::process::Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(dir)
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| line.starts_with("worktree "))
        .count()
}

struct Harness {
    host: HostKernel,
    tracker: Arc<MemoryTracker>,
    agent: Arc<MemoryAgent>,
    sessions: Arc<MemorySessionFactory>,
}

fn harness_with(root: &Path, agent: MemoryAgent) -> Harness {
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready work"));
    let agent = Arc::new(agent);
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

fn harness(root: &Path) -> Harness {
    harness_with(root, MemoryAgent::installed_grok())
}

fn reboot(h: Harness, root: &Path) -> Harness {
    let Harness {
        host,
        tracker,
        agent,
        ..
    } = h;
    drop(host);
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

fn start_unbound(
    host: &mut HostKernel,
    project_id: &str,
    isolation: bool,
    opening: &str,
) -> host_kernel::CommandOutcome {
    let mut values = grok_values();
    values["isolation"] = serde_json::json!(if isolation { "true" } else { "false" });
    host.handle(serde_json::json!({
        "op": "startUnboundRun",
        "projectId": project_id,
        "agentId": "grok-build",
        "values": values,
        "openingText": opening,
    }))
    .unwrap()
}

fn start_bound(
    host: &mut HostKernel,
    project_id: &str,
    isolation: bool,
) -> host_kernel::CommandOutcome {
    let mut values = grok_values();
    values["isolation"] = serde_json::json!(if isolation { "true" } else { "false" });
    host.handle(serde_json::json!({
        "op": "startUnboundRun",
        "projectId": project_id,
        "issueId": "you/garden#1",
        "agentId": "grok-build",
        "values": values,
        "openingText": "ready work\nhttps://github.com/you/garden/issues/1",
    }))
    .unwrap()
}

#[test]
fn isolation_is_off_by_default_when_adapter_can_build_a_tree() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    mark_git(&dir);
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);

    let form = h
        .host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
            "agentId": "grok-build",
        }))
        .unwrap()
        .snapshot
        .launch_form
        .unwrap();
    assert!(form.isolation_supported);
    assert!(form.isolation_reason.is_empty());
    assert_ne!(
        form.values.get("isolation").map(String::as_str),
        Some("true")
    );
    assert_eq!(form.working_directory, dir.display().to_string());
}

#[test]
fn isolation_stays_off_for_a_bound_issue_and_is_not_remembered() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    mark_git(&dir);
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
    assert!(form.isolation_supported);
    assert_ne!(
        form.values.get("isolation").map(String::as_str),
        Some("true")
    );

    let mut values = grok_values();
    values["isolation"] = serde_json::json!("true");
    h.host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
            "agentId": "grok-build",
            "values": values,
            "openingText": "isolated once",
        }))
        .unwrap();

    let stored = std::fs::read_to_string(tmp.path().join("host/settings.json")).unwrap();
    assert!(!stored.contains("isolation"));

    let form = h
        .host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
            "agentId": "grok-build",
        }))
        .unwrap()
        .snapshot
        .launch_form
        .unwrap();
    assert_ne!(
        form.values.get("isolation").map(String::as_str),
        Some("true")
    );
}

#[test]
fn isolation_is_disabled_without_native_capability() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    mark_git(&dir);
    let mut h = harness_with(
        tmp.path(),
        MemoryAgent::installed(CODEX_ID, CODEX_NAME, CODEX_BIN),
    );
    let project_id = register(&mut h.host, &dir);

    let form = h
        .host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
            "agentId": "codex",
        }))
        .unwrap()
        .snapshot
        .launch_form
        .unwrap();
    assert!(!form.isolation_supported);
    assert!(form.isolation_reason.contains("没有原生隔离"));
    assert!(!form.isolation_reason.contains("留给隔离票"));
}

#[test]
fn isolation_is_disabled_when_the_project_is_not_git() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);

    let form = h
        .host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
            "agentId": "grok-build",
        }))
        .unwrap()
        .snapshot
        .launch_form
        .unwrap();
    assert!(!form.isolation_supported);
    assert!(form.isolation_reason.contains("git"));
    assert!(!form.isolation_reason.contains("留给隔离票"));
}

#[test]
fn default_run_uses_the_project_directory_without_worktree_flag() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    mark_git(&dir);
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let out = start_unbound(&mut h.host, &project_id, false, "use main");
    let run = &out.snapshot.runs[0];
    assert_eq!(run.status, RunStatus::Running);
    assert!(!run.isolated);
    assert_eq!(run.working_directory, dir.display().to_string());
    let spawn = h.sessions.last_spawn().unwrap();
    assert_eq!(spawn.cwd, dir);
    assert!(!spawn.argv.iter().any(|arg| arg == "--worktree"));
}

#[test]
fn isolated_run_passes_worktree_and_does_not_add_a_tree() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    init_git(&dir);
    let tree = make_dir(tmp.path(), "work/garden-iso");
    let mut h = harness(tmp.path());
    h.agent.set_isolation_tree(Some(tree.clone()));
    let project_id = register(&mut h.host, &dir);
    let before = git_worktree_count(&dir);

    let out = start_unbound(&mut h.host, &project_id, true, "isolate this");
    let run = &out.snapshot.runs[0];
    assert_eq!(run.status, RunStatus::Running);
    assert!(run.isolated);
    assert_eq!(run.working_directory, tree.display().to_string());
    let spawn = h.sessions.last_spawn().unwrap();
    assert_eq!(spawn.cwd, dir);
    assert!(spawn.argv.iter().any(|arg| arg == "--worktree"));
    assert_eq!(git_worktree_count(&dir), before);
}

#[test]
fn delayed_native_tree_is_discovered_before_changes_or_continue_use_it() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    init_git(&dir);
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let first = start_bound(&mut h.host, &project_id, true).snapshot.runs[0].clone();
    let changes = h
        .host
        .handle(serde_json::json!({"op":"viewChanges", "runId":first.id}));
    let changes = changes.unwrap().view_changes.unwrap();
    assert!(
        !changes.available,
        "must not show the main directory while isolation is unresolved"
    );
    assert!(changes.repos.is_empty());
    assert!(changes
        .unavailable_reason
        .unwrap()
        .contains("尚未确认隔离执行目录"));

    let tree = make_dir(tmp.path(), "work/late-tree");
    h.agent.set_isolation_tree(Some(tree.clone()));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        let snap = h
            .host
            .handle(serde_json::json!({"op":"snapshot"}))
            .unwrap()
            .snapshot;
        if snap.runs[0].working_directory == tree.display().to_string() {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "late native tree was not adopted"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    h.host
        .handle(serde_json::json!({"op":"stopRun", "runId":first.id}))
        .unwrap();
    h.host
        .handle(serde_json::json!({"op":"continueRun", "issueId":"you/garden#1"}))
        .unwrap();
    assert_eq!(h.sessions.last_spawn().unwrap().cwd, tree);
}

#[test]
fn unresolved_isolation_cannot_continue_in_the_main_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    init_git(&dir);
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let run = start_bound(&mut h.host, &project_id, true).snapshot.runs[0].clone();
    h.host
        .handle(serde_json::json!({"op":"stopRun", "runId":run.id}))
        .unwrap();
    let error = h
        .host
        .handle(serde_json::json!({"op":"continueRun", "issueId":"you/garden#1"}))
        .unwrap_err();
    assert!(error.to_string().contains("尚未确认隔离执行目录"));
    assert_eq!(h.sessions.spawn_count(), 1);
}

#[test]
fn overlapping_unconfirmed_isolation_is_rejected_before_spawning() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    init_git(&dir);
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    start_unbound(&mut h.host, &project_id, true, "first");
    let mut values = grok_values();
    values["isolation"] = serde_json::json!("true");
    let error = h
        .host
        .handle(serde_json::json!({
            "op":"startUnboundRun", "projectId": project_id,
            "agentId":"grok-build", "values": values, "openingText":"second"
        }))
        .unwrap_err();
    assert!(error.to_string().contains("尚未确认隔离执行目录"));
    assert_eq!(h.sessions.spawn_count(), 1);
}

#[test]
fn continue_reuses_the_recorded_directory_without_worktree() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    mark_git(&dir);
    let tree = make_dir(tmp.path(), "work/garden-iso");
    let mut h = harness(tmp.path());
    h.agent.set_isolation_tree(Some(tree.clone()));
    h.agent.set_native_session_id(Some("sess-iso".into()));
    let project_id = register(&mut h.host, &dir);
    let first = start_bound(&mut h.host, &project_id, true).snapshot.runs[0].clone();
    h.host
        .handle(serde_json::json!({
            "op": "stopRun",
            "runId": first.id,
        }))
        .unwrap();

    let out = h
        .host
        .handle(serde_json::json!({
            "op": "continueRun",
            "issueId": "you/garden#1",
        }))
        .unwrap();
    let continued = out
        .snapshot
        .runs
        .iter()
        .find(|run| run.id != first.id)
        .unwrap();
    assert_eq!(continued.status, RunStatus::Running);
    assert_eq!(continued.working_directory, tree.display().to_string());
    assert!(continued.isolation_note.is_none());
    let spawn = h.sessions.last_spawn().unwrap();
    assert_eq!(spawn.cwd, tree);
    assert!(!spawn.argv.iter().any(|arg| arg == "--worktree"));
    assert!(spawn
        .argv
        .windows(2)
        .any(|pair| pair == ["--resume", "sess-iso"]));
}

#[test]
fn continue_falls_back_to_the_project_directory_when_the_tree_is_gone() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    mark_git(&dir);
    let tree = make_dir(tmp.path(), "work/garden-iso");
    let mut h = harness(tmp.path());
    h.agent.set_isolation_tree(Some(tree.clone()));
    let project_id = register(&mut h.host, &dir);
    let first = start_bound(&mut h.host, &project_id, true).snapshot.runs[0].clone();
    h.host
        .handle(serde_json::json!({
            "op": "stopRun",
            "runId": first.id,
        }))
        .unwrap();
    std::fs::remove_dir_all(&tree).unwrap();

    let out = h
        .host
        .handle(serde_json::json!({
            "op": "continueRun",
            "issueId": "you/garden#1",
        }))
        .unwrap();
    let continued = out
        .snapshot
        .runs
        .iter()
        .find(|run| run.id != first.id)
        .unwrap();
    assert_eq!(continued.status, RunStatus::Running);
    assert_eq!(continued.working_directory, dir.display().to_string());
    let note = continued.isolation_note.as_deref().unwrap();
    assert!(note.contains("主目录"), "{note}");
    let spawn = h.sessions.last_spawn().unwrap();
    assert_eq!(spawn.cwd, dir);
    assert!(!spawn.argv.iter().any(|arg| arg == "--worktree"));
}

#[test]
fn lock_files_and_sibling_runs_warn_but_do_not_block_launch() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    mark_git(&dir);
    std::fs::write(dir.join(".git").join("index.lock"), "locked").unwrap();
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    start_unbound(&mut h.host, &project_id, false, "first run");

    let form = h
        .host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
            "agentId": "grok-build",
        }))
        .unwrap()
        .snapshot
        .launch_form
        .unwrap();
    let warnings = form.warnings.join(" ");
    assert!(
        warnings.contains("锁") || warnings.contains("端口"),
        "{warnings}"
    );

    let out = start_unbound(&mut h.host, &project_id, false, "second run");
    assert_eq!(out.snapshot.runs.len(), 2);
    assert!(out
        .snapshot
        .runs
        .iter()
        .all(|run| run.status == RunStatus::Running));
    assert!(out.snapshot.launch_form.is_none());
}

#[test]
fn english_fallback_note_uses_the_client_language() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    mark_git(&dir);
    let tree = make_dir(tmp.path(), "work/garden-iso");
    let mut h = harness(tmp.path());
    h.host
        .dispatch(host_kernel::Command::SetLanguage(Language::En))
        .unwrap();
    h.agent.set_isolation_tree(Some(tree.clone()));
    let project_id = register(&mut h.host, &dir);
    let first = start_bound(&mut h.host, &project_id, true).snapshot.runs[0].clone();
    h.host
        .handle(serde_json::json!({
            "op": "stopRun",
            "runId": first.id,
        }))
        .unwrap();
    std::fs::remove_dir_all(&tree).unwrap();
    let continued = h
        .host
        .handle(serde_json::json!({
            "op": "continueRun",
            "issueId": "you/garden#1",
        }))
        .unwrap()
        .snapshot
        .runs
        .into_iter()
        .find(|run| run.id != first.id)
        .unwrap();
    let note = continued.isolation_note.as_deref().unwrap();
    assert!(note.to_ascii_lowercase().contains("project"), "{note}");
}

#[test]
fn failed_isolated_start_is_not_continued_as_an_isolated_run() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    mark_git(&dir);
    let tree = make_dir(tmp.path(), "work/garden-iso");
    let mut h = harness(tmp.path());
    h.agent.set_isolation_tree(Some(tree.clone()));
    h.sessions.fail_next("could not spawn grok");
    let project_id = register(&mut h.host, &dir);

    let first = start_bound(&mut h.host, &project_id, true).snapshot.runs[0].clone();
    assert_eq!(first.status, RunStatus::Ended);
    assert!(!first.isolated);
    assert_eq!(first.working_directory, dir.display().to_string());
    assert!(first.isolation_note.is_none());

    let error = h
        .host
        .handle(serde_json::json!({
            "op": "continueRun",
            "issueId": "you/garden#1",
        }))
        .unwrap_err();
    assert!(error.to_string().contains("execution-stopped"));
    assert_eq!(h.sessions.spawn_count(), 1);
}

fn wait_for_directory(h: &mut Harness, run_id: &str, tree: &Path) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        let snapshot = h
            .host
            .handle(serde_json::json!({"op":"snapshot"}))
            .unwrap()
            .snapshot;
        let run = snapshot.runs.iter().find(|run| run.id == run_id).unwrap();
        if Path::new(&run.working_directory).canonicalize().ok() == tree.canonicalize().ok()
            && run.isolation_pending.is_none()
        {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "directory not recovered"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[test]
fn pending_isolation_recovers_after_host_restart() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    init_git(&dir);
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let run = start_bound(&mut h.host, &project_id, true).snapshot.runs[0].clone();
    let tree = make_dir(tmp.path(), "work/tree");
    let mut rebooted = reboot(h, tmp.path());
    rebooted.agent.set_isolation_tree(Some(tree.clone()));
    wait_for_directory(&mut rebooted, &run.id, &tree);
    rebooted
        .host
        .handle(serde_json::json!({"op":"continueRun", "issueId":"you/garden#1"}))
        .unwrap();
    assert_eq!(rebooted.sessions.last_spawn().unwrap().cwd, tree);
}

#[test]
fn stopped_unresolved_launch_does_not_claim_the_next_runs_tree() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    init_git(&dir);
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let first = start_unbound(&mut h.host, &project_id, true, "first")
        .snapshot
        .runs[0]
        .clone();
    h.host
        .handle(serde_json::json!({"op":"stopRun", "runId":first.id}))
        .unwrap();
    let second = start_unbound(&mut h.host, &project_id, true, "second")
        .snapshot
        .runs
        .last()
        .unwrap()
        .clone();
    let tree = make_dir(tmp.path(), "work/second-tree");
    h.agent.set_isolation_tree(Some(tree.clone()));
    wait_for_directory(&mut h, &second.id, &tree);
    let snapshot = h
        .host
        .handle(serde_json::json!({"op":"snapshot"}))
        .unwrap()
        .snapshot;
    let old = snapshot.runs.iter().find(|run| run.id == first.id).unwrap();
    assert!(old.isolation_pending.is_some());
    assert_eq!(old.working_directory, dir.display().to_string());
}

#[test]
fn legacy_isolated_main_directory_cannot_show_changes_or_continue() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    init_git(&dir);
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let run = start_bound(&mut h.host, &project_id, true).snapshot.runs[0].clone();
    h.host
        .handle(serde_json::json!({"op":"stopRun", "runId":run.id}))
        .unwrap();
    let path = tmp.path().join("host/runs.json");
    let mut stored: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    stored[0]
        .as_object_mut()
        .unwrap()
        .remove("isolationPending");
    std::fs::write(&path, serde_json::to_vec(&stored).unwrap()).unwrap();
    let mut h = reboot(h, tmp.path());
    let changes = h
        .host
        .handle(serde_json::json!({"op":"viewChanges", "runId":run.id}))
        .unwrap()
        .view_changes
        .unwrap();
    assert!(!changes.available);
    assert!(changes.repos.is_empty());
    assert!(changes
        .unavailable_reason
        .unwrap()
        .contains("尚未确认隔离执行目录"));
    let error = h
        .host
        .handle(serde_json::json!({"op":"continueRun", "issueId":"you/garden#1"}))
        .unwrap_err();
    assert!(error.to_string().contains("尚未确认隔离执行目录"));
    assert_eq!(h.sessions.spawn_count(), 0);
}

#[test]
fn delayed_git_tree_retains_the_commit_from_before_launch() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    init_git(&dir);
    let git = |cwd: &Path, args: &[&str]| {
        let output = std::process::Command::new("git")
            .current_dir(cwd)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    };
    git(
        &dir,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--allow-empty",
            "-m",
            "initial",
        ],
    );
    let initial = git(&dir, &["rev-parse", "HEAD"]);
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let run = start_unbound(&mut h.host, &project_id, true, "first")
        .snapshot
        .runs[0]
        .clone();
    let tree = tmp.path().join("work/tree");
    git(
        &dir,
        &["worktree", "add", "--detach", tree.to_str().unwrap()],
    );
    std::fs::write(tree.join("answer.txt"), "isolated answer\n").unwrap();
    git(&tree, &["add", "answer.txt"]);
    git(
        &tree,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-m",
            "answer",
        ],
    );
    wait_for_directory(&mut h, &run.id, &tree);
    let snapshot = h
        .host
        .handle(serde_json::json!({"op":"snapshot"}))
        .unwrap()
        .snapshot;
    let run = snapshot
        .runs
        .iter()
        .find(|candidate| candidate.id == run.id)
        .unwrap();
    assert_eq!(
        run.git_baselines[0].commit.as_deref(),
        Some(initial.as_str())
    );
    let changes = h
        .host
        .handle(serde_json::json!({"op":"viewChanges", "runId":run.id}))
        .unwrap();
    assert!(serde_json::to_string(&changes)
        .unwrap()
        .contains("answer.txt"));
}
