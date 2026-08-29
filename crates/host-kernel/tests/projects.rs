use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use host_kernel::{
    bind_local_rpc, spawn_local_rpc, AuthFailureKind, BootRequest, CredentialSource, GitHubTracker,
    HostKernel, KernelError, MemoryTracker, ProjectConnection, ScriptedGitHub, SystemAppearance,
    TrackerKind, TrackerRouter,
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

fn boot_memory(root: &Path) -> HostKernel {
    HostKernel::boot_with(boot_req(root), Arc::new(MemoryTracker::new())).unwrap()
}

fn boot_memory_with_local(root: &Path) -> HostKernel {
    HostKernel::boot_with(
        boot_req(root),
        Arc::new(TrackerRouter::new(Arc::new(MemoryTracker::new()))),
    )
    .unwrap()
}

fn make_dir(root: &Path, name: &str) -> std::path::PathBuf {
    let dir = root.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn registering_a_github_project_lists_it_and_makes_it_current() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/garden");
    let mut host = boot_memory(tmp.path());

    let out = host
        .handle(serde_json::json!({
            "op": "registerProject",
            "name": "garden",
            "localPath": project_dir,
            "repository": "you/garden",
        }))
        .unwrap();

    assert_eq!(out.snapshot.projects.len(), 1);
    let project = &out.snapshot.projects[0];
    assert_eq!(project.name, "garden");
    assert_eq!(project.local_path, project_dir);
    assert_eq!(project.github_host, "github.com");
    assert_eq!(project.repository, "you/garden");
    assert_eq!(project.tracker, TrackerKind::Github);
    assert_eq!(out.snapshot.focused_project_id, project.id);
    assert!(out.snapshot.empty_actions.is_empty());
    assert!(!project.has_active_run);
    assert!(project.tracker_synced);
    assert!(matches!(
        project.connection,
        ProjectConnection::Ready {
            source: CredentialSource::Cli
        }
    ));
}

#[test]
fn registered_project_is_persisted_without_storing_a_token() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/garden");
    let mut host = boot_memory(tmp.path());
    host.handle(serde_json::json!({
        "op": "registerProject",
        "name": "garden",
        "localPath": project_dir,
        "repository": "you/garden",
    }))
    .unwrap();
    let settings_path = host.snapshot().data.host_settings_path.clone();
    drop(host);

    let settings: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&settings_path).unwrap()).unwrap();
    let stored = &settings["projects"][0];
    assert_eq!(stored["name"], "garden");
    assert_eq!(stored["repository"], "you/garden");
    assert_eq!(stored["githubHost"], "github.com");
    assert!(stored.get("token").is_none());
    assert!(stored.get("pat").is_none());
    let dump = stored.to_string().to_ascii_lowercase();
    assert!(!dump.contains("ghp_"));
    assert!(!dump.contains("token"));

    let host = boot_memory(tmp.path());
    let snap = host.snapshot();
    assert_eq!(snap.projects.len(), 1);
    assert_eq!(snap.projects[0].name, "garden");
    assert_eq!(snap.focused_project_id, snap.projects[0].id);
}

#[test]
fn project_persists_tracker_kind_and_old_settings_default_to_github() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut host = boot_memory(tmp.path());
    register(&mut host, "garden", &dir, "you/garden");
    let settings_path = host.snapshot().data.host_settings_path.clone();
    drop(host);

    // 新注册的项目在设置里显式持久化 tracker 类型
    let mut settings: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&settings_path).unwrap()).unwrap();
    assert_eq!(settings["projects"][0]["tracker"], "github");

    // 模拟旧数据：缺少 tracker 字段时默认 GitHub，不硬编码在模型里
    settings["projects"][0]
        .as_object_mut()
        .unwrap()
        .remove("tracker");
    std::fs::write(&settings_path, settings.to_string()).unwrap();
    let host = boot_memory(tmp.path());
    let project = &host.snapshot().projects[0];
    assert_eq!(project.tracker, host_kernel::TrackerKind::Github);
    assert_eq!(project.repository, "you/garden");
}

#[test]
fn editing_a_project_updates_the_registration() {
    let tmp = tempfile::tempdir().unwrap();
    let garden = make_dir(tmp.path(), "work/garden");
    let notes = make_dir(tmp.path(), "work/notes");
    let mut host = boot_memory(tmp.path());
    let id = host
        .handle(serde_json::json!({
            "op": "registerProject",
            "name": "garden",
            "localPath": garden,
            "repository": "you/garden",
        }))
        .unwrap()
        .snapshot
        .projects[0]
        .id
        .clone();

    let out = host
        .handle(serde_json::json!({
            "op": "editProject",
            "projectId": id,
            "name": "notes",
            "localPath": notes,
            "githubHost": "github.example.com",
            "repository": "acme/notes",
        }))
        .unwrap();

    let project = &out.snapshot.projects[0];
    assert_eq!(project.name, "notes");
    assert_eq!(project.local_path, notes);
    assert_eq!(project.github_host, "github.example.com");
    assert_eq!(project.repository, "acme/notes");
}

#[test]
fn editing_only_the_name_preserves_tracker_state() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut host = boot_memory(tmp.path());
    let id = register(&mut host, "garden", &dir, "you/garden");
    assert!(host.snapshot().projects[0].tracker_synced);

    let out = host
        .handle(serde_json::json!({
            "op": "editProject",
            "projectId": id,
            "name": "renamed",
            "localPath": dir,
            "repository": "you/garden",
        }))
        .unwrap();

    assert_eq!(out.snapshot.projects[0].name, "renamed");
    assert!(out.snapshot.projects[0].tracker_synced);
}

#[test]
fn equivalent_paths_cannot_register_the_same_directory_twice() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/garden");
    let alias = tmp
        .path()
        .join("work")
        .join("nested")
        .join("..")
        .join("garden");
    std::fs::create_dir_all(alias.parent().unwrap()).unwrap();
    let mut host = boot_memory(tmp.path());
    register(&mut host, "garden", &project_dir, "you/garden");

    let error = host
        .handle(serde_json::json!({
            "op": "registerProject",
            "name": "alias",
            "localPath": alias,
            "repository": "you/alias",
        }))
        .unwrap_err();
    assert!(
        matches!(error, KernelError::Protocol(message) if message.contains("already registered"))
    );
}

#[test]
fn inference_is_only_a_candidate_until_register() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/garden");
    std::fs::create_dir(project_dir.join(".git")).unwrap();
    std::fs::write(
        project_dir.join(".git").join("config"),
        "[remote \"origin\"]\n\turl = git@github.com:you/garden.git\n",
    )
    .unwrap();
    let mut host = boot_memory(tmp.path());

    let out = host
        .handle(serde_json::json!({
            "op": "inferProject",
            "localPath": project_dir,
        }))
        .unwrap();

    let inference = out.inference.expect("candidate");
    assert_eq!(inference.name, "garden");
    assert_eq!(inference.github_host, "github.com");
    assert_eq!(inference.repository, "you/garden");
    assert!(out.snapshot.projects.is_empty());

    let out = host
        .handle(serde_json::json!({
            "op": "registerProject",
            "name": inference.name,
            "localPath": inference.local_path,
            "githubHost": inference.github_host,
            "repository": inference.repository,
        }))
        .unwrap();
    assert_eq!(out.snapshot.projects.len(), 1);
    assert_eq!(out.snapshot.projects[0].repository, "you/garden");
}

#[test]
fn inference_prefers_local_markdown_tracker_over_git_remote() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/garden");
    std::fs::create_dir_all(project_dir.join(".scratch/feature/issues")).unwrap();
    std::fs::write(
        project_dir.join(".scratch/feature/issues/01-foundation.md"),
        "# 01 — Foundation\n\nStatus: ready-for-agent\n",
    )
    .unwrap();
    std::fs::create_dir(project_dir.join(".git")).unwrap();
    std::fs::write(
        project_dir.join(".git/config"),
        "[remote \"origin\"]\n\turl = git@gitlab.example.com:acme/garden.git\n",
    )
    .unwrap();
    let mut host = boot_memory(tmp.path());
    let inference = host
        .handle(serde_json::json!({ "op": "inferProject", "localPath": project_dir }))
        .unwrap()
        .inference
        .unwrap();
    assert_eq!(inference.tracker, TrackerKind::LocalMarkdown);
    assert_eq!(inference.github_host, "local");
    assert_eq!(inference.repository, project_dir.to_string_lossy());
}

#[test]
fn inference_accepts_a_non_github_git_remote() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/garden");
    std::fs::create_dir(project_dir.join(".git")).unwrap();
    std::fs::write(
        project_dir.join(".git/config"),
        "[remote \"origin\"]\n\turl = ssh://git@gitlab.example.com/acme/garden.git\n",
    )
    .unwrap();
    let mut host = boot_memory(tmp.path());
    let inference = host
        .handle(serde_json::json!({ "op": "inferProject", "localPath": project_dir }))
        .unwrap()
        .inference
        .unwrap();
    assert_eq!(inference.tracker, TrackerKind::Github);
    assert_eq!(inference.github_host, "gitlab.example.com");
    assert_eq!(inference.repository, "acme/garden");
}

#[test]
fn local_markdown_project_reads_and_claims_issue_files() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/garden");
    let issue_dir = project_dir.join(".scratch/feature/issues");
    std::fs::create_dir_all(&issue_dir).unwrap();
    std::fs::write(
        issue_dir.join("01-foundation.md"),
        "# 01 — Foundation\n\nStatus: ready-for-agent\n\nBlocked by: None\n\nBody\n",
    )
    .unwrap();
    let mut host = boot_memory_with_local(tmp.path());
    let out = host
        .handle(serde_json::json!({
            "op": "registerProject",
            "name": "garden",
            "localPath": project_dir,
            "githubHost": "local",
            "repository": project_dir,
        }))
        .unwrap();
    assert_eq!(out.snapshot.projects[0].tracker, TrackerKind::LocalMarkdown);
    assert_eq!(out.snapshot.projects[0].issue_counts.total, 1);
    assert_eq!(
        out.snapshot
            .board
            .as_ref()
            .and_then(|board| board.columns.as_ref())
            .map(|columns| columns.frontier.len()),
        Some(1)
    );
}

#[test]
fn remove_only_unregisters_and_falls_back_to_the_neighbor() {
    let tmp = tempfile::tempdir().unwrap();
    let first = make_dir(tmp.path(), "work/first");
    let second = make_dir(tmp.path(), "work/second");
    let third = make_dir(tmp.path(), "work/third");
    let mut host = boot_memory(tmp.path());
    let first_id = register(&mut host, "first", &first, "you/first");
    register(&mut host, "second", &second, "you/second");
    register(&mut host, "third", &third, "you/third");

    host.handle(serde_json::json!({
        "op": "focusProject",
        "projectId": first_id,
    }))
    .unwrap();
    let out = host
        .handle(serde_json::json!({
            "op": "removeProject",
            "projectId": first_id,
        }))
        .unwrap();

    assert_eq!(
        out.snapshot
            .projects
            .iter()
            .map(|project| project.name.as_str())
            .collect::<Vec<_>>(),
        vec!["second", "third"]
    );
    assert_eq!(out.snapshot.projects[0].id, out.snapshot.focused_project_id);
    assert!(first.is_dir());
    assert!(second.is_dir());
}

#[test]
fn removing_the_last_project_returns_to_an_empty_host() {
    let tmp = tempfile::tempdir().unwrap();
    let only = make_dir(tmp.path(), "work/only");
    let mut host = boot_memory(tmp.path());
    let id = register(&mut host, "only", &only, "you/only");

    let out = host
        .handle(serde_json::json!({
            "op": "removeProject",
            "projectId": id,
        }))
        .unwrap();

    assert!(out.snapshot.projects.is_empty());
    assert_eq!(out.snapshot.focused_project_id, "");
    assert_eq!(
        out.snapshot.empty_actions,
        vec![
            host_kernel::EmptyAction::RegisterFirstProject,
            host_kernel::EmptyAction::PairAnotherHost
        ]
    );
    assert!(only.is_dir());
}

#[test]
fn a_project_that_never_synced_the_tracker_can_still_be_removed() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/fresh");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.fail_read("you/fresh");
    let mut host = HostKernel::boot_with(boot_req(tmp.path()), tracker).unwrap();
    let id = register(&mut host, "fresh", &dir, "you/fresh");
    assert!(!host.snapshot().projects[0].tracker_synced);

    let out = host
        .handle(serde_json::json!({
            "op": "removeProject",
            "projectId": id,
        }))
        .unwrap();
    assert!(out.snapshot.projects.is_empty());
}

#[test]
fn removing_a_project_preserves_ended_run_history() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/history");
    let mut host = boot_memory(tmp.path());
    let id = register(&mut host, "history", &dir, "you/history");
    let run_id = host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": id,
        }))
        .unwrap()
        .snapshot
        .focused_run_id;
    host.handle(serde_json::json!({
        "op": "stopRun",
        "runId": run_id,
    }))
    .unwrap();

    let out = host
        .handle(serde_json::json!({
            "op": "removeProject",
            "projectId": id,
        }))
        .unwrap();

    assert!(out.snapshot.projects.is_empty());
    assert_eq!(out.snapshot.runs.len(), 1);
    assert_eq!(out.snapshot.runs[0].project_id, id);
    let tracker_snapshot = out
        .snapshot
        .data
        .host_dir
        .join("projects")
        .join(&id)
        .join("tracker-snapshot");
    assert!(tracker_snapshot.is_file());
    drop(host);
    assert_eq!(boot_memory(tmp.path()).snapshot().runs.len(), 1);
}

#[test]
fn project_registration_changes_roll_back_when_settings_cannot_be_persisted() {
    let tmp = tempfile::tempdir().unwrap();
    let first = make_dir(tmp.path(), "work/first");
    let second = make_dir(tmp.path(), "work/second");
    let mut host = boot_memory(tmp.path());
    let id = register(&mut host, "first", &first, "you/first");
    let data = host.snapshot().data;
    let tracker_snapshot = data
        .host_dir
        .join("projects")
        .join(&id)
        .join("tracker-snapshot");
    assert!(tracker_snapshot.is_file());
    let settings_tmp = data.host_settings_path.with_extension("json.tmp");
    std::fs::create_dir(&settings_tmp).unwrap();

    let register_error = host
        .handle(serde_json::json!({
            "op": "registerProject",
            "name": "second",
            "localPath": second,
            "repository": "you/second",
        }))
        .unwrap_err();
    assert!(matches!(register_error, KernelError::Io(_)));
    assert_eq!(host.snapshot().projects.len(), 1);

    let edit_error = host
        .handle(serde_json::json!({
            "op": "editProject",
            "projectId": id,
            "name": "renamed",
            "localPath": first,
            "repository": "you/first",
        }))
        .unwrap_err();
    assert!(matches!(edit_error, KernelError::Io(_)));
    assert_eq!(host.snapshot().projects[0].name, "first");
    assert!(tracker_snapshot.is_file());

    let remove_error = host
        .handle(serde_json::json!({
            "op": "removeProject",
            "projectId": id,
        }))
        .unwrap_err();
    assert!(matches!(remove_error, KernelError::Io(_)));
    assert_eq!(host.snapshot().projects.len(), 1);
}

#[test]
fn active_run_allows_changing_only_the_project_name() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/first");
    let mut host = boot_memory(tmp.path());
    let id = register(&mut host, "first", &dir, "you/first");
    host.handle(serde_json::json!({
        "op": "startUnboundRun",
        "projectId": id,
    }))
    .unwrap();

    let out = host
        .handle(serde_json::json!({
            "op": "editProject",
            "projectId": id,
            "name": "renamed",
            "localPath": dir,
            "repository": "you/first",
        }))
        .unwrap();

    assert_eq!(out.snapshot.projects[0].name, "renamed");
    assert_eq!(out.snapshot.projects[0].repository, "you/first");
    assert!(out.snapshot.projects[0].has_active_run);
}

#[test]
fn active_run_blocks_changing_project_location_or_tracker() {
    let tmp = tempfile::tempdir().unwrap();
    let first = make_dir(tmp.path(), "work/first");
    let second = make_dir(tmp.path(), "work/second");
    let mut host = boot_memory(tmp.path());
    let id = register(&mut host, "first", &first, "you/first");
    host.handle(serde_json::json!({
        "op": "startUnboundRun",
        "projectId": id,
    }))
    .unwrap();

    let error = host
        .handle(serde_json::json!({
            "op": "editProject",
            "projectId": id,
            "name": "renamed",
            "localPath": second,
            "repository": "you/second",
        }))
        .unwrap_err();
    assert!(matches!(error, KernelError::Denied(message) if message.contains("active Run")));
    assert_eq!(host.snapshot().projects[0].name, "first");
}

#[test]
fn an_active_run_blocks_remove() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/busy");
    let mut host = boot_memory(tmp.path());
    let id = register(&mut host, "busy", &dir, "you/busy");
    host.handle(serde_json::json!({
        "op": "startUnboundRun",
        "projectId": id,
    }))
    .unwrap();

    let err = host
        .handle(serde_json::json!({
            "op": "removeProject",
            "projectId": id,
        }))
        .unwrap_err();
    match err {
        KernelError::Denied(message) => {
            assert!(message.contains("active Run"));
        }
        other => panic!("expected denied, got {other}"),
    }
    assert_eq!(host.snapshot().projects.len(), 1);
    assert!(dir.is_dir());
}

#[test]
fn one_project_auth_failure_does_not_degrade_the_others() {
    let tmp = tempfile::tempdir().unwrap();
    let good_dir = make_dir(tmp.path(), "work/good");
    let bad_dir = make_dir(tmp.path(), "work/bad");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.fail_repository("you/bad");
    let mut host = HostKernel::boot_with(boot_req(tmp.path()), tracker).unwrap();
    register(&mut host, "good", &good_dir, "you/good");
    register(&mut host, "bad", &bad_dir, "you/bad");

    let snap = host.snapshot();
    let good = snap
        .projects
        .iter()
        .find(|project| project.repository == "you/good")
        .unwrap();
    let bad = snap
        .projects
        .iter()
        .find(|project| project.repository == "you/bad")
        .unwrap();
    assert!(matches!(good.connection, ProjectConnection::Ready { .. }));
    match &bad.connection {
        ProjectConnection::AuthFailed {
            kind,
            repair,
            message,
            ..
        } => {
            assert_eq!(*kind, AuthFailureKind::Rejected);
            assert!(repair.cli_detected);
            assert_eq!(repair.app_env, "AGENT_TASKBOARD_GITHUB_TOKEN");
            assert!(repair.generic_env.contains("GH_TOKEN"));
            assert_eq!(repair.suggested_scope, "repo");
            assert_eq!(repair.secrets_path, snap.data.host_secrets_path);
            assert!(message.contains("拒绝") || message.contains("rejected"));
        }
        other => panic!("expected auth-failed, got {other:?}"),
    }
}

#[test]
fn credentials_prefer_app_env_then_secrets_then_cli_then_generic_env() {
    assert_eq!(
        probe_source(
            ScriptedGitHub {
                env: [
                    ("AGENT_TASKBOARD_GITHUB_TOKEN".into(), "app-token".into(),),
                    ("GH_TOKEN".into(), "generic-token".into()),
                ]
                .into(),
                gh_detected: true,
                gh_tokens: [("github.com".into(), "cli-token".into())].into(),
                accept_tokens: [
                    "app-token".into(),
                    "secret-token".into(),
                    "cli-token".into(),
                    "generic-token".into(),
                ]
                .into(),
                unreachable: false,
                ..Default::default()
            },
            Some("secret-token"),
        ),
        CredentialSource::AppEnv
    );
    assert_eq!(
        probe_source(
            ScriptedGitHub {
                env: [("GH_TOKEN".into(), "generic-token".into())].into(),
                gh_detected: true,
                gh_tokens: [("github.com".into(), "cli-token".into())].into(),
                accept_tokens: [
                    "secret-token".into(),
                    "cli-token".into(),
                    "generic-token".into(),
                ]
                .into(),
                unreachable: false,
                ..Default::default()
            },
            Some("secret-token"),
        ),
        CredentialSource::SecretsFile
    );
    assert_eq!(
        probe_source(
            ScriptedGitHub {
                env: [("GH_TOKEN".into(), "generic-token".into())].into(),
                gh_detected: true,
                gh_tokens: [("github.com".into(), "cli-token".into())].into(),
                accept_tokens: ["cli-token".into(), "generic-token".into()].into(),
                unreachable: false,
                ..Default::default()
            },
            None,
        ),
        CredentialSource::Cli
    );
    assert_eq!(
        probe_source(
            ScriptedGitHub {
                env: [("GITHUB_TOKEN".into(), "generic-token".into())].into(),
                gh_detected: false,
                gh_tokens: Default::default(),
                accept_tokens: ["generic-token".into()].into(),
                unreachable: false,
                ..Default::default()
            },
            None,
        ),
        CredentialSource::GenericEnv
    );
}

#[test]
fn an_unreachable_host_is_not_reported_as_auth_failure() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut host = HostKernel::boot_with(
        boot_req(tmp.path()),
        Arc::new(GitHubTracker::scripted(ScriptedGitHub {
            env: [("GH_TOKEN".into(), "generic-token".into())].into(),
            gh_detected: false,
            gh_tokens: Default::default(),
            accept_tokens: ["generic-token".into()].into(),
            unreachable: true,
            ..Default::default()
        })),
    )
    .unwrap();
    let connection = host
        .handle(serde_json::json!({
            "op": "registerProject",
            "name": "garden",
            "localPath": dir,
            "repository": "you/garden",
        }))
        .unwrap()
        .snapshot
        .projects
        .pop()
        .unwrap()
        .connection;
    match connection {
        ProjectConnection::Unreachable { message, .. } => {
            assert!(message.contains("连不上") || message.contains("could not be reached"));
        }
        other => panic!("expected unreachable, got {other:?}"),
    }
}

fn register(host: &mut HostKernel, name: &str, dir: &Path, repository: &str) -> String {
    host.handle(serde_json::json!({
        "op": "registerProject",
        "name": name,
        "localPath": dir,
        "repository": repository,
    }))
    .unwrap()
    .snapshot
    .projects
    .iter()
    .find(|project| project.name == name)
    .unwrap()
    .id
    .clone()
}

fn write_pat(root: &Path, host: &str, token: &str) {
    let path = root.join("host").join("secrets.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        path,
        serde_json::json!({ "githubPats": { host: token } }).to_string(),
    )
    .unwrap();
}

#[test]
fn loopback_rpc_registers_a_project_for_the_shell() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let kernel = Arc::new(Mutex::new(boot_memory(tmp.path())));
    let (listener, url) = bind_local_rpc(0).unwrap();
    let addr: SocketAddr = url.trim_start_matches("http://").parse().unwrap();
    spawn_local_rpc(listener, kernel, |_| {});

    let body = serde_json::json!({
        "op": "registerProject",
        "name": "garden",
        "localPath": dir,
        "repository": "you/garden",
    });
    let (status, response) = http_post(addr, &body.to_string());
    assert_eq!(status, 200);
    let value: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert_eq!(value["snapshot"]["projects"][0]["name"], "garden");
    assert_eq!(value["snapshot"]["projects"][0]["repository"], "you/garden");
    assert_eq!(value["snapshot"]["projects"][0]["tracker"], "github");
    assert_eq!(
        value["snapshot"]["projects"][0]["connection"]["status"],
        "ready"
    );
    assert_eq!(value["snapshot"]["emptyActions"], serde_json::json!([]));
    assert!(!value["snapshot"]["focusedProjectId"]
        .as_str()
        .unwrap()
        .is_empty());
}

fn http_post(addr: SocketAddr, body: &str) -> (u16, String) {
    let request = format!(
        "POST /rpc HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nOrigin: http://127.0.0.1:1420\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        addr.port(),
        body.len()
    );
    let mut last_err = None;
    for _ in 0..50 {
        match TcpStream::connect_timeout(&addr, Duration::from_millis(200)) {
            Ok(mut stream) => {
                stream.write_all(request.as_bytes()).unwrap();
                let _ = stream.shutdown(std::net::Shutdown::Write);
                let mut buf = String::new();
                stream.read_to_string(&mut buf).unwrap();
                let (head, body) = buf.split_once("\r\n\r\n").unwrap_or((buf.as_str(), ""));
                let status = head
                    .lines()
                    .next()
                    .unwrap_or("")
                    .split_whitespace()
                    .nth(1)
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(0);
                return (status, body.to_string());
            }
            Err(err) => {
                last_err = Some(err);
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
    panic!("connect {addr} failed: {last_err:?}");
}

fn probe_source(script: ScriptedGitHub, pat: Option<&str>) -> CredentialSource {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut host = HostKernel::boot_with(
        boot_req(tmp.path()),
        Arc::new(GitHubTracker::scripted(script)),
    )
    .unwrap();
    if let Some(token) = pat {
        write_pat(tmp.path(), "github.com", token);
    }
    let connection = host
        .handle(serde_json::json!({
            "op": "registerProject",
            "name": "garden",
            "localPath": dir,
            "repository": "you/garden",
        }))
        .unwrap()
        .snapshot
        .projects
        .pop()
        .unwrap()
        .connection;
    match connection {
        ProjectConnection::Ready { source } => source,
        other => panic!("expected ready, got {other:?}"),
    }
}
