//! Host 内核：桌面窗口、浏览器和以后的远程 Client 都走这一条接缝。

mod agent;
mod board;
mod issue;
mod launch;
mod launch_env;
mod local_rpc;
mod owner;
mod pairing;
mod project;
mod refresh;
mod run;
mod session;
mod tracker;

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

pub use agent::{
    builtin_agents, intent_prefix, probe_binary, AgentField, AgentFieldKind, AgentPort,
    AgentSummary, AntigravityAdapter, ClaudeAdapter, CodexAdapter, GrokAdapter, IntentOption,
    MemoryAgent, PrefillSource, ProbeResult, RunIntent, RunLaunchConfig, RunLaunchForm,
    ANTIGRAVITY_BIN, ANTIGRAVITY_ID, ANTIGRAVITY_NAME, CLAUDE_BIN, CLAUDE_CODE_ID,
    CLAUDE_CODE_NAME, CODEX_BIN, CODEX_ID, CODEX_NAME, GROK_BIN, GROK_BUILD_ID, GROK_BUILD_NAME,
};
pub use board::{
    clamp_recent_limit, BoardColumns, BoardEmptyReason, BoardSnapshot, CenterView, DependencyGraph,
    FrontierEmptyReason, GraphEdge, GraphNode, IssueCard, IssueDetail, IssueLink, RefreshStatus,
    DEFAULT_RECENT_LIMIT,
};
pub use issue::{IssueRecord, TriageRole};
pub use launch_env::{LaunchEnvPort, LaunchEnvironment, MemoryLaunchEnv, ShellLaunchEnv};
pub use local_rpc::{
    bind_local_rpc, local_client_origin_allowed, spawn_local_rpc, LoopbackAssets, LoopbackServer,
    LOCAL_RPC_PORT,
};
pub use pairing::{IssuedPairing, PairedClient, PairingOffer};
pub use project::ProjectInference;
pub use refresh::DEFAULT_REFRESH_INTERVAL_MS;
pub use run::{QuitOffer, RunStatus, RunSummary};
pub use session::{
    AgentSession, MemorySession, MemorySessionFactory, PtyChunk, PtySessionFactory, SessionFactory,
    SpawnRequest,
};
pub use tracker::{
    map_github_issue_node, AuthFailureKind, CredentialSource, GitHubTracker, MemoryTracker,
    ProjectConnection, RepairHint, ScriptedGitHub, TrackerKind, TrackerPort,
};

const LOCAL_HOST_ID: &str = "local";

#[derive(Debug, Clone)]
pub struct BootRequest {
    pub app_local_data_dir: PathBuf,
    pub app_log_dir: PathBuf,
    pub system_locale: String,
    pub system_appearance: SystemAppearance,
    pub host_display_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemAppearance {
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "en")]
    En,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    WarmPaper,
    PlainPaper,
    PlainNight,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    HideWindow,
    ShowWindow,
    QuitHost,
    SetLanguage(Language),
    SetTheme(Theme),
    BeginPairingOffer {
        address: String,
    },
    RedeemPairing {
        code: String,
        client_name: String,
    },
    RevokeClient {
        client_id: String,
    },
    PairRemoteHost {
        address: String,
        code: String,
    },
    FocusHost {
        host_id: String,
    },
    RegisterProject {
        name: String,
        local_path: String,
        github_host: String,
        repository: String,
    },
    EditProject {
        project_id: String,
        name: String,
        local_path: String,
        github_host: String,
        repository: String,
    },
    RemoveProject {
        project_id: String,
    },
    FocusProject {
        project_id: String,
    },
    InferProject {
        local_path: String,
    },
    FocusIssue {
        issue_id: String,
    },
    FilterParent {
        issue_id: String,
    },
    ClearParentFilter,
    SetCenterView {
        view: CenterView,
    },
    SetShowClosedGraphContext {
        show: bool,
    },
    SetRecentCompletedLimit {
        limit: u32,
    },
    Refresh {
        project_id: Option<String>,
    },
    Tick {
        now_ms: Option<u64>,
    },
    SetClientView {
        client_id: String,
        project_id: String,
        visible: bool,
    },
    NoteRunEnded {
        project_id: String,
    },
    ClaimIssue {
        issue_id: String,
    },
    ReleaseIssue {
        issue_id: String,
    },
    AutoAdvance {
        project_id: String,
    },
    CheckIssueClosed {
        issue_id: String,
    },
    StartBoundRun {
        issue_id: String,
    },
    StartUnboundRun {
        project_id: String,
    },
    PrepareRunLaunch {
        project_id: String,
        issue_id: Option<String>,
        agent_id: Option<String>,
        pick_agent: bool,
    },
    CancelRunLaunch,
    StopRun {
        run_id: String,
    },
    FocusRun {
        run_id: String,
    },
    InjectRunInput {
        run_id: String,
        text: String,
    },
    CancelQuit,
    ConfirmQuitStopAll,
    SetRefreshInterval {
        interval_ms: u64,
    },
    StartUnboundRunWithConfig {
        project_id: String,
        config: RunLaunchConfig,
        issue_id: Option<String>,
    },
    SetShowCommandPreview {
        show: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProcessIntent {
    KeepRunning,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EmptyAction {
    RegisterFirstProject,
    PairAnotherHost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoopbackKind {
    Serving,
    Occupied,
    HostNotRunning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum LoopbackPage {
    Serving { url: String },
    Occupied { url: String, reason: String },
    HostNotRunning { url: String, reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum HostEvent {
    RefreshStatusChanged {
        #[serde(rename = "projectId")]
        project_id: String,
        status: RefreshStatus,
    },
    BoardUpdated {
        #[serde(rename = "projectId")]
        project_id: String,
    },
    RunStatusChanged {
        #[serde(rename = "runId")]
        run_id: String,
        status: RunStatus,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandOutcome {
    pub snapshot: HostSnapshot,
    pub process: ProcessIntent,
    pub pairing: Option<IssuedPairing>,
    pub inference: Option<ProjectInference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<HostEvent>,
}

impl CommandOutcome {
    pub fn to_json(&self) -> serde_json::Value {
        let mut value = serde_json::json!({
            "snapshot": self.snapshot,
            "process": self.process,
        });
        if let Some(pairing) = &self.pairing {
            value["pairing"] = serde_json::to_value(pairing).expect("pairing json");
        }
        if let Some(inference) = &self.inference {
            value["inference"] = serde_json::to_value(inference).expect("inference json");
        }
        if !self.events.is_empty() {
            value["events"] = serde_json::to_value(&self.events).expect("events json");
        }
        value
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataLayout {
    pub host_dir: PathBuf,
    pub desktop_client_dir: PathBuf,
    pub host_settings_path: PathBuf,
    pub host_secrets_path: PathBuf,
    pub desktop_client_settings_path: PathBuf,
    pub desktop_client_secrets_path: PathBuf,
    pub log_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostSummary {
    pub id: String,
    pub display_name: String,
    pub local: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub local_path: PathBuf,
    pub tracker: TrackerKind,
    pub github_host: String,
    pub repository: String,
    pub connection: ProjectConnection,
    pub has_active_run: bool,
    pub tracker_synced: bool,
}

#[derive(Debug, Clone)]
struct ProjectRecord {
    id: String,
    name: String,
    local_path: PathBuf,
    github_host: String,
    repository: String,
    connection: ProjectConnection,
    tracker_synced: bool,
}

impl ProjectRecord {
    fn summary(&self, has_active_run: bool) -> ProjectSummary {
        ProjectSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            local_path: self.local_path.clone(),
            tracker: TrackerKind::Github,
            github_host: self.github_host.clone(),
            repository: self.repository.clone(),
            connection: self.connection.clone(),
            has_active_run,
            tracker_synced: self.tracker_synced,
        }
    }

    fn stored(&self) -> StoredProject {
        StoredProject {
            id: self.id.clone(),
            name: self.name.clone(),
            local_path: self.local_path.clone(),
            github_host: self.github_host.clone(),
            repository: self.repository.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppearanceSelection {
    language: Language,
    theme: Theme,
    last_light_theme: Theme,
}

impl AppearanceSelection {
    fn with_language(self, language: Language) -> Self {
        Self { language, ..self }
    }

    fn with_theme(self, theme: Theme) -> Self {
        Self {
            theme,
            last_light_theme: if matches!(theme, Theme::PlainNight) {
                self.last_light_theme
            } else {
                daytime_theme(theme)
            },
            ..self
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceState {
    pub language: Language,
    pub theme: Theme,
    pub last_light_theme: Theme,
    pub languages: Vec<Language>,
    pub themes: Vec<Theme>,
}

impl AppearanceState {
    fn from_selection(selection: AppearanceSelection) -> Self {
        Self {
            language: selection.language,
            theme: selection.theme,
            last_light_theme: selection.last_light_theme,
            languages: vec![Language::ZhCn, Language::En],
            themes: vec![Theme::WarmPaper, Theme::PlainPaper, Theme::PlainNight],
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellCopy {
    pub app_name: String,
    pub register_first_project: String,
    pub pair_another_host: String,
    pub no_project_title: String,
    pub no_project_body: String,
    pub quit_host: String,
    pub show_window: String,
    pub settings: String,
    pub language: String,
    pub theme: String,
    pub language_zh: String,
    pub language_en: String,
    pub theme_warm_paper: String,
    pub theme_plain_paper: String,
    pub theme_plain_night: String,
    pub hosts: String,
    pub projects: String,
    pub this_machine: String,
    pub shade_light: String,
    pub shade_dark: String,
    pub edit_menu: String,
    pub pairing_required: String,
    pub pairing_title: String,
    pub pairing_this_host: String,
    pub pairing_to_another: String,
    pub pairing_address: String,
    pub pairing_show: String,
    pub pairing_copy: String,
    pub pairing_same_payload: String,
    pub pairing_paste: String,
    pub pairing_connect: String,
    pub paired_clients: String,
    pub revoke_client: String,
    pub no_paired_clients: String,
    pub add_project: String,
    pub edit_project: String,
    pub remove_project: String,
    pub register_project_title: String,
    pub edit_project_title: String,
    pub display_name: String,
    pub local_directory: String,
    pub github_host: String,
    pub repository: String,
    pub infer_from_directory: String,
    pub use_inference: String,
    pub inference_hint: String,
    pub save_registration: String,
    pub cancel: String,
    pub remove_confirm_title: String,
    pub remove_confirm_body: String,
    pub remove_confirm: String,
    pub cannot_remove_active_run: String,
    pub cannot_remove_active_run_body: String,
    pub got_it: String,
    pub auth_failed: String,
    pub repair_cli: String,
    pub repair_secrets: String,
    pub repair_env: String,
    pub no_gh_detected: String,
    pub connection_ready: String,
    pub project_menu: String,
    pub board_hint: String,
    pub child_hint: String,
    pub graph_hint: String,
    pub view_board: String,
    pub view_graph: String,
    pub show_closed_context: String,
    pub clear_filter: String,
    pub col_blocked: String,
    pub col_frontier: String,
    pub col_in_progress: String,
    pub col_recent: String,
    pub no_items: String,
    pub no_frontier_blocked: String,
    pub no_frontier_claimed: String,
    pub no_frontier_empty: String,
    pub no_recent: String,
    pub recent_note: String,
    pub empty_no_data: String,
    pub family: String,
    pub deps: String,
    pub parent: String,
    pub children: String,
    pub no_parent: String,
    pub no_kids: String,
    pub only_kids: String,
    pub blocked_by: String,
    pub blocking: String,
    pub none_block: String,
    pub none: String,
    pub claimed: String,
    pub unclaimed: String,
    pub pick_issue: String,
    pub recent_limit: String,
    pub recent_limit_help: String,
    pub unclear_issue: String,
    pub refresh_now: String,
    pub refresh_refreshing: String,
    pub refresh_as_of: String,
    pub refresh_next: String,
    pub refresh_offline: String,
    pub refresh_never: String,
    pub refresh_rate_limited: String,
    pub refresh_retry: String,
    pub refresh_paused: String,
    pub refresh_auth: String,
    pub new_run: String,
    pub execute_run: String,
    pub start_run: String,
    pub switch_agent: String,
    pub pick_agent: String,
    pub launch_title: String,
    pub prefill_current: String,
    pub prefill_other: String,
    pub prefill_seed: String,
    pub isolation: String,
    pub isolation_off_reason: String,
    pub isolation_hint: String,
    pub run_intent: String,
    pub intent_none: String,
    pub intent_modify: String,
    pub intent_continue: String,
    pub intent_answer: String,
    pub intent_review: String,
    pub intent_custom: String,
    pub opening_placeholder: String,
    pub folded_options: String,
    pub command_preview: String,
    pub show_command_preview: String,
    pub instruction_required: String,
    pub working_directory: String,
    pub unbound_issue: String,
    pub stop_run: String,
    pub quit_active_title: String,
    pub quit_active_body: String,
    pub quit_return: String,
    pub quit_stop_all: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostSnapshot {
    pub running: bool,
    pub window_visible: bool,
    pub focused_host_id: String,
    pub focused_project_id: String,
    pub hosts: Vec<HostSummary>,
    pub projects: Vec<ProjectSummary>,
    pub appearance: AppearanceState,
    pub data: DataLayout,
    pub copy: ShellCopy,
    pub empty_actions: Vec<EmptyAction>,
    pub loopback_page: LoopbackPage,
    pub pairing_offer: Option<PairingOffer>,
    pub paired_clients: Vec<PairedClient>,
    pub board: Option<BoardSnapshot>,
    pub recent_completed_limit: u32,
    pub center_view: CenterView,
    pub runs: Vec<RunSummary>,
    pub focused_run_id: String,
    pub quit_offer: Option<QuitOffer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_form: Option<RunLaunchForm>,
    pub show_command_preview: bool,
}

pub struct KernelPorts {
    pub tracker: Arc<dyn TrackerPort>,
    pub agents: Vec<Arc<dyn AgentPort>>,
    pub launch_env: Arc<dyn LaunchEnvPort>,
    pub sessions: Arc<dyn SessionFactory>,
}

impl KernelPorts {
    pub fn live() -> Self {
        Self {
            tracker: Arc::new(GitHubTracker::live()),
            agents: builtin_agents(),
            launch_env: Arc::new(ShellLaunchEnv::live()),
            sessions: Arc::new(PtySessionFactory),
        }
    }

    pub fn for_tests(tracker: Arc<dyn TrackerPort>) -> Self {
        Self {
            tracker,
            agents: vec![Arc::new(MemoryAgent::installed_grok())],
            launch_env: Arc::new(MemoryLaunchEnv::with_path("/mem/bin")),
            sessions: MemorySessionFactory::new(),
        }
    }
}

pub struct HostKernel {
    running: bool,
    window_visible: bool,
    host_display_name: String,
    data: DataLayout,
    appearance: AppearanceSelection,
    projects: Vec<ProjectRecord>,
    focused_project_id: Option<String>,
    tracker: Arc<dyn TrackerPort>,
    agents: Vec<Arc<dyn AgentPort>>,
    launch_env: Arc<dyn LaunchEnvPort>,
    sessions: Arc<dyn SessionFactory>,
    runs: Vec<RunSummary>,
    live: BTreeMap<String, Arc<dyn AgentSession>>,
    focused_run_id: Option<String>,
    quit_offer: Option<QuitOffer>,
    loopback_kind: LoopbackKind,
    loopback_port: u16,
    pairing_offer: Option<pairing::ActiveOffer>,
    host_id: String,
    paired_clients: Vec<pairing::IssuedClient>,
    focused_host_id: String,
    remote_hosts: Vec<pairing::RemoteHost>,
    remote_view: Option<RemoteView>,
    loaded_issues: BTreeMap<String, Vec<IssueRecord>>,
    refresh: BTreeMap<String, ProjectRefreshState>,
    client_views: BTreeMap<String, ClientView>,
    pending_events: Vec<HostEvent>,
    now_ms: u64,
    refresh_interval_ms: u64,
    selected_issue_id: Option<String>,
    parent_filter: Option<String>,
    recent_limit: u32,
    center_view: CenterView,
    show_closed_graph_context: bool,
    launch_defaults: BTreeMap<String, BTreeMap<String, BTreeMap<String, String>>>,
    last_successful_agent: BTreeMap<String, String>,
    launch_form: Option<RunLaunchForm>,
    show_command_preview: bool,
}

#[derive(Debug, Clone)]
struct ProjectRefreshState {
    fetched_at_ms: Option<u64>,
    last_attempt_ms: u64,
    kind: StoredRefreshKind,
    retry_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StoredRefreshKind {
    Ready,
    Offline,
    NeverFetched,
    RateLimited,
    AuthFailed,
}

#[derive(Debug, Clone)]
struct ClientView {
    project_id: String,
    visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefreshTrigger {
    Immediate,
    Action,
    Interval,
    RunEnded,
}

#[derive(Debug, Clone)]
struct RemoteView {
    host_id: String,
    projects: Vec<ProjectSummary>,
    focused_project_id: String,
    empty_actions: Vec<EmptyAction>,
    board: Option<BoardSnapshot>,
    runs: Vec<RunSummary>,
    focused_run_id: String,
    quit_offer: Option<QuitOffer>,
    launch_form: Option<RunLaunchForm>,
}

impl HostKernel {
    pub fn boot(request: BootRequest) -> Result<Self, KernelError> {
        Self::boot_with_ports(request, KernelPorts::live())
    }

    pub fn boot_with(
        request: BootRequest,
        tracker: Arc<dyn TrackerPort>,
    ) -> Result<Self, KernelError> {
        Self::boot_with_ports(request, KernelPorts::for_tests(tracker))
    }

    pub fn boot_with_ports(request: BootRequest, ports: KernelPorts) -> Result<Self, KernelError> {
        let KernelPorts {
            tracker,
            agents,
            launch_env,
            sessions,
        } = ports;
        let data = DataLayout::prepare(&request.app_local_data_dir, &request.app_log_dir)?;
        let settings = load_or_init_host_settings(&data.host_settings_path)?;
        let host_id = settings.id;
        let paired_clients = load_paired_clients(&data.host_secrets_path)?;

        let (
            appearance,
            focused_host_id,
            saved_remotes,
            recent_limit,
            center_view,
            show_command_preview,
        ) = load_or_init_appearance(
            &data.desktop_client_settings_path,
            &request.system_locale,
            request.system_appearance,
        )?;
        let tokens = load_client_tokens(&data.desktop_client_secrets_path)?;
        let remote_hosts = saved_remotes
            .into_iter()
            .filter_map(|saved| {
                tokens.get(&saved.id).map(|token| pairing::RemoteHost {
                    id: saved.id,
                    display_name: saved.display_name,
                    address: saved.address,
                    token: token.clone(),
                })
            })
            .collect::<Vec<_>>();
        let focused_host_id = if focused_host_id == LOCAL_HOST_ID
            || remote_hosts.iter().any(|host| host.id == focused_host_id)
        {
            focused_host_id
        } else {
            LOCAL_HOST_ID.to_string()
        };

        let language = appearance.language;
        let secrets_path = data.host_secrets_path.clone();
        let projects = settings
            .projects
            .into_iter()
            .map(|stored| probe_record(stored, tracker.as_ref(), &secrets_path, language))
            .collect::<Vec<_>>();
        let focused_project_id = settings
            .focused_project_id
            .filter(|id| projects.iter().any(|project| project.id == *id))
            .or_else(|| projects.first().map(|project| project.id.clone()));

        let mut host = Self {
            running: true,
            window_visible: true,
            host_display_name: request.host_display_name,
            data,
            appearance,
            projects,
            focused_project_id,
            tracker,
            agents,
            launch_env,
            sessions,
            runs: Vec::new(),
            live: BTreeMap::new(),
            focused_run_id: None,
            quit_offer: None,
            loopback_kind: LoopbackKind::HostNotRunning,
            loopback_port: LOCAL_RPC_PORT,
            pairing_offer: None,
            host_id,
            paired_clients,
            focused_host_id,
            remote_hosts,
            remote_view: None,
            loaded_issues: BTreeMap::new(),
            refresh: BTreeMap::new(),
            client_views: BTreeMap::new(),
            pending_events: Vec::new(),
            now_ms: refresh::wall_ms(),
            refresh_interval_ms: refresh::clamp_refresh_interval_ms(settings.refresh_interval_ms),
            selected_issue_id: None,
            parent_filter: None,
            recent_limit,
            center_view,
            show_closed_graph_context: false,
            launch_defaults: settings.agent_launch_defaults,
            last_successful_agent: settings.last_successful_agent,
            launch_form: None,
            show_command_preview,
        };
        let project_ids: Vec<String> = host
            .projects
            .iter()
            .map(|project| project.id.clone())
            .collect();
        for project_id in &project_ids {
            host.load_persisted_snapshot(project_id);
        }
        if let Some(project_id) = host.focused_project_id.clone() {
            host.refresh_project(&project_id, RefreshTrigger::Immediate);
        }
        host.pending_events.clear();
        Ok(host)
    }

    pub fn snapshot(&self) -> HostSnapshot {
        let (projects, focused_project_id, empty_actions) = self.board_for_focus();
        let board = self.current_board(&focused_project_id);
        let (runs, focused_run_id, quit_offer) = self.runs_for_focus();
        HostSnapshot {
            running: self.running,
            window_visible: self.window_visible,
            focused_host_id: self.focused_host_id.clone(),
            focused_project_id,
            hosts: self.connected_hosts(),
            projects,
            appearance: AppearanceState::from_selection(self.appearance),
            data: self.data.clone(),
            copy: ShellCopy::for_language(self.appearance.language),
            empty_actions,
            loopback_page: self.loopback_page(),
            pairing_offer: self
                .pairing_offer
                .as_ref()
                .map(pairing::ActiveOffer::to_offer),
            paired_clients: self
                .paired_clients
                .iter()
                .map(pairing::IssuedClient::summary)
                .collect(),
            board,
            recent_completed_limit: self.recent_limit,
            center_view: self.center_view,
            runs,
            focused_run_id,
            quit_offer,
            launch_form: self.launch_form_for_focus(),
            show_command_preview: self.show_command_preview,
        }
    }

    fn outcome(&mut self) -> CommandOutcome {
        self.outcome_with(None, None)
    }

    fn outcome_with(
        &mut self,
        pairing: Option<IssuedPairing>,
        inference: Option<ProjectInference>,
    ) -> CommandOutcome {
        CommandOutcome {
            snapshot: self.snapshot(),
            process: if self.running {
                ProcessIntent::KeepRunning
            } else {
                ProcessIntent::Exit
            },
            pairing,
            inference,
            events: std::mem::take(&mut self.pending_events),
        }
    }

    pub fn pty_session(&self, run_id: &str) -> Result<Arc<dyn AgentSession>, KernelError> {
        self.live
            .get(run_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown run".into()))
    }

    pub fn pty_output(
        &self,
        run_id: &str,
        after: usize,
        wait: Duration,
    ) -> Result<PtyChunk, KernelError> {
        Ok(self.pty_session(run_id)?.read_after(after, wait))
    }

    pub fn note_run_exit(&mut self, run_id: &str, _code: i32) {
        self.mark_run_ended(run_id);
    }

    pub fn write_pty(&self, run_id: &str, data: &[u8]) -> Result<(), KernelError> {
        let session = self
            .live
            .get(run_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown run".into()))?;
        session.write(data).map_err(KernelError::Io)
    }

    pub fn resize_pty(&self, run_id: &str, cols: u16, rows: u16) -> Result<(), KernelError> {
        let session = self
            .live
            .get(run_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown run".into()))?;
        session.resize(cols, rows);
        Ok(())
    }

    pub(crate) fn note_loopback_page(&mut self, kind: LoopbackKind, port: u16) {
        self.loopback_kind = kind;
        self.loopback_port = if port == 0 { LOCAL_RPC_PORT } else { port };
    }

    fn loopback_page(&self) -> LoopbackPage {
        let url = format!("http://127.0.0.1:{}/", self.loopback_port);
        match self.loopback_kind {
            LoopbackKind::Serving => LoopbackPage::Serving { url },
            LoopbackKind::Occupied => LoopbackPage::Occupied {
                url,
                reason: occupied_reason(self.appearance.language, self.loopback_port),
            },
            LoopbackKind::HostNotRunning => LoopbackPage::HostNotRunning {
                url,
                reason: host_not_running_reason(self.appearance.language),
            },
        }
    }

    pub fn dispatch(&mut self, command: Command) -> Result<CommandOutcome, KernelError> {
        self.reap_runs();
        match command {
            Command::HideWindow => self.window_visible = false,
            Command::ShowWindow => {
                if self.running {
                    self.window_visible = true;
                    if let Some(project_id) = self.focused_project_id.clone() {
                        self.refresh_project(&project_id, RefreshTrigger::Immediate);
                    }
                }
            }
            Command::QuitHost => {
                if self.active_run_count() > 0 {
                    self.quit_offer = Some(QuitOffer {
                        active_run_count: self.active_run_count(),
                    });
                } else {
                    self.running = false;
                    self.window_visible = false;
                    self.loopback_kind = LoopbackKind::HostNotRunning;
                }
            }
            Command::SetLanguage(language) => {
                let appearance = self.appearance.with_language(language);
                self.persist_client_settings(&appearance)?;
                self.appearance = appearance;
            }
            Command::SetTheme(theme) => {
                let appearance = self.appearance.with_theme(theme);
                self.persist_client_settings(&appearance)?;
                self.appearance = appearance;
            }
            Command::BeginPairingOffer { address } => {
                let address = pairing::parse_http_url(&address).map_err(KernelError::Protocol)?;
                self.pairing_offer = Some(pairing::ActiveOffer::new(address));
            }
            Command::RedeemPairing { code, client_name } => {
                let pairing = self.redeem_pairing(&code, &client_name)?;
                return Ok(self.outcome_with(Some(pairing), None));
            }
            Command::RevokeClient { client_id } => {
                self.paired_clients.retain(|client| client.id != client_id);
                self.persist_host_secrets()?;
            }
            Command::PairRemoteHost { address, code } => {
                self.pair_remote_host(&address, &code)?;
            }
            Command::FocusHost { host_id } => {
                self.focus_host(&host_id)?;
            }
            Command::RegisterProject {
                name,
                local_path,
                github_host,
                repository,
            } => {
                self.register_project(&name, &local_path, &github_host, &repository)?;
            }
            Command::EditProject {
                project_id,
                name,
                local_path,
                github_host,
                repository,
            } => {
                self.edit_project(&project_id, &name, &local_path, &github_host, &repository)?;
            }
            Command::RemoveProject { project_id } => {
                self.remove_project(&project_id)?;
            }
            Command::FocusProject { project_id } => {
                self.focus_project(&project_id)?;
            }
            Command::InferProject { local_path } => {
                let inference = self.infer_project(&local_path)?;
                return Ok(self.outcome_with(None, inference));
            }
            Command::FocusIssue { issue_id } => {
                self.selected_issue_id = Some(issue_id);
            }
            Command::FilterParent { issue_id } => {
                self.parent_filter = Some(issue_id);
            }
            Command::ClearParentFilter => {
                self.parent_filter = None;
            }
            Command::SetCenterView { view } => {
                self.center_view = view;
                self.persist_client_settings(&self.appearance.clone())?;
            }
            Command::SetShowClosedGraphContext { show } => {
                self.show_closed_graph_context = show;
            }
            Command::SetRecentCompletedLimit { limit } => {
                self.recent_limit = board::clamp_recent_limit(limit);
                self.persist_client_settings(&self.appearance.clone())?;
            }
            Command::Refresh { project_id } => {
                let project_id = project_id
                    .or_else(|| self.focused_project_id.clone())
                    .ok_or_else(|| KernelError::Protocol("missing projectId".into()))?;
                self.refresh_project(&project_id, RefreshTrigger::Immediate);
            }
            Command::Tick { now_ms } => {
                self.now_ms = now_ms.unwrap_or_else(refresh::wall_ms);
                self.maybe_auto_refresh();
            }
            Command::SetClientView {
                client_id,
                project_id,
                visible,
            } => {
                self.set_client_view(&client_id, &project_id, visible);
            }
            Command::NoteRunEnded { project_id } => {
                self.refresh_project(&project_id, RefreshTrigger::RunEnded);
            }
            Command::ClaimIssue { issue_id } => {
                self.require_live_tracker_for_issue(&issue_id)?;
            }
            Command::ReleaseIssue { issue_id } => {
                self.require_live_tracker_for_issue(&issue_id)?;
            }
            Command::AutoAdvance { project_id } => {
                self.require_live_tracker(&project_id)?;
            }
            Command::CheckIssueClosed { issue_id } => {
                self.require_live_tracker_for_issue(&issue_id)?;
            }
            Command::StartBoundRun { issue_id } => {
                self.require_live_tracker_for_issue(&issue_id)?;
            }
            Command::PrepareRunLaunch {
                project_id,
                issue_id,
                agent_id,
                pick_agent,
            } => {
                self.prepare_run_launch(&project_id, issue_id, agent_id, pick_agent)?;
            }
            Command::CancelRunLaunch => {
                self.launch_form = None;
            }
            Command::StartUnboundRun { project_id } => {
                let agent = self.default_agent_for_project(&project_id)?;
                self.start_unbound_run(
                    &project_id,
                    RunLaunchConfig {
                        agent_id: agent.id().to_string(),
                        values: agent.seed_config(),
                        opening_text: String::new(),
                    },
                    None,
                    false,
                )?;
            }
            Command::StartUnboundRunWithConfig {
                project_id,
                config,
                issue_id,
            } => {
                self.start_unbound_run(&project_id, config, issue_id, true)?;
            }
            Command::SetShowCommandPreview { show } => {
                self.show_command_preview = show;
                self.persist_client_settings(&self.appearance.clone())?;
            }
            Command::StopRun { run_id } => {
                self.stop_run(&run_id)?;
            }
            Command::FocusRun { run_id } => {
                self.focus_run(&run_id)?;
            }
            Command::InjectRunInput { run_id, text } => {
                let mut data = text.into_bytes();
                if !data.ends_with(&[b'\n']) {
                    data.push(b'\n');
                }
                self.write_pty(&run_id, &data)?;
            }
            Command::CancelQuit => {
                self.quit_offer = None;
            }
            Command::ConfirmQuitStopAll => {
                self.stop_all_runs();
                self.quit_offer = None;
                self.running = false;
                self.window_visible = false;
                self.loopback_kind = LoopbackKind::HostNotRunning;
            }
            Command::SetRefreshInterval { interval_ms } => {
                self.refresh_interval_ms = refresh::clamp_refresh_interval_ms(interval_ms);
                self.persist_host_settings()?;
            }
        }
        Ok(self.outcome())
    }

    pub fn handle(&mut self, request: serde_json::Value) -> Result<CommandOutcome, KernelError> {
        let op = request
            .get("op")
            .and_then(|value| value.as_str())
            .unwrap_or("snapshot");
        match op {
            "snapshot" => {
                self.reap_runs();
                Ok(self.outcome())
            }
            "hideWindow" => self.dispatch(Command::HideWindow),
            "showWindow" => self.dispatch(Command::ShowWindow),
            "quitHost" => self.dispatch(Command::QuitHost),
            "setLanguage" => {
                let language = serde_json::from_value(
                    request
                        .get("language")
                        .cloned()
                        .ok_or_else(|| KernelError::Protocol("missing language".into()))?,
                )?;
                self.dispatch(Command::SetLanguage(language))
            }
            "setTheme" => {
                let theme = serde_json::from_value(
                    request
                        .get("theme")
                        .cloned()
                        .ok_or_else(|| KernelError::Protocol("missing theme".into()))?,
                )?;
                self.dispatch(Command::SetTheme(theme))
            }
            "beginPairingOffer" => {
                let address = request
                    .get("address")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| KernelError::Protocol("missing address".into()))?
                    .to_string();
                self.dispatch(Command::BeginPairingOffer { address })
            }
            "redeemPairing" => {
                let code = request
                    .get("code")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| KernelError::Protocol("missing code".into()))?
                    .to_string();
                let client_name = request
                    .get("clientName")
                    .and_then(|value| value.as_str())
                    .unwrap_or("Client")
                    .to_string();
                self.dispatch(Command::RedeemPairing { code, client_name })
            }
            "revokeClient" => {
                let client_id = request
                    .get("clientId")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| KernelError::Protocol("missing clientId".into()))?
                    .to_string();
                self.dispatch(Command::RevokeClient { client_id })
            }
            "pairRemoteHost" => {
                let address = request
                    .get("address")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| KernelError::Protocol("missing address".into()))?
                    .to_string();
                let code = request
                    .get("code")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| KernelError::Protocol("missing code".into()))?
                    .to_string();
                self.dispatch(Command::PairRemoteHost { address, code })
            }
            "focusHost" => {
                let host_id = request
                    .get("hostId")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| KernelError::Protocol("missing hostId".into()))?
                    .to_string();
                self.dispatch(Command::FocusHost { host_id })
            }
            "registerProject" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::RegisterProject {
                    name: required_string(&request, "name")?,
                    local_path: required_string(&request, "localPath")?,
                    github_host: optional_string(&request, "githubHost"),
                    repository: required_string(&request, "repository")?,
                })
            }
            "editProject" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::EditProject {
                    project_id: required_string(&request, "projectId")?,
                    name: required_string(&request, "name")?,
                    local_path: required_string(&request, "localPath")?,
                    github_host: optional_string(&request, "githubHost"),
                    repository: required_string(&request, "repository")?,
                })
            }
            "removeProject" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::RemoveProject {
                    project_id: required_string(&request, "projectId")?,
                })
            }
            "focusProject" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::FocusProject {
                    project_id: required_string(&request, "projectId")?,
                })
            }
            "inferProject" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::InferProject {
                    local_path: required_string(&request, "localPath")?,
                })
            }
            "focusIssue" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::FocusIssue {
                    issue_id: required_string(&request, "issueId")?,
                })
            }
            "filterParent" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::FilterParent {
                    issue_id: required_string(&request, "issueId")?,
                })
            }
            "clearParentFilter" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::ClearParentFilter)
            }
            "setCenterView" => {
                let view = serde_json::from_value(
                    request
                        .get("view")
                        .cloned()
                        .ok_or_else(|| KernelError::Protocol("missing view".into()))?,
                )?;
                self.dispatch(Command::SetCenterView { view })
            }
            "setShowClosedGraphContext" => self.dispatch(Command::SetShowClosedGraphContext {
                show: request
                    .get("show")
                    .and_then(|value| value.as_bool())
                    .ok_or_else(|| KernelError::Protocol("missing show".into()))?,
            }),
            "setRecentCompletedLimit" => self.dispatch(Command::SetRecentCompletedLimit {
                limit: request
                    .get("limit")
                    .and_then(|value| value.as_u64())
                    .ok_or_else(|| KernelError::Protocol("missing limit".into()))?
                    as u32,
            }),
            "refresh" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::Refresh {
                    project_id: request
                        .get("projectId")
                        .and_then(|value| value.as_str())
                        .filter(|value| !value.is_empty())
                        .map(ToOwned::to_owned),
                })
            }
            "tick" => self.dispatch(Command::Tick {
                now_ms: request.get("nowMs").and_then(|value| value.as_u64()),
            }),
            "setClientView" => self.dispatch(Command::SetClientView {
                client_id: required_string(&request, "clientId")?,
                project_id: optional_string(&request, "projectId"),
                visible: request
                    .get("visible")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false),
            }),
            "noteRunEnded" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::NoteRunEnded {
                    project_id: required_string(&request, "projectId")?,
                })
            }
            "claimIssue" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::ClaimIssue {
                    issue_id: required_string(&request, "issueId")?,
                })
            }
            "releaseIssue" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::ReleaseIssue {
                    issue_id: required_string(&request, "issueId")?,
                })
            }
            "autoAdvance" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::AutoAdvance {
                    project_id: required_string(&request, "projectId")?,
                })
            }
            "checkIssueClosed" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::CheckIssueClosed {
                    issue_id: required_string(&request, "issueId")?,
                })
            }
            "startBoundRun" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::StartBoundRun {
                    issue_id: required_string(&request, "issueId")?,
                })
            }
            "prepareRunLaunch" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::PrepareRunLaunch {
                    project_id: required_string(&request, "projectId")?,
                    issue_id: request
                        .get("issueId")
                        .and_then(|value| value.as_str())
                        .filter(|value| !value.is_empty())
                        .map(ToOwned::to_owned),
                    agent_id: request
                        .get("agentId")
                        .and_then(|value| value.as_str())
                        .filter(|value| !value.is_empty())
                        .map(ToOwned::to_owned),
                    pick_agent: request
                        .get("pickAgent")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false),
                })
            }
            "cancelRunLaunch" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::CancelRunLaunch)
            }
            "startUnboundRun" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                let project_id = required_string(&request, "projectId")?;
                if request.get("agentId").is_some()
                    || request.get("values").is_some()
                    || request.get("openingText").is_some()
                {
                    self.dispatch(Command::StartUnboundRunWithConfig {
                        project_id,
                        config: parse_launch_config(&request)?,
                        issue_id: request
                            .get("issueId")
                            .and_then(|value| value.as_str())
                            .filter(|value| !value.is_empty())
                            .map(ToOwned::to_owned),
                    })
                } else {
                    self.dispatch(Command::StartUnboundRun { project_id })
                }
            }
            "setShowCommandPreview" => self.dispatch(Command::SetShowCommandPreview {
                show: request
                    .get("show")
                    .and_then(|value| value.as_bool())
                    .ok_or_else(|| KernelError::Protocol("missing show".into()))?,
            }),
            "stopRun" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::StopRun {
                    run_id: required_string(&request, "runId")?,
                })
            }
            "focusRun" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::FocusRun {
                    run_id: required_string(&request, "runId")?,
                })
            }
            "injectRunInput" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::InjectRunInput {
                    run_id: required_string(&request, "runId")?,
                    text: request
                        .get("text")
                        .and_then(|value| value.as_str())
                        .unwrap_or("")
                        .to_string(),
                })
            }
            "cancelQuit" => self.dispatch(Command::CancelQuit),
            "confirmQuitStopAll" => self.dispatch(Command::ConfirmQuitStopAll),
            "setRefreshInterval" => self.dispatch(Command::SetRefreshInterval {
                interval_ms: request
                    .get("intervalMs")
                    .and_then(|value| value.as_u64())
                    .ok_or_else(|| KernelError::Protocol("missing intervalMs".into()))?,
            }),
            other => Err(KernelError::Protocol(format!("unknown op {other}"))),
        }
    }

    pub fn pairing_token_valid(&self, token: &str) -> bool {
        let token = token.trim();
        !token.is_empty()
            && self
                .paired_clients
                .iter()
                .any(|client| client.token == token)
    }

    fn persist_client_settings(&self, appearance: &AppearanceSelection) -> Result<(), KernelError> {
        let file = ClientSettingsFile {
            language: appearance.language,
            theme: appearance.theme,
            last_light_theme: appearance.last_light_theme,
            focused_host_id: self.focused_host_id.clone(),
            remote_hosts: self
                .remote_hosts
                .iter()
                .map(pairing::RemoteHost::to_saved)
                .collect(),
            recent_completed_limit: self.recent_limit,
            center_view: self.center_view,
            show_command_preview: self.show_command_preview,
        };
        write_json(&self.data.desktop_client_settings_path, &file)?;
        let secrets = ClientSecretsFile {
            tokens: self
                .remote_hosts
                .iter()
                .map(|host| (host.id.clone(), host.token.clone()))
                .collect(),
        };
        write_json_inner(&self.data.desktop_client_secrets_path, &secrets, true)
    }

    fn connected_hosts(&self) -> Vec<HostSummary> {
        let mut hosts = vec![HostSummary {
            id: LOCAL_HOST_ID.to_string(),
            display_name: self.host_display_name.clone(),
            local: true,
        }];
        hosts.extend(self.remote_hosts.iter().map(|host| HostSummary {
            id: host.id.clone(),
            display_name: host.display_name.clone(),
            local: false,
        }));
        hosts
    }

    fn board_for_focus(&self) -> (Vec<ProjectSummary>, String, Vec<EmptyAction>) {
        if self.focused_host_id != LOCAL_HOST_ID {
            if let Some(view) = &self.remote_view {
                if view.host_id == self.focused_host_id {
                    return (
                        view.projects.clone(),
                        view.focused_project_id.clone(),
                        view.empty_actions.clone(),
                    );
                }
            }
        }
        let empty_actions = if self.projects.is_empty() {
            vec![
                EmptyAction::RegisterFirstProject,
                EmptyAction::PairAnotherHost,
            ]
        } else {
            Vec::new()
        };
        let projects = self
            .projects
            .iter()
            .map(|project| project.summary(self.project_has_active_run(&project.id)))
            .collect();
        let focused_project_id = self.focused_project_id.clone().unwrap_or_default();
        (projects, focused_project_id, empty_actions)
    }

    fn refresh_remote_view(&mut self, host_id: &str) -> Result<(), KernelError> {
        if host_id == LOCAL_HOST_ID {
            self.remote_view = None;
            return Ok(());
        }
        let remote = self
            .remote_hosts
            .iter()
            .find(|host| host.id == host_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown host".into()))?;
        let response = pairing::post_rpc(
            &remote.address,
            Some(&remote.token),
            &serde_json::json!({ "op": "snapshot" }),
        )
        .map_err(|err| match err {
            KernelError::Io(_) | KernelError::Denied(_) => {
                KernelError::Protocol("address is not reachable".into())
            }
            other => other,
        })?;
        let snapshot = response
            .get("snapshot")
            .cloned()
            .ok_or_else(|| KernelError::Protocol("remote Host returned no snapshot".into()))?;
        let projects = snapshot
            .get("projects")
            .cloned()
            .map(serde_json::from_value)
            .transpose()?
            .unwrap_or_default();
        let empty_actions = snapshot
            .get("emptyActions")
            .cloned()
            .map(serde_json::from_value)
            .transpose()?
            .unwrap_or_default();
        let focused_project_id = snapshot
            .get("focusedProjectId")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        let board = match snapshot.get("board") {
            Some(value) if !value.is_null() => serde_json::from_value(value.clone())?,
            _ => None,
        };
        let runs = snapshot
            .get("runs")
            .cloned()
            .map(serde_json::from_value)
            .transpose()?
            .unwrap_or_default();
        let focused_run_id = snapshot
            .get("focusedRunId")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        let quit_offer = match snapshot.get("quitOffer") {
            Some(value) if !value.is_null() => serde_json::from_value(value.clone())?,
            _ => None,
        };
        self.remote_view = Some(RemoteView {
            host_id: host_id.to_string(),
            projects,
            focused_project_id,
            empty_actions,
            board,
            runs,
            focused_run_id,
            quit_offer,
            launch_form: match snapshot.get("launchForm") {
                Some(value) if !value.is_null() => serde_json::from_value(value.clone())?,
                _ => None,
            },
        });
        Ok(())
    }

    fn launch_form_for_focus(&self) -> Option<RunLaunchForm> {
        if self.focused_host_id != LOCAL_HOST_ID {
            if let Some(view) = &self.remote_view {
                if view.host_id == self.focused_host_id {
                    return view.launch_form.clone();
                }
            }
        }
        self.launch_form.clone()
    }

    fn runs_for_focus(&self) -> (Vec<RunSummary>, String, Option<QuitOffer>) {
        if self.focused_host_id != LOCAL_HOST_ID {
            if let Some(view) = &self.remote_view {
                if view.host_id == self.focused_host_id {
                    return (
                        view.runs.clone(),
                        view.focused_run_id.clone(),
                        view.quit_offer.clone(),
                    );
                }
            }
        }
        (
            self.runs.clone(),
            self.focused_run_id.clone().unwrap_or_default(),
            self.quit_offer.clone(),
        )
    }

    fn default_agent_for_project(
        &self,
        project_id: &str,
    ) -> Result<Arc<dyn AgentPort>, KernelError> {
        let project = self
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .ok_or_else(|| KernelError::Protocol("unknown project".into()))?;
        let summaries = launch::summarize_agents(
            &self.agents,
            self.launch_env.as_ref(),
            &project.local_path,
            self.appearance.language,
        );
        let last = self.last_successful_agent.get(project_id).cloned();
        let selected = launch::default_agent_id(&summaries, last.as_deref(), None);
        self.agents
            .iter()
            .find(|agent| agent.id() == selected)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("no Agent Adapter".into()))
    }

    fn prepare_run_launch(
        &mut self,
        project_id: &str,
        issue_id: Option<String>,
        agent_id: Option<String>,
        pick_agent: bool,
    ) -> Result<(), KernelError> {
        let project = self
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown project".into()))?;
        let language = self.appearance.language;
        let agents = launch::summarize_agents(
            &self.agents,
            self.launch_env.as_ref(),
            &project.local_path,
            language,
        );
        let last = self.last_successful_agent.get(project_id).cloned();
        let selected = launch::default_agent_id(&agents, last.as_deref(), agent_id.as_deref());
        let agent = self
            .agents
            .iter()
            .find(|agent| agent.id() == selected)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown Agent Adapter".into()))?;
        let current = self
            .launch_defaults
            .get(project_id)
            .and_then(|agents| agents.get(&selected));
        let other = launch::other_project_memory(&self.launch_defaults, project_id, &selected);
        let (mut values, prefill_source) =
            launch::merge_prefill(&agent.seed_config(), current, other);
        let mut opening_text = String::new();
        if let Some(issue_id) = issue_id.as_deref() {
            if let Some(issue) = self.issue_by_id(issue_id) {
                let instruction = format!("{}\n{}", issue.title, issue.url);
                values.insert(launch::INITIAL_INSTRUCTION.into(), instruction.clone());
                opening_text = instruction;
            }
        }
        let fields = launch::localize_fields(agent.config_fields(), language);
        let preview = launch::command_preview(&launch::preview_argv(agent.as_ref(), &values));
        let skip_agent_picker = !pick_agent && (last.is_some() || agent_id.is_some());
        self.launch_form = Some(RunLaunchForm {
            project_id: project_id.to_string(),
            issue_id,
            agents,
            selected_agent_id: selected,
            skip_agent_picker,
            fields: fields.clone(),
            values: values.clone(),
            prefill_source,
            working_directory: project.local_path.display().to_string(),
            isolation_supported: false,
            isolation_reason: if agent.native_isolation() {
                launch::isolation_reason(language)
            } else {
                agent.isolation_unavailable_reason(language)
            },
            opening_text,
            command_preview: preview,
            intents: launch::intent_options(language),
            warnings: launch::unknown_enum_warnings(&fields, &values, language),
            error: None,
        });
        Ok(())
    }

    fn issue_by_id(&self, issue_id: &str) -> Option<IssueRecord> {
        self.loaded_issues
            .values()
            .flat_map(|issues| issues.iter())
            .find(|issue| issue.id() == issue_id)
            .cloned()
    }

    fn start_unbound_run(
        &mut self,
        project_id: &str,
        config: RunLaunchConfig,
        issue_id: Option<String>,
        from_form: bool,
    ) -> Result<(), KernelError> {
        let cwd = self
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .map(|project| project.local_path.clone())
            .ok_or_else(|| KernelError::Protocol("unknown project".into()))?;
        let agent = self
            .agents
            .iter()
            .find(|agent| agent.id() == config.agent_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown Agent Adapter".into()))?;
        let language = self.appearance.language;
        let fields = launch::localize_fields(agent.config_fields(), language);
        if let Some(form) = &mut self.launch_form {
            launch::apply_submitted_form(form, &config);
            form.warnings = launch::unknown_enum_warnings(&fields, &config.values, language);
            form.command_preview =
                launch::command_preview(&launch::preview_argv(agent.as_ref(), &config.values));
        }
        if from_form {
            if let Some(err) = launch::missing_required(&fields, &config.values, language)
                .or_else(|| launch::opening_required(&config.opening_text, language))
            {
                if let Some(form) = &mut self.launch_form {
                    form.error = Some(err);
                    return Ok(());
                }
                return Err(KernelError::Protocol(err));
            }
        }
        if let Some(issue_id) = issue_id.as_deref() {
            self.require_live_tracker_for_issue(issue_id)?;
        }
        let result = run::start_unbound(
            project_id,
            &cwd,
            agent.as_ref(),
            self.launch_env.as_ref(),
            self.sessions.as_ref(),
            language,
            &[],
            &config,
            issue_id.as_deref(),
        );
        self.focused_run_id = Some(result.record.id.clone());
        self.pending_events.push(HostEvent::RunStatusChanged {
            run_id: result.record.id.clone(),
            status: result.record.status,
        });
        if let Some(session) = result.session {
            self.live.insert(result.record.id.clone(), session);
            self.remember_launch(project_id, &config)?;
            self.launch_form = None;
        } else if let Some(form) = &mut self.launch_form {
            form.error = result.record.failure.clone();
        }
        self.runs.push(result.record);
        Ok(())
    }

    fn remember_launch(
        &mut self,
        project_id: &str,
        config: &RunLaunchConfig,
    ) -> Result<(), KernelError> {
        let remembered = launch::remembered_values(&config.values);
        self.launch_defaults
            .entry(project_id.to_string())
            .or_default()
            .insert(config.agent_id.clone(), remembered);
        self.last_successful_agent
            .insert(project_id.to_string(), config.agent_id.clone());
        self.persist_host_settings()
    }

    fn stop_run(&mut self, run_id: &str) -> Result<(), KernelError> {
        if let Some(session) = self.live.get(run_id).cloned() {
            session.stop();
        }
        if self.runs.iter().any(|run| run.id == run_id) {
            self.mark_run_ended(run_id);
        }
        Ok(())
    }

    fn focus_run(&mut self, run_id: &str) -> Result<(), KernelError> {
        if !self.runs.iter().any(|run| run.id == run_id) {
            return Err(KernelError::Protocol("unknown run".into()));
        }
        self.focused_run_id = Some(run_id.to_string());
        Ok(())
    }

    fn stop_all_runs(&mut self) {
        let ids = self
            .runs
            .iter()
            .filter(|run| run.is_active())
            .map(|run| run.id.clone())
            .collect::<Vec<_>>();
        for id in ids {
            let _ = self.stop_run(&id);
        }
    }

    fn reap_runs(&mut self) {
        let ended = self
            .live
            .iter()
            .filter_map(|(id, session)| session.exit_code().map(|_| id.clone()))
            .collect::<Vec<_>>();
        for id in ended {
            self.mark_run_ended(&id);
        }
    }

    fn mark_run_ended(&mut self, run_id: &str) {
        if let Some(run) = self.runs.iter_mut().find(|run| run.id == run_id) {
            if run.status != RunStatus::Ended {
                run.status = RunStatus::Ended;
                self.pending_events.push(HostEvent::RunStatusChanged {
                    run_id: run_id.to_string(),
                    status: RunStatus::Ended,
                });
            }
        }
        let active = self.active_run_count();
        if active == 0 {
            self.quit_offer = None;
        } else if let Some(offer) = &mut self.quit_offer {
            offer.active_run_count = active;
        }
    }

    fn project_has_active_run(&self, project_id: &str) -> bool {
        self.runs
            .iter()
            .any(|run| run.project_id == project_id && run.is_active())
    }

    fn active_run_count(&self) -> u32 {
        self.runs.iter().filter(|run| run.is_active()).count() as u32
    }

    fn pair_remote_host(&mut self, address: &str, code: &str) -> Result<(), KernelError> {
        let address = pairing::parse_http_url(address).map_err(KernelError::Protocol)?;
        if self.is_own_loopback(&address) {
            return Err(KernelError::Protocol(
                "cannot pair this window to its own Host".into(),
            ));
        }
        let body = serde_json::json!({
            "op": "redeemPairing",
            "code": code,
            "clientName": self.host_display_name,
        });
        let response = pairing::post_rpc(&address, None, &body).map_err(|err| match err {
            KernelError::Io(_) => KernelError::Protocol("address is not reachable".into()),
            other => other,
        })?;
        let pairing = response
            .get("pairing")
            .cloned()
            .ok_or_else(|| KernelError::Denied("invalid pairing code".into()))?;
        let issued: IssuedPairing = serde_json::from_value(pairing)?;
        if issued.token.is_empty() || issued.host_id.is_empty() {
            return Err(KernelError::Denied("invalid pairing code".into()));
        }
        if issued.host_id == LOCAL_HOST_ID {
            return Err(KernelError::Protocol("remote Host id is invalid".into()));
        }
        self.remote_hosts.retain(|host| host.id != issued.host_id);
        self.remote_hosts.push(pairing::RemoteHost {
            id: issued.host_id,
            display_name: issued.display_name,
            address,
            token: issued.token,
        });
        self.persist_client_settings(&self.appearance.clone())
    }

    fn is_own_loopback(&self, address: &str) -> bool {
        if self.loopback_kind != LoopbackKind::Serving {
            return false;
        }
        let rest = address
            .split_once("://")
            .map(|(_, rest)| rest)
            .unwrap_or(address);
        let hostport = rest.split('/').next().unwrap_or(rest);
        let (host, port) = match hostport.rsplit_once(':') {
            Some((host, port)) if port.bytes().all(|byte| byte.is_ascii_digit()) => {
                (host, port.parse::<u16>().ok())
            }
            _ => (hostport, Some(LOCAL_RPC_PORT)),
        };
        let host = host.trim_start_matches('[').trim_end_matches(']');
        matches!(host, "127.0.0.1" | "localhost" | "::1") && port == Some(self.loopback_port)
    }

    fn focus_host(&mut self, host_id: &str) -> Result<(), KernelError> {
        if host_id != LOCAL_HOST_ID && !self.remote_hosts.iter().any(|host| host.id == host_id) {
            return Err(KernelError::Protocol("unknown host".into()));
        }
        self.focused_host_id = host_id.to_string();
        self.refresh_remote_view(host_id)?;
        self.persist_client_settings(&self.appearance.clone())
    }

    fn redeem_pairing(
        &mut self,
        code: &str,
        client_name: &str,
    ) -> Result<IssuedPairing, KernelError> {
        let Some(offer) = &self.pairing_offer else {
            return Err(KernelError::Denied("invalid pairing code".into()));
        };
        if !pairing::codes_match(&offer.code, code) {
            return Err(KernelError::Denied("invalid pairing code".into()));
        }
        let client_name = client_name.trim();
        if client_name.is_empty() {
            return Err(KernelError::Protocol("missing clientName".into()));
        }
        let client = pairing::IssuedClient {
            id: pairing::random_id(),
            name: client_name.to_string(),
            token: pairing::generate_token(),
        };
        let issued = IssuedPairing {
            token: client.token.clone(),
            host_id: self.host_id.clone(),
            display_name: self.host_display_name.clone(),
        };
        self.pairing_offer = None;
        self.paired_clients.push(client);
        self.persist_host_secrets()?;
        Ok(issued)
    }

    fn persist_host_secrets(&self) -> Result<(), KernelError> {
        let github_pats = read_github_pats(&self.data.host_secrets_path)?;
        let file = HostSecretsFile {
            clients: self.paired_clients.clone(),
            github_pats,
        };
        write_json_inner(&self.data.host_secrets_path, &file, true)
    }

    fn persist_host_settings(&self) -> Result<(), KernelError> {
        let file = HostSettingsFile {
            id: self.host_id.clone(),
            focused_project_id: self.focused_project_id.clone(),
            projects: self.projects.iter().map(ProjectRecord::stored).collect(),
            refresh_interval_ms: self.refresh_interval_ms,
            agent_launch_defaults: self.launch_defaults.clone(),
            last_successful_agent: self.last_successful_agent.clone(),
        };
        write_json(&self.data.host_settings_path, &file)
    }

    fn forward_if_remote(
        &mut self,
        request: &serde_json::Value,
    ) -> Result<Option<CommandOutcome>, KernelError> {
        if self.focused_host_id == LOCAL_HOST_ID {
            return Ok(None);
        }
        let remote = self
            .remote_hosts
            .iter()
            .find(|host| host.id == self.focused_host_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown host".into()))?;
        let response =
            pairing::post_rpc(&remote.address, Some(&remote.token), request).map_err(|err| {
                match err {
                    KernelError::Io(_) | KernelError::Denied(_) => {
                        KernelError::Protocol("address is not reachable".into())
                    }
                    other => other,
                }
            })?;
        let host_id = self.focused_host_id.clone();
        self.refresh_remote_view(&host_id)?;
        let mut outcome = self.outcome();
        if let Some(inference) = response.get("inference").cloned() {
            outcome.inference = serde_json::from_value(inference).ok();
        }
        Ok(Some(outcome))
    }

    fn register_project(
        &mut self,
        name: &str,
        local_path: &str,
        github_host: &str,
        repository: &str,
    ) -> Result<(), KernelError> {
        let name = project::require_name(name).map_err(KernelError::Protocol)?;
        let local_path =
            project::require_local_directory(local_path).map_err(KernelError::Protocol)?;
        let github_host =
            project::normalize_github_host(github_host).map_err(KernelError::Protocol)?;
        let repository =
            project::normalize_repository(repository).map_err(KernelError::Protocol)?;
        if self
            .projects
            .iter()
            .any(|project| project.local_path == local_path)
        {
            return Err(KernelError::Protocol(
                "a Project is already registered for this directory".into(),
            ));
        }
        let connection = self.probe_github(&github_host, &repository);
        let record = ProjectRecord {
            id: pairing::random_id(),
            name,
            local_path,
            github_host,
            repository,
            connection,
            tracker_synced: false,
        };
        let project_id = record.id.clone();
        self.focused_project_id = Some(project_id.clone());
        self.projects.push(record);
        self.selected_issue_id = None;
        self.parent_filter = None;
        self.refresh_project(&project_id, RefreshTrigger::Immediate);
        self.persist_host_settings()
    }

    fn edit_project(
        &mut self,
        project_id: &str,
        name: &str,
        local_path: &str,
        github_host: &str,
        repository: &str,
    ) -> Result<(), KernelError> {
        let name = project::require_name(name).map_err(KernelError::Protocol)?;
        let local_path =
            project::require_local_directory(local_path).map_err(KernelError::Protocol)?;
        let github_host =
            project::normalize_github_host(github_host).map_err(KernelError::Protocol)?;
        let repository =
            project::normalize_repository(repository).map_err(KernelError::Protocol)?;
        if self
            .projects
            .iter()
            .any(|project| project.id != project_id && project.local_path == local_path)
        {
            return Err(KernelError::Protocol(
                "a Project is already registered for this directory".into(),
            ));
        }
        let connection = self.probe_github(&github_host, &repository);
        let project = self
            .projects
            .iter_mut()
            .find(|project| project.id == project_id)
            .ok_or_else(|| KernelError::Protocol("unknown project".into()))?;
        project.name = name;
        project.local_path = local_path;
        project.github_host = github_host;
        project.repository = repository;
        project.connection = connection;
        project.tracker_synced = false;
        self.loaded_issues.remove(project_id);
        self.refresh.remove(project_id);
        refresh::remove_project_data(&self.data.host_dir, project_id);
        if self.focused_project_id.as_deref() == Some(project_id) {
            self.selected_issue_id = None;
            self.parent_filter = None;
            self.refresh_project(project_id, RefreshTrigger::Immediate);
        }
        self.persist_host_settings()
    }

    fn remove_project(&mut self, project_id: &str) -> Result<(), KernelError> {
        let index = self
            .projects
            .iter()
            .position(|project| project.id == project_id)
            .ok_or_else(|| KernelError::Protocol("unknown project".into()))?;
        if self.project_has_active_run(project_id) {
            return Err(KernelError::Denied(
                "cannot remove a Project with an active Run".into(),
            ));
        }
        let was_current = self.focused_project_id.as_deref() == Some(project_id);
        let _removed = self.projects.remove(index);
        self.refresh.remove(project_id);
        self.loaded_issues.remove(project_id);
        refresh::remove_project_data(&self.data.host_dir, project_id);
        if was_current {
            self.selected_issue_id = None;
            self.parent_filter = None;
            self.focused_project_id = if self.projects.is_empty() {
                None
            } else {
                Some(self.projects[index.min(self.projects.len() - 1)].id.clone())
            };
            if let Some(next_id) = self.focused_project_id.clone() {
                self.refresh_project(&next_id, RefreshTrigger::Immediate);
            }
        }
        self.persist_host_settings()
    }

    fn focus_project(&mut self, project_id: &str) -> Result<(), KernelError> {
        let Some(index) = self
            .projects
            .iter()
            .position(|project| project.id == project_id)
        else {
            return Err(KernelError::Protocol("unknown project".into()));
        };
        let host = self.projects[index].github_host.clone();
        let repository = self.projects[index].repository.clone();
        let connection = self.probe_github(&host, &repository);
        self.projects[index].connection = connection;
        self.focused_project_id = Some(project_id.to_string());
        self.selected_issue_id = None;
        self.parent_filter = None;
        self.refresh_project(project_id, RefreshTrigger::Immediate);
        self.persist_host_settings()
    }

    fn infer_project(&self, local_path: &str) -> Result<Option<ProjectInference>, KernelError> {
        let local_path =
            project::require_local_directory(local_path).map_err(KernelError::Protocol)?;
        Ok(project::infer_github_project(&local_path))
    }

    fn current_board(&self, focused_project_id: &str) -> Option<BoardSnapshot> {
        if focused_project_id.is_empty() {
            return None;
        }
        if self.focused_host_id != LOCAL_HOST_ID {
            return self.remote_view.as_ref().and_then(|view| {
                if view.host_id == self.focused_host_id {
                    view.board.clone()
                } else {
                    None
                }
            });
        }
        let loaded = self
            .loaded_issues
            .get(focused_project_id)
            .map(Vec::as_slice);
        Some(board::project_board(
            focused_project_id,
            loaded,
            self.parent_filter.as_deref(),
            self.selected_issue_id.as_deref(),
            self.recent_limit,
            self.refresh_status_for(focused_project_id),
            self.show_closed_graph_context,
        ))
    }

    fn load_persisted_snapshot(&mut self, project_id: &str) {
        let Some(stored) =
            refresh::load_snapshot(&refresh::snapshot_path(&self.data.host_dir, project_id))
        else {
            return;
        };
        self.loaded_issues
            .insert(project_id.to_string(), stored.issues);
        if let Some(project) = self
            .projects
            .iter_mut()
            .find(|project| project.id == project_id)
        {
            project.tracker_synced = true;
        }
        self.refresh.insert(
            project_id.to_string(),
            ProjectRefreshState {
                fetched_at_ms: Some(stored.fetched_at_ms),
                last_attempt_ms: stored.fetched_at_ms,
                kind: StoredRefreshKind::Ready,
                retry_at_ms: None,
            },
        );
    }

    fn refresh_project(&mut self, project_id: &str, _trigger: RefreshTrigger) -> bool {
        let Some(index) = self
            .projects
            .iter()
            .position(|project| project.id == project_id)
        else {
            return false;
        };
        let previous_fetched = self
            .refresh
            .get(project_id)
            .and_then(|state| state.fetched_at_ms);
        self.pending_events.push(HostEvent::RefreshStatusChanged {
            project_id: project_id.to_string(),
            status: RefreshStatus::Refreshing {
                fetched_at_ms: previous_fetched,
            },
        });
        let github_host = self.projects[index].github_host.clone();
        let repository = self.projects[index].repository.clone();
        let pat = read_github_pat(&self.data.host_secrets_path, &github_host);
        let now = self.now_ms;
        let result = self.tracker.read_issues(&tracker::ProbeContext {
            github_host: &github_host,
            repository: &repository,
            secrets_pat: pat.as_deref(),
            secrets_path: &self.data.host_secrets_path,
        });
        match result {
            Ok(issues) => {
                if !matches!(
                    self.projects[index].connection,
                    ProjectConnection::Ready { .. }
                ) {
                    self.projects[index].connection = self.probe_github(&github_host, &repository);
                }
                self.projects[index].tracker_synced = true;
                let _ = refresh::save_snapshot(
                    &refresh::snapshot_path(&self.data.host_dir, project_id),
                    &refresh::StoredTrackerSnapshot {
                        fetched_at_ms: now,
                        issues: issues.clone(),
                    },
                );
                self.loaded_issues.insert(project_id.to_string(), issues);
                self.refresh.insert(
                    project_id.to_string(),
                    ProjectRefreshState {
                        fetched_at_ms: Some(now),
                        last_attempt_ms: now,
                        kind: StoredRefreshKind::Ready,
                        retry_at_ms: None,
                    },
                );
                let status = self.refresh_status_for(project_id);
                self.pending_events.push(HostEvent::RefreshStatusChanged {
                    project_id: project_id.to_string(),
                    status,
                });
                self.pending_events.push(HostEvent::BoardUpdated {
                    project_id: project_id.to_string(),
                });
                true
            }
            Err(tracker::TrackerReadError::RateLimited { retry_after_ms }) => {
                self.refresh.insert(
                    project_id.to_string(),
                    ProjectRefreshState {
                        fetched_at_ms: previous_fetched,
                        last_attempt_ms: now,
                        kind: StoredRefreshKind::RateLimited,
                        retry_at_ms: retry_after_ms.map(|ms| now.saturating_add(ms)),
                    },
                );
                let status = self.refresh_status_for(project_id);
                self.pending_events.push(HostEvent::RefreshStatusChanged {
                    project_id: project_id.to_string(),
                    status,
                });
                false
            }
            Err(tracker::TrackerReadError::Offline {
                source,
                cli_detected,
                detail,
            }) => {
                self.projects[index].connection = ProjectConnection::Unreachable {
                    source,
                    repair: tracker::repair_hint(cli_detected, &self.data.host_secrets_path),
                    message: auth_failure_message(
                        self.appearance.language,
                        AuthFailureKind::Unreachable,
                        detail.as_deref(),
                    ),
                };
                let has_data = self.loaded_issues.contains_key(project_id);
                self.refresh.insert(
                    project_id.to_string(),
                    ProjectRefreshState {
                        fetched_at_ms: previous_fetched,
                        last_attempt_ms: now,
                        kind: if has_data {
                            StoredRefreshKind::Offline
                        } else {
                            StoredRefreshKind::NeverFetched
                        },
                        retry_at_ms: None,
                    },
                );
                let status = self.refresh_status_for(project_id);
                self.pending_events.push(HostEvent::RefreshStatusChanged {
                    project_id: project_id.to_string(),
                    status,
                });
                false
            }
            Err(tracker::TrackerReadError::Auth {
                source,
                kind,
                cli_detected,
                detail,
            }) => {
                self.projects[index].connection = ProjectConnection::AuthFailed {
                    source,
                    kind,
                    repair: tracker::repair_hint(cli_detected, &self.data.host_secrets_path),
                    message: auth_failure_message(
                        self.appearance.language,
                        kind,
                        detail.as_deref(),
                    ),
                };
                self.refresh.insert(
                    project_id.to_string(),
                    ProjectRefreshState {
                        fetched_at_ms: previous_fetched,
                        last_attempt_ms: now,
                        kind: StoredRefreshKind::AuthFailed,
                        retry_at_ms: None,
                    },
                );
                let status = self.refresh_status_for(project_id);
                self.pending_events.push(HostEvent::RefreshStatusChanged {
                    project_id: project_id.to_string(),
                    status,
                });
                false
            }
        }
    }

    fn maybe_auto_refresh(&mut self) {
        let due: Vec<String> = self
            .projects
            .iter()
            .map(|project| project.id.clone())
            .filter(|id| self.should_auto_refresh(id))
            .collect();
        for project_id in due {
            self.refresh_project(&project_id, RefreshTrigger::Interval);
        }
    }

    fn should_auto_refresh(&self, project_id: &str) -> bool {
        if !self.project_watched(project_id) {
            return false;
        }
        let Some(state) = self.refresh.get(project_id) else {
            return true;
        };
        match state.kind {
            StoredRefreshKind::RateLimited => state
                .retry_at_ms
                .is_some_and(|retry_at| self.now_ms >= retry_at),
            StoredRefreshKind::AuthFailed => false,
            StoredRefreshKind::Ready
            | StoredRefreshKind::Offline
            | StoredRefreshKind::NeverFetched => {
                self.now_ms
                    >= state
                        .last_attempt_ms
                        .saturating_add(self.refresh_interval_ms)
            }
        }
    }

    fn project_watched(&self, project_id: &str) -> bool {
        if self.window_visible
            && self.focused_host_id == LOCAL_HOST_ID
            && self.focused_project_id.as_deref() == Some(project_id)
        {
            return true;
        }
        self.client_views
            .values()
            .any(|view| view.visible && view.project_id == project_id)
    }

    fn refresh_status_for(&self, project_id: &str) -> RefreshStatus {
        let Some(state) = self.refresh.get(project_id) else {
            return RefreshStatus::NeverFetched;
        };
        let next = self.next_refresh_in_ms(project_id, state);
        match state.kind {
            StoredRefreshKind::Ready => match state.fetched_at_ms {
                Some(fetched_at_ms) => RefreshStatus::Ready {
                    fetched_at_ms,
                    next_refresh_in_ms: next,
                },
                None => RefreshStatus::NeverFetched,
            },
            StoredRefreshKind::Offline => match state.fetched_at_ms {
                Some(fetched_at_ms) => RefreshStatus::Offline {
                    fetched_at_ms,
                    next_refresh_in_ms: next,
                },
                None => RefreshStatus::NeverFetched,
            },
            StoredRefreshKind::NeverFetched => RefreshStatus::NeverFetched,
            StoredRefreshKind::RateLimited => RefreshStatus::RateLimited {
                fetched_at_ms: state.fetched_at_ms,
                retry_at_ms: state.retry_at_ms,
            },
            StoredRefreshKind::AuthFailed => RefreshStatus::AuthFailed {
                fetched_at_ms: state.fetched_at_ms,
            },
        }
    }

    fn next_refresh_in_ms(&self, project_id: &str, state: &ProjectRefreshState) -> Option<u64> {
        if !self.project_watched(project_id) {
            return None;
        }
        match state.kind {
            StoredRefreshKind::RateLimited => state
                .retry_at_ms
                .map(|retry_at| retry_at.saturating_sub(self.now_ms)),
            StoredRefreshKind::Ready | StoredRefreshKind::Offline => Some(
                (state
                    .last_attempt_ms
                    .saturating_add(self.refresh_interval_ms))
                .saturating_sub(self.now_ms),
            ),
            StoredRefreshKind::NeverFetched | StoredRefreshKind::AuthFailed => None,
        }
    }

    fn set_client_view(&mut self, client_id: &str, project_id: &str, visible: bool) {
        if !visible || project_id.is_empty() {
            self.client_views.remove(client_id);
            return;
        }
        let previous = self.client_views.insert(
            client_id.to_string(),
            ClientView {
                project_id: project_id.to_string(),
                visible: true,
            },
        );
        let changed = previous
            .map(|view| !view.visible || view.project_id != project_id)
            .unwrap_or(true);
        if changed {
            self.refresh_project(project_id, RefreshTrigger::Immediate);
        }
    }

    fn require_live_tracker(&mut self, project_id: &str) -> Result<(), KernelError> {
        if !self.projects.iter().any(|project| project.id == project_id) {
            return Err(KernelError::Protocol("unknown project".into()));
        }
        if self.refresh_project(project_id, RefreshTrigger::Action) {
            Ok(())
        } else {
            Err(KernelError::Denied(self.write_block_reason(project_id)))
        }
    }

    fn require_live_tracker_for_issue(&mut self, issue_id: &str) -> Result<(), KernelError> {
        let project_id = match self.project_id_for_issue(issue_id) {
            Ok(project_id) => project_id,
            Err(_) => self
                .focused_project_id
                .clone()
                .ok_or_else(|| KernelError::Protocol("unknown issue".into()))?,
        };
        self.require_live_tracker(&project_id)
    }

    fn project_id_for_issue(&self, issue_id: &str) -> Result<String, KernelError> {
        self.loaded_issues
            .iter()
            .find_map(|(project_id, issues)| {
                issues
                    .iter()
                    .any(|issue| issue.id() == issue_id)
                    .then(|| project_id.clone())
            })
            .ok_or_else(|| KernelError::Protocol("unknown issue".into()))
    }

    fn write_block_reason(&self, project_id: &str) -> String {
        match self.refresh.get(project_id).map(|state| state.kind) {
            Some(StoredRefreshKind::RateLimited) => "cannot write to tracker: rate-limited".into(),
            Some(StoredRefreshKind::AuthFailed) => "cannot write to tracker: auth-failed".into(),
            Some(StoredRefreshKind::NeverFetched) | None => {
                "cannot write to tracker: never-fetched".into()
            }
            _ => "cannot write to tracker: offline".into(),
        }
    }

    fn probe_github(&self, github_host: &str, repository: &str) -> ProjectConnection {
        let pat = read_github_pat(&self.data.host_secrets_path, github_host);
        let outcome = self.tracker.probe(&tracker::ProbeContext {
            github_host,
            repository,
            secrets_pat: pat.as_deref(),
            secrets_path: &self.data.host_secrets_path,
        });
        connection_from_probe(
            outcome,
            &self.data.host_secrets_path,
            self.appearance.language,
        )
    }
}

impl DataLayout {
    fn prepare(app_local_data_dir: &Path, app_log_dir: &Path) -> Result<Self, KernelError> {
        let host_dir = app_local_data_dir.join("host");
        let desktop_client_dir = app_local_data_dir.join("desktop-client");
        fs::create_dir_all(&host_dir)?;
        fs::create_dir_all(&desktop_client_dir)?;
        fs::create_dir_all(app_log_dir)?;
        Ok(Self {
            host_dir: host_dir.clone(),
            desktop_client_dir: desktop_client_dir.clone(),
            host_settings_path: host_dir.join("settings.json"),
            host_secrets_path: host_dir.join("secrets.json"),
            desktop_client_settings_path: desktop_client_dir.join("settings.json"),
            desktop_client_secrets_path: desktop_client_dir.join("secrets.json"),
            log_dir: app_log_dir.to_path_buf(),
        })
    }
}

impl ShellCopy {
    fn for_language(language: Language) -> Self {
        match language {
            Language::ZhCn => Self {
                app_name: "Agent Taskboard".into(),
                register_first_project: "登记第一个 Project".into(),
                pair_another_host: "配对另一个 Host".into(),
                no_project_title: "这台 Host 上还没有 Project".into(),
                no_project_body: "先登记一个本地目录，并选好 Issue Tracker。".into(),
                quit_host: "退出 Host".into(),
                show_window: "打开窗口".into(),
                settings: "设置".into(),
                language: "界面语言".into(),
                theme: "主题".into(),
                language_zh: "简体中文".into(),
                language_en: "English".into(),
                theme_warm_paper: "暖纸".into(),
                theme_plain_paper: "素纸".into(),
                theme_plain_night: "素纸夜间".into(),
                hosts: "Host".into(),
                projects: "Project".into(),
                this_machine: "本机".into(),
                shade_light: "浅".into(),
                shade_dark: "深".into(),
                edit_menu: "编辑".into(),
                pairing_required: "经 Tailscale、局域网或其它站点访问需要长期令牌。本机回环页 http://127.0.0.1:10529/ 免配对。".into(),
                pairing_title: "配对".into(),
                pairing_this_host: "让别人连这台".into(),
                pairing_to_another: "连到另一台 Host".into(),
                pairing_address: "可到达地址".into(),
                pairing_show: "出示配对码".into(),
                pairing_copy: "复制".into(),
                pairing_same_payload: "二维码和复制文本是同一份信息。连通走你自己的 Tailscale、局域网或 VPN，没有产品中继。".into(),
                pairing_paste: "粘贴配对信息".into(),
                pairing_connect: "连接".into(),
                paired_clients: "已配对的 Client".into(),
                revoke_client: "撤销".into(),
                no_paired_clients: "还没有已配对的 Client。".into(),
                add_project: "登记 Project".into(),
                edit_project: "编辑登记…".into(),
                remove_project: "移除 Project…".into(),
                register_project_title: "登记 Project".into(),
                edit_project_title: "编辑 Project 登记".into(),
                display_name: "显示名称".into(),
                local_directory: "本地目录".into(),
                github_host: "GitHub host".into(),
                repository: "仓库".into(),
                infer_from_directory: "从本地目录推断".into(),
                use_inference: "使用这份推断结果".into(),
                inference_hint: "Host 可以从所填目录推断 git remote。推断结果不会自动写入，必须由你确认采用。".into(),
                save_registration: "保存登记".into(),
                cancel: "取消".into(),
                remove_confirm_title: "移除这个 Project？".into(),
                remove_confirm_body: "只取消这台 Host 上的登记。不会删除本地目录、git 仓库，也不会删除远端 Issue。".into(),
                remove_confirm: "只移除登记".into(),
                cannot_remove_active_run: "现在不能移除".into(),
                cannot_remove_active_run_body: "这个 Project 有活跃 Run。先停止或结束 Run，再回来移除。关闭 Client 或切换 Project 都不会停止 Run。".into(),
                got_it: "知道了".into(),
                auth_failed: "这个 Project 的 GitHub 凭据不可用。".into(),
                repair_cli: "用 gh 登录".into(),
                repair_secrets: "在 Host 秘密文件里写入这个 host 的 PAT".into(),
                repair_env: "设置应用专用或通用环境变量".into(),
                no_gh_detected: "这台电脑上没检测到 gh。".into(),
                connection_ready: "GitHub 已连通".into(),
                project_menu: "管理".into(),
                board_hint: "从左到右：阻塞中 → Frontier → 进行中 → 最近完成。不能拖列关票。".into(),
                child_hint: "只看这些直接子票。仍是看板视图，不是第二种 Frontier。".into(),
                graph_hint: "只画 Dependency，不画父子。点节点只换详情。".into(),
                view_board: "看板".into(),
                view_graph: "依赖图".into(),
                show_closed_context: "也显示已关闭上下文".into(),
                clear_filter: "清除过滤".into(),
                col_blocked: "阻塞中".into(),
                col_frontier: "Frontier".into(),
                col_in_progress: "进行中".into(),
                col_recent: "最近完成".into(),
                no_items: "没有".into(),
                no_frontier_blocked: "没有可领的票。剩下的都还被挡住。".into(),
                no_frontier_claimed: "没有可领的票。未关的都已被认领。".into(),
                no_frontier_empty: "没有可领的票。这个 Project 里没有未关的票。".into(),
                no_recent: "还没有刚关的".into(),
                recent_note: "只留最近几张，不是全部已关闭。不能拖进这一列来关票。".into(),
                empty_no_data: "还没有可显示的数据。".into(),
                family: "属于 / 子票".into(),
                deps: "挡住它的 / 它挡住的".into(),
                parent: "属于".into(),
                children: "子票".into(),
                no_parent: "没有父，仍是一等 Issue。".into(),
                no_kids: "没有子票".into(),
                only_kids: "只看这些子票".into(),
                blocked_by: "挡住它的".into(),
                blocking: "它挡住的".into(),
                none_block: "无，可进 Frontier".into(),
                none: "无".into(),
                claimed: "已认领".into(),
                unclaimed: "未认领".into(),
                pick_issue: "点一张 Issue".into(),
                recent_limit: "最近完成列张数".into(),
                recent_limit_help: "默认 5。只影响最右那一列。不能拖进这一列来关票。".into(),
                unclear_issue: "对端看不清".into(),
                refresh_now: "刷新".into(),
                refresh_refreshing: "正在刷新".into(),
                refresh_as_of: "数据截至".into(),
                refresh_next: "下次刷新".into(),
                refresh_offline: "已离线".into(),
                refresh_never: "还没有成功拉过 Tracker。".into(),
                refresh_rate_limited: "已被限流".into(),
                refresh_retry: "大约可再刷新".into(),
                refresh_paused: "自动刷新已暂停，可手动再试。".into(),
                refresh_auth: "凭据不可用".into(),
                new_run: "新建".into(),
                execute_run: "执行".into(),
                start_run: "启动".into(),
                switch_agent: "换一家".into(),
                pick_agent: "选择 Agent".into(),
                launch_title: "启动配置".into(),
                prefill_current: "预填来自这个 Project 上次成功启动，本次可改。".into(),
                prefill_other: "预填来自其它 Project 上这家 Agent 的记忆，本次可改。".into(),
                prefill_seed: "预填来自本机 CLI 种子，本次可改。".into(),
                isolation: "隔离执行目录".into(),
                isolation_off_reason: "本票先关掉隔离，留给隔离票。".into(),
                isolation_hint: "机制是 git worktree。默认关，不记住。".into(),
                run_intent: "Run 意图".into(),
                intent_none: "不选".into(),
                intent_modify: "修改".into(),
                intent_continue: "继续".into(),
                intent_answer: "只回答".into(),
                intent_review: "复查".into(),
                intent_custom: "自定义".into(),
                opening_placeholder: "要 Agent 做什么".into(),
                folded_options: "附加参数与预览".into(),
                command_preview: "命令预览".into(),
                show_command_preview: "显示命令预览".into(),
                instruction_required: "请填写要 Agent 做什么。".into(),
                working_directory: "执行目录".into(),
                unbound_issue: "未绑定 Issue".into(),
                stop_run: "停止".into(),
                quit_active_title: "还有活跃 Run".into(),
                quit_active_body: "退出 Host 会停掉全部活跃 Run。选择返回则继续跑。".into(),
                quit_return: "返回".into(),
                quit_stop_all: "停掉全部".into(),
            },
            Language::En => Self {
                app_name: "Agent Taskboard".into(),
                register_first_project: "Register the first Project".into(),
                pair_another_host: "Pair another Host".into(),
                no_project_title: "No Project on this Host yet".into(),
                no_project_body: "Register a local folder and pick an Issue Tracker.".into(),
                quit_host: "Quit Host".into(),
                show_window: "Open window".into(),
                settings: "Settings".into(),
                language: "Interface language".into(),
                theme: "Theme".into(),
                language_zh: "简体中文".into(),
                language_en: "English".into(),
                theme_warm_paper: "Warm paper".into(),
                theme_plain_paper: "Plain paper".into(),
                theme_plain_night: "Plain night".into(),
                hosts: "Host".into(),
                projects: "Project".into(),
                this_machine: "This machine".into(),
                shade_light: "Light".into(),
                shade_dark: "Dark".into(),
                edit_menu: "Edit".into(),
                pairing_required: "Access via Tailscale, LAN, or another site needs a long-term token. The loopback page at http://127.0.0.1:10529/ does not require pairing.".into(),
                pairing_title: "Pairing".into(),
                pairing_this_host: "Let others join this Host".into(),
                pairing_to_another: "Connect to another Host".into(),
                pairing_address: "Reachable address".into(),
                pairing_show: "Show pairing code".into(),
                pairing_copy: "Copy".into(),
                pairing_same_payload: "The QR code and the copyable text are the same payload. Reach this Host on your own Tailscale, LAN, or VPN — there is no product relay.".into(),
                pairing_paste: "Paste pairing payload".into(),
                pairing_connect: "Connect".into(),
                paired_clients: "Paired clients".into(),
                revoke_client: "Revoke".into(),
                no_paired_clients: "No paired clients yet.".into(),
                add_project: "Register Project".into(),
                edit_project: "Edit registration…".into(),
                remove_project: "Remove Project…".into(),
                register_project_title: "Register Project".into(),
                edit_project_title: "Edit Project registration".into(),
                display_name: "Display name".into(),
                local_directory: "Local directory".into(),
                github_host: "GitHub host".into(),
                repository: "Repository".into(),
                infer_from_directory: "Infer from local directory".into(),
                use_inference: "Use this inference".into(),
                inference_hint: "The Host can infer a git remote from the folder. Inference is only a candidate until you confirm it.".into(),
                save_registration: "Save registration".into(),
                cancel: "Cancel".into(),
                remove_confirm_title: "Remove this Project?".into(),
                remove_confirm_body: "This only unregisters it on this Host. It does not delete the local directory, git repository, or remote Issues.".into(),
                remove_confirm: "Remove registration only".into(),
                cannot_remove_active_run: "Cannot remove now".into(),
                cannot_remove_active_run_body: "This Project has an active Run. Stop or finish the Run first. Closing a Client or switching Project does not stop the Run.".into(),
                got_it: "Got it".into(),
                auth_failed: "GitHub credentials for this Project are not available.".into(),
                repair_cli: "Sign in with gh".into(),
                repair_secrets: "Write a PAT for this host in the Host secrets file".into(),
                repair_env: "Set the app-specific or generic environment variable".into(),
                no_gh_detected: "gh was not detected on this machine.".into(),
                connection_ready: "GitHub is connected".into(),
                project_menu: "Manage".into(),
                board_hint: "Blocked → Frontier → In progress → Recently closed. Closing is not drag.".into(),
                child_hint: "Direct children only. Still a board, not a second Frontier.".into(),
                graph_hint: "Dependencies only — not parent/child. Click a node to change details.".into(),
                view_board: "Board".into(),
                view_graph: "Dependency graph".into(),
                show_closed_context: "Also show closed context".into(),
                clear_filter: "Clear filter".into(),
                col_blocked: "Blocked".into(),
                col_frontier: "Frontier".into(),
                col_in_progress: "In progress".into(),
                col_recent: "Recently closed".into(),
                no_items: "None".into(),
                no_frontier_blocked: "Nothing to claim. The rest are still blocked.".into(),
                no_frontier_claimed: "Nothing to claim. Open issues are already claimed.".into(),
                no_frontier_empty: "Nothing to claim. This Project has no open issues.".into(),
                no_recent: "None just closed".into(),
                recent_note: "Only the last few, not all closed issues. Dragging here does not close.".into(),
                empty_no_data: "No displayable data yet.".into(),
                family: "Parent / children".into(),
                deps: "Blocked by / blocking".into(),
                parent: "Parent".into(),
                children: "Children".into(),
                no_parent: "No parent. Still a first-class Issue.".into(),
                no_kids: "No children".into(),
                only_kids: "Only these children".into(),
                blocked_by: "Blocked by".into(),
                blocking: "Blocking".into(),
                none_block: "None — can enter Frontier".into(),
                none: "None".into(),
                claimed: "Claimed".into(),
                unclaimed: "Unclaimed".into(),
                pick_issue: "Select an Issue".into(),
                recent_limit: "Recently-closed count".into(),
                recent_limit_help: "Default 5. Only the rightmost column. Dragging here does not close.".into(),
                unclear_issue: "The other side is unclear".into(),
                refresh_now: "Refresh".into(),
                refresh_refreshing: "Refreshing".into(),
                refresh_as_of: "Data as of".into(),
                refresh_next: "Next refresh".into(),
                refresh_offline: "Offline".into(),
                refresh_never: "Tracker has never been fetched successfully.".into(),
                refresh_rate_limited: "Rate limited".into(),
                refresh_retry: "Can retry around".into(),
                refresh_paused: "Auto-refresh is paused. You can try again manually.".into(),
                refresh_auth: "Credentials unavailable".into(),
                new_run: "New".into(),
                execute_run: "Run".into(),
                start_run: "Start".into(),
                switch_agent: "Switch Agent".into(),
                pick_agent: "Choose Agent".into(),
                launch_title: "Launch".into(),
                prefill_current: "Prefill is this Project's last successful launch. You can change it this time.".into(),
                prefill_other: "Prefill is this Agent's memory from another Project. You can change it this time.".into(),
                prefill_seed: "Prefill is the local CLI seed. You can change it this time.".into(),
                isolation: "Isolated work directory".into(),
                isolation_off_reason: "Isolation is disabled on this ticket.".into(),
                isolation_hint: "This uses a git worktree. It stays off and is not remembered.".into(),
                run_intent: "Run intent".into(),
                intent_none: "None".into(),
                intent_modify: "Modify".into(),
                intent_continue: "Continue".into(),
                intent_answer: "Answer only".into(),
                intent_review: "Review".into(),
                intent_custom: "Custom".into(),
                opening_placeholder: "What should the Agent do".into(),
                folded_options: "Extra options".into(),
                command_preview: "Command preview".into(),
                show_command_preview: "Show command preview".into(),
                instruction_required: "Tell the Agent what to do.".into(),
                working_directory: "Working directory".into(),
                unbound_issue: "Unbound Issue".into(),
                stop_run: "Stop".into(),
                quit_active_title: "Active Runs still running".into(),
                quit_active_body: "Quitting Host will stop every active Run. Go back to keep them running.".into(),
                quit_return: "Go back".into(),
                quit_stop_all: "Stop all".into(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HostSettingsFile {
    #[serde(default)]
    id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    focused_project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    projects: Vec<StoredProject>,
    #[serde(default = "default_refresh_interval_ms")]
    refresh_interval_ms: u64,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    agent_launch_defaults: BTreeMap<String, BTreeMap<String, BTreeMap<String, String>>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    last_successful_agent: BTreeMap<String, String>,
}

impl Default for HostSettingsFile {
    fn default() -> Self {
        Self {
            id: String::new(),
            focused_project_id: None,
            projects: Vec::new(),
            refresh_interval_ms: refresh::DEFAULT_REFRESH_INTERVAL_MS,
            agent_launch_defaults: BTreeMap::new(),
            last_successful_agent: BTreeMap::new(),
        }
    }
}

fn default_refresh_interval_ms() -> u64 {
    refresh::DEFAULT_REFRESH_INTERVAL_MS
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredProject {
    id: String,
    name: String,
    local_path: PathBuf,
    github_host: String,
    repository: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct HostSecretsFile {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    clients: Vec<pairing::IssuedClient>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    github_pats: BTreeMap<String, String>,
}

fn load_or_init_host_settings(path: &Path) -> Result<HostSettingsFile, KernelError> {
    let mut file = if path.exists() {
        serde_json::from_str(&fs::read_to_string(path)?)?
    } else {
        HostSettingsFile::default()
    };
    if file.id.trim().is_empty() {
        file.id = pairing::random_id();
        write_json(path, &file)?;
    }
    Ok(file)
}

fn load_paired_clients(path: &Path) -> Result<Vec<pairing::IssuedClient>, KernelError> {
    if !path.exists() {
        write_json_inner(path, &HostSecretsFile::default(), true)?;
        return Ok(Vec::new());
    }
    owner::restrict_to_owner(path)?;
    let raw = fs::read_to_string(path)?;
    let file = serde_json::from_str::<HostSecretsFile>(&raw).unwrap_or_default();
    Ok(file.clients)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClientSettingsFile {
    language: Language,
    theme: Theme,
    last_light_theme: Theme,
    #[serde(default = "local_host_id")]
    focused_host_id: String,
    #[serde(default)]
    remote_hosts: Vec<pairing::SavedRemoteHost>,
    #[serde(default = "default_recent_limit")]
    recent_completed_limit: u32,
    #[serde(default)]
    center_view: CenterView,
    #[serde(default = "default_true")]
    show_command_preview: bool,
}

fn default_true() -> bool {
    true
}

fn default_recent_limit() -> u32 {
    board::DEFAULT_RECENT_LIMIT
}

fn local_host_id() -> String {
    LOCAL_HOST_ID.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ClientSecretsFile {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    tokens: BTreeMap<String, String>,
}

fn load_or_init_appearance(
    path: &Path,
    system_locale: &str,
    system_appearance: SystemAppearance,
) -> Result<
    (
        AppearanceSelection,
        String,
        Vec<pairing::SavedRemoteHost>,
        u32,
        CenterView,
        bool,
    ),
    KernelError,
> {
    if path.exists() {
        let raw = fs::read_to_string(path)?;
        if let Ok(mut file) = serde_json::from_str::<ClientSettingsFile>(&raw) {
            file.last_light_theme = daytime_theme(file.last_light_theme);
            return Ok((
                AppearanceSelection {
                    language: file.language,
                    theme: file.theme,
                    last_light_theme: file.last_light_theme,
                },
                file.focused_host_id,
                file.remote_hosts,
                board::clamp_recent_limit(file.recent_completed_limit),
                file.center_view,
                file.show_command_preview,
            ));
        }
        if let Ok(mut file) = serde_json::from_str::<AppearanceSelection>(&raw) {
            file.last_light_theme = daytime_theme(file.last_light_theme);
            return Ok((
                file,
                LOCAL_HOST_ID.to_string(),
                Vec::new(),
                board::DEFAULT_RECENT_LIMIT,
                CenterView::Board,
                true,
            ));
        }
    }
    let language = match_language(system_locale);
    let theme = match system_appearance {
        SystemAppearance::Light => Theme::WarmPaper,
        SystemAppearance::Dark => Theme::PlainNight,
    };
    let appearance = AppearanceSelection {
        language,
        theme,
        last_light_theme: Theme::WarmPaper,
    };
    let file = ClientSettingsFile {
        language,
        theme,
        last_light_theme: Theme::WarmPaper,
        focused_host_id: LOCAL_HOST_ID.to_string(),
        remote_hosts: Vec::new(),
        recent_completed_limit: board::DEFAULT_RECENT_LIMIT,
        center_view: CenterView::Board,
        show_command_preview: true,
    };
    write_json(path, &file)?;
    Ok((
        appearance,
        LOCAL_HOST_ID.to_string(),
        Vec::new(),
        board::DEFAULT_RECENT_LIMIT,
        CenterView::Board,
        true,
    ))
}

fn load_client_tokens(path: &Path) -> Result<BTreeMap<String, String>, KernelError> {
    if !path.exists() {
        write_json_inner(path, &ClientSecretsFile::default(), true)?;
        return Ok(BTreeMap::new());
    }
    owner::restrict_to_owner(path)?;
    let raw = fs::read_to_string(path)?;
    let file = serde_json::from_str::<ClientSecretsFile>(&raw).unwrap_or_default();
    Ok(file.tokens)
}

fn occupied_reason(language: Language, port: u16) -> String {
    match language {
        Language::ZhCn => {
            format!("本机网页入口没起来：端口 {port} 已被占用。桌面窗口可以继续用。")
        }
        Language::En => format!(
            "The local web entry could not start: port {port} is already in use. The desktop window still works."
        ),
    }
}

fn host_not_running_reason(language: Language) -> String {
    match language {
        Language::ZhCn => "本机没有在跑 Host，所以没有这份回环页。".into(),
        Language::En => {
            "The local Host is not running, so this loopback page is not available.".into()
        }
    }
}

fn match_language(locale: &str) -> Language {
    let normalized = locale.to_ascii_lowercase().replace('_', "-");
    if normalized == "zh" || normalized.starts_with("zh-") {
        Language::ZhCn
    } else {
        Language::En
    }
}

fn daytime_theme(theme: Theme) -> Theme {
    match theme {
        Theme::PlainNight => Theme::WarmPaper,
        other => other,
    }
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), KernelError> {
    write_json_inner(path, value, false)
}

fn write_json_inner<T: Serialize>(
    path: &Path,
    value: &T,
    owner_only: bool,
) -> Result<(), KernelError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_vec_pretty(value)?;
    let tmp = path.with_extension("json.tmp");
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(&body)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
    }
    if owner_only {
        owner::restrict_to_owner(&tmp)?;
    }
    owner::replace_file(&tmp, path)?;
    if owner_only {
        owner::restrict_to_owner(path)?;
    }
    Ok(())
}

fn parse_launch_config(request: &serde_json::Value) -> Result<RunLaunchConfig, KernelError> {
    let agent_id = required_string(request, "agentId")?;
    let values = request
        .get("values")
        .and_then(|value| value.as_object())
        .map(|object| {
            object
                .iter()
                .filter_map(|(key, value)| {
                    value
                        .as_str()
                        .map(|text| (key.clone(), text.to_string()))
                        .or_else(|| value.as_bool().map(|flag| (key.clone(), flag.to_string())))
                        .or_else(|| {
                            value
                                .as_i64()
                                .map(|number| (key.clone(), number.to_string()))
                        })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(RunLaunchConfig {
        agent_id,
        values,
        opening_text: optional_string(request, "openingText"),
    })
}

fn required_string(request: &serde_json::Value, key: &str) -> Result<String, KernelError> {
    request
        .get(key)
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| KernelError::Protocol(format!("missing {key}")))
}

fn optional_string(request: &serde_json::Value, key: &str) -> String {
    request
        .get(key)
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string()
}

fn read_github_pats(path: &Path) -> Result<BTreeMap<String, String>, KernelError> {
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    owner::restrict_to_owner(path)?;
    let raw = fs::read_to_string(path)?;
    let file = serde_json::from_str::<HostSecretsFile>(&raw).unwrap_or_default();
    Ok(file.github_pats)
}

fn read_github_pat(path: &Path, host: &str) -> Option<String> {
    read_github_pats(path)
        .ok()
        .and_then(|pats| pats.get(host).cloned())
        .and_then(|token| {
            let token = token.trim().to_string();
            (!token.is_empty()).then_some(token)
        })
}

fn probe_record(
    stored: StoredProject,
    tracker: &dyn TrackerPort,
    secrets_path: &Path,
    language: Language,
) -> ProjectRecord {
    let pat = read_github_pat(secrets_path, &stored.github_host);
    let outcome = tracker.probe(&tracker::ProbeContext {
        github_host: &stored.github_host,
        repository: &stored.repository,
        secrets_pat: pat.as_deref(),
        secrets_path,
    });
    ProjectRecord {
        id: stored.id,
        name: stored.name,
        local_path: stored.local_path,
        github_host: stored.github_host,
        repository: stored.repository,
        connection: connection_from_probe(outcome, secrets_path, language),
        tracker_synced: false,
    }
}

fn connection_from_probe(
    outcome: tracker::ProbeOutcome,
    secrets_path: &Path,
    language: Language,
) -> ProjectConnection {
    match outcome {
        tracker::ProbeOutcome::Ready { source } => ProjectConnection::Ready { source },
        tracker::ProbeOutcome::Failed {
            source,
            kind: AuthFailureKind::Unreachable,
            cli_detected,
            detail,
        } => ProjectConnection::Unreachable {
            source,
            repair: tracker::repair_hint(cli_detected, secrets_path),
            message: auth_failure_message(
                language,
                AuthFailureKind::Unreachable,
                detail.as_deref(),
            ),
        },
        tracker::ProbeOutcome::Failed {
            source,
            kind,
            cli_detected,
            detail,
        } => ProjectConnection::AuthFailed {
            source,
            kind,
            repair: tracker::repair_hint(cli_detected, secrets_path),
            message: auth_failure_message(language, kind, detail.as_deref()),
        },
    }
}

fn auth_failure_message(language: Language, kind: AuthFailureKind, detail: Option<&str>) -> String {
    let base = match (language, kind) {
        (Language::ZhCn, AuthFailureKind::MissingCredentials) => {
            "没有可用的 GitHub 凭据。".to_string()
        }
        (Language::En, AuthFailureKind::MissingCredentials) => {
            "No GitHub credentials are available.".to_string()
        }
        (Language::ZhCn, AuthFailureKind::Rejected) => "GitHub 拒绝了当前凭据。".to_string(),
        (Language::En, AuthFailureKind::Rejected) => {
            "GitHub rejected the current credentials.".to_string()
        }
        (Language::ZhCn, AuthFailureKind::Unreachable) => "连不上这个 GitHub host。".to_string(),
        (Language::En, AuthFailureKind::Unreachable) => {
            "This GitHub host could not be reached.".to_string()
        }
    };
    match detail {
        Some(detail) if !detail.is_empty() => format!("{base} {detail}"),
        _ => base,
    }
}

#[derive(Debug)]
pub enum KernelError {
    Io(io::Error),
    Json(serde_json::Error),
    Protocol(String),
    Denied(String),
}

impl std::fmt::Display for KernelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KernelError::Io(err) => write!(f, "{err}"),
            KernelError::Json(err) => write!(f, "{err}"),
            KernelError::Protocol(err) => write!(f, "{err}"),
            KernelError::Denied(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for KernelError {}

impl From<io::Error> for KernelError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for KernelError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}
