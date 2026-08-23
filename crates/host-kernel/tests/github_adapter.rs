use std::collections::BTreeMap;
use std::sync::Arc;

use host_kernel::{BootRequest, GitHubTracker, HostKernel, ScriptedGitHub, SystemAppearance};

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
    let mapped = host_kernel::map_github_issue_node(&node(serde_json::json!({})), "you/garden")
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
