use super::*;

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
    let mut host = boot_local(tmp.path());
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
fn local_markdown_parses_status_type_assignee_parent_and_dependency_semantics() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/garden");
    let issues = project_dir.join(".scratch/feature/issues");
    std::fs::create_dir_all(&issues).unwrap();
    std::fs::write(
        issues.join("01-foundation.md"),
        "# 01 — Foundation\n\nStatus: claimed\nType: task\nAssignee: alice\n\n## Comments\n\nStatus: resolved\n",
    )
    .unwrap();
    std::fs::write(
        issues.join("02-follow-up.md"),
        "# 02 — Follow up\n\nStatus: ready-for-agent\nPart of: 01 — Foundation\nBlocked by: 01 — Foundation\n",
    )
    .unwrap();
    let mut host = boot_local(tmp.path());
    let out = host
        .handle(serde_json::json!({
            "op": "registerProject",
            "name": "garden",
            "localPath": project_dir,
            "githubHost": "local",
            "repository": project_dir,
        }))
        .unwrap();
    let board = out.snapshot.board.unwrap();
    let selected = board
        .columns
        .as_ref()
        .and_then(|columns| columns.in_progress.iter().find(|issue| issue.number == 1))
        .expect("claimed issue card");
    assert!(selected.labels.iter().any(|label| label == "type:task"));
    assert!(selected
        .labels
        .iter()
        .any(|label| label == "status:claimed"));
    assert_eq!(selected.claimed_by, vec!["alice"]);
    let follow_up = board
        .columns
        .unwrap()
        .blocked
        .into_iter()
        .find(|issue| issue.number == 2)
        .expect("blocked follow-up");
    assert!(follow_up
        .labels
        .iter()
        .any(|label| label == "ready-for-agent"));
}

#[test]
fn local_markdown_parses_bullet_relationship_sections() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/bullets");
    let issues = project_dir.join(".scratch/feature/issues");
    std::fs::create_dir_all(&issues).unwrap();
    std::fs::write(
        issues.join("01-parent.md"),
        "# 01 — Parent\n\nStatus: ready-for-agent\n",
    )
    .unwrap();
    std::fs::write(
        issues.join("02-child.md"),
        "# 02 — Child\n\nStatus: ready-for-agent\n\n## Parent\n\n- #1\n\n## Blocked by\n\n- #1\n",
    )
    .unwrap();
    let mut host = boot_local(tmp.path());
    let out = host
        .handle(serde_json::json!({
            "op": "registerProject", "name": "bullets", "localPath": project_dir,
            "githubHost": "local", "repository": project_dir,
        }))
        .unwrap();
    let board = out.snapshot.board.unwrap();
    let child = board
        .columns
        .unwrap()
        .blocked
        .into_iter()
        .find(|issue| issue.number == 2)
        .expect("bullet dependency should block child");
    assert_eq!(child.claimed_by, Vec::<String>::new());
}

#[test]
fn local_markdown_invalid_metadata_is_fail_closed_and_does_not_draw_frontier() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/invalid");
    let issues = project_dir.join(".scratch/feature/issues");
    std::fs::create_dir_all(&issues).unwrap();
    std::fs::write(
        issues.join("01-invalid.md"),
        "# 01 — Invalid\n\nStatus: done\nBlocked by: missing\n",
    )
    .unwrap();
    let mut host = boot_local(tmp.path());
    let out = host
        .handle(serde_json::json!({
            "op": "registerProject",
            "name": "invalid",
            "localPath": project_dir,
            "githubHost": "local",
            "repository": project_dir,
        }))
        .unwrap();
    let board = out.snapshot.board.unwrap();
    assert!(
        board.columns.is_none(),
        "invalid metadata must not be treated as a real board"
    );
    assert_eq!(
        board.empty,
        Some(host_kernel::BoardEmptyReason::IncompleteRead)
    );
    match board.refresh {
        host_kernel::RefreshStatus::Incomplete {
            detail: Some(detail),
            ..
        } => {
            assert!(detail.contains("invalid Status"));
            assert!(detail.contains("invalid dependency"));
        }
        other => panic!("expected incomplete refresh, got {other:?}"),
    }
    assert!(!format!("{:?}", out.snapshot.projects[0].connection).contains("GitHub"));
}

#[test]
fn local_markdown_supports_create_edit_comment_and_relationship_writes() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/garden");
    let issues = project_dir.join(".scratch/feature/issues");
    std::fs::create_dir_all(&issues).unwrap();
    std::fs::write(
        issues.join("01-parent.md"),
        "# 01 — Parent\n\nStatus: ready-for-agent\n",
    )
    .unwrap();
    std::fs::write(
        issues.join("02-child.md"),
        "# 02 — Child\n\nStatus: ready-for-agent\n",
    )
    .unwrap();
    let mut host = boot_local(tmp.path());
    let project_id = host
        .handle(serde_json::json!({
            "op": "registerProject", "name": "garden", "localPath": project_dir,
            "githubHost": "local", "repository": project_dir,
        }))
        .unwrap()
        .snapshot
        .focused_project_id;
    host.handle(serde_json::json!({ "op": "updateIssue", "issueId": format!("{}#2", project_dir.display()), "title": "Child renamed", "body": "new body" })).unwrap();
    host.handle(serde_json::json!({ "op": "addIssueComment", "issueId": format!("{}#2", project_dir.display()), "body": "looks good" })).unwrap();
    host.handle(serde_json::json!({ "op": "setIssueParent", "issueId": format!("{}#2", project_dir.display()), "parent": format!("{}#1", project_dir.display()) })).unwrap();
    host.handle(serde_json::json!({ "op": "setIssueBlockedBy", "issueId": format!("{}#2", project_dir.display()), "blockedBy": [format!("{}#1", project_dir.display())] })).unwrap();
    host.handle(serde_json::json!({ "op": "createIssue", "projectId": project_id, "title": "Created", "body": "created body" })).unwrap();
    let body = std::fs::read_to_string(issues.join("02-child.md")).unwrap();
    assert!(body.starts_with("# 02 — Child renamed\n"));
    assert!(body.contains("Child renamed"));
    assert!(body.contains("new body"));
    assert!(body.contains("## Comments"));
    assert!(body.contains("looks good"));
    assert!(body.contains("Part of: 1"));
    assert!(body.contains("Blocked by: 1"));
    assert!(issues.join("03-created.md").exists());
}

#[test]
fn local_markdown_edit_does_not_duplicate_colon_bearing_body_lines() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/colon-body");
    let issues = project_dir.join(".scratch/feature/issues");
    std::fs::create_dir_all(&issues).unwrap();
    std::fs::write(
        issues.join("01-work.md"),
        "# 01 — Work\n\nStatus: ready-for-agent\nType: task\n\nNote: keep this\n",
    )
    .unwrap();
    let mut host = boot_local(tmp.path());
    let issue_id = format!("{}#1", project_dir.display());
    host.handle(serde_json::json!({
        "op": "registerProject", "name": "colon-body", "localPath": project_dir,
        "githubHost": "local", "repository": project_dir,
    }))
    .unwrap();

    host.handle(serde_json::json!({
        "op": "updateIssue",
        "issueId": issue_id,
        "title": "Work",
        "body": "Note: keep this\n\nMore body",
    }))
    .unwrap();

    let body = std::fs::read_to_string(issues.join("01-work.md")).unwrap();
    assert_eq!(body.matches("Note: keep this").count(), 1, "{body}");
    assert!(body.contains("Status: ready-for-agent"), "{body}");
    assert!(body.contains("Type: task"), "{body}");
}

#[test]
fn local_markdown_replaces_blocked_by_atomically() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/atomic-dependencies");
    let issues = project_dir.join(".scratch/feature/issues");
    std::fs::create_dir_all(&issues).unwrap();
    std::fs::write(
        issues.join("01-first.md"),
        "# 01 — First\n\nStatus: ready-for-agent\n",
    )
    .unwrap();
    std::fs::write(
        issues.join("02-second.md"),
        "# 02 — Second\n\nStatus: ready-for-agent\n",
    )
    .unwrap();
    std::fs::write(
        issues.join("03-child.md"),
        "# 03 — Child\n\nStatus: ready-for-agent\nBlocked by: 1\n",
    )
    .unwrap();
    let mut host = boot_local(tmp.path());
    let issue_id = format!("{}#3", project_dir.display());
    host.handle(serde_json::json!({
        "op": "registerProject", "name": "atomic-dependencies", "localPath": project_dir,
        "githubHost": "local", "repository": project_dir,
    }))
    .unwrap();

    let result = host.handle(serde_json::json!({
        "op": "setIssueBlockedBy",
        "issueId": issue_id,
        "blockedBy": [
            format!("{}#2", project_dir.display()),
            format!("{}#99", project_dir.display()),
        ],
    }));
    assert!(result.is_err());

    let body = std::fs::read_to_string(issues.join("03-child.md")).unwrap();
    assert!(body.contains("Blocked by: 1"), "{body}");
    assert!(!body.contains("Blocked by: 2"), "{body}");
}

#[test]
fn local_markdown_rejects_dependency_cycles_before_writing() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/dependency-cycle-write");
    let issues = project_dir.join(".scratch/feature/issues");
    std::fs::create_dir_all(&issues).unwrap();
    std::fs::write(
        issues.join("01-first.md"),
        "# 01 — First\n\nStatus: ready-for-agent\nBlocked by: 2\n",
    )
    .unwrap();
    std::fs::write(
        issues.join("02-second.md"),
        "# 02 — Second\n\nStatus: ready-for-agent\nBlocked by: None\n",
    )
    .unwrap();
    let mut host = boot_local(tmp.path());
    host.handle(serde_json::json!({
        "op": "registerProject", "name": "dependency-cycle-write", "localPath": project_dir,
        "githubHost": "local", "repository": project_dir,
    }))
    .unwrap();

    let result = host.handle(serde_json::json!({
        "op": "setIssueBlockedBy",
        "issueId": format!("{}#2", project_dir.display()),
        "blockedBy": [format!("{}#1", project_dir.display())],
    }));
    assert!(result.is_err(), "a dependency cycle must be rejected");

    let body = std::fs::read_to_string(issues.join("02-second.md")).unwrap();
    assert!(body.contains("Blocked by: None"), "{body}");
    assert!(!body.contains("Blocked by: 1"), "{body}");
}

#[test]
fn local_markdown_rejects_parent_cycles_before_writing() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/parent-cycle-write");
    let issues = project_dir.join(".scratch/feature/issues");
    std::fs::create_dir_all(&issues).unwrap();
    std::fs::write(
        issues.join("01-first.md"),
        "# 01 — First\n\nStatus: ready-for-agent\nPart of: 2\n",
    )
    .unwrap();
    std::fs::write(
        issues.join("02-second.md"),
        "# 02 — Second\n\nStatus: ready-for-agent\n",
    )
    .unwrap();
    let mut host = boot_local(tmp.path());
    host.handle(serde_json::json!({
        "op": "registerProject", "name": "parent-cycle-write", "localPath": project_dir,
        "githubHost": "local", "repository": project_dir,
    }))
    .unwrap();

    let result = host.handle(serde_json::json!({
        "op": "setIssueParent",
        "issueId": format!("{}#2", project_dir.display()),
        "parent": format!("{}#1", project_dir.display()),
    }));
    assert!(result.is_err(), "a parent cycle must be rejected");

    let body = std::fs::read_to_string(issues.join("02-second.md")).unwrap();
    assert!(!body.contains("Part of:"), "{body}");
}

#[test]
fn local_markdown_parent_cycles_are_fail_closed_on_read() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/parent-cycle-read");
    let issues = project_dir.join(".scratch/feature/issues");
    std::fs::create_dir_all(&issues).unwrap();
    std::fs::write(
        issues.join("01-first.md"),
        "# 01 — First\n\nStatus: ready-for-agent\nPart of: 2\n",
    )
    .unwrap();
    std::fs::write(
        issues.join("02-second.md"),
        "# 02 — Second\n\nStatus: ready-for-agent\nPart of: 1\n",
    )
    .unwrap();
    let mut host = boot_local(tmp.path());

    let out = host
        .handle(serde_json::json!({
            "op": "registerProject", "name": "parent-cycle-read", "localPath": project_dir,
            "githubHost": "local", "repository": project_dir,
        }))
        .unwrap();
    let board = out.snapshot.board.unwrap();
    assert!(board.columns.is_none());
    match board.refresh {
        host_kernel::RefreshStatus::Incomplete {
            detail: Some(detail),
            ..
        } => assert!(detail.contains("parent cycle"), "{detail}"),
        other => panic!("expected parent cycle warning, got {other:?}"),
    }
}

#[test]
fn local_markdown_duplicate_issue_numbers_are_fail_closed() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/duplicate-numbers");
    let first = project_dir.join(".scratch/first/issues");
    let second = project_dir.join(".scratch/second/issues");
    std::fs::create_dir_all(&first).unwrap();
    std::fs::create_dir_all(&second).unwrap();
    std::fs::write(
        first.join("01-first.md"),
        "# 01 — First\n\nStatus: ready-for-agent\n",
    )
    .unwrap();
    std::fs::write(
        second.join("01-second.md"),
        "# 01 — Second\n\nStatus: ready-for-agent\n",
    )
    .unwrap();
    let mut host = boot_local(tmp.path());

    let out = host
        .handle(serde_json::json!({
            "op": "registerProject", "name": "duplicate-numbers", "localPath": project_dir,
            "githubHost": "local", "repository": project_dir,
        }))
        .unwrap();
    let board = out.snapshot.board.unwrap();
    assert!(board.columns.is_none());
    match board.refresh {
        host_kernel::RefreshStatus::Incomplete {
            detail: Some(detail),
            ..
        } => assert!(detail.contains("duplicate issue number #1"), "{detail}"),
        other => panic!("expected duplicate number warning, got {other:?}"),
    }
}

#[test]
fn local_markdown_relation_writes_reject_an_incomplete_graph() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/incomplete-parent-write");
    let issues = project_dir.join(".scratch/feature/issues");
    std::fs::create_dir_all(&issues).unwrap();
    std::fs::write(
        issues.join("01-parent.md"),
        "# 01 — Parent\n\nStatus: ready-for-agent\n",
    )
    .unwrap();
    std::fs::write(
        issues.join("02-child.md"),
        "# 02 — Child\n\nStatus: ready-for-agent\n",
    )
    .unwrap();
    std::fs::write(
        issues.join("03-unclear.md"),
        "# 03 — Unclear\n\nStatus: ready-for-agent\nBlocked by: missing\n",
    )
    .unwrap();
    let repository = project_dir.to_string_lossy().to_string();
    let secrets = tmp.path().join("secrets.json");
    let ctx = ProbeContext {
        tracker: TrackerKind::LocalMarkdown,
        github_host: "local",
        repository: &repository,
        secrets_pat: None,
        secrets_path: &secrets,
    };

    let result = LocalMarkdownTracker.set_parent(
        &ctx,
        &format!("{repository}#2"),
        Some(&format!("{repository}#1")),
    );
    assert!(
        result.is_err(),
        "incomplete relation data must block parent writes"
    );

    let body = std::fs::read_to_string(issues.join("02-child.md")).unwrap();
    assert!(!body.contains("Part of:"), "{body}");

    let blocker = format!("{repository}#1");
    let result =
        LocalMarkdownTracker.set_blocked_by(&ctx, &format!("{repository}#2"), &[], &[blocker]);
    assert!(
        result.is_err(),
        "incomplete relation data must block dependency writes"
    );

    let body = std::fs::read_to_string(issues.join("02-child.md")).unwrap();
    assert!(!body.contains("Blocked by:"), "{body}");
}

#[test]
fn local_markdown_can_clear_body_and_release_explicit_assignee() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/clear");
    let issues = project_dir.join(".scratch/feature/issues");
    std::fs::create_dir_all(&issues).unwrap();
    std::fs::write(
        issues.join("01-work.md"),
        "# 01 — Work\n\nStatus: claimed\nAssignee: alice\n\nOld body\n",
    )
    .unwrap();
    let mut host = boot_local(tmp.path());
    let issue_id = format!("{}#1", project_dir.display());
    host.handle(serde_json::json!({
        "op": "registerProject", "name": "clear", "localPath": project_dir,
        "githubHost": "local", "repository": project_dir,
    }))
    .unwrap();
    host.handle(serde_json::json!({ "op": "updateIssue", "issueId": issue_id, "title": "Work", "body": "" }))
        .unwrap();
    host.handle(serde_json::json!({ "op": "releaseIssue", "issueId": issue_id }))
        .unwrap();
    let body = std::fs::read_to_string(issues.join("01-work.md")).unwrap();
    assert!(!body.contains("Assignee:"));
    assert!(!body.contains("Old body"));
    assert!(body.contains("Status: ready-for-agent"));
}

#[test]
fn local_markdown_claim_release_and_restart_preserve_tracker_state() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/garden");
    let issues = project_dir.join(".scratch/feature/issues");
    std::fs::create_dir_all(&issues).unwrap();
    std::fs::write(
        issues.join("01-work.md"),
        "# 01 — Work\n\nStatus: ready-for-agent\n",
    )
    .unwrap();
    let mut host = boot_local(tmp.path());
    let issue_id = format!("{}#1", project_dir.display());
    host.handle(serde_json::json!({ "op": "registerProject", "name": "garden", "localPath": project_dir, "githubHost": "local", "repository": project_dir })).unwrap();
    host.handle(serde_json::json!({ "op": "claimIssue", "issueId": issue_id }))
        .unwrap();
    assert!(std::fs::read_to_string(issues.join("01-work.md"))
        .unwrap()
        .contains("Status: claimed"));
    host.handle(serde_json::json!({ "op": "releaseIssue", "issueId": issue_id }))
        .unwrap();
    assert!(std::fs::read_to_string(issues.join("01-work.md"))
        .unwrap()
        .contains("Status: ready-for-agent"));
    drop(host);
    let host = boot_local(tmp.path());
    let issue = host
        .snapshot()
        .board
        .unwrap()
        .columns
        .unwrap()
        .frontier
        .into_iter()
        .find(|issue| issue.number == 1)
        .expect("reloaded frontier issue");
    assert_eq!(issue.title, "Work");
}

#[test]
fn local_markdown_accepts_legacy_closed_true_but_rejects_dependency_cycles() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/legacy");
    let issues = project_dir.join(".scratch/feature/issues");
    std::fs::create_dir_all(&issues).unwrap();
    std::fs::write(issues.join("01-done.md"), "# 01 — Done\n\nClosed: true\n").unwrap();
    std::fs::write(
        issues.join("02-a.md"),
        "# 02 — A\n\nStatus: ready-for-agent\nBlocked by: 03\n",
    )
    .unwrap();
    std::fs::write(
        issues.join("03-b.md"),
        "# 03 — B\n\nStatus: ready-for-agent\nBlocked by: 02\n",
    )
    .unwrap();
    let mut host = boot_local(tmp.path());
    let out = host.handle(serde_json::json!({ "op": "registerProject", "name": "legacy", "localPath": project_dir, "githubHost": "local", "repository": project_dir })).unwrap();
    let board = out.snapshot.board.unwrap();
    assert!(board.columns.is_none());
    match board.refresh {
        host_kernel::RefreshStatus::Incomplete {
            detail: Some(detail),
            ..
        } => assert!(detail.contains("dependency cycle")),
        other => panic!("expected cycle warning, got {other:?}"),
    }
}

#[test]
fn local_markdown_failure_does_not_mention_github_credentials() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/missing-local-tracker");
    let mut host = boot_local(tmp.path());
    let out = host
        .handle(serde_json::json!({
            "op": "registerProject",
            "name": "missing-local-tracker",
            "localPath": project_dir,
            "githubHost": "local",
            "repository": project_dir,
        }))
        .unwrap();

    match &out.snapshot.projects[0].connection {
        ProjectConnection::Unreachable { message, .. } => {
            assert!(message.contains("本地 Markdown tracker"), "{message}");
            assert!(!message.contains("GitHub"), "{message}");
        }
        other => panic!("expected local tracker failure, got {other:?}"),
    }
}

#[test]
fn local_markdown_clears_legacy_parent_header_and_section_forms() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/legacy-parent-clear");
    let issues = project_dir.join(".scratch/feature/issues");
    std::fs::create_dir_all(&issues).unwrap();
    std::fs::write(
        issues.join("01-parent.md"),
        "# 01 — Parent\n\nStatus: ready-for-agent\n",
    )
    .unwrap();
    std::fs::write(
        issues.join("02-header.md"),
        "# 02 — Header child\n\nStatus: ready-for-agent\nParent: 1\n",
    )
    .unwrap();
    std::fs::write(
        issues.join("03-section.md"),
        "# 03 — Section child\n\nStatus: ready-for-agent\n\n## Parent\n\n- #1\n",
    )
    .unwrap();
    let repository = project_dir.to_string_lossy().to_string();
    let secrets = tmp.path().join("secrets.json");
    let ctx = ProbeContext {
        tracker: TrackerKind::LocalMarkdown,
        github_host: "local",
        repository: &repository,
        secrets_pat: None,
        secrets_path: &secrets,
    };

    for number in [2, 3] {
        LocalMarkdownTracker
            .set_parent(&ctx, &format!("{repository}#{number}"), None)
            .unwrap();
    }

    let outcome = LocalMarkdownTracker.read_all(&ctx).unwrap();
    let issues = match outcome {
        host_kernel::TrackerReadOutcome::Complete { issues } => issues,
        other => panic!("expected complete read, got {other:?}"),
    };
    assert!(issues
        .iter()
        .filter(|issue| issue.number >= 2)
        .all(|issue| issue.parent.is_none()));
}

#[test]
fn local_markdown_legacy_closed_true_does_not_hide_an_invalid_status() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/legacy-invalid-status");
    let issues = project_dir.join(".scratch/feature/issues");
    std::fs::create_dir_all(&issues).unwrap();
    std::fs::write(
        issues.join("01-invalid.md"),
        "# 01 — Invalid\n\nStatus: done\nClosed: true\n",
    )
    .unwrap();
    let repository = project_dir.to_string_lossy().to_string();
    let secrets = tmp.path().join("secrets.json");
    let ctx = ProbeContext {
        tracker: TrackerKind::LocalMarkdown,
        github_host: "local",
        repository: &repository,
        secrets_pat: None,
        secrets_path: &secrets,
    };

    match LocalMarkdownTracker.read_all(&ctx).unwrap() {
        host_kernel::TrackerReadOutcome::Incomplete { detail, .. } => {
            assert!(detail.contains("invalid Status: done"), "{detail}");
        }
        other => panic!("invalid explicit Status must fail closed, got {other:?}"),
    }
}

#[test]
fn local_markdown_can_close_a_legacy_closed_false_issue() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/legacy-close");
    let issues = project_dir.join(".scratch/feature/issues");
    std::fs::create_dir_all(&issues).unwrap();
    std::fs::write(
        issues.join("01-open.md"),
        "# 01 — Open\n\nStatus: ready-for-agent\nClosed: false\n",
    )
    .unwrap();
    let repository = project_dir.to_string_lossy().to_string();
    let secrets = tmp.path().join("secrets.json");
    let ctx = ProbeContext {
        tracker: TrackerKind::LocalMarkdown,
        github_host: "local",
        repository: &repository,
        secrets_pat: None,
        secrets_path: &secrets,
    };

    let closed = LocalMarkdownTracker
        .close_issue(&ctx, &format!("{repository}#1"))
        .unwrap();
    assert!(!closed.open);
    let body = std::fs::read_to_string(issues.join("01-open.md")).unwrap();
    assert!(body.contains("Status: resolved"), "{body}");
    assert!(!body.contains("Closed:"), "{body}");
}

#[test]
fn local_markdown_file_changes_trigger_a_host_refresh() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/file-change-refresh");
    let issues = project_dir.join(".scratch/feature/issues");
    std::fs::create_dir_all(&issues).unwrap();
    std::fs::write(
        issues.join("01-first.md"),
        "# 01 — First\n\nStatus: ready-for-agent\n\noriginal body\n",
    )
    .unwrap();
    let mut host = boot_local(tmp.path());
    let out = host
        .handle(serde_json::json!({
            "op": "registerProject", "name": "file-change-refresh", "localPath": project_dir,
            "githubHost": "local", "repository": project_dir,
        }))
        .unwrap();
    assert_eq!(out.snapshot.projects[0].issue_counts.total, 1);
    let issue_id = format!("{}#1", project_dir.display());
    host.handle(serde_json::json!({ "op": "focusIssue", "issueId": issue_id }))
        .unwrap();
    host.handle(serde_json::json!({ "op": "loadIssueDocument", "issueId": issue_id }))
        .unwrap();
    assert!(matches!(
        host.snapshot().board.unwrap().selected.unwrap().document,
        host_kernel::IssueDocumentState::Ready { ref body, .. } if body.contains("original body")
    ));

    std::fs::write(
        issues.join("01-first.md"),
        "# 01 — First\n\nStatus: ready-for-agent\n\nexternally changed body\n",
    )
    .unwrap();
    std::fs::write(
        issues.join("02-second.md"),
        "# 02 — Second\n\nStatus: ready-for-agent\n",
    )
    .unwrap();
    let refreshed = host
        .handle(serde_json::json!({ "op": "tick", "nowMs": 1_800_000_000_000_u64 }))
        .unwrap();
    assert_eq!(refreshed.snapshot.projects[0].issue_counts.total, 2);
    assert_eq!(
        refreshed.snapshot.board.unwrap().selected.unwrap().document,
        host_kernel::IssueDocumentState::Unloaded
    );
}

#[test]
fn explicit_local_markdown_refresh_invalidates_changed_issue_documents() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = make_dir(tmp.path(), "work/explicit-file-change-refresh");
    let issues = project_dir.join(".scratch/feature/issues");
    std::fs::create_dir_all(&issues).unwrap();
    let issue_path = issues.join("01-first.md");
    std::fs::write(
        &issue_path,
        "# 01 — First\n\nStatus: ready-for-agent\n\noriginal body\n",
    )
    .unwrap();
    let mut host = boot_local(tmp.path());
    let out = host
        .handle(serde_json::json!({
            "op": "registerProject", "name": "explicit-file-change-refresh", "localPath": project_dir,
            "githubHost": "local", "repository": project_dir,
        }))
        .unwrap();
    let project_id = out.snapshot.focused_project_id;
    let issue_id = format!("{}#1", project_dir.display());
    host.handle(serde_json::json!({ "op": "focusIssue", "issueId": issue_id }))
        .unwrap();
    host.handle(serde_json::json!({ "op": "loadIssueDocument", "issueId": issue_id }))
        .unwrap();

    std::fs::write(
        issue_path,
        "# 01 — First\n\nStatus: ready-for-agent\n\nexternally changed body\n",
    )
    .unwrap();
    let refreshed = host
        .handle(serde_json::json!({ "op": "refresh", "projectId": project_id }))
        .unwrap();

    assert_eq!(
        refreshed.snapshot.board.unwrap().selected.unwrap().document,
        host_kernel::IssueDocumentState::Unloaded
    );
}
