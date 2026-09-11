use super::*;

#[test]
fn browser_supplements_issue_115_launch_form_behavior() {
    let tmp = tempfile::tempdir().unwrap();
    let project = make_dir(tmp.path(), "work/issue-115-ui");
    let field = |id: &str, kind: AgentFieldKind, options: &[&str], folded: bool| AgentField {
        id: id.into(),
        label: id.into(),
        kind,
        options: options.iter().map(|value| (*value).to_string()).collect(),
        option_filter: None,
        required: id == "model" || id == "effort",
        folded,
    };
    let fields = vec![
        field("model", AgentFieldKind::Select, &["fast", "deep"], false),
        field("effort", AgentFieldKind::Select, &["low", "high"], false),
        field("always-approve", AgentFieldKind::Boolean, &[], false),
        field("initial-instruction", AgentFieldKind::Multiline, &[], false),
        field("profile", AgentFieldKind::Text, &[], false),
        field(
            "approval",
            AgentFieldKind::Select,
            &["on-request", "never"],
            false,
        ),
        field("sandbox", AgentFieldKind::Select, &[], false),
        field("agent", AgentFieldKind::Text, &[], false),
        field("add-dir", AgentFieldKind::Text, &[], false),
        field("additional-args", AgentFieldKind::Text, &[], true),
    ];
    let seed = BTreeMap::from([
        ("model".into(), "fast".into()),
        ("effort".into(), "low".into()),
        ("always-approve".into(), "false".into()),
        ("initial-instruction".into(), String::new()),
        ("profile".into(), String::new()),
        ("approval".into(), "on-request".into()),
        ("sandbox".into(), "off".into()),
        ("agent".into(), String::new()),
        ("add-dir".into(), String::new()),
        ("additional-args".into(), String::new()),
    ]);
    let grok = host_kernel::MemoryAgent::installed_grok().with_fields(fields.clone(), seed.clone());
    let codex =
        host_kernel::MemoryAgent::installed("codex", "Codex", "codex").with_fields(fields, seed);
    let mut host = HostKernel::boot_with_ports(
        boot_req(tmp.path()),
        host_kernel::KernelPorts {
            tracker: Arc::new(MemoryTracker::new()) as _,
            agents: vec![Arc::new(grok) as _, Arc::new(codex) as _],
            launch_env: Arc::new(host_kernel::MemoryLaunchEnv::with_path("/mem/bin")) as _,
            sessions: host_kernel::MemorySessionFactory::new() as _,
        },
    )
    .unwrap();
    register(&mut host, "garden", &project, "you/issue-115-ui");
    run_browser_e2e(host, "issue-115-launch.mjs", &[]);
}

#[test]
fn browser_clients_resize_move_and_restore_workbench_panels_independently() {
    let tmp = tempfile::tempdir().unwrap();
    let project = make_dir(tmp.path(), "work/issue-116-ui");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open(
        "you/issue-116-ui",
        1,
        "panel layout issue",
    ));
    for number in 2..=24 {
        tracker.add_issue(IssueRecord::open(
            "you/issue-116-ui",
            number,
            format!("scrollable panel issue {number}"),
        ));
    }
    let mut host = HostKernel::boot_with_ports(
        boot_req(tmp.path()),
        host_kernel::KernelPorts {
            tracker,
            agents: vec![Arc::new(host_kernel::MemoryAgent::installed_grok()) as _],
            launch_env: Arc::new(host_kernel::MemoryLaunchEnv::with_path("/mem/bin")) as _,
            sessions: host_kernel::MemorySessionFactory::new() as _,
        },
    )
    .unwrap();
    pin_board_test_time(&mut host);
    let project_id = register(&mut host, "garden", &project, "you/issue-116-ui");
    start_bound_grok(&mut host, &project_id, "you/issue-116-ui#1");

    run_browser_e2e(host, "panel-layout.mjs", &[]);
}

#[test]
fn browser_prevents_duplicate_run_launch_and_preserves_the_failed_draft_for_retry() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 100, "core user journey"));
    let sessions = host_kernel::MemorySessionFactory::new();
    sessions.fail_next("pty unavailable");
    let mut host = HostKernel::boot_with_ports(
        boot_req(tmp.path()),
        host_kernel::KernelPorts {
            tracker,
            agents: vec![Arc::new(host_kernel::MemoryAgent::installed_grok()) as _],
            launch_env: Arc::new(host_kernel::MemoryLaunchEnv::with_path("/mem/bin")) as _,
            sessions,
        },
    )
    .unwrap();
    pin_board_test_time(&mut host);
    register(&mut host, "garden", &dir, "you/garden");
    run_browser_e2e(host, "run-launch-resilience.mjs", &[]);
}

#[test]
fn browser_preserves_the_custom_usage_range_when_submission_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 100, "core user journey"));
    let mut host = boot(tmp.path(), tracker);
    pin_board_test_time(&mut host);
    register(&mut host, "garden", &dir, "you/garden");
    run_browser_e2e(host, "usage-form-resilience.mjs", &[]);
}

#[test]
fn browser_explains_why_an_agent_is_unavailable_before_launch() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 100, "core user journey"));
    let mut host = HostKernel::boot_with_ports(
        boot_req(tmp.path()),
        host_kernel::KernelPorts {
            tracker,
            agents: vec![Arc::new(host_kernel::MemoryAgent::missing_grok()) as _],
            launch_env: Arc::new(host_kernel::MemoryLaunchEnv::with_path("/opt/empty")) as _,
            sessions: host_kernel::MemorySessionFactory::new() as _,
        },
    )
    .unwrap();
    pin_board_test_time(&mut host);
    register(&mut host, "garden", &dir, "you/garden");
    run_browser_e2e(host, "agent-unavailable.mjs", &[]);
}

#[test]
fn browser_manages_projects_from_the_desktop_sidebar_without_losing_context() {
    let tmp = tempfile::tempdir().unwrap();
    let active_dir = make_dir(tmp.path(), "work/active");
    let stopped_dir = make_dir(tmp.path(), "work/stopped");
    let fallback_dir = make_dir(tmp.path(), "work/fallback");
    let added_dir = make_dir(tmp.path(), "work/added");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/active", 1, "active issue"));
    tracker.add_issue(IssueRecord::open("you/stopped", 1, "stopped issue"));
    tracker.add_issue(IssueRecord::open("you/fallback", 1, "fallback issue"));
    tracker.add_issue(IssueRecord::open("you/added", 1, "added issue"));
    let sessions = host_kernel::MemorySessionFactory::new();
    let mut host = HostKernel::boot_with_ports(
        boot_req(tmp.path()),
        host_kernel::KernelPorts {
            tracker,
            agents: vec![Arc::new(host_kernel::MemoryAgent::installed_grok()) as _],
            launch_env: Arc::new(host_kernel::MemoryLaunchEnv::with_path("/mem/bin")) as _,
            sessions: Arc::clone(&sessions) as _,
        },
    )
    .unwrap();
    pin_board_test_time(&mut host);
    let active_id = host
        .handle(serde_json::json!({
            "op": "registerProject",
            "name": "active-project",
            "localPath": active_dir,
            "repository": "you/active",
        }))
        .unwrap()
        .snapshot
        .focused_project_id;
    let stopped_id = host
        .handle(serde_json::json!({
            "op": "registerProject",
            "name": "stopped-project",
            "localPath": stopped_dir,
            "repository": "you/stopped",
        }))
        .unwrap()
        .snapshot
        .focused_project_id;
    host.handle(serde_json::json!({
        "op": "registerProject",
        "name": "fallback-project",
        "localPath": fallback_dir,
        "repository": "you/fallback",
    }))
    .unwrap();
    host.handle(serde_json::json!({
        "op": "focusProject",
        "projectId": stopped_id,
    }))
    .unwrap();
    start_bound_grok(&mut host, &stopped_id, "you/stopped#1");
    sessions.last_session().unwrap().finish(2);
    host.handle(serde_json::json!({ "op": "snapshot" }))
        .unwrap();
    host.handle(serde_json::json!({
        "op": "focusProject",
        "projectId": active_id,
    }))
    .unwrap();
    start_bound_grok(&mut host, &active_id, "you/active#1");
    host.handle(serde_json::json!({
        "op": "focusProject",
        "projectId": stopped_id,
    }))
    .unwrap();
    let snapshot = host.snapshot();
    assert!(
        snapshot
            .projects
            .iter()
            .find(|project| project.id == active_id)
            .unwrap()
            .has_active_run
    );
    assert!(
        snapshot
            .projects
            .iter()
            .find(|project| project.id == stopped_id)
            .unwrap()
            .has_execution_stopped
    );
    run_browser_e2e(
        host,
        "project-management.mjs",
        &[("ADDED_PROJECT_DIR", added_dir.as_path())],
    );
}

#[test]
fn browser_keeps_issue_and_run_lifecycles_distinct_through_terminal_actions() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/lifecycle");
    assert!(Command::new("git")
        .arg("init")
        .current_dir(&dir)
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .args(["config", "user.email", "agent-taskboard@example.invalid"])
        .current_dir(&dir)
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .args(["config", "user.name", "Agent Taskboard"])
        .current_dir(&dir)
        .status()
        .unwrap()
        .success());
    std::fs::write(dir.join("notes.txt"), "baseline\n").unwrap();
    assert!(Command::new("git")
        .args(["add", "notes.txt"])
        .current_dir(&dir)
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .args(["commit", "-m", "test baseline"])
        .current_dir(&dir)
        .status()
        .unwrap()
        .success());

    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open(
        "you/lifecycle",
        1,
        "active lifecycle issue",
    ));
    tracker.add_issue(IssueRecord::open(
        "you/lifecycle",
        2,
        "continue lifecycle issue",
    ));
    tracker.add_issue(IssueRecord::open(
        "you/lifecycle",
        3,
        "release lifecycle issue",
    ));
    tracker.set_issue_body(
        "you/lifecycle#1",
        "# Active work\n\nKeep the complete Issue beside the Terminal while waiting for approval.",
    );
    let sessions = host_kernel::MemorySessionFactory::new();
    let agent = Arc::new(host_kernel::MemoryAgent::installed_grok());
    let mut host = HostKernel::boot_with_ports(
        boot_req(tmp.path()),
        host_kernel::KernelPorts {
            tracker,
            agents: vec![Arc::clone(&agent) as _],
            launch_env: Arc::new(host_kernel::MemoryLaunchEnv::with_path("/mem/bin")) as _,
            sessions: Arc::clone(&sessions) as _,
        },
    )
    .unwrap();
    pin_board_test_time(&mut host);
    let project_id = register(&mut host, "garden", &dir, "you/lifecycle");

    start_bound_grok(&mut host, &project_id, "you/lifecycle#1");
    sessions.last_session().unwrap().set_waiting(true);
    sessions
        .last_session()
        .unwrap()
        .push_output(b"waiting for approval\n");

    let isolated_dir = make_dir(tmp.path(), "isolated/continued");
    agent.set_isolation_tree(Some(isolated_dir.clone()));
    host.handle(serde_json::json!({
        "op": "prepareRunLaunch",
        "projectId": project_id,
        "issueId": "you/lifecycle#2",
        "agentId": "grok-build",
    }))
    .unwrap();
    host.handle(serde_json::json!({
        "op": "startUnboundRun",
        "projectId": project_id,
        "issueId": "you/lifecycle#2",
        "agentId": "grok-build",
        "values": {
            "model": "grok-4.6",
            "effort": "high",
            "permission-mode": "normal",
            "always-approve": "false",
            "sandbox": "off",
            "initial-instruction": "continue lifecycle issue",
            "additional-args": "",
            "isolation": "true"
        },
        "openingText": "continue lifecycle issue",
    }))
    .unwrap();
    sessions.last_session().unwrap().finish(2);
    host.handle(serde_json::json!({ "op": "snapshot" }))
        .unwrap();
    std::fs::remove_dir_all(&isolated_dir).unwrap();
    agent.set_isolation_tree(None);

    start_bound_grok(&mut host, &project_id, "you/lifecycle#3");
    sessions.last_session().unwrap().finish(2);
    host.handle(serde_json::json!({ "op": "snapshot" }))
        .unwrap();
    std::fs::write(dir.join("notes.txt"), "baseline\nchanged during the Run\n").unwrap();
    host.handle(serde_json::json!({
        "op": "focusIssue",
        "issueId": "you/lifecycle#1",
    }))
    .unwrap();
    run_browser_e2e(host, "run-lifecycle.mjs", &[]);
}

#[test]
fn browser_recovers_when_a_bound_pty_disconnects_mid_journey() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/disconnect");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open(
        "you/disconnect",
        1,
        "PTY disconnect issue",
    ));
    let sessions = MemorySessionFactory::new();
    let mut host = HostKernel::boot_with_ports(
        boot_req(tmp.path()),
        KernelPorts {
            tracker,
            agents: vec![Arc::new(MemoryAgent::installed_grok()) as _],
            launch_env: Arc::new(MemoryLaunchEnv::with_path("/mem/bin")) as _,
            sessions: Arc::clone(&sessions) as _,
        },
    )
    .unwrap();
    pin_board_test_time(&mut host);
    register(&mut host, "garden", &dir, "you/disconnect");

    run_browser_e2e_with_pty_disconnect(host, sessions, "run-pty-disconnect.mjs");
}

#[test]
fn browser_explains_an_occupied_loopback_port_without_disabling_the_client() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, "garden", &dir, "you/garden");
    let kernel = Arc::new(Mutex::new(host));
    let occupier = TcpListener::bind((std::net::Ipv4Addr::UNSPECIFIED, 0)).unwrap();
    let occupied_port = occupier.local_addr().unwrap().port();
    let dist = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/desktop/dist")
        .canonicalize()
        .expect("built desktop client");
    let client = LoopbackServer::attach_without_host_tick(
        kernel,
        occupied_port,
        LoopbackAssets::Directory(dist),
        |_| {},
    )
    .unwrap();
    run_browser_script(
        "loopback-occupied.mjs",
        client.protocol_url(),
        &[("OCCUPIED_PORT", &occupied_port.to_string())],
    );
    drop(occupier);
}
