use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use host_kernel::{
    AgentSession, BootRequest, ChangeScope, HostKernel, IssueRecord, KernelPorts, MemoryAgent,
    MemoryLaunchEnv, MemorySessionFactory, MemoryTracker, RunStatus, SystemAppearance, ViewChanges,
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

fn git(dir: &Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} in {} failed: {} {}",
        dir.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn init_repo(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    git(dir, &["init", "-q"]);
    git(dir, &["commit", "--allow-empty", "-m", "init"]);
}

fn write_file(dir: &Path, rel: &str, body: &str) {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, body).unwrap();
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

fn start_unbound(host: &mut HostKernel, project_id: &str, opening: &str) -> String {
    start_run(host, project_id, None, false, opening)
}

fn start_bound(host: &mut HostKernel, project_id: &str) -> String {
    start_run(
        host,
        project_id,
        Some("you/garden#1"),
        false,
        "ready work\nhttps://github.com/you/garden/issues/1",
    )
}

fn start_run(
    host: &mut HostKernel,
    project_id: &str,
    issue_id: Option<&str>,
    isolation: bool,
    opening: &str,
) -> String {
    let mut values = grok_values();
    values["isolation"] = serde_json::json!(if isolation { "true" } else { "false" });
    let mut req = serde_json::json!({
        "op": "startUnboundRun",
        "projectId": project_id,
        "agentId": "grok-build",
        "values": values,
        "openingText": opening,
    });
    if let Some(issue_id) = issue_id {
        req["issueId"] = serde_json::json!(issue_id);
    }
    host.handle(req)
        .unwrap()
        .snapshot
        .runs
        .last()
        .unwrap()
        .id
        .clone()
}

fn view(
    host: &mut HostKernel,
    run_id: Option<&str>,
    issue_id: Option<&str>,
    scope: Option<&str>,
) -> ViewChanges {
    let mut req = serde_json::json!({ "op": "viewChanges" });
    if let Some(run_id) = run_id {
        req["runId"] = serde_json::json!(run_id);
    }
    if let Some(issue_id) = issue_id {
        req["issueId"] = serde_json::json!(issue_id);
    }
    if let Some(scope) = scope {
        req["scope"] = serde_json::json!(scope);
    }
    host.handle(req).unwrap().view_changes.expect("viewChanges")
}

fn file_paths(changes: &ViewChanges) -> Vec<String> {
    let mut paths = Vec::new();
    for repo in &changes.repos {
        for file in &repo.files {
            let prefix = if repo.display_path == "." {
                String::new()
            } else {
                format!("{}/", repo.display_path)
            };
            paths.push(format!("{prefix}{}", file.path));
        }
    }
    paths.sort();
    paths
}

fn dump(changes: &ViewChanges) -> String {
    serde_json::to_string(changes).unwrap()
}

#[test]
fn this_round_includes_committed_and_uncommitted_uncommitted_scope_does_not() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    init_repo(&dir);
    write_file(&dir, "README.md", "base\n");
    git(&dir, &["add", "README.md"]);
    git(&dir, &["commit", "-m", "readme"]);

    let nested = dir.join("vendor/pkg");
    init_repo(&nested);
    write_file(&nested, "src/lib.rs", "fn start() {}\n");
    git(&nested, &["add", "src/lib.rs"]);
    git(&nested, &["commit", "-m", "nested"]);
    write_file(&dir, ".gitignore", "vendor/\nnode_modules/\n");
    git(&dir, &["add", ".gitignore"]);
    git(&dir, &["commit", "-m", "ignore nested"]);

    let hidden = dir.join("node_modules/secret-pkg");
    init_repo(&hidden);
    write_file(&hidden, "index.js", "hidden-at-start\n");
    git(&hidden, &["add", "index.js"]);
    git(&hidden, &["commit", "-m", "hidden"]);

    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let run_id = start_unbound(&mut h.host, &project_id, "look at diffs");
    assert_eq!(h.host.snapshot().runs[0].status, RunStatus::Running);

    write_file(&dir, "round.txt", "committed-this-round\n");
    git(&dir, &["add", "round.txt"]);
    git(&dir, &["commit", "-m", "round"]);
    write_file(&dir, "README.md", "base\ndirty-uncommitted\n");
    write_file(
        &nested,
        "src/lib.rs",
        "fn start() {}\nfn nested_round() {}\n",
    );
    write_file(&hidden, "index.js", "should-not-see-node-modules\n");

    let round = view(&mut h.host, Some(&run_id), None, None);
    assert_eq!(round.scope, ChangeScope::ThisRound);
    assert!(round.available, "{}", dump(&round));
    let round_paths = file_paths(&round);
    assert!(
        round_paths.iter().any(|path| path == "round.txt"),
        "{round_paths:?}"
    );
    assert!(
        round_paths.iter().any(|path| path == "README.md"),
        "{round_paths:?}"
    );
    assert!(
        round_paths
            .iter()
            .any(|path| path == "vendor/pkg/src/lib.rs"),
        "{round_paths:?}"
    );
    let round_dump = dump(&round);
    assert!(round_dump.contains("committed-this-round"), "{round_dump}");
    assert!(round_dump.contains("dirty-uncommitted"), "{round_dump}");
    assert!(round_dump.contains("nested_round"), "{round_dump}");
    assert!(
        !round_dump.contains("should-not-see-node-modules"),
        "{round_dump}"
    );
    assert!(!round_paths.iter().any(|path| path.contains("node_modules")));

    let dirty = view(&mut h.host, Some(&run_id), None, Some("uncommitted"));
    assert_eq!(dirty.scope, ChangeScope::Uncommitted);
    assert!(dirty.available, "{}", dump(&dirty));
    let dirty_paths = file_paths(&dirty);
    assert!(
        dirty_paths.iter().any(|path| path == "README.md"),
        "{dirty_paths:?}"
    );
    assert!(
        !dirty_paths.iter().any(|path| path == "round.txt"),
        "{dirty_paths:?}"
    );
    let dirty_dump = dump(&dirty);
    assert!(dirty_dump.contains("dirty-uncommitted"), "{dirty_dump}");
    assert!(!dirty_dump.contains("committed-this-round"), "{dirty_dump}");
}

#[test]
fn isolated_tree_gone_does_not_fall_back_to_project_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    init_repo(&dir);
    write_file(&dir, "main-secret.txt", "main-secret-xyz\n");
    git(&dir, &["add", "main-secret.txt"]);
    git(&dir, &["commit", "-m", "main"]);

    let tree = make_dir(tmp.path(), "work/garden-iso");
    init_repo(&tree);
    write_file(&tree, "iso.txt", "iso-only\n");
    git(&tree, &["add", "iso.txt"]);
    git(&tree, &["commit", "-m", "iso"]);

    let mut h = harness(tmp.path());
    h.agent.set_isolation_tree(Some(tree.clone()));
    let project_id = register(&mut h.host, &dir);
    let mut values = grok_values();
    values["isolation"] = serde_json::json!("true");
    let run_id = h
        .host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
            "agentId": "grok-build",
            "values": values,
            "openingText": "isolate",
        }))
        .unwrap()
        .snapshot
        .runs[0]
        .id
        .clone();
    assert!(h.host.snapshot().runs[0].isolated);
    assert_eq!(
        h.host.snapshot().runs[0].working_directory,
        tree.display().to_string()
    );

    write_file(&tree, "iso.txt", "iso-only\nchanged-in-tree\n");
    let live = view(&mut h.host, Some(&run_id), None, None);
    assert!(live.available, "{}", dump(&live));
    assert!(dump(&live).contains("changed-in-tree"), "{}", dump(&live));
    assert!(!dump(&live).contains("main-secret-xyz"), "{}", dump(&live));

    std::fs::remove_dir_all(&tree).unwrap();
    write_file(
        &dir,
        "main-secret.txt",
        "main-secret-xyz\nafter-tree-gone\n",
    );

    let gone = view(&mut h.host, Some(&run_id), None, None);
    assert!(!gone.available, "{}", dump(&gone));
    let reason = gone.unavailable_reason.as_deref().unwrap_or("");
    assert!(
        reason.contains("隔离") || reason.contains("不在"),
        "{reason}"
    );
    let gone_dump = dump(&gone);
    assert!(!gone_dump.contains("main-secret-xyz"), "{gone_dump}");
    assert!(!gone_dump.contains("after-tree-gone"), "{gone_dump}");
    assert!(gone
        .repos
        .iter()
        .all(|repo| !repo.available || repo.files.is_empty()));
}

#[test]
fn missing_nested_repo_does_not_compare_against_the_parent() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    init_repo(&dir);
    write_file(&dir, "app.rs", "fn app() {}\n");
    git(&dir, &["add", "app.rs"]);
    git(&dir, &["commit", "-m", "app"]);
    let nested = dir.join("vendor/pkg");
    init_repo(&nested);
    write_file(&nested, "lib.rs", "fn nested() {}\n");
    git(&nested, &["add", "lib.rs"]);
    git(&nested, &["commit", "-m", "nested"]);

    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let run_id = start_unbound(&mut h.host, &project_id, "nested repo");
    write_file(&dir, "app.rs", "fn app() {}\nfn later() {}\n");
    std::fs::remove_dir_all(&nested).unwrap();

    let changes = view(&mut h.host, Some(&run_id), None, None);
    assert!(changes.available, "{}", dump(&changes));
    let nested_repo = changes
        .repos
        .iter()
        .find(|repo| repo.display_path.contains("vendor/pkg"))
        .expect("nested repo listed");
    assert!(!nested_repo.available, "{}", dump(&changes));
    let reason = nested_repo.unavailable_reason.as_deref().unwrap_or("");
    assert!(
        reason.contains("子仓库") || reason.contains("不在"),
        "{reason}"
    );
    assert!(nested_repo.files.is_empty());
    assert!(dump(&changes).contains("later"), "{}", dump(&changes));
}

#[test]
fn view_changes_stays_available_while_running_after_end_and_after_close() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    init_repo(&dir);
    write_file(&dir, "lib.rs", "fn first() {}\n");
    git(&dir, &["add", "lib.rs"]);
    git(&dir, &["commit", "-m", "first"]);

    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let run_id = start_bound(&mut h.host, &project_id);
    write_file(&dir, "lib.rs", "fn first() {}\nfn running() {}\n");

    let during = view(&mut h.host, None, Some("you/garden#1"), None);
    assert_eq!(during.run_id, run_id);
    assert!(during.available);
    assert!(dump(&during).contains("running"), "{}", dump(&during));

    h.host
        .handle(serde_json::json!({ "op": "stopRun", "runId": run_id }))
        .unwrap();
    write_file(
        &dir,
        "lib.rs",
        "fn first() {}\nfn running() {}\nfn ended() {}\n",
    );
    let after = view(&mut h.host, Some(&run_id), None, None);
    assert!(after.available);
    assert!(dump(&after).contains("ended"), "{}", dump(&after));

    h.tracker.close_issue("you/garden", 1);
    h.host
        .handle(serde_json::json!({ "op": "refresh", "projectId": project_id }))
        .unwrap();
    let closed = view(&mut h.host, None, Some("you/garden#1"), None);
    assert_eq!(closed.run_id, run_id);
    assert!(closed.available);
    assert!(dump(&closed).contains("ended"), "{}", dump(&closed));
}

#[test]
fn missing_git_repo_explains_and_does_not_invent_a_diff() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let run_id = start_unbound(&mut h.host, &project_id, "no git");
    let changes = view(&mut h.host, Some(&run_id), None, None);
    assert!(!changes.available, "{}", dump(&changes));
    let reason = changes.unavailable_reason.as_deref().unwrap_or("");
    assert!(reason.contains("git"), "{reason}");
    assert!(changes.repos.iter().all(|repo| repo.files.is_empty()));
}

#[test]
fn repo_that_appears_after_start_is_uncommitted_only() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    init_repo(&dir);
    write_file(&dir, "app.rs", "fn app() {}\n");
    git(&dir, &["add", "app.rs"]);
    git(&dir, &["commit", "-m", "app"]);
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let run_id = start_unbound(&mut h.host, &project_id, "late nested");
    let nested = dir.join("late/pkg");
    init_repo(&nested);
    write_file(&nested, "new.rs", "fn late() {}\n");

    let round = view(&mut h.host, Some(&run_id), None, None);
    let late = round
        .repos
        .iter()
        .find(|repo| repo.display_path.contains("late/pkg"))
        .expect("late repo listed");
    assert!(!late.available, "{}", dump(&round));
    let reason = late.unavailable_reason.as_deref().unwrap_or("");
    assert!(
        reason.contains("启动后") || reason.contains("未提交"),
        "{reason}"
    );
    assert!(late.files.is_empty());

    let dirty = view(&mut h.host, Some(&run_id), None, Some("uncommitted"));
    let late_dirty = dirty
        .repos
        .iter()
        .find(|repo| repo.display_path.contains("late/pkg"))
        .expect("late repo in uncommitted");
    assert!(late_dirty.available, "{}", dump(&dirty));
    assert!(
        late_dirty.files.iter().any(|file| file.path == "new.rs"),
        "{}",
        dump(&dirty)
    );
}

#[test]
fn notes_go_into_the_next_opening_not_the_current_run_or_tracker() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    init_repo(&dir);
    write_file(&dir, "src/main.rs", "fn main() {}\n");
    git(&dir, &["add", "src/main.rs"]);
    git(&dir, &["commit", "-m", "main"]);

    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir);
    let first = start_bound(&mut h.host, &project_id);
    write_file(&dir, "src/main.rs", "fn main() {\n    let x = 1;\n}\n");
    let changes = view(&mut h.host, Some(&first), None, None);
    let file = changes
        .repos
        .iter()
        .flat_map(|repo| repo.files.iter().map(move |file| (repo, file)))
        .find(|(_, file)| file.path == "src/main.rs")
        .expect("changed file");
    let line = file
        .1
        .hunks
        .iter()
        .flat_map(|hunk| hunk.lines.iter())
        .find(|line| line.text.contains("let x = 1"))
        .and_then(|line| line.new_line)
        .expect("added line number");

    let before_comments = h.tracker.assignees("you/garden", 1);
    let before_output = h
        .sessions
        .last_session()
        .unwrap()
        .read_after(0, Duration::from_millis(0))
        .data;
    h.host
        .handle(serde_json::json!({
            "op": "writeChangeNote",
            "runId": first,
            "repo": file.0.display_path,
            "path": "src/main.rs",
            "line": line,
            "text": "这里不要写死 1",
        }))
        .unwrap();
    let after_output = h
        .sessions
        .last_session()
        .unwrap()
        .read_after(0, Duration::from_millis(0))
        .data;
    assert_eq!(before_output, after_output);
    assert!(!String::from_utf8_lossy(&after_output).contains("这里不要写死 1"));
    assert_eq!(h.tracker.assignees("you/garden", 1), before_comments);
    let noted = view(&mut h.host, Some(&first), None, None);
    assert!(
        noted.notes.iter().any(|note| note.text == "这里不要写死 1"
            && note.path == "src/main.rs"
            && note.line == line),
        "{}",
        dump(&noted)
    );

    h.host
        .handle(serde_json::json!({ "op": "stopRun", "runId": first }))
        .unwrap();
    let form = h
        .host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
            "issueId": "you/garden#1",
        }))
        .unwrap()
        .snapshot
        .launch_form
        .unwrap();
    assert!(
        form.opening_text.contains("ready work"),
        "{}",
        form.opening_text
    );
    assert!(
        form.opening_text
            .contains("https://github.com/you/garden/issues/1"),
        "{}",
        form.opening_text
    );
    assert!(
        form.opening_text.contains("src/main.rs") && form.opening_text.contains("这里不要写死 1"),
        "{}",
        form.opening_text
    );

    h.sessions.fail_next("could not spawn grok");
    let failed = h
        .host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
            "issueId": "you/garden#1",
            "agentId": "grok-build",
            "values": grok_values(),
            "openingText": form.opening_text,
        }))
        .unwrap();
    assert_eq!(
        failed.snapshot.runs.last().unwrap().status,
        RunStatus::Ended
    );
    assert!(failed.snapshot.launch_form.is_some());
    let after_fail = view(&mut h.host, None, Some("you/garden#1"), None);
    assert!(
        after_fail
            .notes
            .iter()
            .any(|note| note.text == "这里不要写死 1"),
        "{}",
        dump(&after_fail)
    );
    let still = h
        .host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
            "issueId": "you/garden#1",
        }))
        .unwrap()
        .snapshot
        .launch_form
        .unwrap();
    assert!(
        still.opening_text.contains("这里不要写死 1"),
        "{}",
        still.opening_text
    );

    let started = h
        .host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
            "issueId": "you/garden#1",
            "agentId": "grok-build",
            "values": grok_values(),
            "openingText": still.opening_text,
        }))
        .unwrap();
    assert_eq!(
        started.snapshot.runs.last().unwrap().status,
        RunStatus::Running
    );
    let injected = h
        .sessions
        .last_session()
        .unwrap()
        .read_after(0, Duration::from_millis(0))
        .data;
    let injected = String::from_utf8_lossy(&injected);
    assert!(injected.contains("这里不要写死 1"), "{injected}");
    let next = h
        .host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
            "issueId": "you/garden#1",
        }))
        .unwrap()
        .snapshot
        .launch_form
        .unwrap();
    assert!(
        !next.opening_text.contains("这里不要写死 1"),
        "{}",
        next.opening_text
    );
    assert!(
        next.opening_text.contains("ready work"),
        "{}",
        next.opening_text
    );
}
