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

#[path = "launch_form/discovery.rs"]
mod discovery;
#[path = "launch_form/form.rs"]
mod form;
