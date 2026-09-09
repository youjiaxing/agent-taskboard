use std::collections::BTreeMap;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::board::{BoardSnapshot, CenterView, IssueSearch, ProjectIssueCounts};
use crate::changes::ViewChanges;
use crate::copy::ShellCopy;
use crate::pairing::{IssuedPairing, PairedClient, PairingOffer};
use crate::persist::{daytime_theme, StoredProject};
use crate::project::ProjectInference;
use crate::run::{QuitOffer, RunSummary, UpdateInstallGate};
use crate::tracker::TrackerKind;
use crate::tracker_seam::TrackerSeam;
use crate::usage::UsagePage;
use crate::{
    builtin_agents, AgentPort, DataLayout, GitHubTracker, LaunchEnvPort, MemoryAgent,
    MemoryLaunchEnv, MemorySessionFactory, PendingConfirmation, PtySessionFactory, RefreshStatus,
    RunLaunchConfig, RunLaunchForm, RunStatus, SessionFactory, ShellLaunchEnv, TrackerRouter,
    TrackerWriteError, UsageRange,
};
use crate::{usage, ProjectConnection};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HostMode {
    HostAndClient,
    ClientOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
    LoadIssueDocument {
        issue_id: String,
    },
    FilterParent {
        issue_id: String,
    },
    ClearParentFilter,
    SetCenterView {
        view: CenterView,
    },
    CenterDependencyGraph {
        issue_id: String,
    },
    ShowDependencyGraphOverview,
    SetDependencyGraphComplete {
        complete: bool,
    },
    SetRecentCompletedLimit {
        limit: u32,
    },
    RefreshLaunchEnvironment,
    SearchIssues {
        project_id: String,
        search: IssueSearch,
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
    CreateIssue {
        project_id: String,
        title: String,
        body: String,
    },
    UpdateIssue {
        issue_id: String,
        title: String,
        body: String,
    },
    SetIssueOpen {
        issue_id: String,
        open: bool,
    },
    AddIssueComment {
        issue_id: String,
        body: String,
    },
    SetIssueParent {
        issue_id: String,
        parent: Option<String>,
    },
    SetIssueBlockedBy {
        issue_id: String,
        blocked_by: Vec<String>,
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
    ContinueRun {
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
        defer_discovery: bool,
        language: Language,
    },
    UpdateRunLaunch {
        project_id: String,
        config: RunLaunchConfig,
        language: Language,
    },
    CancelRunLaunch,
    StopRun {
        run_id: String,
    },
    FocusRun {
        run_id: String,
    },
    OpenHostOverview,
    ReturnToBoard,
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
    SetNotificationPrefs {
        desktop: bool,
        sound: bool,
    },
    WriteChangeNote {
        run_id: String,
        repo: String,
        path: String,
        line: u32,
        text: String,
    },
    DeleteChangeNote {
        note_id: String,
    },
    SetHostAutoAdvance {
        enabled: bool,
    },
    SetProjectAutoAdvance {
        project_id: String,
        enabled: bool,
    },
    SetProjectRestoreAutoAdvance {
        project_id: String,
        enabled: bool,
    },
    SetProjectRestoreDelay {
        project_id: String,
        delay_ms: u64,
    },
    VetoPendingConfirmation {
        project_id: String,
    },
    OpenUsage,
    CloseUsage,
    SetUsageRange {
        range: UsageRange,
        custom_from_ms: Option<u64>,
        custom_to_ms: Option<u64>,
    },
    SetUsageFilter {
        project_id: Option<String>,
        agent_id: Option<String>,
        model: Option<String>,
    },
    OpenUsageForRun {
        run_id: String,
    },
    OpenRunFromUsage {
        run_id: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkspaceView {
    Project,
    HostOverview,
    Run,
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
    Waiting {
        #[serde(rename = "runId")]
        run_id: String,
    },
    ExecutionStopped {
        #[serde(rename = "issueId")]
        issue_id: String,
        #[serde(rename = "runId")]
        run_id: String,
    },
    HostCrashedRecovered {
        #[serde(rename = "runIds")]
        run_ids: Vec<String>,
    },
    PendingConfirmationStarted {
        #[serde(rename = "projectId")]
        project_id: String,
        #[serde(rename = "issueId")]
        issue_id: String,
        #[serde(rename = "runId")]
        run_id: String,
    },
    PendingConfirmationEnded {
        #[serde(rename = "projectId")]
        project_id: String,
        #[serde(rename = "issueId")]
        issue_id: String,
        #[serde(rename = "runId")]
        run_id: String,
        advanced: bool,
    },
    Notification {
        kind: NotificationKind,
        #[serde(rename = "runId")]
        run_id: String,
        #[serde(rename = "issueId")]
        issue_id: Option<String>,
        #[serde(rename = "projectId")]
        project_id: String,
    },
    Telemetry {
        #[serde(rename = "runId")]
        run_id: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NotificationKind {
    Waiting,
    Completed,
    AbnormalStop,
    CrashRecovered,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandOutcome {
    pub snapshot: Box<HostSnapshot>,
    pub process: ProcessIntent,
    pub pairing: Option<IssuedPairing>,
    pub inference: Option<ProjectInference>,
    #[serde(
        rename = "updateInstallGate",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub update_install_gate: Option<UpdateInstallGate>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<HostEvent>,
    #[serde(
        rename = "viewChanges",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub view_changes: Option<ViewChanges>,
    #[serde(
        rename = "launchEnvironment",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub launch_environment: Option<LaunchEnvironmentStatus>,
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
        if let Some(gate) = &self.update_install_gate {
            value["updateInstallGate"] = serde_json::to_value(gate).expect("update gate json");
        }
        if !self.events.is_empty() {
            value["events"] = serde_json::to_value(&self.events).expect("events json");
        }
        if let Some(view_changes) = &self.view_changes {
            value["viewChanges"] = serde_json::to_value(view_changes).expect("view changes json");
        }
        if let Some(status) = &self.launch_environment {
            value["launchEnvironment"] =
                serde_json::to_value(status).expect("launch environment json");
        }
        value
    }
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
    #[serde(default)]
    pub has_execution_stopped: bool,
    pub tracker_synced: bool,
    #[serde(default)]
    pub auto_advance: bool,
    #[serde(default)]
    pub restore_auto_advance: bool,
    #[serde(default = "crate::advance::default_restore_delay_ms")]
    pub restore_delay_ms: u64,
    #[serde(default)]
    pub issue_counts: ProjectIssueCounts,
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectRecord {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) local_path: PathBuf,
    pub(crate) tracker: TrackerKind,
    pub(crate) github_host: String,
    pub(crate) repository: String,
    pub(crate) connection: ProjectConnection,
    pub(crate) tracker_synced: bool,
    pub(crate) auto_advance: bool,
    pub(crate) restore_auto_advance: bool,
    pub(crate) restore_delay_ms: u64,
    pub(crate) advance_ready_at_ms: Option<u64>,
}

impl ProjectRecord {
    pub(crate) fn summary(
        &self,
        has_active_run: bool,
        has_execution_stopped: bool,
        issue_counts: ProjectIssueCounts,
    ) -> ProjectSummary {
        ProjectSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            local_path: self.local_path.clone(),
            tracker: self.tracker,
            github_host: self.github_host.clone(),
            repository: self.repository.clone(),
            connection: self.connection.clone(),
            has_active_run,
            has_execution_stopped,
            tracker_synced: self.tracker_synced,
            auto_advance: self.auto_advance,
            restore_auto_advance: self.restore_auto_advance,
            restore_delay_ms: self.restore_delay_ms,
            issue_counts,
        }
    }

    pub(crate) fn stored(&self) -> StoredProject {
        StoredProject {
            id: self.id.clone(),
            name: self.name.clone(),
            local_path: self.local_path.clone(),
            tracker: self.tracker,
            github_host: self.github_host.clone(),
            repository: self.repository.clone(),
            auto_advance: self.auto_advance,
            restore_auto_advance: self.restore_auto_advance,
            restore_delay_ms: self.restore_delay_ms,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppearanceSelection {
    pub(crate) language: Language,
    pub(crate) theme: Theme,
    pub(crate) last_light_theme: Theme,
}

impl AppearanceSelection {
    pub(crate) fn with_language(self, language: Language) -> Self {
        Self { language, ..self }
    }

    pub(crate) fn with_theme(self, theme: Theme) -> Self {
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
    pub(crate) fn from_selection(selection: AppearanceSelection) -> Self {
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
pub struct HostSnapshot {
    pub running: bool,
    pub window_visible: bool,
    pub host_mode: HostMode,
    pub focused_host_id: String,
    pub focused_project_id: String,
    pub hosts: Vec<HostSummary>,
    pub projects: Vec<ProjectSummary>,
    pub appearance: AppearanceState,
    pub data: DataLayout,
    pub copy: ShellCopy,
    pub copy_catalog: BTreeMap<Language, ShellCopy>,
    pub empty_actions: Vec<EmptyAction>,
    pub loopback_page: LoopbackPage,
    pub pairing_offer: Option<PairingOffer>,
    pub paired_clients: Vec<PairedClient>,
    pub board: Option<BoardSnapshot>,
    pub recent_completed_limit: u32,
    pub refresh_interval_ms: u64,
    pub center_view: CenterView,
    pub workspace_view: WorkspaceView,
    pub runs: Vec<RunSummary>,
    pub focused_run_id: String,
    pub quit_offer: Option<QuitOffer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_form: Option<RunLaunchForm>,
    pub show_command_preview: bool,
    pub notify_desktop: bool,
    pub notify_sound: bool,
    pub auto_advance: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_confirmation: Option<PendingConfirmation>,
    pub usage_open: bool,
    pub usage: UsagePage,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchEnvironmentStatus {
    pub status: &'static str,
    pub refreshed_directories: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl Default for LaunchEnvironmentStatus {
    fn default() -> Self {
        Self {
            status: "idle",
            refreshed_directories: 0,
            message: None,
        }
    }
}

#[derive(Clone)]
pub struct KernelPorts {
    pub tracker: Arc<dyn TrackerSeam>,
    pub agents: Vec<Arc<dyn AgentPort>>,
    pub launch_env: Arc<dyn LaunchEnvPort>,
    pub sessions: Arc<dyn SessionFactory>,
}

impl KernelPorts {
    pub fn live() -> Self {
        let launch_env: Arc<dyn LaunchEnvPort> = Arc::new(ShellLaunchEnv::live());
        Self {
            tracker: Arc::new(TrackerRouter::new(Arc::new(GitHubTracker::live(
                launch_env.clone(),
            )))),
            agents: builtin_agents(),
            launch_env,
            sessions: Arc::new(PtySessionFactory),
        }
    }

    pub fn for_tests(tracker: Arc<dyn TrackerSeam>) -> Self {
        Self {
            tracker,
            agents: vec![Arc::new(MemoryAgent::installed_grok())],
            launch_env: Arc::new(MemoryLaunchEnv::with_path("/mem/bin")),
            sessions: MemorySessionFactory::new(),
        }
    }
}

pub(crate) struct ClientView {
    pub(crate) project_id: String,
    pub(crate) visible: bool,
    pub(crate) last_seen_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefreshTrigger {
    Immediate,
    Action,
    Interval,
    RunEnded,
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteView {
    pub(crate) host_id: String,
    pub(crate) projects: Vec<ProjectSummary>,
    pub(crate) focused_project_id: String,
    pub(crate) empty_actions: Vec<EmptyAction>,
    pub(crate) board: Option<BoardSnapshot>,
    pub(crate) runs: Vec<RunSummary>,
    pub(crate) focused_run_id: String,
    pub(crate) workspace_view: WorkspaceView,
    pub(crate) quit_offer: Option<QuitOffer>,
    pub(crate) launch_form: Option<RunLaunchForm>,
    pub(crate) usage_open: bool,
    pub(crate) usage: UsagePage,
    pub(crate) refresh_interval_ms: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct ClientNavigationState {
    pub(crate) focused_host_id: String,
    pub(crate) remote_view: Option<RemoteView>,
    pub(crate) focused_project_id: Option<String>,
    pub(crate) selected_issue_id: Option<String>,
    pub(crate) parent_filter: Option<String>,
    pub(crate) issue_search: BTreeMap<String, IssueSearch>,
    pub(crate) center_view: CenterView,
    pub(crate) workspace_view: WorkspaceView,
    pub(crate) graph_center_issue_id: Option<String>,
    pub(crate) complete_dependency_graph: bool,
    pub(crate) focused_run_id: Option<String>,
    pub(crate) launch_form: Option<RunLaunchForm>,
    pub(crate) usage_open: bool,
    pub(crate) usage_query: usage::UsageQuery,
}

#[derive(Debug)]
pub enum KernelError {
    Io(io::Error),
    Json(serde_json::Error),
    Protocol(String),
    Denied(String),
}

pub(crate) fn write_tracker_error(err: TrackerWriteError) -> KernelError {
    match err {
        TrackerWriteError::Failed { message } => KernelError::Denied(message),
        TrackerWriteError::Offline { detail, .. } => {
            KernelError::Denied(write_error_with_detail("offline", detail))
        }
        TrackerWriteError::Auth { detail, .. } => {
            KernelError::Denied(write_error_with_detail("auth-failed", detail))
        }
        TrackerWriteError::RateLimited { retry_after_ms } => {
            KernelError::Denied(match retry_after_ms {
                Some(ms) => {
                    format!("cannot write to tracker: rate-limited (retry after {ms}ms)")
                }
                None => "cannot write to tracker: rate-limited".into(),
            })
        }
    }
}

pub(crate) fn write_error_with_detail(kind: &str, detail: Option<String>) -> String {
    match detail {
        Some(detail) if !detail.is_empty() => {
            format!("cannot write to tracker: {kind} ({detail})")
        }
        _ => format!("cannot write to tracker: {kind}"),
    }
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
