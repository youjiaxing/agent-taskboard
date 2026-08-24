mod common;

use std::path::Path;
use std::sync::Arc;

use common::{ReadMode, SeamTracker, WriteMode};
use host_kernel::{
    BoardEmptyReason, BootRequest, HostKernel, IssueRecord, KernelError, RefreshStatus,
    SystemAppearance, TrackerWriteOp,
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

fn boot(root: &Path, tracker: Arc<SeamTracker>) -> HostKernel {
    HostKernel::boot_with(boot_req(root), tracker).unwrap()
}

fn register(host: &mut HostKernel, dir: &Path, repository: &str) -> String {
    host.handle(serde_json::json!({
        "op": "registerProject",
        "name": "garden",
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

fn garden_setup(tmp: &tempfile::TempDir) -> (Arc<SeamTracker>, std::path::PathBuf) {
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(SeamTracker::new());
    tracker.set_issues(
        "you/garden",
        vec![
            IssueRecord::open("you/garden", 8, "main"),
            IssueRecord::open("you/garden", 9, "gate"),
        ],
    );
    (tracker, dir)
}

#[test]
fn host_commands_cover_create_update_open_comment_parent_and_dependency() {
    let tmp = tempfile::tempdir().unwrap();
    let (tracker, dir) = garden_setup(&tmp);
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    let project_id = register(&mut host, &dir, "you/garden");
    assert_eq!(frontier_ids(&host), vec!["you/garden#8", "you/garden#9"]);

    // 创建 Issue：新票出现在看板 Frontier
    host.handle(serde_json::json!({
        "op": "createIssue",
        "projectId": project_id,
        "title": "new one",
        "body": "hello",
    }))
    .unwrap();
    assert_eq!(
        frontier_ids(&host),
        vec!["you/garden#8", "you/garden#9", "you/garden#10"]
    );

    // 修改标题与正文
    host.handle(serde_json::json!({
        "op": "updateIssue",
        "issueId": "you/garden#10",
        "title": "renamed",
        "body": "updated body",
    }))
    .unwrap();
    host.handle(serde_json::json!({ "op": "focusIssue", "issueId": "you/garden#10" }))
        .unwrap();
    assert_eq!(
        host.snapshot().board.unwrap().selected.unwrap().title,
        "renamed"
    );

    // 开关 Issue
    host.handle(serde_json::json!({
        "op": "setIssueOpen",
        "issueId": "you/garden#10",
        "open": false,
    }))
    .unwrap();
    let columns = host.snapshot().board.unwrap().columns.unwrap();
    assert!(!columns
        .frontier
        .iter()
        .any(|card| card.id == "you/garden#10"));
    assert!(columns
        .recently_completed
        .iter()
        .any(|card| card.id == "you/garden#10"));

    // 追加评论
    host.handle(serde_json::json!({
        "op": "addIssueComment",
        "issueId": "you/garden#8",
        "body": "please look",
    }))
    .unwrap();
    assert_eq!(tracker.comments("you/garden"), vec!["please look"]);

    // 设置父子关系
    host.handle(serde_json::json!({
        "op": "setIssueParent",
        "issueId": "you/garden#8",
        "parent": "you/garden#9",
    }))
    .unwrap();
    host.handle(serde_json::json!({ "op": "focusIssue", "issueId": "you/garden#8" }))
        .unwrap();
    assert_eq!(
        host.snapshot()
            .board
            .unwrap()
            .selected
            .unwrap()
            .parent
            .unwrap()
            .id,
        "you/garden#9"
    );

    // 设置 Dependency：被阻塞的票不再进 Frontier，依赖图出现对应边
    host.handle(serde_json::json!({
        "op": "setIssueBlockedBy",
        "issueId": "you/garden#8",
        "blockedBy": ["you/garden#9"],
    }))
    .unwrap();
    let board = host.snapshot().board.unwrap();
    let columns = board.columns.unwrap();
    assert!(columns.blocked.iter().any(|card| card.id == "you/garden#8"));
    assert!(!columns
        .frontier
        .iter()
        .any(|card| card.id == "you/garden#8"));
    let graph = board.graph.expect("graph after complete read");
    assert!(graph
        .edges
        .iter()
        .any(|edge| edge.from == "you/garden#9" && edge.to == "you/garden#8"));

    // 空列表表示清空全部阻塞边，票重新进入 Frontier。
    host.handle(serde_json::json!({
        "op": "setIssueBlockedBy",
        "issueId": "you/garden#8",
        "blockedBy": [],
    }))
    .unwrap();
    assert!(frontier_ids(&host).contains(&"you/garden#8".into()));

    // 写操作都走同一个接缝
    let log = tracker.log();
    assert!(log.iter().any(|(_, id, op)| id.is_none()
        && matches!(op, TrackerWriteOp::CreateIssue { title, .. } if title == "new one")));
    assert!(log
        .iter()
        .any(|(_, id, op)| id.as_deref() == Some("you/garden#8")
            && matches!(op, TrackerWriteOp::SetBlockedBy { .. })));
}

#[test]
fn claim_and_release_still_route_through_the_seam() {
    let tmp = tempfile::tempdir().unwrap();
    let (tracker, dir) = garden_setup(&tmp);
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    register(&mut host, &dir, "you/garden");

    host.handle(serde_json::json!({
        "op": "claimIssue",
        "issueId": "you/garden#8",
    }))
    .unwrap();
    let columns = host.snapshot().board.unwrap().columns.unwrap();
    assert!(columns
        .in_progress
        .iter()
        .any(|card| card.id == "you/garden#8"));
    assert!(!frontier_ids(&host).contains(&"you/garden#8".into()));

    host.handle(serde_json::json!({
        "op": "releaseIssue",
        "issueId": "you/garden#8",
    }))
    .unwrap();
    assert_eq!(frontier_ids(&host), vec!["you/garden#8", "you/garden#9"]);
}

#[test]
fn failed_writes_are_denied_and_keep_tracker_details() {
    let tmp = tempfile::tempdir().unwrap();
    let (tracker, dir) = garden_setup(&tmp);
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    register(&mut host, &dir, "you/garden");

    tracker.set_write_mode(
        "you/garden",
        WriteMode::Fail("GitHub rejected the update".into()),
    );
    let err = host
        .handle(serde_json::json!({
            "op": "updateIssue",
            "issueId": "you/garden#8",
            "title": "should not stick",
        }))
        .unwrap_err();
    assert!(
        matches!(err, KernelError::Denied(message) if message.contains("GitHub rejected the update"))
    );

    tracker.set_write_mode("you/garden", WriteMode::Offline);
    let err = host
        .handle(serde_json::json!({
            "op": "setIssueOpen",
            "issueId": "you/garden#8",
            "open": false,
        }))
        .unwrap_err();
    assert!(
        matches!(err, KernelError::Denied(message) if message.contains("offline") && message.contains("network down"))
    );

    tracker.set_write_mode("you/garden", WriteMode::Auth);
    let err = host
        .handle(serde_json::json!({
            "op": "addIssueComment",
            "issueId": "you/garden#8",
            "body": "nope",
        }))
        .unwrap_err();
    assert!(
        matches!(err, KernelError::Denied(message) if message.contains("auth-failed") && message.contains("token rejected"))
    );

    tracker.set_write_mode("you/garden", WriteMode::RateLimited(Some(45_000)));
    let err = host
        .handle(serde_json::json!({
            "op": "setIssueBlockedBy",
            "issueId": "you/garden#8",
            "blockedBy": ["you/garden#9"],
        }))
        .unwrap_err();
    assert!(
        matches!(err, KernelError::Denied(message) if message.contains("rate-limited") && message.contains("45000"))
    );

    // 失败后本地状态保持一致：票仍是开放的 Frontier 票
    tracker.set_write_mode("you/garden", WriteMode::Ok);
    let columns = host.snapshot().board.unwrap().columns.unwrap();
    assert!(columns
        .frontier
        .iter()
        .any(|card| card.id == "you/garden#8"));
    assert!(columns.recently_completed.is_empty());
}

#[test]
fn writes_blocked_by_read_errors_keep_consistent_reasons() {
    let tmp = tempfile::tempdir().unwrap();
    let (tracker, dir) = garden_setup(&tmp);
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    register(&mut host, &dir, "you/garden");

    tracker.set_write_mode("you/garden", WriteMode::Ok);
    tracker.set_read_mode("you/garden", ReadMode::Auth);
    let err = host
        .handle(serde_json::json!({
            "op": "claimIssue",
            "issueId": "you/garden#8",
        }))
        .unwrap_err();
    assert!(matches!(err, KernelError::Denied(message) if message.contains("auth-failed")));

    tracker.set_read_mode("you/garden", ReadMode::Offline);
    let err = host
        .handle(serde_json::json!({
            "op": "claimIssue",
            "issueId": "you/garden#8",
        }))
        .unwrap_err();
    assert!(matches!(err, KernelError::Denied(message) if message.contains("offline")));

    tracker.set_read_mode("you/garden", ReadMode::RateLimited(Some(60_000)));
    let err = host
        .handle(serde_json::json!({
            "op": "claimIssue",
            "issueId": "you/garden#8",
        }))
        .unwrap_err();
    assert!(matches!(err, KernelError::Denied(message) if message.contains("rate-limited")));

    // 从未成功读取过的项目
    let fresh_tmp = tempfile::tempdir().unwrap();
    let fresh_dir = make_dir(fresh_tmp.path(), "work/fresh");
    let fresh_tracker = Arc::new(SeamTracker::new());
    fresh_tracker.set_read_mode("you/fresh", ReadMode::Offline);
    let mut fresh = boot(fresh_tmp.path(), fresh_tracker);
    register(&mut fresh, &fresh_dir, "you/fresh");
    let err = fresh
        .handle(serde_json::json!({
            "op": "claimIssue",
            "issueId": "you/fresh#1",
        }))
        .unwrap_err();
    assert!(matches!(err, KernelError::Denied(message) if message.contains("never-fetched")));
}

#[test]
fn create_requires_valid_project_and_issue_refs() {
    let tmp = tempfile::tempdir().unwrap();
    let (tracker, dir) = garden_setup(&tmp);
    let mut host = boot(tmp.path(), tracker);
    let project_id = register(&mut host, &dir, "you/garden");

    let err = host
        .handle(serde_json::json!({
            "op": "createIssue",
            "projectId": "nope",
            "title": "x",
        }))
        .unwrap_err();
    assert!(matches!(err, KernelError::Protocol(_)));

    let err = host
        .handle(serde_json::json!({
            "op": "createIssue",
            "projectId": project_id,
            "title": "  ",
        }))
        .unwrap_err();
    assert!(matches!(err, KernelError::Protocol(message) if message.contains("missing title")));

    let err = host
        .handle(serde_json::json!({
            "op": "setIssueParent",
            "issueId": "you/garden#8",
            "parent": "not-an-issue",
        }))
        .unwrap_err();
    assert!(matches!(err, KernelError::Protocol(message) if message.contains("invalid issue id")));

    let err = host
        .handle(serde_json::json!({
            "op": "setIssueBlockedBy",
            "issueId": "you/garden#8",
        }))
        .unwrap_err();
    assert!(matches!(err, KernelError::Protocol(message) if message.contains("missing blockedBy")));
}

#[test]
fn incomplete_read_draws_no_frontier_or_graph_and_keeps_details() {
    let tmp = tempfile::tempdir().unwrap();
    let (tracker, dir) = garden_setup(&tmp);
    tracker.set_read_mode(
        "you/garden",
        ReadMode::Incomplete("truncated at 100 issues".into()),
    );
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    register(&mut host, &dir, "you/garden");

    let board = host.snapshot().board.unwrap();
    assert_eq!(board.empty, Some(BoardEmptyReason::IncompleteRead));
    assert!(board.columns.is_none());
    assert!(board.graph.is_none());
    match board.refresh {
        RefreshStatus::Incomplete {
            detail: Some(detail),
            ..
        } => assert_eq!(detail, "truncated at 100 issues"),
        other => panic!("expected incomplete with detail, got {other:?}"),
    }

    // 已知数据的详情仍然可看，认领/放领走实时读写
    host.handle(serde_json::json!({
        "op": "focusIssue",
        "issueId": "you/garden#8",
    }))
    .unwrap();
    assert_eq!(
        host.snapshot().board.unwrap().selected.unwrap().id,
        "you/garden#8"
    );
    host.handle(serde_json::json!({
        "op": "claimIssue",
        "issueId": "you/garden#8",
    }))
    .unwrap();

    // 完整读取后四列与依赖图恢复
    tracker.set_read_mode("you/garden", ReadMode::Complete);
    host.handle(serde_json::json!({ "op": "refresh" })).unwrap();
    let board = host.snapshot().board.unwrap();
    assert!(board.columns.is_some());
    assert!(board.graph.is_some());
    assert!(matches!(board.refresh, RefreshStatus::Ready { .. }));
}

#[test]
fn incomplete_read_surfaces_an_incomplete_status_event() {
    let tmp = tempfile::tempdir().unwrap();
    let (tracker, dir) = garden_setup(&tmp);
    tracker.set_read_mode(
        "you/garden",
        ReadMode::Incomplete("pagination stopped early".into()),
    );
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, &dir, "you/garden");
    let out = host.handle(serde_json::json!({ "op": "refresh" })).unwrap();
    let kinds: Vec<_> = out
        .events
        .iter()
        .filter_map(|event| match event {
            host_kernel::HostEvent::RefreshStatusChanged { status, .. } => Some(status.kind()),
            _ => None,
        })
        .collect();
    assert_eq!(kinds.first().copied(), Some("refreshing"));
    assert_eq!(kinds.last().copied(), Some("incomplete"));
}
