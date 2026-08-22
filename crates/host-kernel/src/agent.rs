use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::{Language, LaunchEnvironment};

pub const GROK_BUILD_ID: &str = "grok-build";
pub const GROK_BUILD_NAME: &str = "Grok Build";
pub const GROK_BIN: &str = "grok";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentFieldKind {
    Text,
    Select,
    Boolean,
    Multiline,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentField {
    pub id: String,
    pub label: String,
    pub kind: AgentFieldKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    pub required: bool,
    pub folded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RunLaunchConfig {
    pub agent_id: String,
    #[serde(default)]
    pub values: BTreeMap<String, String>,
    #[serde(default)]
    pub opening_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSummary {
    pub id: String,
    pub name: String,
    pub installed: bool,
    pub fields: Vec<AgentField>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PrefillSource {
    CurrentProject,
    OtherProject,
    CliSeed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunLaunchForm {
    pub project_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue_id: Option<String>,
    pub agents: Vec<AgentSummary>,
    pub selected_agent_id: String,
    pub skip_agent_picker: bool,
    pub fields: Vec<AgentField>,
    pub values: BTreeMap<String, String>,
    pub prefill_source: PrefillSource,
    pub working_directory: String,
    pub isolation_supported: bool,
    pub isolation_reason: String,
    #[serde(default)]
    pub opening_text: String,
    #[serde(default)]
    pub command_preview: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub intents: Vec<IntentOption>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunIntent {
    Modify,
    Continue,
    Answer,
    Review,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntentOption {
    pub id: String,
    pub label: String,
    pub prefix: String,
}

pub fn intent_prefix(intent: Option<RunIntent>, language: Language) -> String {
    let Some(intent) = intent else {
        return String::new();
    };
    match (language, intent) {
        (Language::ZhCn, RunIntent::Modify) => "根据下面的说明修改实现。".into(),
        (Language::ZhCn, RunIntent::Continue) => "继续当前工作。下面的说明是补充要求。".into(),
        (Language::ZhCn, RunIntent::Answer) => "只回答下面的问题，不要修改文件。".into(),
        (Language::ZhCn, RunIntent::Review) => "复查当前实现并报告你的发现，不要修改文件。".into(),
        (Language::En, RunIntent::Modify) => {
            "Modify the implementation according to the instructions below.".into()
        }
        (Language::En, RunIntent::Continue) => {
            "Continue the current work. Treat the instructions below as additional requirements."
                .into()
        }
        (Language::En, RunIntent::Answer) => {
            "Only answer the questions below. Do not modify any files.".into()
        }
        (Language::En, RunIntent::Review) => {
            "Review the current implementation and report your findings. Do not modify any files."
                .into()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeResult {
    Found {
        executable: PathBuf,
    },
    Missing {
        command: String,
        searched_path: String,
        known_locations: Vec<PathBuf>,
    },
}

pub trait AgentPort: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn bin(&self) -> &str;
    fn known_install_locations(&self) -> Vec<PathBuf>;
    fn probe(&self, env: &LaunchEnvironment) -> ProbeResult;
    fn assemble_argv(&self, executable: &Path) -> Vec<String>;
    fn config_fields(&self) -> Vec<AgentField> {
        Vec::new()
    }
    fn seed_config(&self) -> BTreeMap<String, String> {
        BTreeMap::new()
    }
    fn assemble_argv_for(
        &self,
        executable: &Path,
        _values: &BTreeMap<String, String>,
    ) -> Vec<String> {
        self.assemble_argv(executable)
    }
    fn recent_action(&self) -> Option<String> {
        None
    }
}

#[derive(Debug, Clone)]
pub struct GrokAdapter;

impl GrokAdapter {
    pub fn known_location() -> Option<PathBuf> {
        home_dir().map(|home| home.join(".grok").join("bin"))
    }
}

impl AgentPort for GrokAdapter {
    fn id(&self) -> &str {
        GROK_BUILD_ID
    }

    fn name(&self) -> &str {
        GROK_BUILD_NAME
    }

    fn bin(&self) -> &str {
        GROK_BIN
    }

    fn known_install_locations(&self) -> Vec<PathBuf> {
        Self::known_location().into_iter().collect()
    }

    fn probe(&self, env: &LaunchEnvironment) -> ProbeResult {
        probe_binary(self.bin(), env, &self.known_install_locations())
    }

    fn assemble_argv(&self, executable: &Path) -> Vec<String> {
        vec![executable.to_string_lossy().into_owned()]
    }

    fn config_fields(&self) -> Vec<AgentField> {
        grok_fields()
    }

    fn seed_config(&self) -> BTreeMap<String, String> {
        grok_seed()
    }

    fn assemble_argv_for(
        &self,
        executable: &Path,
        values: &BTreeMap<String, String>,
    ) -> Vec<String> {
        grok_argv(executable, values)
    }
}

#[derive(Debug)]
pub struct MemoryAgent {
    id: String,
    name: String,
    bin: String,
    executable: PathBuf,
    known_locations: Vec<PathBuf>,
    installed: Mutex<bool>,
    recent_action: Mutex<Option<String>>,
}

impl MemoryAgent {
    pub fn installed(
        id: impl Into<String>,
        name: impl Into<String>,
        bin: impl Into<String>,
    ) -> Self {
        let bin = bin.into();
        Self {
            id: id.into(),
            name: name.into(),
            bin: bin.clone(),
            executable: PathBuf::from(format!("/mem/{bin}")),
            known_locations: vec![PathBuf::from("/mem/.grok/bin")],
            installed: Mutex::new(true),
            recent_action: Mutex::new(None),
        }
    }

    pub fn installed_grok() -> Self {
        Self::installed(GROK_BUILD_ID, GROK_BUILD_NAME, GROK_BIN)
    }

    pub fn missing_grok() -> Self {
        let agent = Self::installed_grok();
        agent.set_installed(false);
        agent
    }

    pub fn set_installed(&self, installed: bool) {
        *self.installed.lock().expect("memory agent") = installed;
    }

    pub fn set_recent_action(&self, action: Option<String>) {
        *self.recent_action.lock().expect("memory agent") = action;
    }
}

impl AgentPort for MemoryAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn bin(&self) -> &str {
        &self.bin
    }

    fn known_install_locations(&self) -> Vec<PathBuf> {
        self.known_locations.clone()
    }

    fn probe(&self, env: &LaunchEnvironment) -> ProbeResult {
        if *self.installed.lock().expect("memory agent") {
            ProbeResult::Found {
                executable: self.executable.clone(),
            }
        } else {
            ProbeResult::Missing {
                command: self.bin.clone(),
                searched_path: env.path_raw(),
                known_locations: self.known_locations.clone(),
            }
        }
    }

    fn assemble_argv(&self, executable: &Path) -> Vec<String> {
        vec![executable.to_string_lossy().into_owned()]
    }

    fn config_fields(&self) -> Vec<AgentField> {
        grok_fields()
    }

    fn seed_config(&self) -> BTreeMap<String, String> {
        grok_seed()
    }

    fn assemble_argv_for(
        &self,
        executable: &Path,
        values: &BTreeMap<String, String>,
    ) -> Vec<String> {
        grok_argv(executable, values)
    }

    fn recent_action(&self) -> Option<String> {
        self.recent_action.lock().expect("memory agent").clone()
    }
}

fn grok_fields() -> Vec<AgentField> {
    vec![
        AgentField {
            id: "model".into(),
            label: "model".into(),
            kind: AgentFieldKind::Text,
            options: Vec::new(),
            required: true,
            folded: false,
        },
        AgentField {
            id: "effort".into(),
            label: "effort".into(),
            kind: AgentFieldKind::Select,
            options: vec!["low".into(), "medium".into(), "high".into()],
            required: true,
            folded: false,
        },
        AgentField {
            id: "permission-mode".into(),
            label: "权限模式".into(),
            kind: AgentFieldKind::Select,
            options: vec![
                "normal".into(),
                "plan".into(),
                "auto".into(),
                "always-approve".into(),
            ],
            required: true,
            folded: false,
        },
        AgentField {
            id: "always-approve".into(),
            label: "alwaysApprove".into(),
            kind: AgentFieldKind::Boolean,
            options: Vec::new(),
            required: false,
            folded: false,
        },
        AgentField {
            id: "sandbox".into(),
            label: "sandbox".into(),
            kind: AgentFieldKind::Select,
            options: vec![
                "off".into(),
                "workspace".into(),
                "read-only".into(),
                "strict".into(),
            ],
            required: true,
            folded: false,
        },
        AgentField {
            id: "initial-instruction".into(),
            label: "初始指令".into(),
            kind: AgentFieldKind::Multiline,
            options: Vec::new(),
            required: false,
            folded: false,
        },
        AgentField {
            id: "additional-args".into(),
            label: "附加参数".into(),
            kind: AgentFieldKind::Text,
            options: Vec::new(),
            required: false,
            folded: true,
        },
    ]
}

fn grok_seed() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("model".into(), "grok-4.6".into()),
        ("effort".into(), "high".into()),
        ("permission-mode".into(), "normal".into()),
        ("always-approve".into(), "false".into()),
        ("sandbox".into(), "off".into()),
        ("initial-instruction".into(), String::new()),
        ("additional-args".into(), String::new()),
    ])
}

fn grok_argv(executable: &Path, values: &BTreeMap<String, String>) -> Vec<String> {
    let mut argv = vec![executable.to_string_lossy().into_owned()];
    append_flag(&mut argv, "--model", values.get("model"));
    append_flag(&mut argv, "--effort", values.get("effort"));
    append_flag(&mut argv, "--sandbox", values.get("sandbox"));
    if values
        .get("always-approve")
        .is_some_and(|value| value == "true")
        || values
            .get("permission-mode")
            .is_some_and(|value| value == "always-approve")
    {
        argv.push("--always-approve".into());
    }
    if let Some(args) = values.get("additional-args") {
        argv.extend(args.split_whitespace().map(ToOwned::to_owned));
    }
    argv
}

fn append_flag(argv: &mut Vec<String>, flag: &str, value: Option<&String>) {
    if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
        argv.push(flag.into());
        argv.push(value.clone());
    }
}

pub fn probe_binary(bin: &str, env: &LaunchEnvironment, known: &[PathBuf]) -> ProbeResult {
    let mut dirs = known.to_vec();
    dirs.extend(env.path_dirs());
    for dir in &dirs {
        if let Some(found) = executable_in(dir, bin) {
            return ProbeResult::Found { executable: found };
        }
    }
    ProbeResult::Missing {
        command: bin.to_string(),
        searched_path: env.path_raw(),
        known_locations: known.to_vec(),
    }
}

pub fn format_not_found(
    language: Language,
    command: &str,
    searched_path: &str,
    known_locations: &[PathBuf],
) -> String {
    let known = if known_locations.is_empty() {
        "—".to_string()
    } else {
        known_locations
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(path_sep())
    };
    let searched = if searched_path.is_empty() {
        "—"
    } else {
        searched_path
    };
    match language {
        Language::ZhCn => {
            format!("找不到 {command}。\n已搜 PATH：{searched}\n已知安装位置：{known}")
        }
        Language::En => format!(
            "Could not find {command}.\nSearched PATH: {searched}\nKnown install locations: {known}"
        ),
    }
}

pub fn prepare_launch_env(
    mut env: LaunchEnvironment,
    host_path_prefix: &[PathBuf],
    known_locations: &[PathBuf],
) -> LaunchEnvironment {
    let mut prepend = Vec::new();
    prepend.extend(host_path_prefix.iter().cloned());
    prepend.extend(known_locations.iter().cloned());
    env.prepend_path_dirs(&prepend);
    env.pin_term();
    env
}

fn executable_in(dir: &Path, bin: &str) -> Option<PathBuf> {
    let candidate = dir.join(bin);
    if is_executable(&candidate) {
        return Some(candidate);
    }
    #[cfg(windows)]
    {
        let exe = dir.join(format!("{bin}.exe"));
        if is_executable(&exe) {
            return Some(exe);
        }
    }
    None
}

fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path)
            .map(|meta| meta.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn path_sep() -> &'static str {
    if cfg!(windows) {
        ";"
    } else {
        ":"
    }
}

pub fn path_from_dirs(dirs: &[PathBuf], existing: &str) -> String {
    let mut parts = dirs
        .iter()
        .map(|dir| dir.to_string_lossy().into_owned())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if !existing.is_empty() {
        parts.push(existing.to_string());
    }
    parts.join(path_sep())
}

pub fn split_path(raw: &str) -> Vec<PathBuf> {
    raw.split(if cfg!(windows) { ';' } else { ':' })
        .filter(|part| !part.is_empty())
        .map(PathBuf::from)
        .collect()
}

pub fn env_map_with_path(path: &str) -> BTreeMap<String, String> {
    let mut vars = BTreeMap::new();
    vars.insert("PATH".into(), path.to_string());
    vars
}
