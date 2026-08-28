use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use host_kernel::{
    intent_prefix, AgentField, AgentFieldKind, AgentSession, BootRequest, HostKernel, KernelPorts,
    Language, MemoryAgent, MemoryLaunchEnv, MemorySessionFactory, MemoryTracker, PrefillSource,
    RunIntent, RunStatus, SystemAppearance, ANTIGRAVITY_BIN, ANTIGRAVITY_ID, ANTIGRAVITY_NAME,
    CLAUDE_BIN, CLAUDE_CODE_ID, CLAUDE_CODE_NAME, CODEX_BIN, CODEX_ID, CODEX_NAME,
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

struct Harness {
    host: HostKernel,
    sessions: Arc<MemorySessionFactory>,
}

fn harness_with(root: &Path, agents: Vec<Arc<MemoryAgent>>) -> Harness {
    let launch_env = Arc::new(MemoryLaunchEnv::with_path("/mem/bin"));
    let sessions = MemorySessionFactory::new();
    let host = HostKernel::boot_with_ports(
        boot_req(root),
        KernelPorts {
            tracker: Arc::new(MemoryTracker::new()),
            agents: agents.into_iter().map(|agent| agent as _).collect(),
            launch_env: launch_env as _,
            sessions: Arc::clone(&sessions) as _,
        },
    )
    .unwrap();
    Harness { host, sessions }
}

fn harness(root: &Path) -> Harness {
    harness_with(root, vec![Arc::new(MemoryAgent::installed_grok())])
}

fn register(host: &mut HostKernel, dir: &Path, name: &str, repo: &str) -> String {
    host.handle(serde_json::json!({
        "op": "registerProject",
        "name": name,
        "localPath": dir,
        "repository": repo,
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

fn field(id: &str, folded: bool) -> AgentField {
    AgentField {
        id: id.into(),
        label: id.into(),
        kind: AgentFieldKind::Text,
        options: Vec::new(),
        required: false,
        folded,
    }
}

fn memory_codex() -> MemoryAgent {
    MemoryAgent::installed(CODEX_ID, CODEX_NAME, CODEX_BIN).with_fields(
        vec![
            field("model", false),
            field("effort", false),
            field("approval", false),
            field("sandbox", false),
            field("initial-instruction", false),
            field("profile", true),
            field("additional-args", true),
        ],
        [
            ("model", "gpt-5.1"),
            ("effort", "medium"),
            ("approval", "on-request"),
            ("sandbox", "workspace-write"),
            ("initial-instruction", ""),
            ("profile", ""),
            ("additional-args", ""),
        ]
        .into_iter()
        .map(|(id, value)| (id.to_string(), value.to_string()))
        .collect(),
    )
}

fn grok_values() -> serde_json::Value {
    serde_json::json!({
        "model": "grok-4.6",
        "effort": "high",
        "permission-mode": "normal",
        "always-approve": "false",
        "sandbox": "off",
        "initial-instruction": "",
        "additional-args": ""
    })
}

#[test]
fn prepare_form_uses_cli_seed_concrete_values() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir, "garden", "you/garden");

    let out = h
        .host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
        }))
        .unwrap();
    let form = out.snapshot.launch_form.as_ref().unwrap();
    assert_eq!(form.prefill_source, PrefillSource::CliSeed);
    assert!(!form.skip_agent_picker);
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
    assert!(form.skip_agent_picker);
    assert_eq!(
        form.values.get("model").map(String::as_str),
        Some("grok-4.6")
    );
    assert_eq!(form.values.get("effort").map(String::as_str), Some("high"));
    assert_eq!(form.values.get("sandbox").map(String::as_str), Some("off"));
    assert_eq!(
        form.values.get("initial-instruction").map(String::as_str),
        Some("")
    );
    let dump = serde_json::to_string(&form).unwrap();
    assert!(!dump.contains("使用默认"));
    assert!(!dump.to_ascii_lowercase().contains("use default"));
    assert!(form.fields.iter().any(|field| field.id == "model"));
    assert!(form.fields.iter().any(|field| field.id == "effort"));
    assert!(form
        .fields
        .iter()
        .any(|field| field.id == "permission-mode"));
    assert!(form.fields.iter().any(|field| field.id == "sandbox"));
    assert!(form.isolation_reason.contains("隔离"));
    assert!(!form.isolation_supported);
    assert_eq!(
        form.command_preview,
        "grok --model grok-4.6 --effort high --sandbox off"
    );
    assert!(out.snapshot.show_command_preview);
    h.host
        .handle(serde_json::json!({
            "op": "setShowCommandPreview",
            "show": false,
        }))
        .unwrap();
    assert!(!h.host.snapshot().show_command_preview);
    assert_eq!(out.snapshot.copy.opening_placeholder, "要 Agent 做什么");
}

#[test]
fn start_uses_form_values_not_memory() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir, "garden", "you/garden");
    let mut first = grok_values();
    first["model"] = serde_json::json!("grok-4");
    h.host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
        }))
        .unwrap();
    h.host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
            "agentId": "grok-build",
            "values": first,
            "openingText": "先用旧模型",
        }))
        .unwrap();

    let mut second = grok_values();
    second["model"] = serde_json::json!("grok-4.6");
    second["effort"] = serde_json::json!("low");
    h.host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
        }))
        .unwrap();
    h.host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
            "agentId": "grok-build",
            "values": second,
            "openingText": "改用表单上的值",
        }))
        .unwrap();
    let spawn = h.sessions.last_spawn().unwrap();
    assert_eq!(
        spawn.argv,
        vec![
            "/mem/grok",
            "--model",
            "grok-4.6",
            "--effort",
            "low",
            "--sandbox",
            "off"
        ]
    );
}

#[test]
fn memory_prefers_current_project_then_other_project() {
    let tmp = tempfile::tempdir().unwrap();
    let garden = make_dir(tmp.path(), "work/garden");
    let pond = make_dir(tmp.path(), "work/pond");
    let mut h = harness(tmp.path());
    let garden_id = register(&mut h.host, &garden, "garden", "you/garden");
    let pond_id = register(&mut h.host, &pond, "pond", "you/pond");

    let mut values = grok_values();
    values["model"] = serde_json::json!("from-pond");
    values["effort"] = serde_json::json!("medium");
    h.host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": pond_id,
        }))
        .unwrap();
    h.host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": pond_id,
            "agentId": "grok-build",
            "values": values,
            "openingText": "pond work",
        }))
        .unwrap();

    let form = h
        .host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": garden_id,
        }))
        .unwrap()
        .snapshot
        .launch_form
        .unwrap();
    assert_eq!(form.prefill_source, PrefillSource::OtherProject);
    assert_eq!(
        form.values.get("model").map(String::as_str),
        Some("from-pond")
    );

    let mut garden_values = grok_values();
    garden_values["model"] = serde_json::json!("from-garden");
    h.host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": garden_id,
            "agentId": "grok-build",
            "values": garden_values,
            "openingText": "garden work",
        }))
        .unwrap();
    let form = h
        .host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": garden_id,
        }))
        .unwrap()
        .snapshot
        .launch_form
        .unwrap();
    assert_eq!(form.prefill_source, PrefillSource::CurrentProject);
    assert_eq!(
        form.values.get("model").map(String::as_str),
        Some("from-garden")
    );
    assert!(form.skip_agent_picker);
}

#[test]
fn isolation_intent_and_instruction_are_not_remembered() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir, "garden", "you/garden");
    let mut values = grok_values();
    values["initial-instruction"] = serde_json::json!("不要记住这句话");
    values["isolation"] = serde_json::json!("true");
    h.host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
        }))
        .unwrap();
    h.host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
            "agentId": "grok-build",
            "values": values,
            "openingText": "根据下面的说明修改实现。\n不要记住这句话",
        }))
        .unwrap();

    let stored = std::fs::read_to_string(tmp.path().join("host/settings.json")).unwrap();
    assert!(!stored.contains("不要记住这句话"));
    assert!(!stored.contains("isolation"));
    assert!(!stored.contains("openingText"));
    assert!(!stored.contains("modify"));

    let form = h
        .host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
        }))
        .unwrap()
        .snapshot
        .launch_form
        .unwrap();
    assert_eq!(
        form.values.get("initial-instruction").map(String::as_str),
        Some("")
    );
    assert!(form.opening_text.is_empty());
}

#[test]
fn intent_only_changes_opening_text() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir, "garden", "you/garden");
    let opening = format!(
        "{}\n修一下测试",
        intent_prefix(Some(RunIntent::Modify), Language::ZhCn)
    );
    h.host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
        }))
        .unwrap();
    h.host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
            "agentId": "grok-build",
            "values": grok_values(),
            "openingText": opening,
        }))
        .unwrap();
    let spawn = h.sessions.last_spawn().unwrap();
    assert!(!spawn.argv.iter().any(|arg| arg.contains("modify")));
    let chunk = h
        .sessions
        .last_session()
        .unwrap()
        .read_after(0, Duration::from_millis(10));
    assert_eq!(
        String::from_utf8_lossy(&chunk.data),
        "根据下面的说明修改实现。\n修一下测试\n"
    );
    assert!(h.host.snapshot().launch_form.is_none());
}

#[test]
fn required_instruction_keeps_form_and_creates_no_run() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir, "garden", "you/garden");
    h.host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
        }))
        .unwrap();
    let out = h
        .host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
            "agentId": "grok-build",
            "values": grok_values(),
            "openingText": "  ",
        }))
        .unwrap();
    let form = out.snapshot.launch_form.as_ref().unwrap();
    assert_eq!(form.error.as_deref(), Some("请填写要 Agent 做什么。"));
    assert!(out.snapshot.runs.is_empty());
    assert_eq!(h.sessions.spawn_count(), 0);
}

#[test]
fn unknown_enum_warns_but_does_not_block() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir, "garden", "you/garden");
    let mut values = grok_values();
    values["effort"] = serde_json::json!("xhigh");
    h.host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
        }))
        .unwrap();
    let out = h
        .host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
            "agentId": "grok-build",
            "values": values,
            "openingText": "试未知档位",
        }))
        .unwrap();
    assert!(out.snapshot.launch_form.is_none());
    assert_eq!(out.snapshot.runs[0].status, RunStatus::Running);
    let spawn = h.sessions.last_spawn().unwrap();
    assert!(spawn
        .argv
        .windows(2)
        .any(|pair| pair == ["--effort", "xhigh"]));
}

#[test]
fn spawn_failure_keeps_form_and_does_not_overwrite_memory() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir, "garden", "you/garden");
    let mut first = grok_values();
    first["model"] = serde_json::json!("kept-model");
    h.host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
        }))
        .unwrap();
    h.host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
            "agentId": "grok-build",
            "values": first,
            "openingText": "先成功一次",
        }))
        .unwrap();

    let mut failed = grok_values();
    failed["model"] = serde_json::json!("should-not-stick");
    h.host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
        }))
        .unwrap();
    h.sessions.fail_next("could not spawn grok");
    let out = h
        .host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
            "agentId": "grok-build",
            "values": failed,
            "openingText": "这次会失败",
        }))
        .unwrap();
    let form = out.snapshot.launch_form.as_ref().unwrap();
    assert_eq!(form.error.as_deref(), Some("could not spawn grok"));
    assert_eq!(
        form.values.get("model").map(String::as_str),
        Some("should-not-stick")
    );
    assert_eq!(form.opening_text, "这次会失败");
    assert_eq!(out.snapshot.runs.last().unwrap().status, RunStatus::Ended);
    assert_eq!(h.sessions.spawn_count(), 2);

    let form = h
        .host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
        }))
        .unwrap()
        .snapshot
        .launch_form
        .unwrap();
    assert_eq!(
        form.values.get("model").map(String::as_str),
        Some("kept-model")
    );
}

#[test]
fn last_agent_skips_picker_until_switch() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let grok = Arc::new(MemoryAgent::installed_grok());
    let other = Arc::new(MemoryAgent::installed("other-agent", "Other", "other"));
    let mut h = harness_with(tmp.path(), vec![grok, other]);
    let project_id = register(&mut h.host, &dir, "garden", "you/garden");
    h.host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
        }))
        .unwrap();
    h.host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
            "agentId": "grok-build",
            "values": grok_values(),
            "openingText": "记住这家",
        }))
        .unwrap();
    let form = h
        .host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
        }))
        .unwrap()
        .snapshot
        .launch_form
        .unwrap();
    assert!(form.skip_agent_picker);
    assert_eq!(form.selected_agent_id, "grok-build");

    let form = h
        .host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
            "pickAgent": true,
        }))
        .unwrap()
        .snapshot
        .launch_form
        .unwrap();
    assert!(!form.skip_agent_picker);
    assert_eq!(form.agents.len(), 2);
}

#[test]
fn switching_agent_replaces_fields_and_isolation_reason() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let grok = Arc::new(MemoryAgent::installed_grok());
    let codex = Arc::new(memory_codex());
    let mut h = harness_with(tmp.path(), vec![grok, codex]);
    let project_id = register(&mut h.host, &dir, "garden", "you/garden");

    let grok_form = h
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
    let grok_ids: Vec<_> = grok_form
        .fields
        .iter()
        .map(|field| field.id.as_str())
        .collect();
    assert!(grok_ids.contains(&"permission-mode"));
    assert!(!grok_ids.contains(&"approval"));
    assert!(grok_form.isolation_reason.contains("git"));
    assert!(!grok_form.isolation_supported);
    assert!(!grok_form.isolation_reason.contains("留给隔离票"));

    let codex_form = h
        .host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
            "agentId": "codex",
        }))
        .unwrap()
        .snapshot
        .launch_form
        .unwrap();
    let codex_ids: Vec<_> = codex_form
        .fields
        .iter()
        .map(|field| field.id.as_str())
        .collect();
    assert!(codex_ids.contains(&"approval"));
    assert!(codex_ids.contains(&"profile"));
    assert!(!codex_ids.contains(&"permission-mode"));
    assert_eq!(codex_form.selected_agent_id, "codex");
    assert!(!codex_form.isolation_supported);
    assert!(codex_form.isolation_reason.contains("没有原生隔离"));
    assert!(!codex_form.isolation_reason.contains("留给隔离票"));
}

#[test]
fn default_agent_is_first_installed_in_builtin_order() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let grok = MemoryAgent::missing_grok();
    let codex = MemoryAgent::installed(CODEX_ID, CODEX_NAME, CODEX_BIN);
    let claude = MemoryAgent::installed(CLAUDE_CODE_ID, CLAUDE_CODE_NAME, CLAUDE_BIN);
    let agy = MemoryAgent::installed(ANTIGRAVITY_ID, ANTIGRAVITY_NAME, ANTIGRAVITY_BIN);
    let mut h = harness_with(
        tmp.path(),
        vec![
            Arc::new(grok),
            Arc::new(codex),
            Arc::new(claude),
            Arc::new(agy),
        ],
    );
    let project_id = register(&mut h.host, &dir, "garden", "you/garden");
    let form = h
        .host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
        }))
        .unwrap()
        .snapshot
        .launch_form
        .unwrap();
    assert_eq!(form.selected_agent_id, CODEX_ID);
    assert_eq!(
        form.agents
            .iter()
            .map(|agent| agent.id.as_str())
            .collect::<Vec<_>>(),
        vec!["grok-build", CODEX_ID, CLAUDE_CODE_ID, ANTIGRAVITY_ID]
    );
    assert_eq!(form.agents[3].name, ANTIGRAVITY_NAME);
    assert!(!form.agents.iter().any(|agent| agent.id.contains("gemini")
        || agent.name.contains("Gemini")
        || agent.name.contains("gemini")));
}

#[test]
fn start_without_form_uses_first_installed_agent() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let grok = MemoryAgent::missing_grok();
    let codex = MemoryAgent::installed(CODEX_ID, CODEX_NAME, CODEX_BIN);
    let mut h = harness_with(tmp.path(), vec![Arc::new(grok), Arc::new(codex)]);
    let project_id = register(&mut h.host, &dir, "garden", "you/garden");
    let out = h
        .host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
        }))
        .unwrap();
    let run = out.snapshot.runs.last().unwrap();
    assert_eq!(run.agent_id, CODEX_ID);
    assert_eq!(run.agent_name, CODEX_NAME);
    assert_eq!(run.status, RunStatus::Running);
    assert_eq!(h.sessions.spawn_count(), 1);
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
    let prepared = h
        .host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
            "agentId": "codex",
        }))
        .unwrap();
    let unavailable = prepared
        .snapshot
        .launch_form
        .as_ref()
        .unwrap()
        .agents
        .iter()
        .find(|agent| agent.id == "codex")
        .unwrap()
        .unavailable_reason
        .as_deref()
        .unwrap();
    assert!(unavailable.contains("codex"), "{unavailable}");
    assert!(unavailable.contains("PATH"), "{unavailable}");
    assert!(unavailable.contains("安装位置"), "{unavailable}");
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

#[test]
fn english_intent_prefix_uses_client_language() {
    assert_eq!(
        intent_prefix(Some(RunIntent::Answer), Language::En),
        "Only answer the questions below. Do not modify any files."
    );
    assert_eq!(intent_prefix(None, Language::ZhCn), "");
}
