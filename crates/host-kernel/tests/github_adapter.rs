use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use host_kernel::{
    BootRequest, DependencyRef, GitHubTracker, HostKernel, IssueEdit, MemoryTracker, ProbeContext,
    ScriptedGitHub, SystemAppearance, TrackerPort,
};

fn scripted_host(issues: Vec<serde_json::Value>) -> HostKernel {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    let dir = root.join("work/garden");
    std::fs::create_dir_all(&dir).unwrap();
    let tracker = GitHubTracker::scripted(ScriptedGitHub {
        env: [("GH_TOKEN".into(), "tok".into())].into(),
        accept_tokens: ["tok".into()].into(),
        issues: BTreeMap::from([("you/garden".into(), issues)]),
        ..Default::default()
    });
    let mut host = HostKernel::boot_with(
        BootRequest {
            app_local_data_dir: root,
            app_log_dir: dir.parent().unwrap().join("logs"),
            system_locale: "en-US".into(),
            system_appearance: SystemAppearance::Light,
            host_display_name: "Studio".into(),
        },
        Arc::new(tracker),
    )
    .unwrap();
    host.handle(serde_json::json!({
        "op": "registerProject",
        "name": "garden",
        "localPath": dir,
        "repository": "you/garden",
    }))
    .unwrap();
    host
}

fn node(extra: serde_json::Value) -> serde_json::Value {
    let mut base = serde_json::json!({
        "number": 50,
        "title": "GitHub 四列看板与 Frontier",
        "state": "OPEN",
        "url": "https://github.com/you/garden/issues/50",
        "repository": { "nameWithOwner": "you/garden" },
        "assignees": { "nodes": [{ "login": "ada" }] },
        "labels": { "nodes": [{ "name": "ready-for-agent" }] },
        "parent": {
            "number": 45,
            "title": "spec",
            "state": "OPEN",
            "repository": { "nameWithOwner": "you/garden" }
        },
        "subIssues": { "nodes": [{
            "number": 51,
            "title": "child",
            "state": "OPEN",
            "repository": { "nameWithOwner": "you/garden" }
        }] },
        "issueDependenciesSummary": { "blockedBy": 1 },
        "blockedBy": { "nodes": [{
            "number": 12,
            "title": "gate",
            "state": "OPEN",
            "repository": { "nameWithOwner": "you/other" }
        }] },
        "blocking": { "nodes": [] },
        "relatedIssues": { "nodes": [{
            "number": 99,
            "title": "related not blocking",
            "state": "OPEN",
            "repository": { "nameWithOwner": "you/garden" }
        }] },
        "body": "Blocked by: #77\n\n- [ ] #51"
    });
    if let (Some(base_obj), Some(extra_obj)) = (base.as_object_mut(), extra.as_object()) {
        for (key, value) in extra_obj {
            base_obj.insert(key.clone(), value.clone());
        }
    }
    base
}

fn plain_node(number: u64, title: &str) -> serde_json::Value {
    serde_json::json!({
        "number": number,
        "title": title,
        "state": "OPEN",
        "url": format!("https://github.com/you/garden/issues/{number}"),
        "repository": { "nameWithOwner": "you/garden" },
        "assignees": { "nodes": [] },
        "labels": { "nodes": [] },
        "parent": null,
        "subIssues": { "nodes": [] },
        "issueDependenciesSummary": { "blockedBy": 0 },
        "blockedBy": { "nodes": [] },
        "blocking": { "nodes": [] },
    })
}

fn probe_ctx<'a>(github_host: &'a str, repository: &'a str) -> ProbeContext<'a> {
    ProbeContext {
        github_host,
        repository,
        secrets_pat: None,
        secrets_path: Path::new("/tmp/host-secrets.json"),
    }
}

fn scripted(script: ScriptedGitHub) -> GitHubTracker {
    GitHubTracker::scripted(script)
}

#[test]
fn github_adapter_maps_native_blocked_by_sub_issues_and_assignee() {
    let host = scripted_host(vec![node(serde_json::json!({}))]);
    let board = host.snapshot().board.unwrap();
    let columns = board.columns.unwrap();
    assert_eq!(columns.in_progress[0].id, "you/garden#50");
    assert_eq!(columns.in_progress[0].claimed_by, vec!["ada"]);
    assert!(columns.frontier.is_empty());

    let mut host = host;
    host.handle(serde_json::json!({
        "op": "focusIssue",
        "issueId": "you/garden#50",
    }))
    .unwrap();
    let detail = host.snapshot().board.unwrap().selected.unwrap();
    assert_eq!(detail.parent.unwrap().id, "you/garden#45");
    assert_eq!(detail.children[0].id, "you/garden#51");
    assert_eq!(detail.blocked_by[0].id, "you/other#12");
    assert!(detail
        .blocked_by
        .iter()
        .all(|link| link.id != "you/garden#99"));
    assert!(detail
        .blocked_by
        .iter()
        .all(|link| link.id != "you/garden#77"));
}

#[test]
fn github_adapter_does_not_treat_relates_to_or_body_as_dependency() {
    let host = scripted_host(vec![node(serde_json::json!({
        "issueDependenciesSummary": { "blockedBy": 0 },
        "blockedBy": { "nodes": [] },
        "assignees": { "nodes": [] },
    }))]);
    let columns = host.snapshot().board.unwrap().columns.unwrap();
    assert_eq!(
        columns
            .frontier
            .iter()
            .map(|card| card.id.clone())
            .collect::<Vec<_>>(),
        vec!["you/garden#50"]
    );
}

#[test]
fn github_adapter_ignores_pull_requests_in_the_issue_list() {
    let host = scripted_host(vec![
        serde_json::json!({
            "number": 3,
            "title": "a pr",
            "state": "OPEN",
            "pull_request": { "url": "https://github.com/you/garden/pull/3" },
            "assignees": { "nodes": [] },
            "labels": { "nodes": [] },
            "blockedBy": { "nodes": [] },
        }),
        node(serde_json::json!({
            "number": 4,
            "title": "real issue",
            "assignees": { "nodes": [] },
            "issueDependenciesSummary": { "blockedBy": 0 },
            "blockedBy": { "nodes": [] },
        })),
    ]);
    let columns = host.snapshot().board.unwrap().columns.unwrap();
    let ids: Vec<_> = columns
        .frontier
        .iter()
        .map(|card| card.id.clone())
        .collect();
    assert_eq!(ids, vec!["you/garden#4"]);
}

#[test]
fn github_closed_blocker_is_not_an_unfinished_latch() {
    let host = scripted_host(vec![node(serde_json::json!({
        "assignees": { "nodes": [] },
        "issueDependenciesSummary": { "blockedBy": 0 },
        "blockedBy": { "nodes": [{
            "number": 49,
            "title": "already done",
            "state": "CLOSED",
            "repository": { "nameWithOwner": "you/garden" }
        }] },
    }))]);
    let columns = host.snapshot().board.unwrap().columns.unwrap();
    assert_eq!(columns.frontier[0].id, "you/garden#50");
}

#[test]
fn github_summary_without_visible_open_blocker_is_unclear() {
    let host = scripted_host(vec![node(serde_json::json!({
        "assignees": { "nodes": [] },
        "issueDependenciesSummary": { "blockedBy": 1 },
        "blockedBy": { "nodes": [] },
    }))]);
    let columns = host.snapshot().board.unwrap().columns.unwrap();
    assert_eq!(columns.blocked[0].id, "you/garden#50");
    assert!(columns.frontier.is_empty());
}

#[test]
fn github_adapter_maps_rate_limit_with_retry_after() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("work/garden");
    std::fs::create_dir_all(&dir).unwrap();
    let tracker = GitHubTracker::scripted(ScriptedGitHub {
        env: [("GH_TOKEN".into(), "tok".into())].into(),
        accept_tokens: ["tok".into()].into(),
        rate_limited: true,
        retry_after_ms: Some(45_000),
        ..Default::default()
    });
    let mut host = HostKernel::boot_with(
        BootRequest {
            app_local_data_dir: tmp.path().to_path_buf(),
            app_log_dir: tmp.path().join("logs"),
            system_locale: "en-US".into(),
            system_appearance: SystemAppearance::Light,
            host_display_name: "Studio".into(),
        },
        Arc::new(tracker),
    )
    .unwrap();
    host.handle(serde_json::json!({
        "op": "registerProject",
        "name": "garden",
        "localPath": dir,
        "repository": "you/garden",
    }))
    .unwrap();
    match host.snapshot().board.unwrap().refresh {
        host_kernel::RefreshStatus::RateLimited { retry_at_ms, .. } => {
            assert!(retry_at_ms.is_some());
        }
        other => panic!("expected rate-limited, got {other:?}"),
    }
}

#[test]
fn github_adapter_claim_and_release_use_viewer_assignee() {
    let mut host = scripted_host(vec![node(serde_json::json!({
        "assignees": { "nodes": [] },
        "issueDependenciesSummary": { "blockedBy": 0 },
        "blockedBy": { "nodes": [] },
    }))]);
    assert_eq!(
        host.snapshot()
            .board
            .unwrap()
            .columns
            .unwrap()
            .frontier
            .iter()
            .map(|card| card.id.clone())
            .collect::<Vec<_>>(),
        vec!["you/garden#50"]
    );
    host.handle(serde_json::json!({
        "op": "claimIssue",
        "issueId": "you/garden#50",
    }))
    .unwrap();
    host.handle(serde_json::json!({
        "op": "focusIssue",
        "issueId": "you/garden#50",
    }))
    .unwrap();
    let detail = host.snapshot().board.unwrap().selected.unwrap();
    assert_eq!(detail.claimed_by, vec!["me"]);
    assert_eq!(detail.parent.unwrap().id, "you/garden#45");
    assert!(host
        .snapshot()
        .board
        .unwrap()
        .columns
        .unwrap()
        .frontier
        .is_empty());

    host.handle(serde_json::json!({
        "op": "releaseIssue",
        "issueId": "you/garden#50",
    }))
    .unwrap();
    host.handle(serde_json::json!({
        "op": "focusIssue",
        "issueId": "you/garden#50",
    }))
    .unwrap();
    assert!(host
        .snapshot()
        .board
        .unwrap()
        .selected
        .unwrap()
        .claimed_by
        .is_empty());
}

#[test]
fn github_adapter_claim_failure_does_not_claim() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("work/garden");
    std::fs::create_dir_all(&dir).unwrap();
    let tracker = GitHubTracker::scripted(ScriptedGitHub {
        env: [("GH_TOKEN".into(), "tok".into())].into(),
        accept_tokens: ["tok".into()].into(),
        issues: BTreeMap::from([(
            "you/garden".into(),
            vec![node(serde_json::json!({
                "assignees": { "nodes": [] },
                "issueDependenciesSummary": { "blockedBy": 0 },
                "blockedBy": { "nodes": [] },
            }))],
        )]),
        write_fail: true,
        ..Default::default()
    });
    let mut host = HostKernel::boot_with(
        BootRequest {
            app_local_data_dir: tmp.path().to_path_buf(),
            app_log_dir: tmp.path().join("logs"),
            system_locale: "en-US".into(),
            system_appearance: SystemAppearance::Light,
            host_display_name: "Studio".into(),
        },
        Arc::new(tracker),
    )
    .unwrap();
    host.handle(serde_json::json!({
        "op": "registerProject",
        "name": "garden",
        "localPath": dir,
        "repository": "you/garden",
    }))
    .unwrap();
    let err = host
        .handle(serde_json::json!({
            "op": "claimIssue",
            "issueId": "you/garden#50",
        }))
        .unwrap_err();
    assert!(matches!(err, host_kernel::KernelError::Denied(_)));
    host.handle(serde_json::json!({
        "op": "focusIssue",
        "issueId": "you/garden#50",
    }))
    .unwrap();
    assert!(host
        .snapshot()
        .board
        .unwrap()
        .selected
        .unwrap()
        .claimed_by
        .is_empty());
}

#[test]
fn map_github_issue_node_ignores_related_payload() {
    let mapped = host_kernel::map_github_issue_node(
        &node(serde_json::json!({})),
        "you/garden",
        "github.com",
    )
    .expect("issue");
    assert_eq!(mapped.assignees, vec!["ada"]);
    assert_eq!(mapped.parent.unwrap().number, 45);
    assert_eq!(mapped.children[0].number, 51);
    assert_eq!(
        mapped.blocked_by.len(),
        1,
        "relatedIssues and body must not become extra blockers"
    );
}

#[test]
fn github_adapter_creates_edits_closes_reopens_and_comments() {
    let tracker = scripted(ScriptedGitHub {
        env: [("GH_TOKEN".into(), "tok".into())].into(),
        accept_tokens: ["tok".into()].into(),
        ..Default::default()
    });
    let ctx = probe_ctx("github.com", "you/garden");

    let created = tracker.create_issue(&ctx, "new idea", "details").unwrap();
    assert_eq!(created.repository, "you/garden");
    assert_eq!(created.number, 1);
    assert_eq!(created.title, "new idea");
    assert!(created.open);
    assert_eq!(created.url, "https://github.com/you/garden/issues/1");

    let edited = tracker
        .update_issue(
            &ctx,
            "you/garden#1",
            IssueEdit {
                title: Some("renamed"),
                body: Some("new body"),
            },
        )
        .unwrap();
    assert_eq!(edited.title, "renamed");
    assert!(edited.open);

    let closed = tracker.close_issue(&ctx, "you/garden#1").unwrap();
    assert!(!closed.open);
    assert!(closed.closed_at.is_some());

    let reopened = tracker.reopen_issue(&ctx, "you/garden#1").unwrap();
    assert!(reopened.open);

    let comment = tracker.add_comment(&ctx, "you/garden#1", "done").unwrap();
    assert_eq!(comment.body, "done");
    assert_eq!(
        comment.url,
        "https://github.com/you/garden/issues/1#issuecomment-1"
    );

    let issues = tracker.read_issues(&ctx).unwrap();
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].title, "renamed");
    assert!(issues[0].open);
    assert_eq!(issues[0].url, "https://github.com/you/garden/issues/1");
}

#[test]
fn github_adapter_writes_sub_issue_and_dependency_edges_natively() {
    let tracker = scripted(ScriptedGitHub {
        env: [("GH_TOKEN".into(), "tok".into())].into(),
        accept_tokens: ["tok".into()].into(),
        issues: BTreeMap::from([(
            "you/garden".into(),
            vec![
                plain_node(1, "parent"),
                plain_node(2, "child"),
                plain_node(9, "blocker"),
            ],
        )]),
        ..Default::default()
    });
    let ctx = probe_ctx("github.com", "you/garden");

    tracker
        .set_parent(&ctx, "you/garden#2", Some("you/garden#1"))
        .unwrap();
    tracker
        .add_blocked_by(&ctx, "you/garden#2", "you/garden#9")
        .unwrap();

    let issues = tracker.read_issues(&ctx).unwrap();
    let parent = issues.iter().find(|issue| issue.number == 1).unwrap();
    let child = issues.iter().find(|issue| issue.number == 2).unwrap();
    let blocker = issues.iter().find(|issue| issue.number == 9).unwrap();
    assert_eq!(child.parent.as_ref().unwrap().number, 1);
    assert!(parent.children.iter().any(|child| child.number == 2));
    assert!(matches!(
        &child.blocked_by[0],
        DependencyRef::Known(known) if known.number == 9
    ));
    assert!(blocker.blocking.iter().any(|blocked| blocked.number == 2));

    tracker
        .update_issue(
            &ctx,
            "you/garden#2",
            IssueEdit {
                body: Some("Blocked by: #1"),
                ..Default::default()
            },
        )
        .unwrap();
    let issues = tracker.read_issues(&ctx).unwrap();
    let child = issues.iter().find(|issue| issue.number == 2).unwrap();
    assert_eq!(child.blocked_by.len(), 1, "body must not dual-write edges");
    assert_eq!(
        child.parent.as_ref().unwrap().number,
        1,
        "body must not re-parent"
    );

    tracker
        .remove_blocked_by(&ctx, "you/garden#2", "you/garden#9")
        .unwrap();
    tracker.set_parent(&ctx, "you/garden#2", None).unwrap();

    let issues = tracker.read_issues(&ctx).unwrap();
    let parent = issues.iter().find(|issue| issue.number == 1).unwrap();
    let child = issues.iter().find(|issue| issue.number == 2).unwrap();
    let blocker = issues.iter().find(|issue| issue.number == 9).unwrap();
    assert!(child.parent.is_none());
    assert!(parent.children.is_empty());
    assert!(child.blocked_by.is_empty());
    assert!(blocker.blocking.is_empty());
}

#[test]
fn github_adapter_reads_more_than_five_hundred_issues_via_cursor_pages() {
    let issues: Vec<_> = (1..=510)
        .map(|number| plain_node(number, &format!("issue {number}")))
        .collect();
    let tracker = scripted(ScriptedGitHub {
        env: [("GH_TOKEN".into(), "tok".into())].into(),
        accept_tokens: ["tok".into()].into(),
        issues: BTreeMap::from([("you/garden".into(), issues)]),
        issue_page_size: 100,
        ..Default::default()
    });
    let ctx = probe_ctx("github.com", "you/garden");
    let read = tracker.read_issues(&ctx).unwrap();
    assert_eq!(read.len(), 510, "list must not be capped at 500");
    assert_eq!(read[0].number, 1);
    assert_eq!(read[509].number, 510);
}

#[test]
fn github_adapter_paginates_blocked_by_blocking_and_sub_issues_to_end() {
    let blockers: Vec<_> = (1..=5)
        .map(|n| {
            serde_json::json!({
                "number": 100 + n,
                "title": format!("blocker {n}"),
                "state": "OPEN",
                "repository": { "nameWithOwner": "you/other" }
            })
        })
        .collect();
    let blockings: Vec<_> = (1..=3)
        .map(|n| {
            serde_json::json!({
                "number": 200 + n,
                "title": format!("blocking {n}"),
                "state": "OPEN",
                "repository": { "nameWithOwner": "you/garden" }
            })
        })
        .collect();
    let sub_issues: Vec<_> = (1..=4)
        .map(|n| {
            serde_json::json!({
                "number": 300 + n,
                "title": format!("sub {n}"),
                "state": "OPEN",
                "repository": { "nameWithOwner": "you/garden" }
            })
        })
        .collect();
    let main = serde_json::json!({
        "number": 50,
        "title": "heavy",
        "state": "OPEN",
        "url": "https://github.com/you/garden/issues/50",
        "repository": { "nameWithOwner": "you/garden" },
        "assignees": { "nodes": [] },
        "labels": { "nodes": [] },
        "parent": null,
        "subIssues": { "nodes": sub_issues },
        "issueDependenciesSummary": { "blockedBy": 5 },
        "blockedBy": { "nodes": blockers },
        "blocking": { "nodes": blockings },
    });
    let tracker = scripted(ScriptedGitHub {
        env: [("GH_TOKEN".into(), "tok".into())].into(),
        accept_tokens: ["tok".into()].into(),
        issues: BTreeMap::from([("you/garden".into(), vec![main])]),
        edge_page_size: 2,
        ..Default::default()
    });
    let ctx = probe_ctx("github.com", "you/garden");
    let read = tracker.read_issues(&ctx).unwrap();
    assert_eq!(read.len(), 1);
    let issue = &read[0];
    assert_eq!(issue.blocked_by.len(), 5);
    assert_eq!(issue.blocking.len(), 3);
    assert_eq!(issue.children.len(), 4);
    assert!(
        issue
            .blocked_by
            .iter()
            .all(|dep| matches!(dep, DependencyRef::Known(_))),
        "all blockers visible, no unclear padding"
    );
}

#[test]
fn github_adapter_graphql_business_error_keeps_detail() {
    let message = "Could not resolve to a Repository with the name 'you/garden'.";
    let script = ScriptedGitHub {
        env: [("GH_TOKEN".into(), "tok".into())].into(),
        accept_tokens: ["tok".into()].into(),
        graphql_business_error: Some(message.into()),
        ..Default::default()
    };
    let tracker = scripted(script.clone());
    let ctx = probe_ctx("github.com", "you/garden");
    let err = tracker.read_issues(&ctx).unwrap_err();
    match err {
        host_kernel::TrackerReadError::Failed { detail } => {
            assert_eq!(detail.as_deref(), Some(message));
        }
        other => panic!("expected business error, got {other:?}"),
    }

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("work/garden");
    std::fs::create_dir_all(&dir).unwrap();
    let mut host = HostKernel::boot_with(
        BootRequest {
            app_local_data_dir: tmp.path().to_path_buf(),
            app_log_dir: tmp.path().join("logs"),
            system_locale: "en-US".into(),
            system_appearance: SystemAppearance::Light,
            host_display_name: "Studio".into(),
        },
        Arc::new(GitHubTracker::scripted(script)),
    )
    .unwrap();
    host.handle(serde_json::json!({
        "op": "registerProject",
        "name": "garden",
        "localPath": dir,
        "repository": "you/garden",
    }))
    .unwrap();
    let board = host.snapshot().board.unwrap();
    assert_eq!(board.empty, Some(host_kernel::BoardEmptyReason::NoData));
    assert!(matches!(
        board.refresh,
        host_kernel::RefreshStatus::TrackerError { detail: Some(detail), .. }
            if detail == message
    ));
}

#[test]
fn github_adapter_graphql_auth_error_maps_to_auth_with_detail() {
    let tracker = scripted(ScriptedGitHub {
        env: [("GH_TOKEN".into(), "tok".into())].into(),
        accept_tokens: ["tok".into()].into(),
        graphql_auth_error: Some("Bad credentials".into()),
        ..Default::default()
    });
    let ctx = probe_ctx("github.com", "you/garden");
    let err = tracker.read_issues(&ctx).unwrap_err();
    match err {
        host_kernel::TrackerReadError::Auth {
            kind: host_kernel::AuthFailureKind::Rejected,
            detail: Some(detail),
            ..
        } => assert_eq!(detail, "Bad credentials"),
        other => panic!("expected auth failure, got {other:?}"),
    }
}

#[test]
fn github_adapter_offline_keeps_network_detail() {
    let tracker = scripted(ScriptedGitHub {
        env: [("GH_TOKEN".into(), "tok".into())].into(),
        accept_tokens: ["tok".into()].into(),
        unreachable: true,
        ..Default::default()
    });
    let ctx = probe_ctx("github.com", "you/garden");
    let err = tracker.read_issues(&ctx).unwrap_err();
    match err {
        host_kernel::TrackerReadError::Offline { detail, .. } => {
            assert!(detail.is_some());
        }
        other => panic!("expected offline, got {other:?}"),
    }
}

#[test]
fn github_adapter_self_hosted_links_use_registered_host() {
    let mut self_hosted = node(serde_json::json!({
        "number": 50,
        "title": "self hosted",
        "assignees": { "nodes": [] },
        "issueDependenciesSummary": { "blockedBy": 1 },
    }));
    let object = self_hosted.as_object_mut().unwrap();
    object.remove("url");
    object
        .get_mut("parent")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("url");
    object["subIssues"]["nodes"][0]
        .as_object_mut()
        .unwrap()
        .remove("url");
    object["blockedBy"]["nodes"][0]
        .as_object_mut()
        .unwrap()
        .remove("url");
    let tracker = scripted(ScriptedGitHub {
        env: [("GH_TOKEN".into(), "tok".into())].into(),
        accept_tokens: ["tok".into()].into(),
        issues: BTreeMap::from([("you/garden".into(), vec![self_hosted])]),
        ..Default::default()
    });
    let ctx = probe_ctx("ghe.example.com", "you/garden");

    let read = tracker.read_issues(&ctx).unwrap();
    assert_eq!(read[0].url, "https://ghe.example.com/you/garden/issues/50");
    assert_eq!(
        read[0].parent.as_ref().unwrap().url,
        "https://ghe.example.com/you/garden/issues/45"
    );
    assert_eq!(
        read[0].children[0].url,
        "https://ghe.example.com/you/garden/issues/51"
    );
    let DependencyRef::Known(blocker) = &read[0].blocked_by[0] else {
        panic!("expected known blocker");
    };
    assert_eq!(blocker.url, "https://ghe.example.com/you/other/issues/12");

    let created = tracker.create_issue(&ctx, "self", "hosted").unwrap();
    assert_eq!(created.number, 51, "next number after #50");
    assert_eq!(created.url, "https://ghe.example.com/you/garden/issues/51");

    let comment = tracker.add_comment(&ctx, "you/garden#50", "note").unwrap();
    assert!(comment
        .url
        .starts_with("https://ghe.example.com/you/garden/issues/50"));
}

#[test]
fn memory_tracker_covers_full_capability_surface() {
    let tracker = MemoryTracker::new();
    let ctx = probe_ctx("github.com", "you/garden");

    let created = tracker.create_issue(&ctx, "new", "body").unwrap();
    assert_eq!(created.repository, "you/garden");
    assert_eq!(created.number, 1);
    assert_eq!(created.url, "https://github.com/you/garden/issues/1");

    let edited = tracker
        .update_issue(
            &ctx,
            "you/garden#1",
            IssueEdit {
                title: Some("renamed"),
                body: None,
            },
        )
        .unwrap();
    assert_eq!(edited.title, "renamed");

    let closed = TrackerPort::close_issue(&tracker, &ctx, "you/garden#1").unwrap();
    assert!(!closed.open);
    let reopened = tracker.reopen_issue(&ctx, "you/garden#1").unwrap();
    assert!(reopened.open);

    let comment = tracker.add_comment(&ctx, "you/garden#1", "note").unwrap();
    assert_eq!(comment.body, "note");

    let claimed = tracker.claim_issue(&ctx, "you/garden#1").unwrap();
    assert_eq!(claimed.assignees, vec!["me"]);
    let released = tracker.release_issue(&ctx, "you/garden#1").unwrap();
    assert!(released.assignees.is_empty());

    tracker.add_issue(host_kernel::IssueRecord::open("you/garden", 2, "child"));
    tracker.add_issue(host_kernel::IssueRecord::open("you/garden", 9, "blocker"));
    tracker
        .set_parent(&ctx, "you/garden#2", Some("you/garden#1"))
        .unwrap();
    tracker
        .add_blocked_by(&ctx, "you/garden#2", "you/garden#9")
        .unwrap();

    let issues = tracker.read_issues(&ctx).unwrap();
    let child = issues.iter().find(|issue| issue.number == 2).unwrap();
    let parent = issues.iter().find(|issue| issue.number == 1).unwrap();
    let blocker = issues.iter().find(|issue| issue.number == 9).unwrap();
    assert_eq!(child.parent.as_ref().unwrap().number, 1);
    assert!(parent.children.iter().any(|child| child.number == 2));
    assert!(matches!(
        &child.blocked_by[0],
        DependencyRef::Known(known) if known.number == 9
    ));
    assert!(blocker.blocking.iter().any(|blocked| blocked.number == 2));

    tracker
        .update_issue(
            &ctx,
            "you/garden#2",
            IssueEdit {
                body: Some("Blocked by: #9"),
                ..Default::default()
            },
        )
        .unwrap();
    let issues = tracker.read_issues(&ctx).unwrap();
    let child = issues.iter().find(|issue| issue.number == 2).unwrap();
    assert_eq!(child.blocked_by.len(), 1, "body must not dual-write edges");

    tracker
        .remove_blocked_by(&ctx, "you/garden#2", "you/garden#9")
        .unwrap();
    tracker.set_parent(&ctx, "you/garden#2", None).unwrap();
    let issues = tracker.read_issues(&ctx).unwrap();
    let child = issues.iter().find(|issue| issue.number == 2).unwrap();
    let parent = issues.iter().find(|issue| issue.number == 1).unwrap();
    let blocker = issues.iter().find(|issue| issue.number == 9).unwrap();
    assert!(child.parent.is_none());
    assert!(parent.children.is_empty());
    assert!(child.blocked_by.is_empty());
    assert!(blocker.blocking.is_empty());
}

#[test]
fn memory_tracker_self_hosted_create_links_use_host() {
    let tracker = MemoryTracker::new();
    let ctx = probe_ctx("ghe.example.com", "you/garden");
    let created = tracker.create_issue(&ctx, "t", "b").unwrap();
    assert_eq!(created.url, "https://ghe.example.com/you/garden/issues/1");
}

#[test]
fn memory_tracker_write_failure_is_reported() {
    let tracker = MemoryTracker::new();
    tracker.fail_claim("you/garden");
    let ctx = probe_ctx("github.com", "you/garden");
    assert!(matches!(
        tracker.create_issue(&ctx, "t", "b"),
        Err(host_kernel::TrackerWriteError::Failed { .. })
    ));
    assert!(matches!(
        tracker.add_comment(&ctx, "you/garden#1", "hi"),
        Err(host_kernel::TrackerWriteError::Failed { .. })
    ));
    assert!(matches!(
        tracker.add_blocked_by(&ctx, "you/garden#1", "you/garden#2"),
        Err(host_kernel::TrackerWriteError::Failed { .. })
    ));
}
