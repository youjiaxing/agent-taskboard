use super::*;

#[test]
fn discovered_config_is_cached_until_launch_environment_refresh() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let grok = Arc::new(MemoryAgent::installed_grok());
    let mut model = field("model", false);
    model.kind = AgentFieldKind::Select;
    model.options = vec!["live-a".into(), "live-b".into()];
    grok.set_discovery_result(AgentConfigDiscovery {
        fields: vec![model],
        seed: BTreeMap::from([("model".into(), "live-a".into())]),
    });
    let mut h = harness_with(tmp.path(), vec![Arc::clone(&grok)]);
    let project_id = register(&mut h.host, &dir, "garden", "you/garden");

    for _ in 0..2 {
        let form = h
            .host
            .handle(serde_json::json!({
                "op": "prepareRunLaunch",
                "projectId": project_id,
                "agentId": "grok-build",
            }))
            .unwrap()
            .snapshot
            .launch_form
            .unwrap();
        assert_eq!(form.fields[0].options, vec!["live-a", "live-b"]);
        assert_eq!(form.values["model"], "live-a");
    }
    assert_eq!(grok.discovery_count(), 1);

    h.host
        .handle(serde_json::json!({ "op": "refreshLaunchEnvironment" }))
        .unwrap();
    h.host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
            "agentId": "grok-build",
        }))
        .unwrap();
    assert_eq!(grok.discovery_count(), 2);
}

#[test]
fn discovery_failure_keeps_manual_fields_and_reports_a_readable_error() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let grok = Arc::new(MemoryAgent::installed_grok());
    grok.set_discovery_error("models endpoint unavailable");
    let mut h = harness_with(tmp.path(), vec![Arc::clone(&grok)]);
    let project_id = register(&mut h.host, &dir, "garden", "you/garden");

    let form = h
        .host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
            "agentId": "grok-build",
        }))
        .unwrap()
        .snapshot
        .launch_form
        .unwrap();

    assert!(form.fields.iter().any(|field| field.id == "model"));
    assert_eq!(form.values["model"], "grok-4.6");
    assert!(form.option_discovery_pending.is_none());
    assert!(form
        .option_discovery_error
        .as_deref()
        .is_some_and(|error| error.contains("models endpoint unavailable")));
    assert!(form
        .option_discovery_error
        .as_deref()
        .is_some_and(|error| error.contains("仍可手动输入")));

    let english = h
        .host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
            "agentId": "grok-build",
            "language": "en",
        }))
        .unwrap()
        .snapshot
        .launch_form
        .unwrap()
        .option_discovery_error
        .unwrap();
    assert!(english.contains("Could not read available CLI options"));
    assert!(english.contains("models endpoint unavailable"));
    assert!(english.contains("Manual input is still available"));
    assert_eq!(
        grok.discovery_count(),
        1,
        "cached failure should be localized on read"
    );
}

#[test]
fn missing_cli_discovery_reports_command_path_locations_and_remediation() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness_with_ports(tmp.path(), vec![Arc::new(MissingOnDiscoveryAgent::new())]);
    let project_id = register(&mut h.host, &dir, "garden", "you/garden");

    let error = h
        .host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
            "agentId": "grok-build",
        }))
        .unwrap()
        .snapshot
        .launch_form
        .unwrap()
        .option_discovery_error
        .unwrap();

    assert!(error.contains("grok"), "{error}");
    assert!(error.contains("/mem/bin"), "{error}");
    assert!(error.contains("/known/grok/bin"), "{error}");
    assert!(error.contains("PATH"), "{error}");
    assert!(error.contains("安装"), "{error}");
    assert!(error.contains("login"), "{error}");
}

#[test]
fn uninstalled_agent_cannot_start_from_form() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let grok = Arc::new(MemoryAgent::installed_grok());
    let missing = memory_codex();
    missing.set_installed(false);
    let mut h = harness_with(tmp.path(), vec![grok, Arc::new(missing)]);
    let project_id = register(&mut h.host, &dir, "garden", "you/garden");
    h.host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
            "agentId": "codex",
        }))
        .unwrap();
    let out = h
        .host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
            "agentId": "codex",
            "values": {
                "model": "gpt-5.1",
                "effort": "medium",
                "approval": "on-request",
                "sandbox": "workspace-write",
                "initial-instruction": "",
                "additional-args": ""
            },
            "openingText": "试未安装的 Codex",
        }))
        .unwrap();
    let form = out.snapshot.launch_form.as_ref().unwrap();
    let error = form.error.as_deref().unwrap();
    assert!(error.contains("codex"), "{error}");
    assert!(!error.to_ascii_lowercase().contains("login"), "{error}");
    assert!(!error.contains("登录"), "{error}");
    assert_eq!(out.snapshot.runs.last().unwrap().status, RunStatus::Ended);
    assert_eq!(h.sessions.spawn_count(), 0);
}
