use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::usage::TelemetrySample;
use crate::{Language, LaunchEnvironment};

mod antigravity;
mod claude;
mod codex;
mod discovery;
mod grok;
mod hooks;

pub use antigravity::{AntigravityAdapter, ANTIGRAVITY_BIN, ANTIGRAVITY_ID, ANTIGRAVITY_NAME};
pub use claude::{ClaudeAdapter, CLAUDE_BIN, CLAUDE_CODE_ID, CLAUDE_CODE_NAME};
pub use codex::{CodexAdapter, CODEX_BIN, CODEX_ID, CODEX_NAME};
pub use grok::{GrokAdapter, GROK_BIN, GROK_BUILD_ID, GROK_BUILD_NAME};
pub use hooks::{CompletionHookPlan, CompletionSignals};

pub fn builtin_agents() -> Vec<Arc<dyn AgentPort>> {
    vec![
        Arc::new(GrokAdapter),
        Arc::new(CodexAdapter),
        Arc::new(ClaudeAdapter),
        Arc::new(AntigravityAdapter),
    ]
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub option_filter: Option<AgentFieldOptionFilter>,
    pub required: bool,
    pub folded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentFieldOptionFilter {
    pub field_id: String,
    pub options_by_value: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConfigDiscovery {
    pub fields: Vec<AgentField>,
    pub seed: BTreeMap<String, String>,
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
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub change_notes_text: String,
    #[serde(default)]
    pub command_preview: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub intents: Vec<IntentOption>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub option_discovery_error: Option<String>,
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
    fn discover_config(
        &self,
        _executable: &Path,
        _env: &LaunchEnvironment,
    ) -> Result<AgentConfigDiscovery, String> {
        Ok(AgentConfigDiscovery {
            fields: self.config_fields(),
            seed: self.seed_config(),
        })
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

    fn native_session_id(&self) -> Option<String> {
        None
    }

    fn assemble_argv_for_resume(
        &self,
        executable: &Path,
        values: &BTreeMap<String, String>,
        _session_id: &str,
    ) -> Vec<String> {
        self.assemble_argv_for(executable, values)
    }

    fn native_isolation(&self) -> bool {
        false
    }

    fn isolation_unavailable_reason(&self, language: Language) -> String {
        match language {
            Language::ZhCn => format!("{} 没有原生隔离执行目录。", self.name()),
            Language::En => format!("{} has no native isolated work directory.", self.name()),
        }
    }

    fn isolation_tree_after_launch(
        &self,
        _project_dir: &Path,
        _before: &[PathBuf],
    ) -> Option<PathBuf> {
        None
    }

    fn completion_hooks_supported(&self) -> bool {
        false
    }

    fn inject_supported(&self) -> bool {
        true
    }

    fn attach_completion_hooks(
        &self,
        _sink_dir: &Path,
        _project_dir: &Path,
    ) -> Result<CompletionHookPlan, String> {
        Err("completion hooks unsupported".into())
    }

    fn read_completion_signals(&self, sink_dir: &Path) -> CompletionSignals {
        hooks::read_signals(sink_dir)
    }

    fn drain_telemetry(&self) -> Vec<TelemetrySample> {
        Vec::new()
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
    fields: Vec<AgentField>,
    seed: BTreeMap<String, String>,
    native_isolation: bool,
    native_session_id: Mutex<Option<String>>,
    isolation_tree: Mutex<Option<PathBuf>>,
    hooks_supported: Mutex<bool>,
    attach_fail: Mutex<bool>,
    telemetry: Mutex<Vec<TelemetrySample>>,
    discovery_result: Mutex<Option<Result<AgentConfigDiscovery, String>>>,
    discovery_count: Mutex<u32>,
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
            fields: grok::grok_fields(),
            seed: grok::grok_seed(),
            native_isolation: false,
            native_session_id: Mutex::new(None),
            isolation_tree: Mutex::new(None),
            hooks_supported: Mutex::new(true),
            attach_fail: Mutex::new(false),
            telemetry: Mutex::new(Vec::new()),
            discovery_result: Mutex::new(None),
            discovery_count: Mutex::new(0),
        }
    }

    pub fn installed_grok() -> Self {
        let mut agent = Self::installed(GROK_BUILD_ID, GROK_BUILD_NAME, GROK_BIN);
        agent.native_isolation = true;
        agent
    }

    pub fn with_fields(mut self, fields: Vec<AgentField>, seed: BTreeMap<String, String>) -> Self {
        self.fields = fields;
        self.seed = seed;
        self
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

    pub fn set_native_session_id(&self, session_id: Option<String>) {
        *self.native_session_id.lock().expect("memory agent") = session_id;
    }

    pub fn set_isolation_tree(&self, path: Option<PathBuf>) {
        *self.isolation_tree.lock().expect("memory agent") = path;
    }

    pub fn set_hooks_supported(&self, supported: bool) {
        *self.hooks_supported.lock().expect("memory agent") = supported;
    }

    pub fn fail_attach_hooks(&self) {
        *self.attach_fail.lock().expect("memory agent") = true;
    }

    pub fn push_telemetry(&self, sample: TelemetrySample) {
        self.telemetry.lock().expect("memory agent").push(sample);
    }

    pub fn set_discovery_result(&self, discovery: AgentConfigDiscovery) {
        *self.discovery_result.lock().expect("memory agent") = Some(Ok(discovery));
    }

    pub fn set_discovery_error(&self, error: impl Into<String>) {
        *self.discovery_result.lock().expect("memory agent") = Some(Err(error.into()));
    }

    pub fn discovery_count(&self) -> u32 {
        *self.discovery_count.lock().expect("memory agent")
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
        self.fields.clone()
    }

    fn seed_config(&self) -> BTreeMap<String, String> {
        self.seed.clone()
    }

    fn discover_config(
        &self,
        _executable: &Path,
        _env: &LaunchEnvironment,
    ) -> Result<AgentConfigDiscovery, String> {
        *self.discovery_count.lock().expect("memory agent") += 1;
        self.discovery_result
            .lock()
            .expect("memory agent")
            .clone()
            .unwrap_or_else(|| {
                Ok(AgentConfigDiscovery {
                    fields: self.fields.clone(),
                    seed: self.seed.clone(),
                })
            })
    }

    fn assemble_argv_for(
        &self,
        executable: &Path,
        values: &BTreeMap<String, String>,
    ) -> Vec<String> {
        grok::grok_argv(executable, values, self.native_isolation)
    }

    fn recent_action(&self) -> Option<String> {
        self.recent_action.lock().expect("memory agent").clone()
    }

    fn native_session_id(&self) -> Option<String> {
        self.native_session_id.lock().expect("memory agent").clone()
    }

    fn assemble_argv_for_resume(
        &self,
        executable: &Path,
        values: &BTreeMap<String, String>,
        session_id: &str,
    ) -> Vec<String> {
        let mut argv = grok::grok_argv(executable, values, false);
        argv.push("--resume".into());
        argv.push(session_id.to_string());
        argv
    }

    fn native_isolation(&self) -> bool {
        self.native_isolation
    }

    fn isolation_tree_after_launch(
        &self,
        _project_dir: &Path,
        _before: &[PathBuf],
    ) -> Option<PathBuf> {
        self.isolation_tree.lock().expect("memory agent").clone()
    }

    fn completion_hooks_supported(&self) -> bool {
        *self.hooks_supported.lock().expect("memory agent")
    }

    fn attach_completion_hooks(
        &self,
        sink_dir: &Path,
        _project_dir: &Path,
    ) -> Result<CompletionHookPlan, String> {
        if !self.completion_hooks_supported() || *self.attach_fail.lock().expect("memory agent") {
            return Err("completion hooks unavailable".into());
        }
        fs::create_dir_all(sink_dir).map_err(|err| err.to_string())?;
        Ok(CompletionHookPlan {
            extra_argv: Vec::new(),
            extra_env: hooks::sink_env(sink_dir),
        })
    }

    fn drain_telemetry(&self) -> Vec<TelemetrySample> {
        std::mem::take(&mut *self.telemetry.lock().expect("memory agent"))
    }
}

fn isolation_enabled(values: &BTreeMap<String, String>) -> bool {
    values.get("isolation").is_some_and(|value| value == "true")
}

pub(super) fn append_isolation_flag(
    argv: &mut Vec<String>,
    values: &BTreeMap<String, String>,
    native: bool,
) {
    if native && isolation_enabled(values) && !argv.iter().any(|arg| arg == "--worktree") {
        argv.push("--worktree".into());
    }
}

fn append_flag(argv: &mut Vec<String>, flag: &str, value: Option<&String>) {
    if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
        argv.push(flag.into());
        argv.push(value.clone());
    }
}

fn append_switch(argv: &mut Vec<String>, flag: &str, values: &BTreeMap<String, String>, id: &str) {
    if values.get(id).is_some_and(|value| value == "true") {
        argv.push(flag.into());
    }
}

fn append_additional_args(argv: &mut Vec<String>, values: &BTreeMap<String, String>) {
    if let Some(args) = values.get("additional-args") {
        argv.extend(
            args.split_whitespace()
                .filter(|part| !part.is_empty())
                .map(ToOwned::to_owned),
        );
    }
}

fn text_field(id: &str, label: &str, required: bool, folded: bool) -> AgentField {
    AgentField {
        id: id.into(),
        label: label.into(),
        kind: AgentFieldKind::Text,
        options: Vec::new(),
        option_filter: None,
        required,
        folded,
    }
}

fn select_field(
    id: &str,
    label: &str,
    options: &[&str],
    required: bool,
    folded: bool,
) -> AgentField {
    AgentField {
        id: id.into(),
        label: label.into(),
        kind: AgentFieldKind::Select,
        options: options.iter().map(|option| (*option).to_string()).collect(),
        option_filter: None,
        required,
        folded,
    }
}

fn boolean_field(id: &str, label: &str, folded: bool) -> AgentField {
    AgentField {
        id: id.into(),
        label: label.into(),
        kind: AgentFieldKind::Boolean,
        options: Vec::new(),
        option_filter: None,
        required: false,
        folded,
    }
}

fn multiline_field(id: &str, label: &str) -> AgentField {
    AgentField {
        id: id.into(),
        label: label.into(),
        kind: AgentFieldKind::Multiline,
        options: Vec::new(),
        option_filter: None,
        required: false,
        folded: false,
    }
}

fn additional_args_field() -> AgentField {
    text_field("additional-args", "附加参数", false, true)
}

fn initial_instruction_field() -> AgentField {
    multiline_field("initial-instruction", "初始指令")
}

fn local_bin() -> Option<PathBuf> {
    home_dir().map(|home| home.join(".local").join("bin"))
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
    let exe = dir.join(format!("{bin}.exe"));
    if exe != candidate && is_executable(&exe) {
        return Some(exe);
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

pub(crate) fn home_dir() -> Option<PathBuf> {
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
