use std::sync::Arc;

use host_kernel::{
    BootRequest, HostKernel, IssueDocumentFailureKind, IssueDocumentState, IssueRecord,
    MemoryTracker, SystemAppearance,
};

fn host_with_document() -> (HostKernel, Arc<MemoryTracker>) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    let project_dir = root.join("work/garden");
    std::fs::create_dir_all(&project_dir).unwrap();
    let tracker = Arc::new(MemoryTracker::new());
    tracker.set_issues(
        "you/garden",
        vec![IssueRecord::open("you/garden", 98, "Document detail")],
    );
    tracker.set_issue_body(
        "you/garden#98",
        "# Question\n\nRead **all** constraints.\n\n- first\n- second",
    );
    let mut host = HostKernel::boot_with(
        BootRequest {
            app_local_data_dir: root,
            app_log_dir: project_dir.parent().unwrap().join("logs"),
            system_locale: "en-US".into(),
            system_appearance: SystemAppearance::Light,
            host_display_name: "Studio".into(),
        },
        tracker.clone(),
    )
    .unwrap();
    host.handle(serde_json::json!({
        "op": "registerProject",
        "name": "garden",
        "localPath": project_dir,
        "repository": "you/garden",
    }))
    .unwrap();
    (host, tracker)
}

#[test]
fn issue_document_load_has_explicit_states_and_keeps_the_last_body_on_failure() {
    let (mut host, tracker) = host_with_document();
    host.handle(serde_json::json!({
        "op": "focusIssue",
        "issueId": "you/garden#98",
    }))
    .unwrap();
    let selected = host.snapshot().board.unwrap().selected.unwrap();
    assert_eq!(
        selected.document,
        IssueDocumentState::Loading {
            body: None,
            fetched_at_ms: None,
        }
    );

    host.handle(serde_json::json!({
        "op": "loadIssueDocument",
        "issueId": "you/garden#98",
    }))
    .unwrap();
    let selected = host.snapshot().board.unwrap().selected.unwrap();
    let (loaded_body, fetched_at_ms) = match selected.document {
        IssueDocumentState::Ready {
            body,
            fetched_at_ms,
        } => (body, fetched_at_ms),
        state => panic!("expected ready, got {state:?}"),
    };
    assert_eq!(
        loaded_body,
        "# Question\n\nRead **all** constraints.\n\n- first\n- second"
    );
    assert!(fetched_at_ms > 0);

    tracker.fail_issue_document_offline("you/garden#98");
    host.handle(serde_json::json!({
        "op": "loadIssueDocument",
        "issueId": "you/garden#98",
    }))
    .unwrap();
    let selected = host.snapshot().board.unwrap().selected.unwrap();
    match selected.document {
        IssueDocumentState::Stale {
            body,
            fetched_at_ms: stale_at,
            failure,
        } => {
            assert_eq!(body, loaded_body);
            assert_eq!(stale_at, fetched_at_ms);
            assert_eq!(failure.kind, IssueDocumentFailureKind::Offline);
        }
        state => panic!("expected stale, got {state:?}"),
    }
}

#[test]
fn first_issue_document_failure_is_not_rendered_as_an_empty_success() {
    let (mut host, tracker) = host_with_document();
    tracker.fail_issue_document_rate_limited("you/garden#98", Some(45_000));
    host.handle(serde_json::json!({
        "op": "focusIssue",
        "issueId": "you/garden#98",
    }))
    .unwrap();
    host.handle(serde_json::json!({
        "op": "loadIssueDocument",
        "issueId": "you/garden#98",
    }))
    .unwrap();

    let selected = host.snapshot().board.unwrap().selected.unwrap();
    match selected.document {
        IssueDocumentState::Failed { failure } => {
            assert_eq!(failure.kind, IssueDocumentFailureKind::RateLimited);
            assert_eq!(failure.retry_after_ms, Some(45_000));
        }
        state => panic!("expected failed, got {state:?}"),
    }
}

#[test]
fn a_loaded_document_survives_host_restart_and_remains_available_when_detail_refresh_is_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = tmp.path().join("work/garden");
    std::fs::create_dir_all(&project_dir).unwrap();
    let tracker = Arc::new(MemoryTracker::new());
    tracker.set_issues(
        "you/garden",
        vec![IssueRecord::open("you/garden", 98, "Persistent document")],
    );
    tracker.set_issue_body(
        "you/garden#98",
        "# Persisted\n\nLast successful Tracker body.",
    );
    let request = || BootRequest {
        app_local_data_dir: tmp.path().to_path_buf(),
        app_log_dir: tmp.path().join("logs"),
        system_locale: "en-US".into(),
        system_appearance: SystemAppearance::Light,
        host_display_name: "Studio".into(),
    };
    let mut host = HostKernel::boot_with(request(), tracker.clone()).unwrap();
    host.handle(serde_json::json!({
        "op": "registerProject",
        "name": "garden",
        "localPath": project_dir,
        "repository": "you/garden",
    }))
    .unwrap();
    host.handle(serde_json::json!({ "op": "focusIssue", "issueId": "you/garden#98" }))
        .unwrap();
    host.handle(serde_json::json!({ "op": "loadIssueDocument", "issueId": "you/garden#98" }))
        .unwrap();
    drop(host);

    tracker.fail_issue_document_offline("you/garden#98");
    let mut restarted = HostKernel::boot_with(request(), tracker).unwrap();
    restarted
        .handle(serde_json::json!({ "op": "focusIssue", "issueId": "you/garden#98" }))
        .unwrap();
    assert!(matches!(
        restarted.snapshot().board.unwrap().selected.unwrap().document,
        IssueDocumentState::Loading { body: Some(ref body), fetched_at_ms: Some(_) }
            if body.contains("Last successful Tracker body")
    ));
    restarted
        .handle(serde_json::json!({ "op": "loadIssueDocument", "issueId": "you/garden#98" }))
        .unwrap();
    assert!(matches!(
        restarted.snapshot().board.unwrap().selected.unwrap().document,
        IssueDocumentState::Stale { ref body, .. } if body.contains("Last successful Tracker body")
    ));
}

#[test]
fn changing_a_project_tracker_registration_discards_its_loaded_document() {
    let (mut host, _) = host_with_document();
    let project = host.snapshot().projects.into_iter().next().unwrap();
    host.handle(serde_json::json!({ "op": "focusIssue", "issueId": "you/garden#98" }))
        .unwrap();
    host.handle(serde_json::json!({ "op": "loadIssueDocument", "issueId": "you/garden#98" }))
        .unwrap();
    assert!(matches!(
        host.snapshot().board.unwrap().selected.unwrap().document,
        IssueDocumentState::Ready { .. }
    ));

    host.handle(serde_json::json!({
        "op": "editProject",
        "projectId": project.id,
        "name": project.name,
        "localPath": project.local_path,
        "githubHost": "github.example.com",
        "repository": project.repository,
    }))
    .unwrap();
    host.handle(serde_json::json!({ "op": "focusIssue", "issueId": "you/garden#98" }))
        .unwrap();

    assert_eq!(
        host.snapshot().board.unwrap().selected.unwrap().document,
        IssueDocumentState::Loading {
            body: None,
            fetched_at_ms: None,
        }
    );
}
