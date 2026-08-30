use std::path::Path;
use std::sync::Arc;

use host_kernel::{
    BootRequest, CenterView, HostKernel, IssueRecord, MemoryTracker, SystemAppearance,
    WorkspaceView,
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

fn start_run(host: &mut HostKernel, client_id: &str, project_id: &str) -> String {
    host.handle(serde_json::json!({
        "op": "startUnboundRun",
        "clientInstanceId": client_id,
        "projectId": project_id,
        "agentId": "grok-build",
        "values": {
            "model": "grok-4.6",
            "effort": "high",
            "permission-mode": "default",
            "always-approve": "false",
            "sandbox": "off",
            "initial-instruction": "",
            "additional-args": ""
        },
        "openingText": "client state integration",
    }))
    .unwrap()
    .snapshot
    .focused_run_id
}

#[test]
fn each_client_keeps_its_own_focused_run() {
    let tmp = tempfile::tempdir().unwrap();
    let garden = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    let mut host = HostKernel::boot_with(boot_req(tmp.path()), tracker).unwrap();
    let project_id = host
        .handle(serde_json::json!({
            "op": "registerProject",
            "clientInstanceId": "tauri-desktop",
            "name": "garden",
            "localPath": garden,
            "repository": "you/garden",
        }))
        .unwrap()
        .snapshot
        .focused_project_id
        .clone();

    let first_run = start_run(&mut host, "tauri-desktop", &project_id);
    host.handle(serde_json::json!({
        "op": "focusRun",
        "clientInstanceId": "tauri-desktop",
        "runId": first_run,
    }))
    .unwrap();
    host.handle(serde_json::json!({
        "op": "focusProject",
        "clientInstanceId": "browser-window",
        "projectId": project_id,
    }))
    .unwrap();
    let second_run = start_run(&mut host, "browser-window", &project_id);
    host.handle(serde_json::json!({
        "op": "focusRun",
        "clientInstanceId": "browser-window",
        "runId": second_run,
    }))
    .unwrap();
    assert_ne!(first_run, second_run);

    let desktop = host
        .handle(serde_json::json!({
            "op": "snapshot",
            "clientInstanceId": "tauri-desktop",
        }))
        .unwrap();
    assert_eq!(desktop.snapshot.focused_run_id, first_run);
    assert_eq!(desktop.snapshot.workspace_view, WorkspaceView::Run);

    let browser = host
        .handle(serde_json::json!({
            "op": "snapshot",
            "clientInstanceId": "browser-window",
        }))
        .unwrap();
    assert_eq!(browser.snapshot.focused_run_id, second_run);
    assert_eq!(browser.snapshot.workspace_view, WorkspaceView::Run);
}

fn make_dir(root: &Path, name: &str) -> std::path::PathBuf {
    let dir = root.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn browser_and_desktop_keep_project_issue_and_center_view_isolated() {
    let tmp = tempfile::tempdir().unwrap();
    let garden = make_dir(tmp.path(), "work/garden");
    let notes = make_dir(tmp.path(), "work/notes");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "garden issue"));
    tracker.add_issue(IssueRecord::open("you/notes", 2, "notes issue"));
    let mut host = HostKernel::boot_with(boot_req(tmp.path()), tracker).unwrap();

    let garden_id = host
        .handle(serde_json::json!({
            "op": "registerProject",
            "clientInstanceId": "tauri-desktop",
            "name": "garden",
            "localPath": garden,
            "repository": "you/garden",
        }))
        .unwrap()
        .snapshot
        .focused_project_id
        .clone();
    let notes_id = host
        .handle(serde_json::json!({
            "op": "registerProject",
            "clientInstanceId": "browser-window",
            "name": "notes",
            "localPath": notes,
            "repository": "you/notes",
        }))
        .unwrap()
        .snapshot
        .focused_project_id
        .clone();

    host.handle(serde_json::json!({
        "op": "focusProject",
        "clientInstanceId": "tauri-desktop",
        "projectId": garden_id,
    }))
    .unwrap();
    host.handle(serde_json::json!({
        "op": "focusIssue",
        "clientInstanceId": "tauri-desktop",
        "issueId": "you/garden#1",
    }))
    .unwrap();
    host.handle(serde_json::json!({
        "op": "setCenterView",
        "clientInstanceId": "tauri-desktop",
        "view": "graph",
    }))
    .unwrap();

    host.handle(serde_json::json!({
        "op": "focusProject",
        "clientInstanceId": "browser-window",
        "projectId": notes_id,
    }))
    .unwrap();
    host.handle(serde_json::json!({
        "op": "focusIssue",
        "clientInstanceId": "browser-window",
        "issueId": "you/notes#2",
    }))
    .unwrap();

    let desktop = host
        .handle(serde_json::json!({
            "op": "snapshot",
            "clientInstanceId": "tauri-desktop",
        }))
        .unwrap()
        .snapshot;
    assert_eq!(desktop.focused_project_id, garden_id);
    assert_eq!(desktop.center_view, CenterView::Graph);
    assert_eq!(desktop.board.unwrap().selected.unwrap().id, "you/garden#1");

    let browser = host
        .handle(serde_json::json!({
            "op": "snapshot",
            "clientInstanceId": "browser-window",
        }))
        .unwrap()
        .snapshot;
    assert_eq!(browser.focused_project_id, notes_id);
    assert_eq!(browser.center_view, CenterView::Board);
    assert_eq!(browser.board.unwrap().selected.unwrap().id, "you/notes#2");
}
