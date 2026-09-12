use super::*;

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
fn github_adapter_marks_missing_issue_cursor_as_incomplete() {
    let tracker = scripted(ScriptedGitHub {
        env: [("GH_TOKEN".into(), "tok".into())].into(),
        accept_tokens: ["tok".into()].into(),
        issues: BTreeMap::from([(
            "you/garden".into(),
            vec![plain_node(1, "one"), plain_node(2, "two")],
        )]),
        issue_page_size: 1,
        missing_issue_cursor: true,
        ..Default::default()
    });
    let ctx = probe_ctx("github.com", "you/garden");
    match TrackerPort::read_all(&tracker, &ctx).unwrap() {
        host_kernel::TrackerReadOutcome::Incomplete { issues, detail } => {
            assert_eq!(issues.len(), 1);
            assert!(detail.contains("without a cursor"));
        }
        other => panic!("expected incomplete read, got {other:?}"),
    }
}

#[test]
fn github_adapter_marks_missing_edge_cursor_as_incomplete() {
    let mut issue = plain_node(1, "one");
    issue["blockedBy"] = serde_json::json!({
        "nodes": (2..=102)
            .map(|number| issue_ref(number, &format!("issue {number}")))
            .collect::<Vec<_>>()
    });
    let tracker = scripted(ScriptedGitHub {
        env: [("GH_TOKEN".into(), "tok".into())].into(),
        accept_tokens: ["tok".into()].into(),
        issues: BTreeMap::from([("you/garden".into(), vec![issue])]),
        edge_page_size: 1,
        missing_edge_cursor: true,
        ..Default::default()
    });
    let ctx = probe_ctx("github.com", "you/garden");
    match TrackerPort::read_all(&tracker, &ctx).unwrap() {
        host_kernel::TrackerReadOutcome::Incomplete { detail, .. } => {
            assert!(detail.contains("blockedBy"));
        }
        other => panic!("expected incomplete read, got {other:?}"),
    }
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
