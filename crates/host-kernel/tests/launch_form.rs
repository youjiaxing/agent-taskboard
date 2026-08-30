use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use host_kernel::{
    intent_prefix, AgentConfigDiscovery, AgentField, AgentFieldKind, AgentFieldOptionFilter,
    AgentPort, AgentSession, BootRequest, HostKernel, KernelPorts, Language, LaunchEnvironment,
    MemoryAgent, MemoryLaunchEnv, MemorySessionFactory, MemoryTracker, PrefillSource, ProbeResult,
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
    harness_with_ports(
        root,
        agents
            .into_iter()
            .map(|agent| agent as Arc<dyn AgentPort>)
            .collect(),
    )
}

fn harness_with_ports(root: &Path, agents: Vec<Arc<dyn AgentPort>>) -> Harness {
    let launch_env = Arc::new(MemoryLaunchEnv::with_path("/mem/bin"));
    let sessions = MemorySessionFactory::new();
    let host = HostKernel::boot_with_ports(
        boot_req(root),
        KernelPorts {
            tracker: Arc::new(MemoryTracker::new()),
            agents,
            launch_env: launch_env as _,
            sessions: Arc::clone(&sessions) as _,
        },
    )
    .unwrap();
    Harness { host, sessions }
}

#[derive(Debug)]
struct MissingOnDiscoveryAgent {
    probes: Mutex<u32>,
}

impl MissingOnDiscoveryAgent {
    fn new() -> Self {
        Self {
            probes: Mutex::new(0),
        }
    }
}

impl AgentPort for MissingOnDiscoveryAgent {
    fn id(&self) -> &str {
        "grok-build"
    }

    fn name(&self) -> &str {
        "Grok Build"
    }

    fn bin(&self) -> &str {
        "grok"
    }

    fn known_install_locations(&self) -> Vec<PathBuf> {
        vec![PathBuf::from("/known/grok/bin")]
    }

    fn probe(&self, env: &LaunchEnvironment) -> ProbeResult {
        let mut probes = self.probes.lock().unwrap();
        *probes += 1;
        if *probes == 1 {
            ProbeResult::Found {
                executable: PathBuf::from("/mem/grok"),
            }
        } else {
            ProbeResult::Missing {
                command: "grok".into(),
                searched_path: env.path_raw(),
                known_locations: self.known_install_locations(),
            }
        }
    }

    fn assemble_argv(&self, executable: &Path) -> Vec<String> {
        vec![executable.display().to_string()]
    }

    fn config_fields(&self) -> Vec<AgentField> {
        vec![field("model", false)]
    }

    fn seed_config(&self) -> BTreeMap<String, String> {
        BTreeMap::from([("model".into(), "manual-model".into())])
    }
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
        option_filter: None,
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
        "permission-mode": "default",
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
    assert!(form.selected_agent_id.is_empty());
    assert!(form.fields.is_empty());
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
        "grok --model grok-4.6 --effort high --permission-mode default --sandbox off"
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
            "--permission-mode",
            "default",
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
            "agentId": "grok-build",
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
fn draft_update_keeps_preview_warnings_and_final_argv_in_sync() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let grok = Arc::new(MemoryAgent::installed_grok());
    let mut model = field("model", false);
    model.kind = AgentFieldKind::Select;
    model.options = vec!["fast".into(), "deep".into()];
    model.required = true;
    let mut effort = field("effort", false);
    effort.kind = AgentFieldKind::Select;
    effort.options = vec!["low".into(), "high".into()];
    effort.required = true;
    effort.option_filter = Some(AgentFieldOptionFilter {
        field_id: "model".into(),
        options_by_value: BTreeMap::from([
            ("fast".into(), vec!["low".into()]),
            ("deep".into(), vec!["high".into()]),
        ]),
    });
    grok.set_discovery_result(AgentConfigDiscovery {
        fields: vec![model, effort, field("initial-instruction", false)],
        seed: BTreeMap::from([
            ("model".into(), "fast".into()),
            ("effort".into(), "low".into()),
            ("initial-instruction".into(), String::new()),
        ]),
    });
    let mut h = harness_with(tmp.path(), vec![grok]);
    let project_id = register(&mut h.host, &dir, "garden", "you/garden");
    h.host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
            "agentId": "grok-build",
        }))
        .unwrap();

    let values = serde_json::json!({
        "model": "deep",
        "effort": "low",
        "initial-instruction": "",
        "isolation": "false"
    });
    let form = h
        .host
        .handle(serde_json::json!({
            "op": "updateRunLaunch",
            "projectId": project_id,
            "agentId": "grok-build",
            "values": values,
            "openingText": "verify sync",
        }))
        .unwrap()
        .snapshot
        .launch_form
        .unwrap();
    assert_eq!(form.command_preview, "grok --model deep --effort low");
    assert!(form.warnings.iter().any(|warning| warning.contains("low")));

    h.host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
            "agentId": "grok-build",
            "values": values,
            "openingText": "verify sync",
        }))
        .unwrap();
    let spawn = h.sessions.last_spawn().unwrap();
    assert_eq!(
        spawn.argv,
        vec!["/mem/grok", "--model", "deep", "--effort", "low"]
    );
}

#[test]
fn first_open_keeps_builtin_order_without_selecting_an_agent() {
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
    assert!(form.selected_agent_id.is_empty());
    assert!(!form.skip_agent_picker);
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
fn start_without_form_does_not_silently_choose_the_first_installed_agent() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let grok = MemoryAgent::missing_grok();
    let codex = MemoryAgent::installed(CODEX_ID, CODEX_NAME, CODEX_BIN);
    let mut h = harness_with(tmp.path(), vec![Arc::new(grok), Arc::new(codex)]);
    let project_id = register(&mut h.host, &dir, "garden", "you/garden");
    let error = h
        .host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
        }))
        .unwrap_err()
        .to_string();
    assert!(error.contains("choose an Agent"), "{error}");
    assert_eq!(h.sessions.spawn_count(), 0);
}

#[test]
fn missing_last_successful_agent_returns_to_an_unselected_picker() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let grok = Arc::new(MemoryAgent::installed_grok());
    let codex = Arc::new(memory_codex());
    let mut h = harness_with(tmp.path(), vec![Arc::clone(&grok), Arc::clone(&codex)]);
    let project_id = register(&mut h.host, &dir, "garden", "you/garden");

    h.host
        .handle(serde_json::json!({
            "op": "prepareRunLaunch",
            "projectId": project_id,
            "agentId": "codex",
        }))
        .unwrap();
    h.host
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
                "profile": "",
                "additional-args": ""
            },
            "openingText": "remember Codex",
        }))
        .unwrap();
    codex.set_installed(false);

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

    assert!(!form.skip_agent_picker);
    assert!(form.selected_agent_id.is_empty());
    assert!(form.fields.is_empty());
    assert!(form
        .agents
        .iter()
        .any(|agent| agent.id == "grok-build" && agent.installed));
    assert!(form
        .agents
        .iter()
        .any(|agent| agent.id == "codex" && !agent.installed));
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

#[test]
fn english_intent_prefix_uses_client_language() {
    assert_eq!(
        intent_prefix(Some(RunIntent::Answer), Language::En),
        "Only answer the questions below. Do not modify any files."
    );
    assert_eq!(intent_prefix(None, Language::ZhCn), "");
}
