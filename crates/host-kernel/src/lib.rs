//! Host 内核：桌面窗口、浏览器和以后的远程 Client 都走这一条接缝。

mod advance;
mod agent;
mod board;
mod changes;
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
mod tracker_seam;
mod usage;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

pub use advance::{PendingConfirmation, DEFAULT_RESTORE_DELAY_MS, PENDING_CONFIRM_MS};
pub use agent::{
    builtin_agents, intent_prefix, probe_binary, AgentField, AgentFieldKind, AgentPort,
    AgentSummary, AntigravityAdapter, ClaudeAdapter, CodexAdapter, CompletionHookPlan,
    CompletionSignals, GrokAdapter, IntentOption, MemoryAgent, PrefillSource, ProbeResult,
    RunIntent, RunLaunchConfig, RunLaunchForm, ANTIGRAVITY_BIN, ANTIGRAVITY_ID, ANTIGRAVITY_NAME,
    CLAUDE_BIN, CLAUDE_CODE_ID, CLAUDE_CODE_NAME, CODEX_BIN, CODEX_ID, CODEX_NAME, GROK_BIN,
    GROK_BUILD_ID, GROK_BUILD_NAME,
};
pub use board::{
    clamp_recent_limit, BoardColumns, BoardEmptyReason, BoardSnapshot, CenterView, DependencyGraph,
    DependencyGraphMode, FrontierEmptyReason, GraphEdge, GraphNode, GraphRelation, IssueActivity,
    IssueCard, IssueDetail, IssueDocumentFailure, IssueDocumentFailureKind, IssueDocumentState,
    IssueLink, IssueSearch, IssueStateFilter, ProjectIssueCounts, RefreshStatus,
    DEFAULT_RECENT_LIMIT,
};
pub use changes::{
    ChangeFile, ChangeHunk, ChangeLine, ChangeLineKind, ChangeNote, ChangeRepo, ChangeScope,
    GitBaseline, ViewChanges,
};
pub use issue::{parse_issue_id, DependencyRef, IssueRecord, IssueRef, TriageRole};
pub use launch_env::{LaunchEnvPort, LaunchEnvironment, MemoryLaunchEnv, ShellLaunchEnv};
pub use local_rpc::{
    bind_local_rpc, local_client_origin_allowed, spawn_local_rpc, LoopbackAssets, LoopbackServer,
    LOCAL_RPC_PORT,
};
pub use pairing::{IssuedPairing, PairedClient, PairingOffer};
pub use project::ProjectInference;
pub use refresh::DEFAULT_REFRESH_INTERVAL_MS;
pub use run::{QuitOffer, RunEndedReason, RunStatus, RunSummary, UpdateInstallGate};
pub use session::{
    AgentSession, MemorySession, MemorySessionFactory, PtyChunk, PtySessionFactory, SessionFactory,
    SpawnRequest,
};
pub use tracker::{
    gh_known_install_locations, map_github_issue_node, resolve_gh, AuthFailureKind,
    CredentialSource, GitHubTracker, IssueComment, IssueDocument, IssueEdit, LocalMarkdownTracker,
    MemoryTracker, ProbeContext, ProbeOutcome, ProjectConnection, RepairHint, ScriptedGitHub,
    TrackerKind, TrackerPort, TrackerReadError, TrackerWriteError,
};
pub use tracker_seam::{TrackerReadOutcome, TrackerRouter, TrackerSeam, TrackerWriteOp};
pub use usage::{
    BucketKind, RunTelemetryLane, TelemetryLane, TelemetryPoint, TelemetrySample, TokenCounts,
    UsageFilter, UsagePage, UsageRange, RING_LEN,
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
    ForgetRemoteHost {
        host_id: String,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkspaceView {
    #[default]
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
    #[serde(default)]
    pub has_execution_stopped: bool,
    pub tracker_synced: bool,
    #[serde(default)]
    pub auto_advance: bool,
    #[serde(default)]
    pub restore_auto_advance: bool,
    #[serde(default = "advance::default_restore_delay_ms")]
    pub restore_delay_ms: u64,
    #[serde(default)]
    pub issue_counts: ProjectIssueCounts,
}

#[derive(Debug, Clone)]
struct ProjectRecord {
    id: String,
    name: String,
    local_path: PathBuf,
    tracker: TrackerKind,
    github_host: String,
    repository: String,
    connection: ProjectConnection,
    tracker_synced: bool,
    auto_advance: bool,
    restore_auto_advance: bool,
    restore_delay_ms: u64,
    advance_ready_at_ms: Option<u64>,
}

impl ProjectRecord {
    fn summary(
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

    fn stored(&self) -> StoredProject {
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
    pub updates: String,
    pub check_for_updates: String,
    pub update_checking: String,
    pub update_available: String,
    pub update_ready: String,
    pub update_notes: String,
    pub update_confirm: String,
    pub update_later: String,
    pub update_current: String,
    pub update_unavailable_browser: String,
    pub update_active_runs: String,
    pub update_installing: String,
    pub update_failed: String,
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
    pub next_step: String,
    pub forget_host: String,
    pub forget_host_confirm_title: String,
    pub forget_host_confirm_body: String,
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
    pub choose_directory: String,
    pub choose_directory_desktop_only: String,
    pub inferring_from_directory: String,
    pub inference_failed: String,
    pub active_project_edit_hint: String,
    pub remote_project_hint: String,
    pub operation_pending: String,
    pub inference_pending: String,
    pub retry_inference: String,
    pub removal_pending: String,
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
    pub remove_keep_claims_body: String,
    pub continue_run: String,
    pub release_claim: String,
    pub execution_stopped: String,
    pub waiting: String,
    pub running: String,
    pub inject_line: String,
    pub inject_placeholder: String,
    pub notify_desktop: String,
    pub notify_sound: String,
    pub notify_waiting: String,
    pub notify_completed: String,
    pub notify_abnormal: String,
    pub notify_crash: String,
    pub got_it: String,
    pub auth_failed: String,
    pub connection_unavailable: String,
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
    pub view_dependencies: String,
    pub graph_overview: String,
    pub graph_return_overview: String,
    pub graph_truncated: String,
    pub graph_no_dependencies: String,
    pub show_closed_context: String,
    pub graph_center: String,
    pub graph_center_here: String,
    pub graph_show_complete: String,
    pub graph_show_neighborhood: String,
    pub graph_show_more: String,
    pub graph_canvas_limit: String,
    pub graph_complete_list: String,
    pub graph_search_placeholder: String,
    pub graph_upstream: String,
    pub graph_downstream: String,
    pub graph_both: String,
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
    pub empty_incomplete: String,
    pub empty_tracker_error: String,
    pub issue_document: String,
    pub issue_document_loading: String,
    pub issue_document_retry: String,
    pub issue_document_stale: String,
    pub issue_document_failed: String,
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
    pub refresh_interval: String,
    pub refresh_interval_help: String,
    pub unclear_issue: String,
    pub refresh_now: String,
    pub refresh_refreshing: String,
    pub refresh_as_of: String,
    pub refresh_next: String,
    pub refresh_offline: String,
    pub refresh_offline_recovery: String,
    pub refresh_never: String,
    pub refresh_rate_limited: String,
    pub refresh_retry: String,
    pub refresh_paused: String,
    pub refresh_auth: String,
    pub refresh_auth_recovery: String,
    pub refresh_incomplete: String,
    pub refresh_tracker_error: String,
    pub new_run: String,
    pub execute_run: String,
    pub start_run: String,
    pub start_run_pending: String,
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
    pub view_changes: String,
    pub focus_run: String,
    pub open_issue: String,
    pub search_title: String,
    pub search_placeholder: String,
    pub search_all_triage: String,
    pub search_all_states: String,
    pub search_open: String,
    pub search_closed: String,
    pub search_submit: String,
    pub keyboard_help: String,
    pub keyboard_help_body: String,
    pub this_round: String,
    pub uncommitted: String,
    pub add_change_note: String,
    pub change_note_placeholder: String,
    pub delete_change_note: String,
    pub auto_advance: String,
    pub auto_advance_help: String,
    pub project_auto_advance: String,
    pub restore_auto_advance: String,
    pub restore_delay: String,
    pub pending_confirmation: String,
    pub veto_advance: String,
    pub usage: String,
    pub usage_hint: String,
    pub host_overview: String,
    pub host_overview_hint: String,
    pub host_overview_empty: String,
    pub return_to_board: String,
    pub show_sidebar: String,
    pub hide_sidebar: String,
    pub show_issue_detail: String,
    pub hide_issue_detail: String,
    pub show_ended_runs: String,
    pub run_group_waiting: String,
    pub run_group_running: String,
    pub run_group_stopped: String,
    pub run_group_ended: String,
    pub range_24_hours: String,
    pub range_today: String,
    pub range_7_days: String,
    pub range_30_days: String,
    pub range_custom: String,
    pub filter_all: String,
    pub filter_project: String,
    pub filter_agent: String,
    pub filter_model: String,
    pub token_input: String,
    pub token_output: String,
    pub token_cache_read: String,
    pub token_cache_write: String,
    pub token_reasoning: String,
    pub token_total: String,
    pub ttft: String,
    pub gen_rate: String,
    pub cache_hit: String,
    pub spike: String,
    pub proxy_disclaimer: String,
    pub open_host_usage: String,
    pub open_this_run: String,
    pub lane_main: String,
    pub lane_subagent: String,
    pub lane_switched: String,
    pub usage_empty: String,
    pub close_usage: String,
    pub mobile_switch_scope: String,
    pub mobile_board: String,
    pub mobile_issue: String,
    pub mobile_run: String,
    pub mobile_recent_output: String,
    pub mobile_live_terminal: String,
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

pub struct HostKernel {
    running: bool,
    window_visible: bool,
    host_mode: HostMode,
    exiting: bool,
    host_display_name: String,
    data: DataLayout,
    appearance: AppearanceSelection,
    projects: Vec<ProjectRecord>,
    focused_project_id: Option<String>,
    tracker: Arc<dyn TrackerSeam>,
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
    remote_client_views: BTreeMap<String, BTreeMap<String, RemoteView>>,
    loaded_issues: BTreeMap<String, Vec<IssueRecord>>,
    issue_documents: BTreeMap<String, BTreeMap<String, IssueDocumentState>>,
    issue_documents_in_flight: BTreeSet<(String, String)>,
    refresh: BTreeMap<String, ProjectRefreshState>,
    refresh_in_flight: BTreeSet<String>,
    client_views: BTreeMap<String, ClientView>,
    client_launch_forms: BTreeMap<String, RunLaunchForm>,
    precomputed_project_connection: Option<(String, String, ProjectConnection)>,
    defer_tracker_refreshes: bool,
    deferred_refresh_tasks: Vec<BackgroundRefreshTask>,
    preclaimed_issue_id: Option<String>,
    pending_events: Vec<HostEvent>,
    now_ms: u64,
    refresh_interval_ms: u64,
    selected_issue_id: Option<String>,
    parent_filter: Option<String>,
    recent_limit: u32,
    issue_search: BTreeMap<String, IssueSearch>,
    center_view: CenterView,
    workspace_view: WorkspaceView,
    graph_center_issue_id: Option<String>,
    complete_dependency_graph: bool,
    launch_defaults: BTreeMap<String, BTreeMap<String, BTreeMap<String, String>>>,
    last_successful_agent: BTreeMap<String, String>,
    launch_form: Option<RunLaunchForm>,
    show_command_preview: bool,
    notify_desktop: bool,
    notify_sound: bool,
    change_notes: Vec<ChangeNote>,
    host_auto_advance: bool,
    pending_advance: BTreeMap<String, advance::PendingAdvance>,
    open_view_changes_run_id: Option<String>,
    usage_open: bool,
    usage_query: usage::UsageQuery,
    usage_samples: Vec<TelemetrySample>,
    update_installing: bool,
}

#[derive(Debug, Clone)]
struct ProjectRefreshState {
    fetched_at_ms: Option<u64>,
    last_attempt_ms: u64,
    kind: StoredRefreshKind,
    retry_at_ms: Option<u64>,
    /// 最近一次成功读取是否完整；不完整时不能当作全量数据计算 Frontier/依赖图。
    complete: bool,
    /// 不完整读取的可读详情。
    detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StoredRefreshKind {
    Ready,
    Offline,
    NeverFetched,
    RateLimited,
    AuthFailed,
    Incomplete,
    TrackerError,
}

#[derive(Debug, Clone)]
struct ClientView {
    project_id: String,
    visible: bool,
    last_seen_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClientSnapshotView {
    #[serde(default)]
    focused_host_id: String,
    #[serde(default)]
    focused_project_id: String,
    #[serde(default)]
    selected_issue_id: Option<String>,
    #[serde(default)]
    focused_run_id: String,
    #[serde(default)]
    center_view: CenterView,
    #[serde(default)]
    workspace_view: WorkspaceView,
    #[serde(default)]
    parent_filter_id: Option<String>,
    #[serde(default)]
    search: IssueSearch,
    #[serde(default)]
    graph_mode: ClientGraphMode,
    #[serde(default)]
    graph_center_issue_id: Option<String>,
    #[serde(default)]
    complete_dependency_graph: bool,
    #[serde(default)]
    usage_open: bool,
    #[serde(default)]
    usage_query: usage::UsageQuery,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum ClientGraphMode {
    #[default]
    Overview,
    Focused,
}

struct PreviousRun {
    id: String,
    native_session_id: Option<String>,
    working_directory: String,
    isolated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefreshTrigger {
    Immediate,
    Action,
    Interval,
    RunEnded,
}

#[derive(Debug, Clone)]
enum RefreshContinuation {
    RunEnded(String),
    PendingAdvance(String),
    SelfCheck(String),
}

pub(crate) struct BackgroundRefreshTask {
    project_id: String,
    github_host: String,
    repository: String,
    host_secrets_path: PathBuf,
    tracker: Arc<dyn TrackerSeam>,
    now_ms: u64,
    previous: Option<ProjectRefreshState>,
    probe_connection: bool,
    language: Language,
    continuation: Option<RefreshContinuation>,
}

pub(crate) struct BackgroundRefreshCompletion {
    task: BackgroundRefreshTask,
    result: Result<TrackerReadOutcome, TrackerReadError>,
    connection: Option<ProjectConnection>,
}

pub(crate) struct BackgroundIssueDocumentTask {
    project_id: String,
    issue_id: String,
    github_host: String,
    repository: String,
    host_secrets_path: PathBuf,
    tracker: Arc<dyn TrackerSeam>,
    now_ms: u64,
    previous_body: Option<(String, u64)>,
}

pub(crate) struct BackgroundIssueDocumentCompletion {
    task: BackgroundIssueDocumentTask,
    result: Result<IssueDocument, TrackerReadError>,
}

pub(crate) struct BackgroundRemoteRequestTask {
    host_id: String,
    address: String,
    token: String,
    request: serde_json::Value,
    client_id: Option<String>,
    client_view: Option<ClientSnapshotView>,
    focus_host: bool,
}

pub(crate) struct BackgroundRemoteRequestCompletion {
    task: BackgroundRemoteRequestTask,
    result: Result<serde_json::Value, KernelError>,
}

pub(crate) struct BackgroundPairRemoteHostTask {
    address: String,
    code: String,
    client_name: String,
}

pub(crate) struct BackgroundPairRemoteHostCompletion {
    task: BackgroundPairRemoteHostTask,
    result: Result<IssuedPairing, KernelError>,
}

pub(crate) struct BackgroundTrackerWriteTask {
    refresh: BackgroundRefreshTask,
    issue_id: Option<String>,
    op: tracker_seam::TrackerWriteOp,
    after_request: Option<serde_json::Value>,
}

pub(crate) struct BackgroundTrackerWriteCompletion {
    refresh: BackgroundRefreshCompletion,
    issue_id: Option<String>,
    op: tracker_seam::TrackerWriteOp,
    write_result: Option<Result<IssueRecord, TrackerWriteError>>,
    after_request: Option<serde_json::Value>,
}

pub(crate) struct BackgroundClaimRollbackTask {
    project_id: String,
    issue_id: String,
    github_host: String,
    repository: String,
    host_secrets_path: PathBuf,
    tracker: Arc<dyn TrackerSeam>,
}

pub(crate) struct BackgroundClaimRollbackCompletion {
    task: BackgroundClaimRollbackTask,
    result: Result<IssueRecord, TrackerWriteError>,
}

pub(crate) struct BackgroundAutoAdvanceTask {
    pending: advance::PendingAdvance,
    issue_id: String,
    github_host: String,
    repository: String,
    host_secrets_path: PathBuf,
    tracker: Arc<dyn TrackerSeam>,
}

pub(crate) struct BackgroundAutoAdvanceCompletion {
    task: BackgroundAutoAdvanceTask,
    result: Result<IssueRecord, TrackerWriteError>,
}

pub(crate) struct BackgroundTrackerWriteFinish {
    pub(crate) result: Result<CommandOutcome, KernelError>,
    pub(crate) rollback: Option<BackgroundClaimRollbackTask>,
}

pub(crate) struct BackgroundProjectProbeTask {
    request: serde_json::Value,
    github_host: String,
    repository: String,
    host_secrets_path: PathBuf,
    tracker: Arc<dyn TrackerSeam>,
    language: Language,
}

pub(crate) struct BackgroundProjectProbeCompletion {
    task: BackgroundProjectProbeTask,
    connection: ProjectConnection,
}

impl BackgroundRefreshTask {
    pub(crate) fn execute(self) -> BackgroundRefreshCompletion {
        let pat = read_github_pat(&self.host_secrets_path, &self.github_host);
        let ctx = tracker::ProbeContext {
            github_host: &self.github_host,
            repository: &self.repository,
            secrets_pat: pat.as_deref(),
            secrets_path: &self.host_secrets_path,
        };
        let connection = self.probe_connection.then(|| {
            connection_from_probe(
                self.tracker.probe(&ctx),
                &self.host_secrets_path,
                self.language,
            )
        });
        let result = self.tracker.read_all(&ctx);
        BackgroundRefreshCompletion {
            task: self,
            result,
            connection,
        }
    }
}

impl BackgroundIssueDocumentTask {
    pub(crate) fn execute(self) -> BackgroundIssueDocumentCompletion {
        let pat = read_github_pat(&self.host_secrets_path, &self.github_host);
        let result = self.tracker.read_issue_document(
            &tracker::ProbeContext {
                github_host: &self.github_host,
                repository: &self.repository,
                secrets_pat: pat.as_deref(),
                secrets_path: &self.host_secrets_path,
            },
            &self.issue_id,
        );
        BackgroundIssueDocumentCompletion { task: self, result }
    }
}

impl BackgroundRemoteRequestTask {
    pub(crate) fn execute(self) -> BackgroundRemoteRequestCompletion {
        let result =
            pairing::post_rpc(&self.address, Some(&self.token), &self.request).map_err(|error| {
                match error {
                    KernelError::Io(_) => KernelError::Protocol("address is not reachable".into()),
                    other => other,
                }
            });
        BackgroundRemoteRequestCompletion { task: self, result }
    }
}

impl BackgroundPairRemoteHostTask {
    pub(crate) fn execute(self) -> BackgroundPairRemoteHostCompletion {
        let response = pairing::post_rpc(
            &self.address,
            None,
            &serde_json::json!({
                "op": "redeemPairing",
                "code": self.code,
                "clientName": self.client_name,
            }),
        )
        .map_err(|error| match error {
            KernelError::Io(_) => KernelError::Protocol("address is not reachable".into()),
            other => other,
        });
        let result = response.and_then(|response| {
            let pairing = response
                .get("pairing")
                .cloned()
                .ok_or_else(|| KernelError::Denied("invalid pairing code".into()))?;
            serde_json::from_value(pairing).map_err(KernelError::from)
        });
        BackgroundPairRemoteHostCompletion { task: self, result }
    }
}

impl BackgroundTrackerWriteTask {
    pub(crate) fn execute(self) -> BackgroundTrackerWriteCompletion {
        let pat = read_github_pat(&self.refresh.host_secrets_path, &self.refresh.github_host);
        let ctx = tracker::ProbeContext {
            github_host: &self.refresh.github_host,
            repository: &self.refresh.repository,
            secrets_pat: pat.as_deref(),
            secrets_path: &self.refresh.host_secrets_path,
        };
        let read_result = self.refresh.tracker.read_all(&ctx);
        let connection = self.refresh.probe_connection.then(|| {
            connection_from_probe(
                self.refresh.tracker.probe(&ctx),
                &self.refresh.host_secrets_path,
                self.refresh.language,
            )
        });
        let write_result = read_result.as_ref().ok().map(|_| {
            self.refresh
                .tracker
                .write_issue(&ctx, self.issue_id.as_deref(), &self.op)
        });
        BackgroundTrackerWriteCompletion {
            refresh: BackgroundRefreshCompletion {
                task: self.refresh,
                result: read_result,
                connection,
            },
            issue_id: self.issue_id,
            op: self.op,
            write_result,
            after_request: self.after_request,
        }
    }
}

impl BackgroundClaimRollbackTask {
    pub(crate) fn execute(self) -> BackgroundClaimRollbackCompletion {
        let pat = read_github_pat(&self.host_secrets_path, &self.github_host);
        let result = self.tracker.write_issue(
            &tracker::ProbeContext {
                github_host: &self.github_host,
                repository: &self.repository,
                secrets_pat: pat.as_deref(),
                secrets_path: &self.host_secrets_path,
            },
            Some(&self.issue_id),
            &tracker_seam::TrackerWriteOp::Release,
        );
        BackgroundClaimRollbackCompletion { task: self, result }
    }
}

impl BackgroundAutoAdvanceTask {
    pub(crate) fn execute(self) -> BackgroundAutoAdvanceCompletion {
        let pat = read_github_pat(&self.host_secrets_path, &self.github_host);
        let result = self.tracker.write_issue(
            &tracker::ProbeContext {
                github_host: &self.github_host,
                repository: &self.repository,
                secrets_pat: pat.as_deref(),
                secrets_path: &self.host_secrets_path,
            },
            Some(&self.issue_id),
            &tracker_seam::TrackerWriteOp::Claim,
        );
        BackgroundAutoAdvanceCompletion { task: self, result }
    }

    fn rollback_task(&self) -> BackgroundClaimRollbackTask {
        BackgroundClaimRollbackTask {
            project_id: self.pending.project_id.clone(),
            issue_id: self.issue_id.clone(),
            github_host: self.github_host.clone(),
            repository: self.repository.clone(),
            host_secrets_path: self.host_secrets_path.clone(),
            tracker: Arc::clone(&self.tracker),
        }
    }
}

impl BackgroundProjectProbeTask {
    pub(crate) fn execute(self) -> BackgroundProjectProbeCompletion {
        let pat = read_github_pat(&self.host_secrets_path, &self.github_host);
        let outcome = self.tracker.probe(&tracker::ProbeContext {
            github_host: &self.github_host,
            repository: &self.repository,
            secrets_pat: pat.as_deref(),
            secrets_path: &self.host_secrets_path,
        });
        let connection = connection_from_probe(outcome, &self.host_secrets_path, self.language);
        BackgroundProjectProbeCompletion {
            task: self,
            connection,
        }
    }
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
    workspace_view: WorkspaceView,
    quit_offer: Option<QuitOffer>,
    launch_form: Option<RunLaunchForm>,
    usage_open: bool,
    usage: UsagePage,
    refresh_interval_ms: u64,
    auto_advance: bool,
    pending_confirmation: Option<PendingConfirmation>,
}

impl HostKernel {
    pub fn boot(request: BootRequest) -> Result<Self, KernelError> {
        Self::boot_with_mode(request, KernelPorts::live(), HostMode::HostAndClient)
    }

    pub fn boot_client_only(request: BootRequest) -> Result<Self, KernelError> {
        Self::boot_with_mode(request, KernelPorts::live(), HostMode::ClientOnly)
    }

    pub fn boot_with(
        request: BootRequest,
        tracker: Arc<dyn TrackerSeam>,
    ) -> Result<Self, KernelError> {
        Self::boot_with_ports(request, KernelPorts::for_tests(tracker))
    }

    pub fn boot_with_ports(request: BootRequest, ports: KernelPorts) -> Result<Self, KernelError> {
        Self::boot_with_mode(request, ports, HostMode::HostAndClient)
    }

    fn boot_with_mode(
        request: BootRequest,
        ports: KernelPorts,
        host_mode: HostMode,
    ) -> Result<Self, KernelError> {
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
            notify_desktop,
            notify_sound,
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
        let projects = if host_mode == HostMode::HostAndClient {
            settings
                .projects
                .into_iter()
                .map(|stored| probe_record(stored, tracker.as_ref(), &secrets_path, language))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let focused_project_id = settings
            .focused_project_id
            .filter(|id| projects.iter().any(|project| project.id == *id))
            .or_else(|| projects.first().map(|project| project.id.clone()));
        let focused_host_id =
            if host_mode == HostMode::ClientOnly && focused_host_id == LOCAL_HOST_ID {
                remote_hosts
                    .first()
                    .map(|host| host.id.clone())
                    .unwrap_or_default()
            } else {
                focused_host_id
            };

        let mut host = Self {
            running: host_mode == HostMode::HostAndClient,
            window_visible: true,
            host_mode,
            exiting: false,
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
            remote_client_views: BTreeMap::new(),
            loaded_issues: BTreeMap::new(),
            issue_documents: BTreeMap::new(),
            issue_documents_in_flight: BTreeSet::new(),
            refresh: BTreeMap::new(),
            refresh_in_flight: BTreeSet::new(),
            client_views: BTreeMap::new(),
            client_launch_forms: BTreeMap::new(),
            precomputed_project_connection: None,
            defer_tracker_refreshes: false,
            deferred_refresh_tasks: Vec::new(),
            preclaimed_issue_id: None,
            pending_events: Vec::new(),
            now_ms: refresh::wall_ms(),
            refresh_interval_ms: refresh::clamp_refresh_interval_ms(settings.refresh_interval_ms),
            selected_issue_id: None,
            parent_filter: None,
            recent_limit,
            issue_search: BTreeMap::new(),
            center_view,
            workspace_view: WorkspaceView::Project,
            graph_center_issue_id: None,
            complete_dependency_graph: false,
            launch_defaults: settings.agent_launch_defaults,
            last_successful_agent: settings.last_successful_agent,
            launch_form: None,
            show_command_preview,
            notify_desktop,
            notify_sound,
            change_notes: Vec::new(),
            host_auto_advance: settings.auto_advance,
            pending_advance: BTreeMap::new(),
            open_view_changes_run_id: None,
            usage_open: false,
            usage_query: usage::UsageQuery::default(),
            usage_samples: Vec::new(),
            update_installing: false,
        };
        let project_ids: Vec<String> = host
            .projects
            .iter()
            .map(|project| project.id.clone())
            .collect();
        for project_id in &project_ids {
            host.load_persisted_snapshot(project_id);
        }
        let crashed_ids = if host.host_mode == HostMode::HostAndClient {
            let crashed_ids = host.load_persisted_runs();
            host.load_usage_samples();
            host.load_change_notes();
            crashed_ids
        } else {
            Vec::new()
        };
        if host.host_mode == HostMode::HostAndClient {
            if let Some(project_id) = host.focused_project_id.clone() {
                host.refresh_project(&project_id, RefreshTrigger::Immediate);
            }
        } else if !host.focused_host_id.is_empty() {
            let focused = host.focused_host_id.clone();
            let _ = host.refresh_remote_view(&focused);
        }
        host.pending_events.clear();
        host.arm_cold_start();
        host.note_crash_recovery(crashed_ids);
        Ok(host)
    }

    pub fn snapshot(&self) -> HostSnapshot {
        let (projects, focused_project_id, empty_actions) = self.board_for_focus();
        let board = self.current_board(&focused_project_id);
        let (runs, focused_run_id, workspace_view, quit_offer) = self.runs_for_focus();
        HostSnapshot {
            running: self.running,
            window_visible: self.window_visible,
            host_mode: self.host_mode,
            focused_host_id: self.focused_host_id.clone(),
            focused_project_id: focused_project_id.clone(),
            hosts: self.connected_hosts(),
            projects,
            appearance: AppearanceState::from_selection(self.appearance),
            data: self.data.clone(),
            copy: ShellCopy::for_language(self.appearance.language),
            copy_catalog: BTreeMap::from([
                (Language::ZhCn, ShellCopy::for_language(Language::ZhCn)),
                (Language::En, ShellCopy::for_language(Language::En)),
            ]),
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
            refresh_interval_ms: self.refresh_interval_for_focus(),
            center_view: self.center_view,
            workspace_view,
            runs,
            focused_run_id,
            quit_offer,
            launch_form: self.launch_form_for_focus(),
            show_command_preview: self.show_command_preview,
            notify_desktop: self.notify_desktop,
            notify_sound: self.notify_sound,
            auto_advance: self.host_auto_advance,
            pending_confirmation: self
                .pending_advance
                .get(&focused_project_id)
                .map(|pending| pending.to_snapshot(self.now_ms)),
            usage_open: self.usage_open_for_focus(),
            usage: self.usage_for_focus(),
        }
    }

    fn snapshot_for_client(
        &self,
        client_id: Option<&str>,
        view: &ClientSnapshotView,
    ) -> HostSnapshot {
        let mut snapshot = self.snapshot();
        let requested_host = if view.focused_host_id.is_empty() {
            LOCAL_HOST_ID
        } else {
            view.focused_host_id.as_str()
        };
        if requested_host != LOCAL_HOST_ID {
            snapshot.focused_host_id = requested_host.to_string();
            snapshot.center_view = view.center_view;
            snapshot.workspace_view = view.workspace_view;
            if let Some(remote) = client_id
                .and_then(|client_id| self.remote_client_views.get(client_id))
                .and_then(|views| views.get(requested_host))
                .or_else(|| {
                    self.remote_view
                        .as_ref()
                        .filter(|remote| remote.host_id == requested_host)
                })
            {
                snapshot.focused_project_id = remote.focused_project_id.clone();
                snapshot.projects = remote.projects.clone();
                snapshot.empty_actions = remote.empty_actions.clone();
                snapshot.board = remote.board.clone();
                snapshot.runs = remote.runs.clone();
                snapshot.focused_run_id = remote.focused_run_id.clone();
                snapshot.workspace_view = remote.workspace_view;
                snapshot.quit_offer = remote.quit_offer.clone();
                snapshot.launch_form = remote.launch_form.clone();
                snapshot.usage_open = remote.usage_open;
                snapshot.usage = remote.usage.clone();
                snapshot.refresh_interval_ms = remote.refresh_interval_ms;
                snapshot.auto_advance = remote.auto_advance;
                snapshot.pending_confirmation = remote.pending_confirmation.clone();
            } else {
                snapshot.focused_project_id.clear();
                snapshot.projects.clear();
                snapshot.empty_actions.clear();
                snapshot.board = None;
                snapshot.runs.clear();
                snapshot.focused_run_id.clear();
                snapshot.quit_offer = None;
                snapshot.launch_form = None;
                snapshot.usage_open = false;
                snapshot.pending_confirmation = None;
            }
            return snapshot;
        }

        let project_id = self
            .projects
            .iter()
            .find(|project| project.id == view.focused_project_id)
            .map(|project| project.id.clone())
            .or_else(|| self.projects.first().map(|project| project.id.clone()))
            .unwrap_or_default();
        let selected_issue_id = view.selected_issue_id.as_deref().filter(|issue_id| {
            self.loaded_issues
                .get(&project_id)
                .is_some_and(|issues| issues.iter().any(|issue| issue.id() == *issue_id))
        });
        let graph_center_issue_id = match view.graph_mode {
            ClientGraphMode::Overview => None,
            ClientGraphMode::Focused => view.graph_center_issue_id.as_deref(),
        };
        let board = self.current_local_board(
            &project_id,
            view.parent_filter_id.as_deref(),
            selected_issue_id,
            graph_center_issue_id,
            view.complete_dependency_graph,
            view.search.clone(),
        );
        let focused_run_id = self
            .runs
            .iter()
            .find(|run| run.id == view.focused_run_id)
            .map(|run| run.id.clone())
            .or_else(|| {
                selected_issue_id.and_then(|issue_id| self.active_run_id_for_issue(issue_id))
            })
            .unwrap_or_default();

        snapshot.focused_host_id = LOCAL_HOST_ID.to_string();
        snapshot.focused_project_id = project_id.clone();
        snapshot.board = board;
        snapshot.center_view = view.center_view;
        snapshot.workspace_view = view.workspace_view;
        snapshot.runs = self.decorate_runs(&self.runs);
        snapshot.focused_run_id = focused_run_id;
        snapshot.launch_form = client_id
            .and_then(|client_id| self.client_launch_forms.get(client_id).cloned())
            .or_else(|| {
                client_id
                    .is_none()
                    .then(|| self.launch_form.clone())
                    .flatten()
            });
        snapshot.usage_open = view.usage_open;
        snapshot.usage = self.build_usage_for(&view.usage_query);
        snapshot.pending_confirmation = self
            .pending_advance
            .get(&project_id)
            .map(|pending| pending.to_snapshot(self.now_ms));
        snapshot
    }

    fn outcome(&mut self) -> CommandOutcome {
        self.outcome_with(None, None)
    }

    fn outcome_with(
        &mut self,
        pairing: Option<IssuedPairing>,
        inference: Option<ProjectInference>,
    ) -> CommandOutcome {
        let view_changes = self.open_view_changes_run_id.take().and_then(|run_id| {
            self.view_changes(Some(&run_id), None, ChangeScope::ThisRound)
                .ok()
        });
        CommandOutcome {
            snapshot: Box::new(self.snapshot()),
            process: if self.process_alive() {
                ProcessIntent::KeepRunning
            } else {
                ProcessIntent::Exit
            },
            pairing,
            inference,
            update_install_gate: None,
            events: std::mem::take(&mut self.pending_events),
            view_changes,
            launch_environment: None,
        }
    }

    pub fn process_alive(&self) -> bool {
        !self.exiting
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

    pub fn note_run_exit(&mut self, run_id: &str, code: i32) {
        let stopped = self
            .live
            .get(run_id)
            .is_some_and(|session| session.was_stopped());
        self.mark_run_ended(run_id, RunEndedReason::from_exit(code, stopped));
    }

    pub(crate) fn note_run_exit_with_deferred_refreshes(
        &mut self,
        run_id: &str,
        code: i32,
    ) -> Vec<BackgroundRefreshTask> {
        let deferred_at_start = self.deferred_refresh_tasks.len();
        let previous = self.defer_tracker_refreshes;
        self.defer_tracker_refreshes = true;
        self.note_run_exit(run_id, code);
        self.defer_tracker_refreshes = previous;
        self.deferred_refresh_tasks.split_off(deferred_at_start)
    }

    pub fn update_install_gate(&mut self) -> UpdateInstallGate {
        self.observe_live_runs();
        let active_run_count = self.active_run_count();
        UpdateInstallGate {
            allowed: active_run_count == 0,
            active_run_count,
        }
    }

    pub fn begin_client_only_switch(&mut self) -> UpdateInstallGate {
        let gate = self.update_install_gate();
        if gate.allowed {
            self.update_installing = true;
        }
        gate
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
        self.observe_live_runs();
        match command {
            Command::HideWindow => self.window_visible = false,
            Command::ShowWindow => {
                if self.process_alive() {
                    self.window_visible = true;
                    self.now_ms = refresh::wall_ms();
                    if self.running {
                        if let Some(project_id) = self.focused_project_id.clone() {
                            self.refresh_project(&project_id, RefreshTrigger::Immediate);
                        }
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
                    self.exiting = true;
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
            Command::ForgetRemoteHost { host_id } => {
                self.forget_remote_host(&host_id)?;
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
                self.usage_open = false;
                self.usage_query.highlighted_run_id = None;
                self.workspace_view = WorkspaceView::Project;
                self.focused_run_id = None;
                self.focus_project(&project_id)?;
            }
            Command::InferProject { local_path } => {
                let inference = self.infer_project(&local_path)?;
                return Ok(self.outcome_with(None, inference));
            }
            Command::FocusIssue { issue_id } => {
                self.focus_issue(&issue_id);
            }
            Command::LoadIssueDocument { issue_id } => {
                let project_id = self
                    .focused_project_id
                    .as_deref()
                    .filter(|project_id| self.project_contains_issue(project_id, &issue_id))
                    .map(ToOwned::to_owned)
                    .or_else(|| self.project_id_for_issue(&issue_id).ok())
                    .ok_or_else(|| KernelError::Protocol("unknown issue".into()))?;
                self.load_issue_document(&project_id, &issue_id)?;
            }
            Command::FilterParent { issue_id } => {
                self.parent_filter = Some(issue_id);
            }
            Command::ClearParentFilter => {
                self.parent_filter = None;
            }
            Command::SetCenterView { view } => {
                if view == CenterView::Graph
                    && self.center_view != CenterView::Graph
                    && self.focused_host_id == LOCAL_HOST_ID
                {
                    self.graph_center_issue_id = self.selected_issue_id.clone().or_else(|| {
                        self.focused_project_id
                            .as_ref()
                            .and_then(|project_id| self.loaded_issues.get(project_id))
                            .and_then(|issues| {
                                issues
                                    .iter()
                                    .find(|issue| issue.open)
                                    .or_else(|| issues.first())
                            })
                            .map(IssueRecord::id)
                    });
                    self.complete_dependency_graph = false;
                }
                self.center_view = view;
                self.persist_client_settings(&self.appearance.clone())?;
            }
            Command::CenterDependencyGraph { issue_id } => {
                self.focus_issue(&issue_id);
                self.graph_center_issue_id = Some(issue_id);
            }
            Command::SetDependencyGraphComplete { complete } => {
                self.complete_dependency_graph = complete;
            }
            Command::SetRecentCompletedLimit { limit } => {
                self.recent_limit = board::clamp_recent_limit(limit);
                self.persist_client_settings(&self.appearance.clone())?;
            }
            Command::RefreshLaunchEnvironment => {
                let status = self.refresh_launch_environment()?;
                let mut outcome = self.outcome();
                outcome.launch_environment = Some(status);
                return Ok(outcome);
            }
            Command::SearchIssues { project_id, search } => {
                if !self.projects.iter().any(|project| project.id == project_id) {
                    return Err(KernelError::Protocol(format!(
                        "unknown project: {project_id}"
                    )));
                }
                self.focused_project_id = Some(project_id.clone());
                self.issue_search.insert(project_id, search);
            }
            Command::Refresh { project_id } => {
                let project_id = project_id
                    .or_else(|| self.focused_project_id.clone())
                    .ok_or_else(|| KernelError::Protocol("missing projectId".into()))?;
                self.refresh_project(&project_id, RefreshTrigger::Immediate);
            }
            Command::Tick { now_ms } => {
                self.now_ms = now_ms.unwrap_or_else(refresh::wall_ms);
                self.expire_stale_client_views();
                self.maybe_auto_refresh();
                self.finish_due_pending();
            }
            Command::SetClientView {
                client_id,
                project_id,
                visible,
            } => {
                if self.set_client_view(&client_id, &project_id, visible) {
                    self.refresh_project(&project_id, RefreshTrigger::Immediate);
                }
            }
            Command::NoteRunEnded { project_id } => {
                self.refresh_project(&project_id, RefreshTrigger::RunEnded);
            }
            Command::ClaimIssue { issue_id } => {
                self.claim_issue(&issue_id)?;
            }
            Command::ReleaseIssue { issue_id } => {
                self.release_issue(&issue_id)?;
            }
            Command::CreateIssue {
                project_id,
                title,
                body,
            } => {
                self.create_issue(&project_id, &title, &body)?;
            }
            Command::UpdateIssue {
                issue_id,
                title,
                body,
            } => {
                self.update_issue(&issue_id, &title, &body)?;
            }
            Command::SetIssueOpen { issue_id, open } => {
                self.set_issue_open(&issue_id, open)?;
            }
            Command::AddIssueComment { issue_id, body } => {
                self.add_issue_comment(&issue_id, &body)?;
            }
            Command::SetIssueParent { issue_id, parent } => {
                self.set_issue_parent(&issue_id, parent.as_deref())?;
            }
            Command::SetIssueBlockedBy {
                issue_id,
                blocked_by,
            } => {
                self.set_issue_blocked_by(&issue_id, &blocked_by)?;
            }
            Command::AutoAdvance { project_id } => {
                self.require_live_tracker(&project_id)?;
                self.finish_pending_if_due(&project_id);
            }
            Command::CheckIssueClosed { issue_id } => {
                self.require_live_tracker_for_issue(&issue_id)?;
            }
            Command::StartBoundRun { issue_id } => {
                self.start_bound_run(&issue_id)?;
            }
            Command::ContinueRun { issue_id } => {
                self.continue_run(&issue_id)?;
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
                    None,
                )?;
            }
            Command::StartUnboundRunWithConfig {
                project_id,
                config,
                issue_id,
            } => {
                self.start_unbound_run(&project_id, config, issue_id, true, None)?;
            }
            Command::SetShowCommandPreview { show } => {
                self.show_command_preview = show;
                self.persist_client_settings(&self.appearance.clone())?;
            }
            Command::SetNotificationPrefs { desktop, sound } => {
                self.notify_desktop = desktop;
                self.notify_sound = sound;
                self.persist_client_settings(&self.appearance.clone())?;
            }
            Command::StopRun { run_id } => {
                self.stop_run(&run_id)?;
            }
            Command::FocusRun { run_id } => {
                self.usage_open = false;
                self.usage_query.highlighted_run_id = None;
                self.focus_run(&run_id)?;
                self.workspace_view = WorkspaceView::Run;
            }
            Command::OpenHostOverview => {
                self.usage_open = false;
                self.usage_query.highlighted_run_id = None;
                self.focused_run_id = None;
                self.workspace_view = WorkspaceView::HostOverview;
            }
            Command::ReturnToBoard => {
                self.usage_open = false;
                self.usage_query.highlighted_run_id = None;
                self.focused_run_id = self
                    .selected_issue_id
                    .clone()
                    .and_then(|issue_id| self.active_run_id_for_issue(&issue_id));
                self.workspace_view = WorkspaceView::Project;
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
                self.exiting = true;
                self.loopback_kind = LoopbackKind::HostNotRunning;
            }
            Command::SetRefreshInterval { interval_ms } => {
                self.refresh_interval_ms = refresh::clamp_refresh_interval_ms(interval_ms);
                self.persist_host_settings()?;
            }
            Command::WriteChangeNote {
                run_id,
                repo,
                path,
                line,
                text,
            } => {
                self.write_change_note(&run_id, repo, path, line, text)?;
            }
            Command::DeleteChangeNote { note_id } => {
                self.delete_change_note(&note_id)?;
            }
            Command::SetHostAutoAdvance { enabled } => {
                self.set_host_auto_advance(enabled)?;
            }
            Command::SetProjectAutoAdvance {
                project_id,
                enabled,
            } => {
                self.set_project_auto_advance(&project_id, enabled)?;
            }
            Command::SetProjectRestoreAutoAdvance {
                project_id,
                enabled,
            } => {
                self.set_project_restore_auto_advance(&project_id, enabled)?;
            }
            Command::SetProjectRestoreDelay {
                project_id,
                delay_ms,
            } => {
                self.set_project_restore_delay(&project_id, delay_ms)?;
            }
            Command::VetoPendingConfirmation { project_id } => {
                self.veto_pending(&project_id);
            }
            Command::OpenUsage => {
                self.usage_open = true;
            }
            Command::CloseUsage => {
                self.usage_open = false;
                self.usage_query.highlighted_run_id = None;
            }
            Command::SetUsageRange {
                range,
                custom_from_ms,
                custom_to_ms,
            } => {
                self.usage_query.range = range;
                self.usage_query.custom_from_ms = custom_from_ms;
                self.usage_query.custom_to_ms = custom_to_ms;
            }
            Command::SetUsageFilter {
                project_id,
                agent_id,
                model,
            } => {
                self.usage_query.filter.project_id = project_id;
                self.usage_query.filter.agent_id = agent_id;
                self.usage_query.filter.model = model;
            }
            Command::OpenUsageForRun { run_id } => {
                self.open_usage_for_run(&run_id)?;
            }
            Command::OpenRunFromUsage { run_id } => {
                self.open_run_from_usage(&run_id)?;
            }
        }
        Ok(self.outcome())
    }

    pub fn handle(&mut self, request: serde_json::Value) -> Result<CommandOutcome, KernelError> {
        if let Some(task) = self.begin_background_remote_request(&request)? {
            return self.finish_background_remote_request(task.execute());
        }
        let client_id = request
            .get("clientId")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let client_view: Option<ClientSnapshotView> = request
            .get("clientView")
            .cloned()
            .map(serde_json::from_value)
            .transpose()?;
        let op = request
            .get("op")
            .and_then(|value| value.as_str())
            .unwrap_or("snapshot");
        let mut outcome = if let Some(client_id) = client_id.as_deref() {
            match op {
                "prepareRunLaunch" | "cancelRunLaunch" | "startUnboundRun" => {
                    self.handle_client_launch_request(request, client_id)?
                }
                _ => self.handle_inner(request)?,
            }
        } else {
            self.handle_inner(request)?
        };
        if let Some(view) = client_view.as_ref() {
            outcome.snapshot = Box::new(self.snapshot_for_client(client_id.as_deref(), view));
        }
        Ok(outcome)
    }

    pub(crate) fn handle_with_deferred_refreshes(
        &mut self,
        request: serde_json::Value,
    ) -> (
        Result<CommandOutcome, KernelError>,
        Vec<BackgroundRefreshTask>,
    ) {
        let deferred_at_start = self.deferred_refresh_tasks.len();
        let previous = self.defer_tracker_refreshes;
        self.defer_tracker_refreshes = true;
        let result = self.handle(request);
        self.defer_tracker_refreshes = previous;
        let tasks = self.deferred_refresh_tasks.split_off(deferred_at_start);
        (result, tasks)
    }

    fn handle_client_launch_request(
        &mut self,
        request: serde_json::Value,
        client_id: &str,
    ) -> Result<CommandOutcome, KernelError> {
        let host_form = self.launch_form.take();
        self.launch_form = self.client_launch_forms.remove(client_id);
        let result = self.handle_inner(request);
        let client_form = self.launch_form.take();
        self.launch_form = host_form;
        if let Some(form) = client_form {
            self.client_launch_forms.insert(client_id.to_string(), form);
        } else {
            self.client_launch_forms.remove(client_id);
        }
        result
    }

    pub(crate) fn begin_background_remote_request(
        &self,
        request: &serde_json::Value,
    ) -> Result<Option<BackgroundRemoteRequestTask>, KernelError> {
        let op = request
            .get("op")
            .and_then(|value| value.as_str())
            .unwrap_or("snapshot");
        let client_id = request
            .get("clientId")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let client_view: Option<ClientSnapshotView> = request
            .get("clientView")
            .cloned()
            .map(serde_json::from_value)
            .transpose()?;

        let (host_id, remote_request, focus_host) = if op == "focusHost" {
            let host_id = required_string(request, "hostId")?;
            if host_id == LOCAL_HOST_ID {
                return Ok(None);
            }
            (host_id, serde_json::json!({ "op": "snapshot" }), true)
        } else {
            if client_local_operation(op) {
                return Ok(None);
            }
            let host_id = client_view
                .as_ref()
                .map(|view| view.focused_host_id.as_str())
                .filter(|host_id| !host_id.is_empty())
                .unwrap_or(self.focused_host_id.as_str())
                .to_string();
            if host_id.is_empty() || host_id == LOCAL_HOST_ID {
                return Ok(None);
            }
            let mut remote_request = request.clone();
            if let Some(view) = remote_request
                .get_mut("clientView")
                .and_then(serde_json::Value::as_object_mut)
            {
                view.insert(
                    "focusedHostId".into(),
                    serde_json::Value::String(LOCAL_HOST_ID.into()),
                );
            }
            (host_id, remote_request, false)
        };
        let remote = self
            .remote_hosts
            .iter()
            .find(|host| host.id == host_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown host".into()))?;
        Ok(Some(BackgroundRemoteRequestTask {
            host_id,
            address: remote.address,
            token: remote.token,
            request: remote_request,
            client_id,
            client_view,
            focus_host,
        }))
    }

    pub(crate) fn finish_background_remote_request(
        &mut self,
        completion: BackgroundRemoteRequestCompletion,
    ) -> Result<CommandOutcome, KernelError> {
        let BackgroundRemoteRequestCompletion { task, result } = completion;
        let response = result?;
        if task.focus_host {
            self.focused_host_id = task.host_id.clone();
            self.persist_client_settings(&self.appearance.clone())?;
        }
        self.store_remote_view_response(&task.host_id, task.client_id.as_deref(), &response)?;
        let mut outcome = self.outcome();
        if let Some(view) = task.client_view.as_ref() {
            outcome.snapshot = Box::new(self.snapshot_for_client(task.client_id.as_deref(), view));
        }
        if let Some(inference) = response.get("inference").cloned() {
            outcome.inference = serde_json::from_value(inference).ok();
        }
        if let Some(changes) = response.get("viewChanges").cloned() {
            outcome.view_changes = serde_json::from_value(changes).ok();
        }
        Ok(outcome)
    }

    pub(crate) fn begin_background_pair_remote_host_request(
        &self,
        request: &serde_json::Value,
    ) -> Result<Option<BackgroundPairRemoteHostTask>, KernelError> {
        if request.get("op").and_then(|value| value.as_str()) != Some("pairRemoteHost") {
            return Ok(None);
        }
        let address = pairing::parse_http_url(&required_string(request, "address")?)
            .map_err(KernelError::Protocol)?;
        if self.is_own_loopback(&address) {
            return Err(KernelError::Protocol(
                "cannot pair this window to its own Host".into(),
            ));
        }
        Ok(Some(BackgroundPairRemoteHostTask {
            address,
            code: required_string(request, "code")?,
            client_name: self.host_display_name.clone(),
        }))
    }

    pub(crate) fn finish_background_pair_remote_host_request(
        &mut self,
        request: &serde_json::Value,
        completion: BackgroundPairRemoteHostCompletion,
    ) -> Result<CommandOutcome, KernelError> {
        self.apply_pair_remote_host_completion(completion)?;
        self.outcome_for_request(request)
    }

    pub(crate) fn begin_background_refresh_request(
        &mut self,
        request: &serde_json::Value,
    ) -> Result<(CommandOutcome, Option<BackgroundRefreshTask>), KernelError> {
        let client_view: Option<ClientSnapshotView> = request
            .get("clientView")
            .cloned()
            .map(serde_json::from_value)
            .transpose()?;
        let requested_host = client_view
            .as_ref()
            .map(|view| view.focused_host_id.as_str())
            .filter(|host_id| !host_id.is_empty())
            .unwrap_or(self.focused_host_id.as_str());
        if requested_host != LOCAL_HOST_ID {
            return self.handle(request.clone()).map(|outcome| (outcome, None));
        }
        self.observe_live_runs();
        let project_id = request
            .get("projectId")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| {
                client_view
                    .as_ref()
                    .map(|view| view.focused_project_id.clone())
                    .filter(|value| !value.is_empty())
            })
            .or_else(|| self.focused_project_id.clone())
            .ok_or_else(|| KernelError::Protocol("missing projectId".into()))?;
        let task = self.begin_refresh_task(&project_id, RefreshTrigger::Immediate);
        let mut outcome = self.outcome();
        if let Some(view) = client_view.as_ref() {
            outcome.snapshot = Box::new(self.snapshot_for_client(
                request.get("clientId").and_then(|value| value.as_str()),
                view,
            ));
        }
        Ok((outcome, task))
    }

    pub(crate) fn begin_background_issue_document_request(
        &mut self,
        request: &serde_json::Value,
    ) -> Result<(CommandOutcome, Option<BackgroundIssueDocumentTask>), KernelError> {
        let client_view: Option<ClientSnapshotView> = request
            .get("clientView")
            .cloned()
            .map(serde_json::from_value)
            .transpose()?;
        let requested_host = client_view
            .as_ref()
            .map(|view| view.focused_host_id.as_str())
            .filter(|host_id| !host_id.is_empty())
            .unwrap_or(self.focused_host_id.as_str());
        if requested_host != LOCAL_HOST_ID {
            return self.handle(request.clone()).map(|outcome| (outcome, None));
        }
        self.observe_live_runs();
        let issue_id = required_string(request, "issueId")?;
        let project_id = self.project_id_for_issue_request(request, &issue_id)?;
        let task = self.begin_issue_document_task(&project_id, &issue_id)?;
        let mut outcome = self.outcome();
        if let Some(view) = client_view.as_ref() {
            outcome.snapshot = Box::new(self.snapshot_for_client(
                request.get("clientId").and_then(|value| value.as_str()),
                view,
            ));
        }
        Ok((outcome, task))
    }

    pub(crate) fn begin_background_client_view_request(
        &mut self,
        request: &serde_json::Value,
    ) -> Result<(CommandOutcome, Option<BackgroundRefreshTask>), KernelError> {
        let client_view: Option<ClientSnapshotView> = request
            .get("clientView")
            .cloned()
            .map(serde_json::from_value)
            .transpose()?;
        let requested_host = client_view
            .as_ref()
            .map(|view| view.focused_host_id.as_str())
            .filter(|host_id| !host_id.is_empty())
            .unwrap_or(self.focused_host_id.as_str());
        if requested_host != LOCAL_HOST_ID {
            return self.handle(request.clone()).map(|outcome| (outcome, None));
        }
        let client_id = required_string(request, "clientId")?;
        let project_id = request
            .get("projectId")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        let visible = request
            .get("visible")
            .and_then(|value| value.as_bool())
            .ok_or_else(|| KernelError::Protocol("missing visible".into()))?;
        self.observe_live_runs();
        let changed = self.set_client_view(&client_id, &project_id, visible);
        let task = (changed && visible)
            .then(|| self.begin_refresh_task(&project_id, RefreshTrigger::Immediate))
            .flatten();
        let mut outcome = self.outcome();
        if let Some(view) = client_view.as_ref() {
            outcome.snapshot = Box::new(self.snapshot_for_client(
                request.get("clientId").and_then(|value| value.as_str()),
                view,
            ));
        }
        Ok((outcome, task))
    }

    pub(crate) fn begin_background_tick_request(
        &mut self,
        request: &serde_json::Value,
    ) -> Result<(CommandOutcome, Vec<BackgroundRefreshTask>), KernelError> {
        let client_view: Option<ClientSnapshotView> = request
            .get("clientView")
            .cloned()
            .map(serde_json::from_value)
            .transpose()?;
        let requested_host = client_view
            .as_ref()
            .map(|view| view.focused_host_id.as_str())
            .filter(|host_id| !host_id.is_empty())
            .unwrap_or(LOCAL_HOST_ID);
        if requested_host != LOCAL_HOST_ID {
            return self
                .handle(request.clone())
                .map(|outcome| (outcome, Vec::new()));
        }
        let deferred_at_start = self.deferred_refresh_tasks.len();
        let previous = self.defer_tracker_refreshes;
        self.defer_tracker_refreshes = true;
        self.observe_live_runs();
        self.now_ms = request
            .get("nowMs")
            .and_then(|value| value.as_u64())
            .unwrap_or_else(refresh::wall_ms);
        self.expire_stale_client_views();
        self.finish_due_pending();
        let due: Vec<String> = self
            .projects
            .iter()
            .map(|project| project.id.clone())
            .filter(|project_id| self.should_auto_refresh(project_id))
            .collect();
        let tasks = due
            .into_iter()
            .filter_map(|project_id| self.begin_refresh_task(&project_id, RefreshTrigger::Interval))
            .collect::<Vec<_>>();
        self.defer_tracker_refreshes = previous;
        let mut deferred = self.deferred_refresh_tasks.split_off(deferred_at_start);
        deferred.extend(tasks);
        let mut outcome = self.outcome();
        if let Some(view) = client_view.as_ref() {
            outcome.snapshot = Box::new(self.snapshot_for_client(
                request.get("clientId").and_then(|value| value.as_str()),
                view,
            ));
        }
        Ok((outcome, deferred))
    }

    pub(crate) fn begin_background_tracker_write_request(
        &mut self,
        request: &serde_json::Value,
    ) -> Result<Option<BackgroundTrackerWriteTask>, KernelError> {
        let requested_host = request
            .get("clientView")
            .and_then(|view| view.get("focusedHostId"))
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .unwrap_or(self.focused_host_id.as_str());
        if requested_host != LOCAL_HOST_ID {
            return Ok(None);
        }
        let op_name = request
            .get("op")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let (project_id, issue_id, op, after_request) = match op_name {
            "claimIssue" | "releaseIssue" => {
                let issue_id = required_string(request, "issueId")?;
                let project_id = self.project_id_for_issue(&issue_id)?;
                let op = if op_name == "claimIssue" {
                    tracker_seam::TrackerWriteOp::Claim
                } else {
                    tracker_seam::TrackerWriteOp::Release
                };
                (project_id, Some(issue_id), op, None)
            }
            "createIssue" => {
                let project_id = required_string(request, "projectId")?;
                if !self.projects.iter().any(|project| project.id == project_id) {
                    return Err(KernelError::Protocol("unknown project".into()));
                }
                let title = required_string(request, "title")?.trim().to_string();
                (
                    project_id,
                    None,
                    tracker_seam::TrackerWriteOp::CreateIssue {
                        title,
                        body: optional_string(request, "body"),
                    },
                    None,
                )
            }
            "updateIssue" => {
                let issue_id = required_string(request, "issueId")?;
                let project_id = self.project_id_for_issue(&issue_id)?;
                let title = required_string(request, "title")?.trim().to_string();
                (
                    project_id,
                    Some(issue_id),
                    tracker_seam::TrackerWriteOp::UpdateIssue {
                        title,
                        body: optional_string(request, "body"),
                    },
                    None,
                )
            }
            "setIssueOpen" => {
                let issue_id = required_string(request, "issueId")?;
                let project_id = self.project_id_for_issue(&issue_id)?;
                let open = request
                    .get("open")
                    .and_then(|value| value.as_bool())
                    .ok_or_else(|| KernelError::Protocol("missing open".into()))?;
                (
                    project_id,
                    Some(issue_id),
                    tracker_seam::TrackerWriteOp::SetOpen { open },
                    None,
                )
            }
            "addIssueComment" => {
                let issue_id = required_string(request, "issueId")?;
                let project_id = self.project_id_for_issue(&issue_id)?;
                let body = required_string(request, "body")?.trim().to_string();
                (
                    project_id,
                    Some(issue_id),
                    tracker_seam::TrackerWriteOp::AddComment { body },
                    None,
                )
            }
            "setIssueParent" => {
                let issue_id = required_string(request, "issueId")?;
                let project_id = self.project_id_for_issue(&issue_id)?;
                let parent = request
                    .get("parent")
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.is_empty())
                    .map(parse_issue_ref)
                    .transpose()?;
                (
                    project_id,
                    Some(issue_id),
                    tracker_seam::TrackerWriteOp::SetParent { parent },
                    None,
                )
            }
            "setIssueBlockedBy" => {
                let issue_id = required_string(request, "issueId")?;
                let project_id = self.project_id_for_issue(&issue_id)?;
                let blocked_by = request
                    .get("blockedBy")
                    .and_then(|value| value.as_array())
                    .ok_or_else(|| KernelError::Protocol("missing blockedBy".into()))?
                    .iter()
                    .map(|value| {
                        value
                            .as_str()
                            .ok_or_else(|| KernelError::Protocol("invalid blockedBy".into()))
                            .and_then(parse_issue_ref)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                (
                    project_id,
                    Some(issue_id),
                    tracker_seam::TrackerWriteOp::SetBlockedBy { blocked_by },
                    None,
                )
            }
            "startBoundRun" => {
                let issue_id = required_string(request, "issueId")?;
                let project_id = self.project_id_for_issue(&issue_id)?;
                (
                    project_id,
                    Some(issue_id),
                    tracker_seam::TrackerWriteOp::Claim,
                    Some(request.clone()),
                )
            }
            "startUnboundRun" => {
                let Some(issue_id) = request
                    .get("issueId")
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned)
                else {
                    return Ok(None);
                };
                let project_id = self.project_id_for_issue(&issue_id)?;
                (
                    project_id,
                    Some(issue_id),
                    tracker_seam::TrackerWriteOp::Claim,
                    Some(request.clone()),
                )
            }
            _ => return Ok(None),
        };
        let refresh = self
            .begin_refresh_task(&project_id, RefreshTrigger::Action)
            .ok_or_else(|| KernelError::Denied(self.write_block_reason(&project_id)))?;
        Ok(Some(BackgroundTrackerWriteTask {
            refresh,
            issue_id,
            op,
            after_request,
        }))
    }

    pub(crate) fn begin_background_project_probe_request(
        &self,
        request: &serde_json::Value,
    ) -> Result<Option<BackgroundProjectProbeTask>, KernelError> {
        let requested_host = request
            .get("clientView")
            .and_then(|view| view.get("focusedHostId"))
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .unwrap_or(self.focused_host_id.as_str());
        if requested_host != LOCAL_HOST_ID {
            return Ok(None);
        }
        let op = request
            .get("op")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let (github_host, repository) = match op {
            "registerProject" => (
                project::normalize_github_host(&optional_string(request, "githubHost"))
                    .map_err(KernelError::Protocol)?,
                project::normalize_repository(&required_string(request, "repository")?)
                    .map_err(KernelError::Protocol)?,
            ),
            "editProject" => {
                let project_id = required_string(request, "projectId")?;
                let github_host =
                    project::normalize_github_host(&optional_string(request, "githubHost"))
                        .map_err(KernelError::Protocol)?;
                let repository =
                    project::normalize_repository(&required_string(request, "repository")?)
                        .map_err(KernelError::Protocol)?;
                let current = self
                    .projects
                    .iter()
                    .find(|project| project.id == project_id)
                    .ok_or_else(|| KernelError::Protocol("unknown project".into()))?;
                let local_path =
                    project::require_local_directory(&required_string(request, "localPath")?)
                        .map_err(KernelError::Protocol)?;
                if current.local_path == local_path
                    && current.github_host == github_host
                    && current.repository == repository
                {
                    return Ok(None);
                }
                (github_host, repository)
            }
            "focusProject" => {
                let project_id = required_string(request, "projectId")?;
                let project = self
                    .projects
                    .iter()
                    .find(|project| project.id == project_id)
                    .ok_or_else(|| KernelError::Protocol("unknown project".into()))?;
                (project.github_host.clone(), project.repository.clone())
            }
            _ => return Ok(None),
        };
        Ok(Some(BackgroundProjectProbeTask {
            request: request.clone(),
            github_host,
            repository,
            host_secrets_path: self.data.host_secrets_path.clone(),
            tracker: Arc::clone(&self.tracker),
            language: self.appearance.language,
        }))
    }

    pub(crate) fn finish_background_project_probe_request(
        &mut self,
        completion: BackgroundProjectProbeCompletion,
    ) -> Result<(CommandOutcome, Vec<BackgroundRefreshTask>), KernelError> {
        let BackgroundProjectProbeCompletion { task, connection } = completion;
        self.precomputed_project_connection = Some((task.github_host, task.repository, connection));
        self.defer_tracker_refreshes = true;
        let result = self.handle(task.request);
        self.defer_tracker_refreshes = false;
        self.precomputed_project_connection = None;
        let tasks = std::mem::take(&mut self.deferred_refresh_tasks);
        result.map(|outcome| (outcome, tasks))
    }

    pub(crate) fn finish_background_tracker_write_request(
        &mut self,
        request: &serde_json::Value,
        completion: BackgroundTrackerWriteCompletion,
    ) -> BackgroundTrackerWriteFinish {
        let project_id = completion.refresh.task.project_id.clone();
        let rollback_seed = (
            completion.refresh.task.github_host.clone(),
            completion.refresh.task.repository.clone(),
            completion.refresh.task.host_secrets_path.clone(),
            Arc::clone(&completion.refresh.task.tracker),
        );
        let issue_id = completion.issue_id;
        let op = completion.op;
        let write_result = completion.write_result;
        let after_request = completion.after_request;
        if !self.finish_refresh_task(completion.refresh) {
            return BackgroundTrackerWriteFinish {
                result: Err(KernelError::Denied(self.write_block_reason(&project_id))),
                rollback: None,
            };
        }
        let updated = match write_result {
            Some(Ok(updated)) => updated,
            Some(Err(error)) => {
                return BackgroundTrackerWriteFinish {
                    result: Err(write_tracker_error(error)),
                    rollback: None,
                };
            }
            None => {
                return BackgroundTrackerWriteFinish {
                    result: Err(KernelError::Denied(self.write_block_reason(&project_id))),
                    rollback: None,
                };
            }
        };
        if let Some(expected_issue_id) = issue_id.as_deref() {
            if updated.id() != expected_issue_id {
                return BackgroundTrackerWriteFinish {
                    result: Err(KernelError::Protocol(
                        "tracker returned a different Issue".into(),
                    )),
                    rollback: None,
                };
            }
        }
        self.merge_issue(&project_id, updated, &op);
        if let Some(after_request) = after_request {
            self.preclaimed_issue_id = issue_id.clone();
            let result = self.handle(after_request);
            self.preclaimed_issue_id = None;
            let launched = issue_id
                .as_deref()
                .is_some_and(|issue_id| self.active_run_id_for_issue(issue_id).is_some());
            let rollback = (!launched).then(|| {
                let (github_host, repository, host_secrets_path, tracker) = rollback_seed;
                BackgroundClaimRollbackTask {
                    project_id,
                    issue_id: issue_id.expect("launch write always has an Issue"),
                    github_host,
                    repository,
                    host_secrets_path,
                    tracker,
                }
            });
            BackgroundTrackerWriteFinish { result, rollback }
        } else {
            BackgroundTrackerWriteFinish {
                result: self.outcome_for_request(request),
                rollback: None,
            }
        }
    }

    pub(crate) fn finish_background_claim_rollback(
        &mut self,
        request: &serde_json::Value,
        completion: BackgroundClaimRollbackCompletion,
    ) -> Result<CommandOutcome, KernelError> {
        self.apply_background_claim_rollback(completion)?;
        self.outcome_for_request(request)
    }

    fn apply_background_claim_rollback(
        &mut self,
        completion: BackgroundClaimRollbackCompletion,
    ) -> Result<(), KernelError> {
        let BackgroundClaimRollbackCompletion { task, result } = completion;
        let project_is_current = self.projects.iter().any(|project| {
            project.id == task.project_id
                && project.github_host == task.github_host
                && project.repository == task.repository
        });
        if !project_is_current {
            return Ok(());
        }
        let updated = result.map_err(write_tracker_error)?;
        if updated.id() != task.issue_id {
            return Err(KernelError::Protocol(
                "tracker returned a different Issue".into(),
            ));
        }
        self.merge_issue(
            &task.project_id,
            updated,
            &tracker_seam::TrackerWriteOp::Release,
        );
        Ok(())
    }

    fn outcome_for_request(
        &mut self,
        request: &serde_json::Value,
    ) -> Result<CommandOutcome, KernelError> {
        let mut outcome = self.outcome();
        if let Some(view) = request
            .get("clientView")
            .cloned()
            .map(serde_json::from_value::<ClientSnapshotView>)
            .transpose()?
            .as_ref()
        {
            outcome.snapshot = Box::new(self.snapshot_for_client(
                request.get("clientId").and_then(|value| value.as_str()),
                view,
            ));
        }
        Ok(outcome)
    }

    fn handle_inner(&mut self, request: serde_json::Value) -> Result<CommandOutcome, KernelError> {
        let op = request
            .get("op")
            .and_then(|value| value.as_str())
            .unwrap_or("snapshot");
        match op {
            "snapshot" => {
                self.observe_live_runs();
                Ok(self.outcome())
            }
            "updateInstallGate" => {
                let gate = self.update_install_gate();
                let mut outcome = self.outcome();
                outcome.update_install_gate = Some(gate);
                Ok(outcome)
            }
            "beginUpdateInstall" => {
                let gate = self.update_install_gate();
                if gate.allowed {
                    self.update_installing = true;
                }
                let mut outcome = self.outcome();
                outcome.update_install_gate = Some(gate);
                Ok(outcome)
            }
            "cancelUpdateInstall" => {
                self.update_installing = false;
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
            "forgetRemoteHost" => self.dispatch(Command::ForgetRemoteHost {
                host_id: required_string(&request, "hostId")?,
            }),
            "focusHost" => {
                let host_id = request
                    .get("hostId")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| KernelError::Protocol("missing hostId".into()))?
                    .to_string();
                self.dispatch(Command::FocusHost { host_id })
            }
            "registerProject" => self.dispatch(Command::RegisterProject {
                name: required_string(&request, "name")?,
                local_path: required_string(&request, "localPath")?,
                github_host: optional_string(&request, "githubHost"),
                repository: required_string(&request, "repository")?,
            }),
            "editProject" => self.dispatch(Command::EditProject {
                project_id: required_string(&request, "projectId")?,
                name: required_string(&request, "name")?,
                local_path: required_string(&request, "localPath")?,
                github_host: optional_string(&request, "githubHost"),
                repository: required_string(&request, "repository")?,
            }),
            "removeProject" => self.dispatch(Command::RemoveProject {
                project_id: required_string(&request, "projectId")?,
            }),
            "focusProject" => self.dispatch(Command::FocusProject {
                project_id: required_string(&request, "projectId")?,
            }),
            "inferProject" => self.dispatch(Command::InferProject {
                local_path: required_string(&request, "localPath")?,
            }),
            "focusIssue" => self.dispatch(Command::FocusIssue {
                issue_id: required_string(&request, "issueId")?,
            }),
            "loadIssueDocument" => {
                self.observe_live_runs();
                let issue_id = required_string(&request, "issueId")?;
                let project_id = self.project_id_for_issue_request(&request, &issue_id)?;
                self.load_issue_document(&project_id, &issue_id)?;
                Ok(self.outcome())
            }
            "filterParent" => self.dispatch(Command::FilterParent {
                issue_id: required_string(&request, "issueId")?,
            }),
            "clearParentFilter" => self.dispatch(Command::ClearParentFilter),
            "setCenterView" => {
                let view = serde_json::from_value(
                    request
                        .get("view")
                        .cloned()
                        .ok_or_else(|| KernelError::Protocol("missing view".into()))?,
                )?;
                self.dispatch(Command::SetCenterView { view })
            }
            "centerDependencyGraph" => self.dispatch(Command::CenterDependencyGraph {
                issue_id: required_string(&request, "issueId")?,
            }),
            "setDependencyGraphComplete" => self.dispatch(Command::SetDependencyGraphComplete {
                complete: request
                    .get("complete")
                    .and_then(|value| value.as_bool())
                    .ok_or_else(|| KernelError::Protocol("missing complete".into()))?,
            }),
            "setShowClosedGraphContext" => self.dispatch(Command::SetDependencyGraphComplete {
                complete: request
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
            "refreshLaunchEnvironment" => {
                if self.focused_host_id != LOCAL_HOST_ID {
                    return Err(KernelError::Denied(
                        "switch to this machine to reread its launch environment".into(),
                    ));
                }
                self.dispatch(Command::RefreshLaunchEnvironment)
            }
            "searchIssues" => {
                let triage_role = request
                    .get("triageRole")
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.is_empty())
                    .map(|value| serde_json::from_value(serde_json::Value::String(value.into())))
                    .transpose()
                    .map_err(|_| KernelError::Protocol("invalid triageRole".into()))?;
                let state = request
                    .get("state")
                    .and_then(|value| value.as_str())
                    .map(|value| serde_json::from_value(serde_json::Value::String(value.into())))
                    .transpose()
                    .map_err(|_| KernelError::Protocol("invalid state".into()))?
                    .unwrap_or_default();
                self.dispatch(Command::SearchIssues {
                    project_id: required_string(&request, "projectId")?,
                    search: IssueSearch {
                        title: optional_string(&request, "title"),
                        triage_role,
                        state,
                    },
                })
            }
            "refresh" => self.dispatch(Command::Refresh {
                project_id: request
                    .get("projectId")
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned),
            }),
            "tick" => {
                let now_ms = request
                    .get("nowMs")
                    .and_then(|value| value.as_u64())
                    .unwrap_or_else(refresh::wall_ms);
                self.now_ms = now_ms;
                if let Some(client_id) = request
                    .get("clientId")
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.is_empty())
                {
                    self.set_client_view(
                        client_id,
                        &optional_string(&request, "projectId"),
                        request
                            .get("visible")
                            .and_then(|value| value.as_bool())
                            .unwrap_or(false),
                    );
                }
                self.dispatch(Command::Tick {
                    now_ms: Some(now_ms),
                })
            }
            "setClientView" => self.dispatch(Command::SetClientView {
                client_id: required_string(&request, "clientId")?,
                project_id: optional_string(&request, "projectId"),
                visible: request
                    .get("visible")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false),
            }),
            "noteRunEnded" => self.dispatch(Command::NoteRunEnded {
                project_id: required_string(&request, "projectId")?,
            }),
            "claimIssue" => self.dispatch(Command::ClaimIssue {
                issue_id: required_string(&request, "issueId")?,
            }),
            "releaseIssue" => self.dispatch(Command::ReleaseIssue {
                issue_id: required_string(&request, "issueId")?,
            }),
            "createIssue" => self.dispatch(Command::CreateIssue {
                project_id: required_string(&request, "projectId")?,
                title: required_string(&request, "title")?,
                body: optional_string(&request, "body"),
            }),
            "updateIssue" => self.dispatch(Command::UpdateIssue {
                issue_id: required_string(&request, "issueId")?,
                title: required_string(&request, "title")?,
                body: optional_string(&request, "body"),
            }),
            "setIssueOpen" => self.dispatch(Command::SetIssueOpen {
                issue_id: required_string(&request, "issueId")?,
                open: request
                    .get("open")
                    .and_then(|value| value.as_bool())
                    .ok_or_else(|| KernelError::Protocol("missing open".into()))?,
            }),
            "addIssueComment" => self.dispatch(Command::AddIssueComment {
                issue_id: required_string(&request, "issueId")?,
                body: required_string(&request, "body")?,
            }),
            "setIssueParent" => {
                let parent = request
                    .get("parent")
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned);
                if let Some(id) = parent.as_deref() {
                    parse_issue_ref(id)?;
                }
                self.dispatch(Command::SetIssueParent {
                    issue_id: required_string(&request, "issueId")?,
                    parent,
                })
            }
            "setIssueBlockedBy" => {
                let blocked_by = request
                    .get("blockedBy")
                    .and_then(|value| value.as_array())
                    .ok_or_else(|| KernelError::Protocol("missing blockedBy".into()))?
                    .iter()
                    .map(|item| {
                        item.as_str()
                            .map(ToOwned::to_owned)
                            .ok_or_else(|| KernelError::Protocol("invalid blockedBy".into()))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                for id in &blocked_by {
                    parse_issue_ref(id)?;
                }
                self.dispatch(Command::SetIssueBlockedBy {
                    issue_id: required_string(&request, "issueId")?,
                    blocked_by,
                })
            }
            "autoAdvance" => self.dispatch(Command::AutoAdvance {
                project_id: required_string(&request, "projectId")?,
            }),
            "checkIssueClosed" => self.dispatch(Command::CheckIssueClosed {
                issue_id: required_string(&request, "issueId")?,
            }),
            "startBoundRun" => self.dispatch(Command::StartBoundRun {
                issue_id: required_string(&request, "issueId")?,
            }),
            "continueRun" => self.dispatch(Command::ContinueRun {
                issue_id: required_string(&request, "issueId")?,
            }),
            "prepareRunLaunch" => self.dispatch(Command::PrepareRunLaunch {
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
            }),
            "cancelRunLaunch" => self.dispatch(Command::CancelRunLaunch),
            "startUnboundRun" => {
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
            "setNotificationPrefs" => self.dispatch(Command::SetNotificationPrefs {
                desktop: request
                    .get("desktop")
                    .and_then(|value| value.as_bool())
                    .ok_or_else(|| KernelError::Protocol("missing desktop".into()))?,
                sound: request
                    .get("sound")
                    .and_then(|value| value.as_bool())
                    .ok_or_else(|| KernelError::Protocol("missing sound".into()))?,
            }),
            "stopRun" => self.dispatch(Command::StopRun {
                run_id: required_string(&request, "runId")?,
            }),
            "focusRun" => self.dispatch(Command::FocusRun {
                run_id: required_string(&request, "runId")?,
            }),
            "openHostOverview" => self.dispatch(Command::OpenHostOverview),
            "returnToBoard" => self.dispatch(Command::ReturnToBoard),
            "injectRunInput" => self.dispatch(Command::InjectRunInput {
                run_id: required_string(&request, "runId")?,
                text: request
                    .get("text")
                    .and_then(|value| value.as_str())
                    .unwrap_or("")
                    .to_string(),
            }),
            "cancelQuit" => self.dispatch(Command::CancelQuit),
            "confirmQuitStopAll" => self.dispatch(Command::ConfirmQuitStopAll),
            "setRefreshInterval" => self.dispatch(Command::SetRefreshInterval {
                interval_ms: request
                    .get("intervalMs")
                    .and_then(|value| value.as_u64())
                    .ok_or_else(|| KernelError::Protocol("missing intervalMs".into()))?,
            }),
            "viewChanges" => {
                let scope =
                    ChangeScope::parse(request.get("scope").and_then(|value| value.as_str()))
                        .map_err(KernelError::Protocol)?;
                let view = self.view_changes(
                    request
                        .get("runId")
                        .and_then(|value| value.as_str())
                        .filter(|value| !value.is_empty()),
                    request
                        .get("issueId")
                        .and_then(|value| value.as_str())
                        .filter(|value| !value.is_empty()),
                    scope,
                )?;
                let mut outcome = self.outcome();
                outcome.view_changes = Some(view);
                Ok(outcome)
            }
            "writeChangeNote" => self.dispatch(Command::WriteChangeNote {
                run_id: required_string(&request, "runId")?,
                repo: optional_string(&request, "repo"),
                path: required_string(&request, "path")?,
                line: request
                    .get("line")
                    .and_then(|value| value.as_u64())
                    .ok_or_else(|| KernelError::Protocol("missing line".into()))?
                    as u32,
                text: required_string(&request, "text")?,
            }),
            "deleteChangeNote" => self.dispatch(Command::DeleteChangeNote {
                note_id: required_string(&request, "noteId")?,
            }),
            "setHostAutoAdvance" => self.dispatch(Command::SetHostAutoAdvance {
                enabled: request
                    .get("enabled")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false),
            }),
            "setProjectAutoAdvance" => self.dispatch(Command::SetProjectAutoAdvance {
                project_id: required_string(&request, "projectId")?,
                enabled: request
                    .get("enabled")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false),
            }),
            "setProjectRestoreAutoAdvance" => {
                self.dispatch(Command::SetProjectRestoreAutoAdvance {
                    project_id: required_string(&request, "projectId")?,
                    enabled: request
                        .get("enabled")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false),
                })
            }
            "setProjectRestoreDelay" => self.dispatch(Command::SetProjectRestoreDelay {
                project_id: required_string(&request, "projectId")?,
                delay_ms: request
                    .get("delayMs")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(advance::DEFAULT_RESTORE_DELAY_MS),
            }),
            "vetoPendingConfirmation" => self.dispatch(Command::VetoPendingConfirmation {
                project_id: required_string(&request, "projectId")?,
            }),
            "openUsage" => self.dispatch(Command::OpenUsage),
            "closeUsage" => self.dispatch(Command::CloseUsage),
            "setUsageRange" => {
                let range = serde_json::from_value(
                    request
                        .get("range")
                        .cloned()
                        .ok_or_else(|| KernelError::Protocol("missing range".into()))?,
                )?;
                self.dispatch(Command::SetUsageRange {
                    range,
                    custom_from_ms: request.get("fromMs").and_then(|value| value.as_u64()),
                    custom_to_ms: request.get("toMs").and_then(|value| value.as_u64()),
                })
            }
            "setUsageFilter" => self.dispatch(Command::SetUsageFilter {
                project_id: request
                    .get("projectId")
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned),
                agent_id: request
                    .get("agentId")
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned),
                model: request
                    .get("model")
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned),
            }),
            "openUsageForRun" => self.dispatch(Command::OpenUsageForRun {
                run_id: required_string(&request, "runId")?,
            }),
            "openRunFromUsage" => self.dispatch(Command::OpenRunFromUsage {
                run_id: required_string(&request, "runId")?,
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
            notify_desktop: self.notify_desktop,
            notify_sound: self.notify_sound,
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
        let mut hosts = if self.host_mode == HostMode::HostAndClient {
            vec![HostSummary {
                id: LOCAL_HOST_ID.to_string(),
                display_name: self.host_display_name.clone(),
                local: true,
            }]
        } else {
            Vec::new()
        };
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
            .map(|project| {
                project.summary(
                    self.project_has_active_run(&project.id),
                    self.project_has_execution_stopped(&project.id),
                    self.project_issue_counts(&project.id),
                )
            })
            .collect();
        let focused_project_id = self.focused_project_id.clone().unwrap_or_default();
        (projects, focused_project_id, empty_actions)
    }

    fn project_issue_counts(&self, project_id: &str) -> ProjectIssueCounts {
        let refresh = self.refresh_status_for(project_id);
        board::project_issue_counts(
            self.loaded_issues.get(project_id).map(Vec::as_slice),
            &refresh,
        )
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
            KernelError::Io(_) => KernelError::Protocol("address is not reachable".into()),
            other => other,
        })?;
        self.store_remote_view_response(host_id, None, &response)
    }

    fn store_remote_view_response(
        &mut self,
        host_id: &str,
        client_id: Option<&str>,
        response: &serde_json::Value,
    ) -> Result<(), KernelError> {
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
        let workspace_view = snapshot
            .get("workspaceView")
            .cloned()
            .map(serde_json::from_value)
            .transpose()?
            .unwrap_or(WorkspaceView::Project);
        let quit_offer = match snapshot.get("quitOffer") {
            Some(value) if !value.is_null() => serde_json::from_value(value.clone())?,
            _ => None,
        };
        let remote_view = RemoteView {
            host_id: host_id.to_string(),
            projects,
            focused_project_id,
            empty_actions,
            board,
            runs,
            focused_run_id,
            workspace_view,
            quit_offer,
            launch_form: match snapshot.get("launchForm") {
                Some(value) if !value.is_null() => serde_json::from_value(value.clone())?,
                _ => None,
            },
            usage_open: snapshot
                .get("usageOpen")
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
            usage: match snapshot.get("usage") {
                Some(value) if !value.is_null() => serde_json::from_value(value.clone())?,
                _ => usage::build_usage_page(&usage::UsageQuery::default(), 0, 0, &[], &[]),
            },
            refresh_interval_ms: snapshot
                .get("refreshIntervalMs")
                .and_then(|value| value.as_u64())
                .map(refresh::clamp_refresh_interval_ms)
                .unwrap_or(refresh::DEFAULT_REFRESH_INTERVAL_MS),
            auto_advance: snapshot
                .get("autoAdvance")
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
            pending_confirmation: match snapshot.get("pendingConfirmation") {
                Some(value) if !value.is_null() => serde_json::from_value(value.clone())?,
                _ => None,
            },
        };
        if let Some(client_id) = client_id.filter(|client_id| !client_id.is_empty()) {
            self.remote_client_views
                .entry(client_id.to_string())
                .or_default()
                .insert(host_id.to_string(), remote_view.clone());
        }
        self.remote_view = Some(remote_view);
        Ok(())
    }

    fn refresh_interval_for_focus(&self) -> u64 {
        if self.focused_host_id != LOCAL_HOST_ID {
            if let Some(view) = &self.remote_view {
                if view.host_id == self.focused_host_id {
                    return view.refresh_interval_ms;
                }
            }
        }
        self.refresh_interval_ms
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

    fn runs_for_focus(&self) -> (Vec<RunSummary>, String, WorkspaceView, Option<QuitOffer>) {
        if self.focused_host_id != LOCAL_HOST_ID {
            if let Some(view) = &self.remote_view {
                if view.host_id == self.focused_host_id {
                    return (
                        view.runs.clone(),
                        view.focused_run_id.clone(),
                        view.workspace_view,
                        view.quit_offer.clone(),
                    );
                }
            }
        }
        (
            self.decorate_runs(&self.runs),
            self.focused_run_id.clone().unwrap_or_default(),
            self.workspace_view,
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

    fn refresh_launch_environment(&mut self) -> Result<LaunchEnvironmentStatus, KernelError> {
        let directories = if self.projects.is_empty() {
            std::env::var_os("HOME")
                .or_else(|| std::env::var_os("USERPROFILE"))
                .map(PathBuf::from)
                .into_iter()
                .collect::<Vec<_>>()
        } else {
            self.projects
                .iter()
                .map(|project| project.local_path.clone())
                .collect::<Vec<_>>()
        };
        if directories.is_empty() {
            return Ok(LaunchEnvironmentStatus {
                status: "ready",
                refreshed_directories: 0,
                message: None,
            });
        }

        let mut failures = Vec::new();
        let mut refreshed = 0;
        for directory in directories {
            match self.launch_env.refresh(&directory) {
                Ok(_) => refreshed += 1,
                Err(err) => failures.push(format!("{}: {err}", directory.display())),
            }
        }
        if failures.is_empty() {
            Ok(LaunchEnvironmentStatus {
                status: "ready",
                refreshed_directories: refreshed,
                message: None,
            })
        } else {
            Err(KernelError::Denied(failures.join("\n")))
        }
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
        let pending = changes::pending_notes(&self.change_notes, project_id, issue_id.as_deref());
        let change_notes_text = changes::format_notes(&pending);
        opening_text = changes::append_notes(&opening_text, &pending);
        let fields = launch::localize_fields(agent.config_fields(), language);
        values.insert(launch::ISOLATION_FIELD.into(), "false".into());
        let (isolation_supported, isolation_reason) =
            launch::isolation_availability(agent.as_ref(), &project.local_path, language);
        let preview = launch::command_preview(&launch::preview_argv(agent.as_ref(), &values));
        let remembered_available = last.as_deref().is_some_and(|id| {
            agents
                .iter()
                .any(|candidate| candidate.id == id && candidate.installed)
        });
        let explicit_available = agent_id.as_deref().is_some_and(|id| {
            agents
                .iter()
                .any(|candidate| candidate.id == id && candidate.installed)
        });
        let skip_agent_picker = !pick_agent && (remembered_available || explicit_available);
        let mut warnings = launch::unknown_enum_warnings(&fields, &values, language);
        warnings.extend(launch::side_effect_warnings(
            &project.local_path,
            self.runs
                .iter()
                .any(|run| run.project_id == project_id && run.is_active()),
            language,
        ));
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
            isolation_supported,
            isolation_reason,
            opening_text,
            change_notes_text,
            command_preview: preview,
            intents: launch::intent_options(language),
            warnings,
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
        mut config: RunLaunchConfig,
        issue_id: Option<String>,
        from_form: bool,
        previous: Option<PreviousRun>,
    ) -> Result<(), KernelError> {
        if self.update_installing {
            return Err(KernelError::Denied("update install is starting".into()));
        }
        let project_dir = self
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
        let (supported, _) = launch::isolation_availability(agent.as_ref(), &project_dir, language);
        let mut cwd = project_dir.clone();
        let mut isolation_note = None;
        let mut isolate = false;
        if let Some(previous) = &previous {
            config.values.remove(launch::ISOLATION_FIELD);
            if previous.isolated {
                let recorded = PathBuf::from(&previous.working_directory);
                if !previous.working_directory.is_empty() && recorded.exists() {
                    cwd = recorded;
                } else {
                    isolation_note = Some(launch::isolation_missing_tree_note(language));
                }
            }
        } else {
            isolate = launch::isolation_requested(&config.values) && supported;
            if !isolate {
                config
                    .values
                    .insert(launch::ISOLATION_FIELD.into(), "false".into());
            }
        }
        let fields = launch::localize_fields(agent.config_fields(), language);
        if let Some(form) = &mut self.launch_form {
            launch::apply_submitted_form(form, &config);
            let mut warnings = launch::unknown_enum_warnings(&fields, &config.values, language);
            warnings.extend(launch::side_effect_warnings(
                &project_dir,
                self.runs
                    .iter()
                    .any(|run| run.project_id == project_id && run.is_active()),
                language,
            ));
            form.warnings = warnings;
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
        if !from_form {
            let pending =
                changes::pending_notes(&self.change_notes, project_id, issue_id.as_deref());
            config.opening_text = changes::append_notes(&config.opening_text, &pending);
        }
        let (previous_run_id, resume_session_id) = match &previous {
            Some(previous) => (
                Some(previous.id.clone()),
                previous.native_session_id.clone(),
            ),
            None => (None, None),
        };
        if let Some(issue_id) = issue_id.as_deref() {
            if self.active_run_id_for_issue(issue_id).is_some() {
                return Err(KernelError::Denied(
                    "issue already has an active Run".into(),
                ));
            }
            if previous_run_id.is_none() {
                if let Err(err) = self.claim_issue(issue_id) {
                    if let Some(form) = &mut self.launch_form {
                        form.error = Some(err.to_string());
                        return Ok(());
                    }
                    return Err(err);
                }
            }
        }
        let before = if isolate {
            launch::git_worktrees(&project_dir)
        } else {
            Vec::new()
        };
        let early_baselines = if isolate || !cwd.exists() {
            None
        } else {
            Some(changes::record_baselines(&cwd))
        };
        let mut hook_plan = None;
        let mut hook_dir = None;
        if issue_id.is_some() && agent.completion_hooks_supported() {
            let dir = self
                .data
                .host_dir
                .join("projects")
                .join(project_id)
                .join("hooks")
                .join(pairing::random_id());
            if let Ok(mut plan) = agent.attach_completion_hooks(&dir, &project_dir) {
                plan.extra_env
                    .entry("AGENT_TASKBOARD_HOOK_SINK".into())
                    .or_insert_with(|| dir.to_string_lossy().into_owned());
                hook_dir = Some(dir);
                hook_plan = Some(plan);
            }
        }
        let mut result = run::start_unbound(
            project_id,
            &cwd,
            agent.as_ref(),
            self.launch_env.as_ref(),
            self.sessions.as_ref(),
            language,
            &[],
            &config,
            issue_id.as_deref(),
            previous_run_id.as_deref(),
            resume_session_id.as_deref(),
            hook_plan.as_ref(),
        );
        result.record.hook_dir = hook_dir;
        result.record.hooks_attached = hook_plan.is_some();
        let using_recorded_tree = previous.as_ref().is_some_and(|previous| previous.isolated)
            && isolation_note.is_none()
            && cwd != project_dir;
        result.record.working_directory = cwd.display().to_string();
        result.record.isolated = (isolate && result.session.is_some()) || using_recorded_tree;
        result.record.isolation_note = isolation_note;
        if isolate && result.session.is_some() {
            if let Some(tree) = agent
                .isolation_tree_after_launch(&project_dir, &before)
                .or_else(|| launch::new_git_worktree(&project_dir, &before))
            {
                result.record.working_directory = tree.display().to_string();
            }
        }
        result.record.started_at_ms = self.now_ms;
        result.record.git_baselines = if isolate {
            let recorded_cwd = PathBuf::from(&result.record.working_directory);
            if recorded_cwd.exists() {
                changes::record_baselines(&recorded_cwd)
            } else {
                Vec::new()
            }
        } else {
            early_baselines.unwrap_or_default()
        };
        self.focused_run_id = Some(result.record.id.clone());
        self.pending_events.push(HostEvent::RunStatusChanged {
            run_id: result.record.id.clone(),
            status: result.record.status,
        });
        if let Some(session) = result.session {
            self.live.insert(result.record.id.clone(), session);
            if let Some(issue_id) = issue_id.as_deref() {
                self.selected_issue_id = Some(issue_id.to_string());
            }
            self.remember_launch(project_id, &config)?;
            self.launch_form = None;
            self.clear_pending_notes(project_id, issue_id.as_deref())?;
        } else if let Some(form) = &mut self.launch_form {
            form.error = result.record.failure.clone();
        }
        self.runs.push(result.record);
        self.persist_runs()?;
        Ok(())
    }

    fn start_bound_run(&mut self, issue_id: &str) -> Result<(), KernelError> {
        let project_id = self.project_id_for_issue(issue_id)?;
        let issue = self
            .issue_by_id(issue_id)
            .ok_or_else(|| KernelError::Protocol("unknown issue".into()))?;
        let agent = self.default_agent_for_project(&project_id)?;
        let mut values = self
            .launch_defaults
            .get(&project_id)
            .and_then(|agents| agents.get(agent.id()))
            .cloned()
            .unwrap_or_else(|| agent.seed_config());
        let opening = format!("{}\n{}", issue.title, issue.url);
        values.insert(launch::INITIAL_INSTRUCTION.into(), opening.clone());
        self.start_unbound_run(
            &project_id,
            RunLaunchConfig {
                agent_id: agent.id().to_string(),
                values,
                opening_text: opening,
            },
            Some(issue_id.to_string()),
            false,
            None,
        )
    }

    fn continue_run(&mut self, issue_id: &str) -> Result<(), KernelError> {
        if !self.execution_stopped(issue_id) {
            return Err(KernelError::Denied("issue is not execution-stopped".into()));
        }
        let last = self
            .last_bound_run(issue_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown run".into()))?;
        let agent = self
            .agents
            .iter()
            .find(|agent| agent.id() == last.agent_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown Agent Adapter".into()))?;
        let values = self
            .launch_defaults
            .get(&last.project_id)
            .and_then(|agents| agents.get(&last.agent_id))
            .cloned()
            .unwrap_or_else(|| agent.seed_config());
        self.start_unbound_run(
            &last.project_id,
            RunLaunchConfig {
                agent_id: last.agent_id.clone(),
                values,
                opening_text: String::new(),
            },
            Some(issue_id.to_string()),
            false,
            Some(PreviousRun {
                id: last.id.clone(),
                native_session_id: last.native_session_id.clone(),
                working_directory: last.working_directory.clone(),
                isolated: last.isolated,
            }),
        )
    }

    fn claim_issue(&mut self, issue_id: &str) -> Result<(), KernelError> {
        if self.preclaimed_issue_id.as_deref() == Some(issue_id) {
            self.preclaimed_issue_id = None;
            return Ok(());
        }
        self.require_live_tracker_for_issue(issue_id)?;
        self.write_claim(issue_id, true)
    }

    fn release_issue(&mut self, issue_id: &str) -> Result<(), KernelError> {
        self.require_live_tracker_for_issue(issue_id)?;
        self.write_claim(issue_id, false)
    }

    fn write_claim(&mut self, issue_id: &str, claim: bool) -> Result<(), KernelError> {
        let project_id = self.project_id_for_issue(issue_id)?;
        let op = if claim {
            tracker_seam::TrackerWriteOp::Claim
        } else {
            tracker_seam::TrackerWriteOp::Release
        };
        self.write_issue_op(&project_id, Some(issue_id), op)?;
        Ok(())
    }

    fn create_issue(
        &mut self,
        project_id: &str,
        title: &str,
        body: &str,
    ) -> Result<(), KernelError> {
        let title = title.trim();
        if title.is_empty() {
            return Err(KernelError::Protocol("missing title".into()));
        }
        if !self.projects.iter().any(|project| project.id == project_id) {
            return Err(KernelError::Protocol("unknown project".into()));
        }
        self.require_live_tracker(project_id)?;
        self.write_issue_op(
            project_id,
            None,
            tracker_seam::TrackerWriteOp::CreateIssue {
                title: title.to_string(),
                body: body.to_string(),
            },
        )?;
        Ok(())
    }

    fn update_issue(&mut self, issue_id: &str, title: &str, body: &str) -> Result<(), KernelError> {
        let title = title.trim();
        if title.is_empty() {
            return Err(KernelError::Protocol("missing title".into()));
        }
        self.require_live_tracker_for_issue(issue_id)?;
        self.write_issue_op(
            &self.project_id_for_issue(issue_id)?,
            Some(issue_id),
            tracker_seam::TrackerWriteOp::UpdateIssue {
                title: title.to_string(),
                body: body.to_string(),
            },
        )?;
        Ok(())
    }

    fn set_issue_open(&mut self, issue_id: &str, open: bool) -> Result<(), KernelError> {
        self.require_live_tracker_for_issue(issue_id)?;
        self.write_issue_op(
            &self.project_id_for_issue(issue_id)?,
            Some(issue_id),
            tracker_seam::TrackerWriteOp::SetOpen { open },
        )?;
        Ok(())
    }

    fn add_issue_comment(&mut self, issue_id: &str, body: &str) -> Result<(), KernelError> {
        let body = body.trim();
        if body.is_empty() {
            return Err(KernelError::Protocol("missing body".into()));
        }
        self.require_live_tracker_for_issue(issue_id)?;
        self.write_issue_op(
            &self.project_id_for_issue(issue_id)?,
            Some(issue_id),
            tracker_seam::TrackerWriteOp::AddComment {
                body: body.to_string(),
            },
        )?;
        Ok(())
    }

    fn set_issue_parent(
        &mut self,
        issue_id: &str,
        parent: Option<&str>,
    ) -> Result<(), KernelError> {
        self.require_live_tracker_for_issue(issue_id)?;
        let parent = parent.map(parse_issue_ref).transpose()?;
        self.write_issue_op(
            &self.project_id_for_issue(issue_id)?,
            Some(issue_id),
            tracker_seam::TrackerWriteOp::SetParent { parent },
        )?;
        Ok(())
    }

    fn set_issue_blocked_by(
        &mut self,
        issue_id: &str,
        blocked_by: &[String],
    ) -> Result<(), KernelError> {
        self.require_live_tracker_for_issue(issue_id)?;
        let blocked_by = blocked_by
            .iter()
            .map(|id| parse_issue_ref(id))
            .collect::<Result<Vec<_>, _>>()?;
        self.write_issue_op(
            &self.project_id_for_issue(issue_id)?,
            Some(issue_id),
            tracker_seam::TrackerWriteOp::SetBlockedBy { blocked_by },
        )?;
        Ok(())
    }

    fn write_issue_op(
        &mut self,
        project_id: &str,
        issue_id: Option<&str>,
        op: tracker_seam::TrackerWriteOp,
    ) -> Result<IssueRecord, KernelError> {
        let project = self
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown project".into()))?;
        let pat = read_github_pat(&self.data.host_secrets_path, &project.github_host);
        let ctx = tracker::ProbeContext {
            github_host: &project.github_host,
            repository: &project.repository,
            secrets_pat: pat.as_deref(),
            secrets_path: &self.data.host_secrets_path,
        };
        let updated = self
            .tracker
            .write_issue(&ctx, issue_id, &op)
            .map_err(write_tracker_error)?;
        self.merge_issue(project_id, updated.clone(), &op);
        Ok(updated)
    }

    fn merge_issue(
        &mut self,
        project_id: &str,
        updated: IssueRecord,
        op: &tracker_seam::TrackerWriteOp,
    ) {
        let issues = self
            .loaded_issues
            .entry(project_id.to_string())
            .or_default();
        if let Some(existing) = issues.iter_mut().find(|issue| issue.id() == updated.id()) {
            // 按操作合并对应字段，避免用适配器响应里缺失的关系字段覆盖本地已知数据。
            match op {
                tracker_seam::TrackerWriteOp::CreateIssue { .. } => {}
                tracker_seam::TrackerWriteOp::UpdateIssue { .. } => {
                    existing.title = updated.title;
                    existing.url = updated.url;
                }
                tracker_seam::TrackerWriteOp::SetOpen { .. } => {
                    existing.open = updated.open;
                    existing.closed_at = updated.closed_at;
                }
                tracker_seam::TrackerWriteOp::AddComment { .. } => {}
                tracker_seam::TrackerWriteOp::Claim | tracker_seam::TrackerWriteOp::Release => {
                    existing.assignees = updated.assignees;
                    existing.open = updated.open;
                    existing.title = updated.title;
                    existing.url = updated.url;
                    existing.closed_at = updated.closed_at;
                }
                tracker_seam::TrackerWriteOp::SetParent { .. } => {
                    existing.parent = updated.parent;
                }
                tracker_seam::TrackerWriteOp::SetBlockedBy { .. } => {
                    existing.blocked_by = updated.blocked_by;
                }
            }
        } else {
            issues.push(updated);
        }
        self.persist_tracker_snapshot(project_id);
        self.pending_events.push(HostEvent::BoardUpdated {
            project_id: project_id.to_string(),
        });
    }

    fn stored_issue_documents(
        &self,
        project_id: &str,
    ) -> BTreeMap<String, refresh::StoredIssueDocument> {
        self.issue_documents
            .get(project_id)
            .into_iter()
            .flat_map(|documents| documents.iter())
            .filter_map(|(issue_id, state)| {
                issue_document_body(Some(state)).map(|(body, fetched_at_ms)| {
                    (
                        issue_id.clone(),
                        refresh::StoredIssueDocument {
                            body,
                            fetched_at_ms,
                        },
                    )
                })
            })
            .collect()
    }

    fn persist_tracker_snapshot(&self, project_id: &str) {
        let Some(state) = self.refresh.get(project_id) else {
            return;
        };
        let Some(fetched_at_ms) = state.fetched_at_ms else {
            return;
        };
        let Some(issues) = self.loaded_issues.get(project_id) else {
            return;
        };
        let _ = refresh::save_snapshot(
            &refresh::snapshot_path(&self.data.host_dir, project_id),
            &refresh::StoredTrackerSnapshot {
                fetched_at_ms,
                complete: state.complete,
                detail: state.detail.clone(),
                issues: issues.clone(),
                documents: self.stored_issue_documents(project_id),
            },
        );
    }

    fn persist_runs(&self) -> Result<(), KernelError> {
        write_json(&self.data.host_dir.join("runs.json"), &self.runs)
    }

    fn usage_samples_path(&self) -> PathBuf {
        self.data.host_dir.join("usage-samples.json")
    }

    fn persist_usage_samples(&self) -> Result<(), KernelError> {
        write_json(&self.usage_samples_path(), &self.usage_samples)
    }

    fn load_usage_samples(&mut self) {
        let Ok(raw) = fs::read_to_string(self.usage_samples_path()) else {
            return;
        };
        if let Ok(samples) = serde_json::from_str(&raw) {
            self.usage_samples = samples;
        }
    }

    fn decorate_runs(&self, runs: &[RunSummary]) -> Vec<RunSummary> {
        runs.iter()
            .map(|run| {
                let mut run = run.clone();
                run.telemetry = usage::run_telemetry(&self.usage_samples, &run.id);
                run
            })
            .collect()
    }

    fn usage_open_for_focus(&self) -> bool {
        if self.focused_host_id != LOCAL_HOST_ID {
            return self
                .remote_view
                .as_ref()
                .filter(|view| view.host_id == self.focused_host_id)
                .map(|view| view.usage_open)
                .unwrap_or(false);
        }
        self.usage_open
    }

    fn usage_for_focus(&self) -> UsagePage {
        if self.focused_host_id != LOCAL_HOST_ID {
            if let Some(view) = &self.remote_view {
                if view.host_id == self.focused_host_id {
                    return view.usage.clone();
                }
            }
        }
        self.build_usage()
    }

    fn build_usage(&self) -> UsagePage {
        self.build_usage_for(&self.usage_query)
    }

    fn build_usage_for(&self, query: &usage::UsageQuery) -> UsagePage {
        let runs = self
            .runs
            .iter()
            .map(|run| usage::UsageRun {
                id: run.id.clone(),
                project_id: run.project_id.clone(),
                project_name: self
                    .projects
                    .iter()
                    .find(|project| project.id == run.project_id)
                    .map(|project| project.name.clone())
                    .unwrap_or_else(|| run.project_id.clone()),
                agent_id: run.agent_id.clone(),
                agent_name: run.agent_name.clone(),
                issue_id: run.issue_id.clone(),
                started_at_ms: run.started_at_ms,
            })
            .collect::<Vec<_>>();
        usage::build_usage_page(
            query,
            self.now_ms,
            usage::local_offset_secs(),
            &runs,
            &self.usage_samples,
        )
    }

    fn ingest_telemetry(&mut self) {
        let incoming: Vec<TelemetrySample> = self
            .agents
            .iter()
            .flat_map(|agent| agent.drain_telemetry())
            .collect();
        if incoming.is_empty() {
            return;
        }
        let mut seen = Vec::new();
        for mut sample in incoming {
            let Some(run) = self.runs.iter().find(|run| run.id == sample.run_id) else {
                continue;
            };
            sample.project_id = run.project_id.clone();
            sample.agent_id = run.agent_id.clone();
            if sample.at_ms == 0 {
                sample.at_ms = self.now_ms;
            }
            if !seen.contains(&sample.run_id) {
                seen.push(sample.run_id.clone());
            }
            self.usage_samples.push(sample);
        }
        let keep_after = self.now_ms.saturating_sub(usage::SAMPLE_RETENTION_MS);
        self.usage_samples
            .retain(|sample| sample.at_ms >= keep_after);
        for run_id in seen {
            self.pending_events.push(HostEvent::Telemetry { run_id });
        }
        let _ = self.persist_usage_samples();
    }

    fn open_usage_for_run(&mut self, run_id: &str) -> Result<(), KernelError> {
        if !self.runs.iter().any(|run| run.id == run_id) {
            return Err(KernelError::Protocol("unknown run".into()));
        }
        self.usage_open = true;
        self.usage_query.highlighted_run_id = Some(run_id.to_string());
        Ok(())
    }

    fn open_run_from_usage(&mut self, run_id: &str) -> Result<(), KernelError> {
        let run = self
            .runs
            .iter()
            .find(|run| run.id == run_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown run".into()))?;
        self.usage_open = false;
        self.usage_query.highlighted_run_id = None;
        self.focused_run_id = Some(run.id.clone());
        if self
            .projects
            .iter()
            .any(|project| project.id == run.project_id)
        {
            self.focused_project_id = Some(run.project_id.clone());
        }
        if let Some(issue_id) = run.issue_id {
            self.selected_issue_id = Some(issue_id);
        }
        Ok(())
    }

    fn change_notes_path(&self) -> PathBuf {
        self.data.host_dir.join("change-notes.json")
    }

    fn persist_change_notes(&self) -> Result<(), KernelError> {
        write_json(&self.change_notes_path(), &self.change_notes)
    }

    fn load_change_notes(&mut self) {
        let Ok(raw) = fs::read_to_string(self.change_notes_path()) else {
            return;
        };
        if let Ok(notes) = serde_json::from_str(&raw) {
            self.change_notes = notes;
        }
    }

    fn clear_pending_notes(
        &mut self,
        project_id: &str,
        issue_id: Option<&str>,
    ) -> Result<(), KernelError> {
        self.change_notes
            .retain(|note| note.project_id != project_id || note.issue_id.as_deref() != issue_id);
        self.persist_change_notes()
    }

    fn run_for_changes(
        &self,
        run_id: Option<&str>,
        issue_id: Option<&str>,
    ) -> Result<&RunSummary, KernelError> {
        if let Some(run_id) = run_id.filter(|id| !id.is_empty()) {
            return self
                .runs
                .iter()
                .find(|run| run.id == run_id)
                .ok_or_else(|| KernelError::Protocol("unknown run".into()));
        }
        let issue_id = issue_id
            .filter(|id| !id.is_empty())
            .ok_or_else(|| KernelError::Protocol("missing runId".into()))?;
        self.runs
            .iter()
            .rev()
            .find(|run| run.issue_id.as_deref() == Some(issue_id) && run.is_active())
            .or_else(|| {
                self.runs
                    .iter()
                    .rev()
                    .find(|run| run.issue_id.as_deref() == Some(issue_id))
            })
            .ok_or_else(|| KernelError::Protocol("unknown run".into()))
    }

    fn view_changes(
        &self,
        run_id: Option<&str>,
        issue_id: Option<&str>,
        scope: ChangeScope,
    ) -> Result<ViewChanges, KernelError> {
        let run = self.run_for_changes(run_id, issue_id)?;
        Ok(changes::compute_view(
            run,
            scope,
            &self.change_notes,
            self.appearance.language,
        ))
    }

    fn write_change_note(
        &mut self,
        run_id: &str,
        repo: String,
        path: String,
        line: u32,
        text: String,
    ) -> Result<(), KernelError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(KernelError::Protocol("missing text".into()));
        }
        let run = self
            .runs
            .iter()
            .find(|run| run.id == run_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown run".into()))?;
        self.change_notes.push(changes::new_note(
            &run,
            if repo.trim().is_empty() {
                ".".into()
            } else {
                repo
            },
            path,
            line,
            text.to_string(),
        ));
        self.persist_change_notes()
    }

    fn delete_change_note(&mut self, note_id: &str) -> Result<(), KernelError> {
        let before = self.change_notes.len();
        self.change_notes.retain(|note| note.id != note_id);
        if self.change_notes.len() == before {
            return Err(KernelError::Protocol("unknown note".into()));
        }
        self.persist_change_notes()
    }

    fn load_persisted_runs(&mut self) -> Vec<String> {
        let path = self.data.host_dir.join("runs.json");
        let Ok(raw) = fs::read_to_string(&path) else {
            return Vec::new();
        };
        let Ok(mut runs) = serde_json::from_str::<Vec<RunSummary>>(&raw) else {
            return Vec::new();
        };
        let mut crashed_ids = Vec::new();
        for run in &mut runs {
            if run.is_active() {
                run.status = RunStatus::Ended;
                run.waiting_for_user = false;
                run.ended_reason = Some(RunEndedReason::Crash);
                crashed_ids.push(run.id.clone());
            }
        }
        self.runs = runs;
        if !crashed_ids.is_empty() {
            let _ = self.persist_runs();
        }
        crashed_ids
    }

    fn note_crash_recovery(&mut self, crashed_ids: Vec<String>) {
        if crashed_ids.is_empty() {
            return;
        }
        self.pending_events.push(HostEvent::HostCrashedRecovered {
            run_ids: crashed_ids.clone(),
        });
        for run_id in crashed_ids {
            self.push_notification(NotificationKind::CrashRecovered, &run_id);
        }
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
            self.mark_run_ended(run_id, RunEndedReason::Stopped);
        }
        Ok(())
    }

    fn focus_run(&mut self, run_id: &str) -> Result<(), KernelError> {
        let run = self
            .runs
            .iter()
            .find(|run| run.id == run_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown run".into()))?;
        self.focused_run_id = Some(run.id);
        if self
            .projects
            .iter()
            .any(|project| project.id == run.project_id)
        {
            self.focused_project_id = Some(run.project_id);
        }
        if let Some(issue_id) = run.issue_id {
            self.selected_issue_id = Some(issue_id);
        }
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
            .filter_map(|(id, session)| {
                session.exit_code().map(|code| {
                    (
                        id.clone(),
                        RunEndedReason::from_exit(code, session.was_stopped()),
                    )
                })
            })
            .collect::<Vec<_>>();
        for (id, reason) in ended {
            self.mark_run_ended(&id, reason);
        }
    }

    fn observe_live_runs(&mut self) {
        self.ingest_telemetry();
        self.harvest_live_signals();
        let stop_failures = self
            .runs
            .iter()
            .filter(|run| run.is_active() && run.stop_failure && !run.self_check_attempted)
            .map(|run| run.id.clone())
            .collect::<Vec<_>>();
        for run_id in stop_failures {
            self.maybe_inject_self_check(&run_id);
        }
        self.reap_runs();
        let waiting = self
            .live
            .iter()
            .map(|(id, session)| (id.clone(), session.waiting_for_user()))
            .collect::<Vec<_>>();
        let mut became_waiting = Vec::new();
        for (id, is_waiting) in waiting {
            let Some(run) = self.runs.iter_mut().find(|run| run.id == id) else {
                continue;
            };
            if !run.is_active() || run.waiting_for_user == is_waiting {
                continue;
            }
            run.waiting_for_user = is_waiting;
            if is_waiting {
                became_waiting.push(id);
            }
        }
        for id in became_waiting {
            self.pending_events
                .push(HostEvent::Waiting { run_id: id.clone() });
            self.push_notification(NotificationKind::Waiting, &id);
        }
    }

    fn push_notification(&mut self, kind: NotificationKind, run_id: &str) {
        let Some(run) = self.runs.iter().find(|run| run.id == run_id) else {
            return;
        };
        self.pending_events.push(HostEvent::Notification {
            kind,
            run_id: run.id.clone(),
            issue_id: run.issue_id.clone(),
            project_id: run.project_id.clone(),
        });
    }

    fn issue_waiting(&self, issue_id: &str) -> bool {
        self.runs.iter().any(|run| {
            run.issue_id.as_deref() == Some(issue_id) && run.is_active() && run.waiting_for_user
        })
    }

    fn run_for_issue(&self, issue_id: &str) -> Option<&RunSummary> {
        self.runs
            .iter()
            .rev()
            .find(|run| run.issue_id.as_deref() == Some(issue_id) && run.is_active())
            .or_else(|| {
                self.runs
                    .iter()
                    .rev()
                    .find(|run| run.issue_id.as_deref() == Some(issue_id))
            })
    }

    fn issue_activity(&self, issue_id: &str) -> Option<IssueActivity> {
        if self.issue_waiting(issue_id) {
            Some(IssueActivity::Waiting)
        } else if self.active_run_id_for_issue(issue_id).is_some() {
            Some(IssueActivity::Running)
        } else if self.execution_stopped(issue_id) {
            Some(IssueActivity::ExecutionStopped)
        } else {
            None
        }
    }

    fn mark_run_ended(&mut self, run_id: &str, reason: RunEndedReason) {
        self.harvest_run_signals(run_id);
        let recent_output = self.live.get(run_id).map(|session| {
            let chunk = session.read_after(0, Duration::ZERO);
            String::from_utf8_lossy(&chunk.data)
                .chars()
                .rev()
                .take(16_000)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>()
        });
        let mut issue_id = None;
        let mut project_id = None;
        let mut newly_ended = false;
        if let Some(run) = self.runs.iter_mut().find(|run| run.id == run_id) {
            if run.status != RunStatus::Ended {
                run.status = RunStatus::Ended;
                run.waiting_for_user = false;
                run.ended_reason = Some(reason);
                if let Some(output) = &recent_output {
                    run.recent_output = output.clone();
                }
                issue_id = run.issue_id.clone();
                project_id = Some(run.project_id.clone());
                newly_ended = true;
            }
        }
        if newly_ended {
            self.pending_events.push(HostEvent::RunStatusChanged {
                run_id: run_id.to_string(),
                status: RunStatus::Ended,
            });
            match reason {
                RunEndedReason::Exited => {
                    self.push_notification(NotificationKind::Completed, run_id);
                }
                RunEndedReason::Abnormal => {
                    self.push_notification(NotificationKind::AbnormalStop, run_id);
                }
                RunEndedReason::Stopped | RunEndedReason::Crash => {}
            }
        }
        self.live.remove(run_id);
        if let Some(issue_id) = issue_id {
            if self.execution_stopped(&issue_id) {
                self.pending_events.push(HostEvent::ExecutionStopped {
                    issue_id,
                    run_id: run_id.to_string(),
                });
            }
        }
        let active = self.active_run_count();
        if active == 0 {
            self.quit_offer = None;
        } else if let Some(offer) = &mut self.quit_offer {
            offer.active_run_count = active;
        }
        let _ = self.persist_runs();
        if newly_ended {
            if let Some(project_id) = project_id {
                self.refresh_project_with_continuation(
                    &project_id,
                    RefreshTrigger::RunEnded,
                    RefreshContinuation::RunEnded(run_id.to_string()),
                );
            }
        }
    }

    fn project_has_active_run(&self, project_id: &str) -> bool {
        self.runs
            .iter()
            .any(|run| run.project_id == project_id && run.is_active())
    }

    fn project_has_execution_stopped(&self, project_id: &str) -> bool {
        self.loaded_issues
            .get(project_id)
            .into_iter()
            .flatten()
            .any(|issue| self.execution_stopped(&issue.id()))
    }

    fn active_run_id_for_issue(&self, issue_id: &str) -> Option<String> {
        self.runs
            .iter()
            .find(|run| run.issue_id.as_deref() == Some(issue_id) && run.is_active())
            .map(|run| run.id.clone())
    }

    fn focus_issue(&mut self, issue_id: &str) {
        self.selected_issue_id = Some(issue_id.to_string());
        if let Some(project_id) = self.focused_project_id.clone() {
            let previous_body = self
                .issue_documents
                .get(&project_id)
                .and_then(|documents| documents.get(issue_id))
                .and_then(|state| issue_document_body(Some(state)));
            let should_start_loading = self
                .issue_documents
                .get(&project_id)
                .and_then(|documents| documents.get(issue_id))
                .is_none_or(|state| !matches!(state, IssueDocumentState::Ready { .. }));
            if should_start_loading {
                self.issue_documents.entry(project_id).or_default().insert(
                    issue_id.to_string(),
                    IssueDocumentState::Loading {
                        body: previous_body.as_ref().map(|(body, _)| body.clone()),
                        fetched_at_ms: previous_body.map(|(_, fetched_at_ms)| fetched_at_ms),
                    },
                );
            }
        }
        self.focused_run_id = self.active_run_id_for_issue(issue_id);
        self.workspace_view = WorkspaceView::Project;
    }

    fn last_bound_run(&self, issue_id: &str) -> Option<&RunSummary> {
        self.runs
            .iter()
            .rev()
            .find(|run| run.issue_id.as_deref() == Some(issue_id))
    }

    fn execution_stopped(&self, issue_id: &str) -> bool {
        let claimed = self
            .issue_by_id(issue_id)
            .is_some_and(|issue| issue.claimed());
        if !claimed || self.active_run_id_for_issue(issue_id).is_some() {
            return false;
        }
        self.last_bound_run(issue_id)
            .and_then(|run| run.ended_reason)
            .is_some_and(RunEndedReason::execution_stopped)
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
        self.apply_pair_remote_host_completion(
            BackgroundPairRemoteHostTask {
                address,
                code: code.to_string(),
                client_name: self.host_display_name.clone(),
            }
            .execute(),
        )
    }

    fn apply_pair_remote_host_completion(
        &mut self,
        completion: BackgroundPairRemoteHostCompletion,
    ) -> Result<(), KernelError> {
        let BackgroundPairRemoteHostCompletion { task, result } = completion;
        let issued = result?;
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
            address: task.address,
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
        if host_id == LOCAL_HOST_ID {
            self.remote_view = None;
        }
        self.persist_client_settings(&self.appearance.clone())
    }

    fn forget_remote_host(&mut self, host_id: &str) -> Result<(), KernelError> {
        if host_id == LOCAL_HOST_ID {
            return Err(KernelError::Protocol(
                "local host cannot be forgotten".into(),
            ));
        }
        let before = self.remote_hosts.len();
        self.remote_hosts.retain(|host| host.id != host_id);
        if self.remote_hosts.len() == before {
            return Err(KernelError::Protocol("unknown host".into()));
        }
        self.remote_client_views.values_mut().for_each(|views| {
            views.remove(host_id);
        });
        if self.focused_host_id == host_id {
            self.remote_view = None;
            self.focused_host_id = if self.host_mode == HostMode::HostAndClient {
                LOCAL_HOST_ID.to_string()
            } else {
                self.remote_hosts
                    .first()
                    .map(|host| host.id.clone())
                    .unwrap_or_default()
            };
        }
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
        self.persist_host_settings_state(&self.projects, self.focused_project_id.as_deref())
    }

    fn persist_host_settings_state(
        &self,
        projects: &[ProjectRecord],
        focused_project_id: Option<&str>,
    ) -> Result<(), KernelError> {
        let file = HostSettingsFile {
            id: self.host_id.clone(),
            focused_project_id: focused_project_id.map(ToOwned::to_owned),
            projects: projects.iter().map(ProjectRecord::stored).collect(),
            refresh_interval_ms: self.refresh_interval_ms,
            agent_launch_defaults: self.launch_defaults.clone(),
            last_successful_agent: self.last_successful_agent.clone(),
            auto_advance: self.host_auto_advance,
        };
        write_json(&self.data.host_settings_path, &file)
    }

    fn auto_advance_allowed(&self, project_id: &str) -> bool {
        if !self.host_auto_advance {
            return false;
        }
        self.projects
            .iter()
            .find(|project| project.id == project_id)
            .is_some_and(|project| {
                project.auto_advance
                    && project
                        .advance_ready_at_ms
                        .is_some_and(|ready| self.now_ms >= ready)
            })
    }

    fn arm_cold_start(&mut self) {
        let boot = self.now_ms;
        let host_on = self.host_auto_advance;
        for project in &mut self.projects {
            project.advance_ready_at_ms =
                if host_on && project.auto_advance && project.restore_auto_advance {
                    Some(boot.saturating_add(project.restore_delay_ms))
                } else {
                    None
                };
        }
    }

    fn arm_project_now(&mut self, project_id: &str) {
        let now = self.now_ms;
        let allowed = self.host_auto_advance
            && self
                .projects
                .iter()
                .find(|project| project.id == project_id)
                .is_some_and(|project| project.auto_advance);
        if let Some(project) = self
            .projects
            .iter_mut()
            .find(|project| project.id == project_id)
        {
            project.advance_ready_at_ms = allowed.then_some(now);
        }
        if !allowed {
            self.clear_pending(project_id, false);
        }
    }

    fn set_host_auto_advance(&mut self, enabled: bool) -> Result<(), KernelError> {
        self.host_auto_advance = enabled;
        let ids: Vec<String> = self
            .projects
            .iter()
            .map(|project| project.id.clone())
            .collect();
        for id in ids {
            self.arm_project_now(&id);
        }
        self.persist_host_settings()
    }

    fn set_project_auto_advance(
        &mut self,
        project_id: &str,
        enabled: bool,
    ) -> Result<(), KernelError> {
        let project = self
            .projects
            .iter_mut()
            .find(|project| project.id == project_id)
            .ok_or_else(|| KernelError::Protocol("unknown project".into()))?;
        project.auto_advance = enabled;
        self.arm_project_now(project_id);
        self.persist_host_settings()
    }

    fn set_project_restore_auto_advance(
        &mut self,
        project_id: &str,
        enabled: bool,
    ) -> Result<(), KernelError> {
        let project = self
            .projects
            .iter_mut()
            .find(|project| project.id == project_id)
            .ok_or_else(|| KernelError::Protocol("unknown project".into()))?;
        project.restore_auto_advance = enabled;
        self.persist_host_settings()
    }

    fn set_project_restore_delay(
        &mut self,
        project_id: &str,
        delay_ms: u64,
    ) -> Result<(), KernelError> {
        let project = self
            .projects
            .iter_mut()
            .find(|project| project.id == project_id)
            .ok_or_else(|| KernelError::Protocol("unknown project".into()))?;
        project.restore_delay_ms = advance::clamp_restore_delay_ms(delay_ms);
        self.persist_host_settings()
    }

    fn veto_pending(&mut self, project_id: &str) {
        self.clear_pending(project_id, false);
    }

    fn clear_pending(&mut self, project_id: &str, advanced: bool) {
        if let Some(pending) = self.pending_advance.remove(project_id) {
            self.pending_events
                .push(HostEvent::PendingConfirmationEnded {
                    project_id: pending.project_id,
                    issue_id: pending.issue_id,
                    run_id: pending.run_id,
                    advanced,
                });
        }
    }

    fn begin_pending(&mut self, run: &RunSummary, issue_id: &str) {
        let pending = advance::PendingAdvance {
            project_id: run.project_id.clone(),
            issue_id: issue_id.to_string(),
            run_id: run.id.clone(),
            agent_id: run.agent_id.clone(),
            deadline_ms: self.now_ms.saturating_add(advance::PENDING_CONFIRM_MS),
        };
        self.pending_events
            .push(HostEvent::PendingConfirmationStarted {
                project_id: pending.project_id.clone(),
                issue_id: pending.issue_id.clone(),
                run_id: pending.run_id.clone(),
            });
        self.pending_advance.insert(run.project_id.clone(), pending);
    }

    fn finish_due_pending(&mut self) {
        let due: Vec<String> = self
            .pending_advance
            .iter()
            .filter(|(_, pending)| pending.deadline_ms <= self.now_ms)
            .map(|(project_id, _)| project_id.clone())
            .collect();
        for project_id in due {
            self.finish_pending_if_due(&project_id);
        }
    }

    fn finish_pending_if_due(&mut self, project_id: &str) {
        let Some(pending) = self.pending_advance.get(project_id).cloned() else {
            return;
        };
        if pending.deadline_ms > self.now_ms {
            return;
        }
        self.refresh_project_with_continuation(
            project_id,
            RefreshTrigger::Action,
            RefreshContinuation::PendingAdvance(project_id.to_string()),
        );
    }

    fn finish_pending_after_refresh(
        &mut self,
        project_id: &str,
    ) -> Option<BackgroundAutoAdvanceTask> {
        let Some(pending) = self.pending_advance.get(project_id).cloned() else {
            return None;
        };
        let Some(issue_id) = self.next_auto_pool(project_id) else {
            self.clear_pending(project_id, false);
            return None;
        };
        let Some(project) = self
            .projects
            .iter()
            .find(|project| project.id == project_id)
        else {
            self.clear_pending(project_id, false);
            return None;
        };
        Some(BackgroundAutoAdvanceTask {
            pending,
            issue_id,
            github_host: project.github_host.clone(),
            repository: project.repository.clone(),
            host_secrets_path: self.data.host_secrets_path.clone(),
            tracker: Arc::clone(&self.tracker),
        })
    }

    fn next_auto_pool(&self, project_id: &str) -> Option<String> {
        self.loaded_issues
            .get(project_id)?
            .iter()
            .find(|issue| advance::in_auto_pool(issue))
            .map(IssueRecord::id)
    }

    fn harvest_live_signals(&mut self) {
        let ids: Vec<String> = self.live.keys().cloned().collect();
        for id in ids {
            self.harvest_run_signals(&id);
        }
    }

    fn harvest_run_signals(&mut self, run_id: &str) {
        let session_signals = self
            .live
            .get(run_id)
            .map(|session| session.completion_signals())
            .unwrap_or_default();
        let (hook_dir, agent_id) = self
            .runs
            .iter()
            .find(|run| run.id == run_id)
            .map(|run| (run.hook_dir.clone(), run.agent_id.clone()))
            .unwrap_or((None, String::new()));
        let file_signals = hook_dir
            .as_ref()
            .and_then(|dir| {
                self.agents
                    .iter()
                    .find(|agent| agent.id() == agent_id)
                    .map(|agent| agent.read_completion_signals(dir))
            })
            .unwrap_or_default();
        if let Some(run) = self.runs.iter_mut().find(|run| run.id == run_id) {
            run.session_end |= session_signals.session_end || file_signals.session_end;
            run.stop_failure |= session_signals.stop_failure || file_signals.stop_failure;
        }
    }

    fn consider_auto_advance(&mut self, run_id: &str) {
        let Some(run) = self.runs.iter().find(|run| run.id == run_id).cloned() else {
            return;
        };
        let Some(issue_id) = run.issue_id.clone() else {
            return;
        };
        if matches!(
            run.ended_reason,
            Some(RunEndedReason::Stopped | RunEndedReason::Crash)
        ) {
            return;
        }
        if !self.auto_advance_allowed(&run.project_id) || !run.hooks_attached {
            return;
        }
        let issue_closed = self.issue_by_id(&issue_id).is_some_and(|issue| !issue.open);
        let process_ok = run.ended_reason == Some(RunEndedReason::Exited);
        let normal = advance::normal_completion(
            issue_closed,
            run.hooks_attached,
            run.session_end,
            run.stop_failure,
            process_ok,
        );
        if run.self_check {
            if normal {
                self.begin_pending(&run, &issue_id);
            }
            return;
        }
        if normal {
            self.begin_pending(&run, &issue_id);
            return;
        }
        if issue_closed {
            self.open_view_changes_run_id = Some(run.id.clone());
            return;
        }
        self.launch_self_check_run(&run, &issue_id);
    }

    fn launch_self_check_run(&mut self, previous: &RunSummary, issue_id: &str) {
        if previous.self_check {
            return;
        }
        if let Some(run) = self.runs.iter_mut().find(|run| run.id == previous.id) {
            run.self_check_attempted = true;
        }
        let agent = match self
            .agents
            .iter()
            .find(|agent| agent.id() == previous.agent_id)
            .cloned()
        {
            Some(agent) => agent,
            None => return,
        };
        let values = self
            .launch_defaults
            .get(&previous.project_id)
            .and_then(|agents| agents.get(&previous.agent_id))
            .cloned()
            .unwrap_or_else(|| agent.seed_config());
        let opening = advance::self_check_text(self.appearance.language);
        if self
            .start_unbound_run(
                &previous.project_id,
                RunLaunchConfig {
                    agent_id: previous.agent_id.clone(),
                    values,
                    opening_text: opening,
                },
                Some(issue_id.to_string()),
                false,
                Some(PreviousRun {
                    id: previous.id.clone(),
                    native_session_id: previous.native_session_id.clone(),
                    working_directory: previous.working_directory.clone(),
                    isolated: previous.isolated,
                }),
            )
            .is_err()
        {
            return;
        }
        if let Some(run) = self
            .runs
            .iter_mut()
            .rev()
            .find(|run| run.issue_id.as_deref() == Some(issue_id))
        {
            run.self_check = true;
            run.self_check_attempted = true;
        }
    }

    fn maybe_inject_self_check(&mut self, run_id: &str) {
        let Some(run) = self.runs.iter().find(|run| run.id == run_id).cloned() else {
            return;
        };
        if !run.is_active() || !run.stop_failure || run.self_check_attempted {
            return;
        }
        if !self.auto_advance_allowed(&run.project_id) || !run.hooks_attached {
            return;
        }
        self.refresh_project_with_continuation(
            &run.project_id,
            RefreshTrigger::Action,
            RefreshContinuation::SelfCheck(run_id.to_string()),
        );
    }

    fn finish_self_check_after_refresh(&mut self, run_id: &str) {
        let Some(run) = self.runs.iter().find(|run| run.id == run_id).cloned() else {
            return;
        };
        if !run.is_active() || !run.stop_failure || run.self_check_attempted {
            return;
        }
        if run
            .issue_id
            .as_ref()
            .and_then(|issue_id| self.issue_by_id(issue_id))
            .is_some_and(|issue| !issue.open)
        {
            return;
        }
        if let Some(run) = self.runs.iter_mut().find(|run| run.id == run_id) {
            run.self_check = true;
            run.self_check_attempted = true;
        }
        let text = format!("{}\n", advance::self_check_text(self.appearance.language));
        if self.write_pty(run_id, text.as_bytes()).is_ok() {
            return;
        }
        let previous = self
            .runs
            .iter()
            .find(|run| run.id == run_id)
            .cloned()
            .unwrap_or(run);
        self.mark_run_ended(run_id, RunEndedReason::Abnormal);
        if let Some(issue_id) = previous.issue_id.clone() {
            self.launch_self_check_run(&previous, &issue_id);
        }
    }

    fn start_bound_run_with_agent(
        &mut self,
        issue_id: &str,
        agent_id: &str,
    ) -> Result<(), KernelError> {
        let project_id = self.project_id_for_issue(issue_id)?;
        let issue = self
            .issue_by_id(issue_id)
            .ok_or_else(|| KernelError::Protocol("unknown issue".into()))?;
        let agent = self
            .agents
            .iter()
            .find(|agent| agent.id() == agent_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown Agent Adapter".into()))?;
        let mut values = self
            .launch_defaults
            .get(&project_id)
            .and_then(|agents| agents.get(agent.id()))
            .cloned()
            .unwrap_or_else(|| agent.seed_config());
        let opening = format!("{}\n{}", issue.title, issue.url);
        values.insert(launch::INITIAL_INSTRUCTION.into(), opening.clone());
        self.start_unbound_run(
            &project_id,
            RunLaunchConfig {
                agent_id: agent.id().to_string(),
                values,
                opening_text: opening,
            },
            Some(issue_id.to_string()),
            false,
            None,
        )
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
            .any(|project| project::same_local_directory(&project.local_path, &local_path))
        {
            return Err(KernelError::Protocol(
                "a Project is already registered for this directory".into(),
            ));
        }
        let tracker_kind = if github_host == "local" {
            TrackerKind::LocalMarkdown
        } else {
            TrackerKind::Github
        };
        let connection = self.probe_github(&github_host, &repository);
        let record = ProjectRecord {
            id: pairing::random_id(),
            name,
            local_path,
            tracker: tracker_kind,
            github_host,
            repository,
            connection,
            tracker_synced: false,
            auto_advance: false,
            restore_auto_advance: false,
            restore_delay_ms: advance::DEFAULT_RESTORE_DELAY_MS,
            advance_ready_at_ms: None,
        };
        let project_id = record.id.clone();
        let mut projects = self.projects.clone();
        projects.push(record);
        self.persist_host_settings_state(&projects, Some(&project_id))?;
        self.projects = projects;
        self.focused_project_id = Some(project_id.clone());
        self.selected_issue_id = None;
        self.graph_center_issue_id = None;
        self.complete_dependency_graph = false;
        self.parent_filter = None;
        self.refresh_project(&project_id, RefreshTrigger::Immediate);
        Ok(())
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
        if self.project_has_active_run(project_id) {
            let current = self
                .projects
                .iter()
                .find(|project| project.id == project_id)
                .ok_or_else(|| KernelError::Protocol("unknown project".into()))?;
            if current.local_path != local_path
                || current.github_host != github_host
                || current.repository != repository
            {
                return Err(KernelError::Denied(
                    "cannot change the directory or GitHub connection while a Project has an active Run".into(),
                ));
            }
        }
        if self.projects.iter().any(|project| {
            project.id != project_id
                && project::same_local_directory(&project.local_path, &local_path)
        }) {
            return Err(KernelError::Protocol(
                "a Project is already registered for this directory".into(),
            ));
        }
        let current = self
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .ok_or_else(|| KernelError::Protocol("unknown project".into()))?;
        let tracker_kind = if github_host == "local" {
            TrackerKind::LocalMarkdown
        } else {
            TrackerKind::Github
        };
        let registration_changed = current.local_path != local_path
            || current.tracker != tracker_kind
            || current.github_host != github_host
            || current.repository != repository;
        if registration_changed && self.refresh_in_flight.contains(project_id) {
            return Err(KernelError::Denied(
                "cannot change a Project registration while Tracker I/O is in progress".into(),
            ));
        }
        let connection = registration_changed.then(|| self.probe_github(&github_host, &repository));
        let mut projects = self.projects.clone();
        let project = projects
            .iter_mut()
            .find(|project| project.id == project_id)
            .expect("validated project");
        project.name = name;
        if registration_changed {
            project.local_path = local_path;
            project.tracker = tracker_kind;
            project.github_host = github_host;
            project.repository = repository;
            project.connection = connection.expect("changed registration connection");
            project.tracker_synced = false;
        }
        self.persist_host_settings_state(&projects, self.focused_project_id.as_deref())?;
        self.projects = projects;
        if registration_changed {
            self.loaded_issues.remove(project_id);
            self.issue_documents.remove(project_id);
            self.refresh.remove(project_id);
            refresh::remove_project_data(&self.data.host_dir, project_id)?;
            if self.focused_project_id.as_deref() == Some(project_id) {
                self.selected_issue_id = None;
                self.graph_center_issue_id = None;
                self.complete_dependency_graph = false;
                self.parent_filter = None;
                self.refresh_project(project_id, RefreshTrigger::Immediate);
            }
        }
        Ok(())
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
        if self.refresh_in_flight.contains(project_id) {
            return Err(KernelError::Denied(
                "cannot remove a Project while Tracker I/O is in progress".into(),
            ));
        }
        let was_current = self.focused_project_id.as_deref() == Some(project_id);
        let mut projects = self.projects.clone();
        projects.remove(index);
        let focused_project_id = if was_current {
            if projects.is_empty() {
                None
            } else {
                Some(projects[index.min(projects.len() - 1)].id.clone())
            }
        } else {
            self.focused_project_id.clone()
        };
        self.persist_host_settings_state(&projects, focused_project_id.as_deref())?;

        self.projects = projects;
        self.focused_project_id = focused_project_id;
        self.refresh.remove(project_id);
        self.loaded_issues.remove(project_id);
        self.issue_documents.remove(project_id);
        self.clear_pending(project_id, false);
        if was_current {
            self.selected_issue_id = None;
            self.graph_center_issue_id = None;
            self.complete_dependency_graph = false;
            self.parent_filter = None;
            if let Some(next_id) = self.focused_project_id.clone() {
                self.refresh_project(&next_id, RefreshTrigger::Immediate);
            }
        }
        Ok(())
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
        self.graph_center_issue_id = None;
        self.complete_dependency_graph = false;
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
        self.current_local_board(
            focused_project_id,
            self.parent_filter.as_deref(),
            self.selected_issue_id.as_deref(),
            self.graph_center_issue_id.as_deref(),
            self.complete_dependency_graph,
            self.issue_search
                .get(focused_project_id)
                .cloned()
                .unwrap_or_default(),
        )
    }

    fn current_local_board(
        &self,
        project_id: &str,
        parent_filter_id: Option<&str>,
        selected_issue_id: Option<&str>,
        graph_center_issue_id: Option<&str>,
        complete_dependency_graph: bool,
        search: IssueSearch,
    ) -> Option<BoardSnapshot> {
        if project_id.is_empty() {
            return None;
        }
        let loaded = self.loaded_issues.get(project_id).map(Vec::as_slice);
        let mut board = board::project_board(
            project_id,
            loaded,
            parent_filter_id,
            selected_issue_id,
            self.recent_limit,
            self.refresh_status_for(project_id),
            graph_center_issue_id,
            complete_dependency_graph,
            search,
        );
        if let Some(columns) = board.columns.as_mut() {
            for card in &mut columns.in_progress {
                card.activity = self.issue_activity(&card.id);
                card.run_id = self.run_for_issue(&card.id).map(|run| run.id.clone());
            }
            for card in &mut columns.recently_completed {
                card.run_id = self.run_for_issue(&card.id).map(|run| run.id.clone());
            }
        }
        if let Some(selected) = board.selected.as_mut() {
            selected.document = self
                .issue_documents
                .get(project_id)
                .and_then(|documents| documents.get(&selected.id))
                .cloned()
                .unwrap_or_default();
            selected.active_run_id = self.active_run_id_for_issue(&selected.id);
            selected.execution_stopped = self.execution_stopped(&selected.id);
            selected.waiting_for_user = self.issue_waiting(&selected.id);
        }
        Some(board)
    }

    fn load_issue_document(&mut self, project_id: &str, issue_id: &str) -> Result<(), KernelError> {
        let Some(task) = self.begin_issue_document_task(project_id, issue_id)? else {
            return Ok(());
        };
        let completion = task.execute();
        self.finish_issue_document_task(completion);
        Ok(())
    }

    fn begin_issue_document_task(
        &mut self,
        project_id: &str,
        issue_id: &str,
    ) -> Result<Option<BackgroundIssueDocumentTask>, KernelError> {
        let identity = (project_id.to_string(), issue_id.to_string());
        if self.issue_documents_in_flight.contains(&identity) {
            return Ok(None);
        }
        let project = self
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown project".into()))?;
        if !self
            .loaded_issues
            .get(project_id)
            .is_some_and(|issues| issues.iter().any(|issue| issue.id() == issue_id))
        {
            return Err(KernelError::Protocol("unknown issue".into()));
        }
        let previous = self
            .issue_documents
            .get(project_id)
            .and_then(|documents| documents.get(issue_id))
            .cloned();
        let previous_body = issue_document_body(previous.as_ref());
        self.issue_documents
            .entry(project_id.to_string())
            .or_default()
            .insert(
                issue_id.to_string(),
                IssueDocumentState::Loading {
                    body: previous_body.as_ref().map(|(body, _)| body.clone()),
                    fetched_at_ms: previous_body
                        .as_ref()
                        .map(|(_, fetched_at_ms)| *fetched_at_ms),
                },
            );
        self.issue_documents_in_flight.insert(identity);
        Ok(Some(BackgroundIssueDocumentTask {
            project_id: project_id.to_string(),
            issue_id: issue_id.to_string(),
            github_host: project.github_host,
            repository: project.repository,
            host_secrets_path: self.data.host_secrets_path.clone(),
            tracker: Arc::clone(&self.tracker),
            now_ms: self.now_ms,
            previous_body,
        }))
    }

    pub(crate) fn finish_issue_document_task(
        &mut self,
        completion: BackgroundIssueDocumentCompletion,
    ) {
        let BackgroundIssueDocumentCompletion { task, result } = completion;
        let BackgroundIssueDocumentTask {
            project_id,
            issue_id,
            github_host,
            repository,
            now_ms,
            previous_body,
            ..
        } = task;
        self.issue_documents_in_flight
            .remove(&(project_id.clone(), issue_id.clone()));
        let project_is_current = self.projects.iter().any(|project| {
            project.id == project_id
                && project.github_host == github_host
                && project.repository == repository
        });
        let issue_is_current = self
            .loaded_issues
            .get(&project_id)
            .is_some_and(|issues| issues.iter().any(|issue| issue.id() == issue_id));
        if !project_is_current || !issue_is_current {
            return;
        }
        let state = match result {
            Ok(document) => {
                if let Some(issues) = self.loaded_issues.get_mut(&project_id) {
                    if let Some(existing) = issues.iter_mut().find(|issue| issue.id() == issue_id) {
                        // 单 Issue REST 响应不保证带原生父子与 Dependency；详情刷新只合并
                        // 该响应确实拥有的基础字段，关系仍由列表/GraphQL 真源维护。
                        existing.title = document.issue.title;
                        existing.url = document.issue.url;
                        existing.open = document.issue.open;
                        existing.closed_at = document.issue.closed_at;
                        existing.assignees = document.issue.assignees;
                        existing.labels = document.issue.labels;
                    }
                }
                IssueDocumentState::Ready {
                    body: document.body,
                    fetched_at_ms: now_ms,
                }
            }
            Err(error) => {
                let failure = issue_document_failure(error);
                match previous_body {
                    Some((body, fetched_at_ms)) => IssueDocumentState::Stale {
                        body,
                        fetched_at_ms,
                        failure,
                    },
                    None => IssueDocumentState::Failed { failure },
                }
            }
        };
        self.issue_documents
            .entry(project_id.clone())
            .or_default()
            .insert(issue_id, state);
        self.persist_tracker_snapshot(&project_id);
    }

    fn load_persisted_snapshot(&mut self, project_id: &str) {
        let Some(stored) =
            refresh::load_snapshot(&refresh::snapshot_path(&self.data.host_dir, project_id))
        else {
            return;
        };
        let documents = self
            .issue_documents
            .entry(project_id.to_string())
            .or_default();
        for (issue_id, document) in &stored.documents {
            documents.insert(
                issue_id.clone(),
                IssueDocumentState::Loading {
                    body: Some(document.body.clone()),
                    fetched_at_ms: Some(document.fetched_at_ms),
                },
            );
        }
        self.loaded_issues
            .insert(project_id.to_string(), stored.issues);
        if let Some(project) = self
            .projects
            .iter_mut()
            .find(|project| project.id == project_id)
        {
            project.tracker_synced = stored.complete;
        }
        self.refresh.insert(
            project_id.to_string(),
            ProjectRefreshState {
                fetched_at_ms: Some(stored.fetched_at_ms),
                last_attempt_ms: stored.fetched_at_ms,
                kind: if stored.complete {
                    StoredRefreshKind::Ready
                } else {
                    StoredRefreshKind::Incomplete
                },
                retry_at_ms: None,
                complete: stored.complete,
                detail: stored.detail,
            },
        );
    }

    fn refresh_project(&mut self, project_id: &str, trigger: RefreshTrigger) -> bool {
        if self.defer_tracker_refreshes {
            if let Some(task) = self.begin_refresh_task(project_id, trigger) {
                self.deferred_refresh_tasks.push(task);
                return true;
            }
            return false;
        }
        let Some(task) = self.begin_refresh_task(project_id, trigger) else {
            return false;
        };
        let completion = task.execute();
        self.finish_refresh_task(completion)
    }

    fn refresh_project_with_continuation(
        &mut self,
        project_id: &str,
        trigger: RefreshTrigger,
        continuation: RefreshContinuation,
    ) -> bool {
        let Some(mut task) = self.begin_refresh_task(project_id, trigger) else {
            if matches!(continuation, RefreshContinuation::PendingAdvance(_)) {
                self.clear_pending(project_id, false);
            }
            return false;
        };
        task.continuation = Some(continuation);
        if self.defer_tracker_refreshes {
            self.deferred_refresh_tasks.push(task);
            true
        } else {
            self.finish_refresh_task(task.execute())
        }
    }

    fn begin_refresh_task(
        &mut self,
        project_id: &str,
        trigger: RefreshTrigger,
    ) -> Option<BackgroundRefreshTask> {
        if !self.should_attempt_refresh(project_id, trigger) {
            return None;
        }
        if self.refresh_in_flight.contains(project_id) {
            return None;
        }
        let Some(index) = self
            .projects
            .iter()
            .position(|project| project.id == project_id)
        else {
            return None;
        };
        let previous = self.refresh.get(project_id).cloned();
        let previous_fetched = previous.as_ref().and_then(|state| state.fetched_at_ms);
        self.pending_events.push(HostEvent::RefreshStatusChanged {
            project_id: project_id.to_string(),
            status: RefreshStatus::Refreshing {
                fetched_at_ms: previous_fetched,
            },
        });
        let github_host = self.projects[index].github_host.clone();
        let repository = self.projects[index].repository.clone();
        self.refresh_in_flight.insert(project_id.to_string());
        Some(BackgroundRefreshTask {
            project_id: project_id.to_string(),
            github_host,
            repository,
            host_secrets_path: self.data.host_secrets_path.clone(),
            tracker: Arc::clone(&self.tracker),
            now_ms: self.now_ms,
            previous,
            probe_connection: !matches!(
                self.projects[index].connection,
                ProjectConnection::Ready { .. }
            ),
            language: self.appearance.language,
            continuation: None,
        })
    }

    pub(crate) fn finish_refresh_task(&mut self, completion: BackgroundRefreshCompletion) -> bool {
        let (live, auto_advance) = self.finish_background_refresh_task(completion);
        if let Some(task) = auto_advance {
            let completion = task.execute();
            if let Some(rollback) = self.finish_background_auto_advance(completion) {
                let completion = rollback.execute();
                self.finish_background_auto_advance_rollback(completion);
            }
        }
        live
    }

    pub(crate) fn finish_background_refresh_task(
        &mut self,
        completion: BackgroundRefreshCompletion,
    ) -> (bool, Option<BackgroundAutoAdvanceTask>) {
        let continuation = completion.task.continuation.clone();
        let live = self.apply_refresh_completion(completion);
        let auto_advance = continuation
            .and_then(|continuation| self.finish_refresh_continuation(continuation, live));
        (live, auto_advance)
    }

    fn finish_refresh_continuation(
        &mut self,
        continuation: RefreshContinuation,
        live: bool,
    ) -> Option<BackgroundAutoAdvanceTask> {
        match continuation {
            RefreshContinuation::RunEnded(run_id) if live => {
                self.consider_auto_advance(&run_id);
                None
            }
            RefreshContinuation::PendingAdvance(project_id) => {
                if live {
                    self.finish_pending_after_refresh(&project_id)
                } else {
                    self.clear_pending(&project_id, false);
                    None
                }
            }
            RefreshContinuation::SelfCheck(run_id) if live => {
                self.finish_self_check_after_refresh(&run_id);
                None
            }
            RefreshContinuation::RunEnded(_) | RefreshContinuation::SelfCheck(_) => None,
        }
    }

    pub(crate) fn finish_background_auto_advance(
        &mut self,
        completion: BackgroundAutoAdvanceCompletion,
    ) -> Option<BackgroundClaimRollbackTask> {
        let BackgroundAutoAdvanceCompletion { task, result } = completion;
        let rollback = task.rollback_task();
        let pending_is_current = self
            .pending_advance
            .get(&task.pending.project_id)
            .is_some_and(|pending| pending == &task.pending);
        let project_is_current = self.projects.iter().any(|project| {
            project.id == task.pending.project_id
                && project.github_host == task.github_host
                && project.repository == task.repository
        });
        let updated = match result {
            Ok(updated) => updated,
            Err(_) => {
                if pending_is_current {
                    self.clear_pending(&task.pending.project_id, false);
                }
                return None;
            }
        };
        if updated.id() != task.issue_id || !pending_is_current || !project_is_current {
            if pending_is_current {
                self.clear_pending(&task.pending.project_id, false);
            }
            return Some(rollback);
        }
        self.merge_issue(
            &task.pending.project_id,
            updated,
            &tracker_seam::TrackerWriteOp::Claim,
        );
        self.preclaimed_issue_id = Some(task.issue_id.clone());
        let _ = self.start_bound_run_with_agent(&task.issue_id, &task.pending.agent_id);
        self.preclaimed_issue_id = None;
        let launched = self.active_run_id_for_issue(&task.issue_id).is_some();
        self.clear_pending(&task.pending.project_id, launched);
        (!launched).then_some(rollback)
    }

    pub(crate) fn finish_background_auto_advance_rollback(
        &mut self,
        completion: BackgroundClaimRollbackCompletion,
    ) {
        let _ = self.apply_background_claim_rollback(completion);
    }

    fn apply_refresh_completion(&mut self, completion: BackgroundRefreshCompletion) -> bool {
        let BackgroundRefreshCompletion {
            task,
            result,
            connection,
        } = completion;
        let BackgroundRefreshTask {
            project_id,
            github_host,
            repository,
            now_ms: now,
            previous,
            ..
        } = task;
        self.refresh_in_flight.remove(&project_id);
        let Some(index) = self.projects.iter().position(|project| {
            project.id == project_id
                && project.github_host == github_host
                && project.repository == repository
        }) else {
            return false;
        };
        let previous_fetched = previous.as_ref().and_then(|state| state.fetched_at_ms);
        match result {
            Ok(tracker_seam::TrackerReadOutcome::Complete { issues }) => {
                self.apply_read(
                    &project_id,
                    index,
                    &github_host,
                    &repository,
                    now,
                    issues,
                    true,
                    None,
                    connection.clone(),
                );
                let status = self.refresh_status_for(&project_id);
                self.pending_events.push(HostEvent::RefreshStatusChanged {
                    project_id: project_id.clone(),
                    status,
                });
                self.pending_events.push(HostEvent::BoardUpdated {
                    project_id: project_id.clone(),
                });
                true
            }
            Ok(tracker_seam::TrackerReadOutcome::Incomplete { issues, detail }) => {
                self.apply_read(
                    &project_id,
                    index,
                    &github_host,
                    &repository,
                    now,
                    issues,
                    false,
                    Some(detail),
                    connection,
                );
                let status = self.refresh_status_for(&project_id);
                self.pending_events.push(HostEvent::RefreshStatusChanged {
                    project_id: project_id.clone(),
                    status,
                });
                self.pending_events.push(HostEvent::BoardUpdated {
                    project_id: project_id.clone(),
                });
                true
            }
            Err(tracker::TrackerReadError::RateLimited { retry_after_ms }) => {
                self.refresh.insert(
                    project_id.clone(),
                    ProjectRefreshState {
                        fetched_at_ms: previous_fetched,
                        last_attempt_ms: now,
                        kind: StoredRefreshKind::RateLimited,
                        retry_at_ms: retry_after_ms.map(|ms| now.saturating_add(ms)),
                        complete: previous
                            .as_ref()
                            .map(|state| state.complete)
                            .unwrap_or(false),
                        detail: previous.as_ref().and_then(|state| state.detail.clone()),
                    },
                );
                let status = self.refresh_status_for(&project_id);
                self.pending_events.push(HostEvent::RefreshStatusChanged {
                    project_id: project_id.clone(),
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
                let has_data = self.loaded_issues.contains_key(&project_id);
                self.refresh.insert(
                    project_id.clone(),
                    ProjectRefreshState {
                        fetched_at_ms: previous_fetched,
                        last_attempt_ms: now,
                        kind: if has_data {
                            StoredRefreshKind::Offline
                        } else {
                            StoredRefreshKind::NeverFetched
                        },
                        retry_at_ms: None,
                        complete: previous
                            .as_ref()
                            .map(|state| state.complete)
                            .unwrap_or(false),
                        detail: previous.as_ref().and_then(|state| state.detail.clone()),
                    },
                );
                let status = self.refresh_status_for(&project_id);
                self.pending_events.push(HostEvent::RefreshStatusChanged {
                    project_id: project_id.clone(),
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
                    project_id.clone(),
                    ProjectRefreshState {
                        fetched_at_ms: previous_fetched,
                        last_attempt_ms: now,
                        kind: StoredRefreshKind::AuthFailed,
                        retry_at_ms: None,
                        complete: previous
                            .as_ref()
                            .map(|state| state.complete)
                            .unwrap_or(false),
                        detail: previous.as_ref().and_then(|state| state.detail.clone()),
                    },
                );
                let status = self.refresh_status_for(&project_id);
                self.pending_events.push(HostEvent::RefreshStatusChanged {
                    project_id: project_id.clone(),
                    status,
                });
                false
            }
            Err(tracker::TrackerReadError::Failed { detail }) => {
                let detail = detail.unwrap_or_else(|| "tracker business error".into());
                let complete = previous
                    .as_ref()
                    .map(|state| state.complete)
                    .unwrap_or(false);
                self.refresh.insert(
                    project_id.clone(),
                    ProjectRefreshState {
                        fetched_at_ms: previous_fetched,
                        last_attempt_ms: now,
                        kind: StoredRefreshKind::TrackerError,
                        retry_at_ms: None,
                        complete,
                        detail: Some(detail),
                    },
                );
                let status = self.refresh_status_for(&project_id);
                self.pending_events
                    .push(HostEvent::RefreshStatusChanged { project_id, status });
                false
            }
        }
    }

    /// 记录一次成功读取（完整或不完整）的结果并持久化快照。
    fn apply_read(
        &mut self,
        project_id: &str,
        index: usize,
        _github_host: &str,
        _repository: &str,
        now: u64,
        mut issues: Vec<IssueRecord>,
        complete: bool,
        detail: Option<String>,
        connection: Option<ProjectConnection>,
    ) {
        if let Some(connection) = connection {
            self.projects[index].connection = connection;
        }
        if !complete {
            let mut seen = issues.iter().map(IssueRecord::id).collect::<BTreeSet<_>>();
            if let Some(previous) = self.loaded_issues.get(project_id) {
                issues.extend(
                    previous
                        .iter()
                        .filter(|issue| seen.insert(issue.id()))
                        .cloned(),
                );
            }
        }
        let snapshot = refresh::StoredTrackerSnapshot {
            fetched_at_ms: now,
            complete,
            detail: detail.clone(),
            issues: issues.clone(),
            documents: self.stored_issue_documents(project_id),
        };
        if let Err(err) = refresh::save_snapshot(
            &refresh::snapshot_path(&self.data.host_dir, project_id),
            &snapshot,
        ) {
            self.projects[index].tracker_synced = false;
            self.loaded_issues.insert(project_id.to_string(), issues);
            self.refresh.insert(
                project_id.to_string(),
                ProjectRefreshState {
                    fetched_at_ms: Some(now),
                    last_attempt_ms: now,
                    kind: StoredRefreshKind::TrackerError,
                    retry_at_ms: None,
                    complete: false,
                    detail: Some(format!("tracker snapshot could not be persisted: {err}")),
                },
            );
            return;
        }
        self.projects[index].tracker_synced = complete;
        self.loaded_issues.insert(project_id.to_string(), issues);
        self.refresh.insert(
            project_id.to_string(),
            ProjectRefreshState {
                fetched_at_ms: Some(now),
                last_attempt_ms: now,
                kind: if complete {
                    StoredRefreshKind::Ready
                } else {
                    StoredRefreshKind::Incomplete
                },
                retry_at_ms: None,
                complete,
                detail,
            },
        );
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

    fn should_attempt_refresh(&self, project_id: &str, trigger: RefreshTrigger) -> bool {
        let Some(state) = self.refresh.get(project_id) else {
            return true;
        };
        match state.kind {
            StoredRefreshKind::RateLimited => match trigger {
                RefreshTrigger::Immediate | RefreshTrigger::Action | RefreshTrigger::RunEnded => {
                    true
                }
                RefreshTrigger::Interval => state
                    .retry_at_ms
                    .is_some_and(|retry_at| self.now_ms >= retry_at),
            },
            StoredRefreshKind::AuthFailed => {
                matches!(trigger, RefreshTrigger::Immediate | RefreshTrigger::Action)
            }
            _ => true,
        }
    }

    fn expire_stale_client_views(&mut self) {
        let ttl = self.refresh_interval_ms.saturating_mul(3).max(90_000);
        let now = self.now_ms;
        self.client_views
            .retain(|_, view| now.saturating_sub(view.last_seen_ms) <= ttl);
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
            | StoredRefreshKind::Incomplete
            | StoredRefreshKind::TrackerError
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
        if self.refresh_in_flight.contains(project_id) {
            return RefreshStatus::Refreshing {
                fetched_at_ms: self
                    .refresh
                    .get(project_id)
                    .and_then(|state| state.fetched_at_ms),
            };
        }
        let Some(state) = self.refresh.get(project_id) else {
            return RefreshStatus::NeverFetched;
        };
        let next = self.next_refresh_in_ms(project_id, state);
        // 拿到的是不完整数据时优先表达 incomplete：看板不能当作全量数据计算 Frontier/依赖图；
        // 从未读到过数据（complete=false 且无已加载数据）则按错误状态表达。
        if !state.complete && self.loaded_issues.contains_key(project_id) {
            match state.kind {
                StoredRefreshKind::Offline
                | StoredRefreshKind::RateLimited
                | StoredRefreshKind::AuthFailed => {}
                StoredRefreshKind::TrackerError => {
                    return RefreshStatus::TrackerError {
                        fetched_at_ms: state.fetched_at_ms,
                        data_complete: false,
                        next_refresh_in_ms: next,
                        detail: state.detail.clone(),
                    };
                }
                _ => {
                    return RefreshStatus::Incomplete {
                        fetched_at_ms: state.fetched_at_ms,
                        next_refresh_in_ms: next,
                        detail: state.detail.clone(),
                    };
                }
            }
        }
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
            StoredRefreshKind::Incomplete => RefreshStatus::Incomplete {
                fetched_at_ms: state.fetched_at_ms,
                next_refresh_in_ms: next,
                detail: state.detail.clone(),
            },
            StoredRefreshKind::TrackerError => RefreshStatus::TrackerError {
                fetched_at_ms: state.fetched_at_ms,
                data_complete: state.complete,
                next_refresh_in_ms: next,
                detail: state.detail.clone(),
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
            StoredRefreshKind::Ready
            | StoredRefreshKind::Offline
            | StoredRefreshKind::Incomplete
            | StoredRefreshKind::TrackerError => Some(
                (state
                    .last_attempt_ms
                    .saturating_add(self.refresh_interval_ms))
                .saturating_sub(self.now_ms),
            ),
            StoredRefreshKind::NeverFetched | StoredRefreshKind::AuthFailed => None,
        }
    }

    fn set_client_view(&mut self, client_id: &str, project_id: &str, visible: bool) -> bool {
        if !visible || project_id.is_empty() {
            self.client_views.remove(client_id);
            return false;
        }
        let previous = self.client_views.insert(
            client_id.to_string(),
            ClientView {
                project_id: project_id.to_string(),
                visible: true,
                last_seen_ms: self.now_ms,
            },
        );
        let changed = previous
            .map(|view| !view.visible || view.project_id != project_id)
            .unwrap_or(true);
        changed
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

    fn project_id_for_issue_request(
        &self,
        request: &serde_json::Value,
        issue_id: &str,
    ) -> Result<String, KernelError> {
        request
            .get("projectId")
            .and_then(|value| value.as_str())
            .filter(|project_id| self.project_contains_issue(project_id, issue_id))
            .or_else(|| {
                request
                    .get("clientView")
                    .and_then(|view| view.get("focusedProjectId"))
                    .and_then(|value| value.as_str())
                    .filter(|project_id| self.project_contains_issue(project_id, issue_id))
            })
            .or_else(|| {
                self.focused_project_id
                    .as_deref()
                    .filter(|project_id| self.project_contains_issue(project_id, issue_id))
            })
            .map(ToOwned::to_owned)
            .or_else(|| self.project_id_for_issue(issue_id).ok())
            .ok_or_else(|| KernelError::Protocol("unknown issue".into()))
    }

    fn project_contains_issue(&self, project_id: &str, issue_id: &str) -> bool {
        self.loaded_issues
            .get(project_id)
            .is_some_and(|issues| issues.iter().any(|issue| issue.id() == issue_id))
    }

    fn write_block_reason(&self, project_id: &str) -> String {
        match self.refresh.get(project_id).map(|state| state.kind) {
            Some(StoredRefreshKind::RateLimited) => {
                match self
                    .refresh
                    .get(project_id)
                    .and_then(|state| state.retry_at_ms)
                {
                    Some(retry_at_ms) => format!(
                        "cannot write to tracker: rate-limited (retry after {retry_at_ms}ms)"
                    ),
                    None => "cannot write to tracker: rate-limited".into(),
                }
            }
            Some(StoredRefreshKind::AuthFailed) => "cannot write to tracker: auth-failed".into(),
            Some(StoredRefreshKind::NeverFetched) | None => {
                "cannot write to tracker: never-fetched".into()
            }
            Some(StoredRefreshKind::Incomplete) => "cannot write to tracker: incomplete".into(),
            Some(StoredRefreshKind::TrackerError) => self
                .refresh
                .get(project_id)
                .and_then(|state| state.detail.as_deref())
                .map(|detail| format!("cannot write to tracker: tracker-error ({detail})"))
                .unwrap_or_else(|| "cannot write to tracker: tracker-error".into()),
            _ => "cannot write to tracker: offline".into(),
        }
    }

    fn probe_github(&mut self, github_host: &str, repository: &str) -> ProjectConnection {
        if self
            .precomputed_project_connection
            .as_ref()
            .is_some_and(|(host, repo, _)| host == github_host && repo == repository)
        {
            return self
                .precomputed_project_connection
                .take()
                .expect("checked precomputed Project connection")
                .2;
        }
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
                updates: "应用更新".into(),
                check_for_updates: "检查更新".into(),
                update_checking: "正在检查…".into(),
                update_available: "发现新版本".into(),
                update_ready: "有新版本可安装。确认后才会下载和安装。".into(),
                update_notes: "版本说明".into(),
                update_confirm: "下载并安装".into(),
                update_later: "稍后".into(),
                update_current: "已经是最新版本。".into(),
                update_unavailable_browser: "浏览器 Client 不能给 Host 换包。请在本机桌面应用中检查更新。".into(),
                update_active_runs: "还有活跃 Run，不能安装更新。请先让全部 Run 结束或停止。".into(),
                update_installing: "正在下载并安装…".into(),
                update_failed: "更新失败，Host 数据和 Client 设置都没有改变。".into(),
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
                next_step: "下一步".into(),
                forget_host: "忘记 Host".into(),
                forget_host_confirm_title: "忘记这个远程 Host？".into(),
                forget_host_confirm_body: "只会从当前 Client 移除连接信息，不会停止远程 Host 或撤销其他 Client。".into(),
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
                choose_directory: "选择目录".into(),
                choose_directory_desktop_only: "系统目录选择只在本机桌面窗口可用。浏览器 Client 请手动粘贴 Host 上的绝对路径。".into(),
                inferring_from_directory: "正在从本地目录推断…".into(),
                inference_failed: "这个目录没有可用的 Git remote，请手动填写仓库。".into(),
                active_project_edit_hint: "这个 Project 有活跃 Run，只能修改显示名称；要改目录或 GitHub 连接，请先停止所有活跃 Run。".into(),
                remote_project_hint: "这里填写远程 Host 上的绝对路径。请在远程 Host 上确认目录，或手动粘贴该路径；不会选择这台 Client 上的目录。".into(),
                operation_pending: "保存中…".into(),
                inference_pending: "推断中…".into(),
                retry_inference: "重试推断".into(),
                removal_pending: "移除中…".into(),
                github_host: "Git remote host".into(),
                repository: "仓库".into(),
                infer_from_directory: "从本地目录推断".into(),
                use_inference: "使用这份推断结果".into(),
                inference_hint: "选好本地目录后，显示名称默认用目录名；若只有一个合法 Git remote 会自动填充。检测到多个 remote 时才显示候选供确认；手工填写始终有效。".into(),
                save_registration: "保存登记".into(),
                cancel: "取消".into(),
                remove_confirm_title: "移除这个 Project？".into(),
                remove_confirm_body: "只取消这台 Host 上的登记。不会删除本地目录、git 仓库，也不会删除远端 Issue。".into(),
                remove_confirm: "只移除登记".into(),
                cannot_remove_active_run: "现在不能移除".into(),
                cannot_remove_active_run_body: "这个 Project 有活跃 Run。先停止或结束 Run，再回来移除。关闭 Client 或切换 Project 都不会停止 Run。".into(),
                remove_keep_claims_body: "这个 Project 有执行已停的票。移除只取消本机登记，Tracker 上的认领不会自动释放。".into(),
                continue_run: "继续".into(),
                release_claim: "释放认领".into(),
                execution_stopped: "执行已停".into(),
                waiting: "等待操作".into(),
                running: "运行中".into(),
                inject_line: "注入".into(),
                inject_placeholder: "注入一行".into(),
                notify_desktop: "桌面通知".into(),
                notify_sound: "通知声音".into(),
                notify_waiting: "等待操作".into(),
                notify_completed: "Run 已正常完成".into(),
                notify_abnormal: "Run 异常停止".into(),
                notify_crash: "Host 崩溃后已捡回".into(),
                got_it: "知道了".into(),
                auth_failed: "这个 Project 的 GitHub 凭据不可用。".into(),
                connection_unavailable: "这个 Project 暂时连不上 GitHub。".into(),
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
                view_dependencies: "查看依赖".into(),
                graph_overview: "未关闭 Issue 依赖概览".into(),
                graph_return_overview: "返回依赖概览".into(),
                graph_truncated: "共 {total} 个未关闭 Issue，展示 {shown} 个（已达上限）".into(),
                graph_no_dependencies: "当前范围没有 Dependency；仍可点击任意 Issue 查看其上下游。".into(),
                show_closed_context: "也显示已关闭上下文".into(),
                graph_center: "中心 Issue：{issue}".into(),
                graph_center_here: "从此处展开".into(),
                graph_show_complete: "查看完整上下游（{count} 个 Issue）".into(),
                graph_show_neighborhood: "收起到一跳上下游".into(),
                graph_show_more: "继续显示节点".into(),
                graph_canvas_limit: "画布显示 {shown}/{total}；其余 Issue 可在完整关系列表中搜索。".into(),
                graph_complete_list: "完整关系列表".into(),
                graph_search_placeholder: "搜索上下游 Issue".into(),
                graph_upstream: "上游".into(),
                graph_downstream: "下游".into(),
                graph_both: "上下游".into(),
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
                empty_incomplete: "Issue 数据没有完整读完。为避免误判，暂不显示 Frontier 和依赖图。".into(),
                empty_tracker_error: "Tracker 返回业务错误。为避免使用过期数据误判，暂不显示 Frontier 和依赖图。".into(),
                issue_document: "Issue 正文".into(),
                issue_document_loading: "正在读取 Issue 正文…".into(),
                issue_document_retry: "重试读取".into(),
                issue_document_stale: "正文读取失败；以下为只读的上次内容，数据截至".into(),
                issue_document_failed: "正文尚未成功读取。".into(),
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
                refresh_interval: "自动刷新间隔（秒）".into(),
                refresh_interval_help: "有人在看这块看板时，按这个间隔拉 Tracker。最短 15 秒，不设最长时间。".into(),
                unclear_issue: "对端看不清".into(),
                refresh_now: "刷新".into(),
                refresh_refreshing: "正在刷新".into(),
                refresh_as_of: "数据截至".into(),
                refresh_next: "下次刷新".into(),
                refresh_offline: "已离线".into(),
                refresh_offline_recovery: "检查运行 Host 的电脑网络后点“刷新”重试。".into(),
                refresh_never: "还没有成功拉过 Tracker。".into(),
                refresh_rate_limited: "已被限流".into(),
                refresh_retry: "大约可再刷新".into(),
                refresh_paused: "自动刷新已暂停，可手动再试。".into(),
                refresh_auth: "凭据不可用".into(),
                refresh_auth_recovery: "在运行 Host 的电脑更新 GitHub 凭据后点“刷新”重试。".into(),
                refresh_incomplete: "数据不完整".into(),
                refresh_tracker_error: "Tracker 业务错误".into(),
                new_run: "新建".into(),
                execute_run: "执行".into(),
                start_run: "启动".into(),
                start_run_pending: "启动中…".into(),
                switch_agent: "换一家".into(),
                pick_agent: "选择 Agent".into(),
                launch_title: "启动配置".into(),
                prefill_current: "预填来自这个 Project 上次成功启动，本次可改。".into(),
                prefill_other: "预填来自其它 Project 上这家 Agent 的记忆，本次可改。".into(),
                prefill_seed: "预填来自本机 CLI 种子，本次可改。".into(),
                isolation: "隔离执行目录".into(),
                isolation_off_reason: "为什么不可用".into(),
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
                view_changes: "查看改动".into(),
                focus_run: "聚焦".into(),
                open_issue: "浏览器打开".into(),
                search_title: "搜索 Issue".into(),
                search_placeholder: "按标题搜索，回车才查".into(),
                search_all_triage: "全部 triage".into(),
                search_all_states: "open / closed".into(),
                search_open: "open".into(),
                search_closed: "closed".into(),
                search_submit: "搜索".into(),
                keyboard_help: "键盘帮助".into(),
                keyboard_help_body: "J / K 或方向键在看板卡片间移动；Enter 打开详情；/ 聚焦搜索；? 打开或关闭帮助；Escape 关闭帮助。终端聚焦时快捷键全部交给官方 TUI。".into(),
                this_round: "这一轮".into(),
                uncommitted: "未提交".into(),
                add_change_note: "写下改动备注".into(),
                change_note_placeholder: "一句话，下次开跑才带上".into(),
                delete_change_note: "删掉备注".into(),
                auto_advance: "自动推进".into(),
                auto_advance_help: "Host 总开关。还要在 Project 上打开才会领下一张 ready-for-agent。".into(),
                project_auto_advance: "这个 Project 自动推进".into(),
                restore_auto_advance: "冷启动后恢复自动推进".into(),
                restore_delay: "冷启动后等待秒数".into(),
                pending_confirmation: "待确认：即将领下一张 ready-for-agent".into(),
                veto_advance: "否决".into(),
                usage: "用量".into(),
                usage_hint: "这台 Host 上全部 Project 的 token 流水。不估美元，不管账号额度。".into(),
                host_overview: "总览".into(),
                host_overview_hint: "当前 Host 的 Project 态势与 Run；Run 按终端状态分组。".into(),
                host_overview_empty: "尚无通过 Agent Taskboard 启动的 Run；Project 态势仍在上方可见。".into(),
                return_to_board: "返回看板".into(),
                show_sidebar: "显示侧栏".into(),
                hide_sidebar: "收起侧栏".into(),
                show_issue_detail: "显示详情".into(),
                hide_issue_detail: "收起详情".into(),
                show_ended_runs: "显示已结束".into(),
                run_group_waiting: "等待操作".into(),
                run_group_running: "进行中".into(),
                run_group_stopped: "执行已停".into(),
                run_group_ended: "已结束".into(),
                range_24_hours: "24 小时".into(),
                range_today: "今天".into(),
                range_7_days: "7 天".into(),
                range_30_days: "30 天".into(),
                range_custom: "自定义".into(),
                filter_all: "全部".into(),
                filter_project: "Project".into(),
                filter_agent: "Agent".into(),
                filter_model: "模型".into(),
                token_input: "input".into(),
                token_output: "output".into(),
                token_cache_read: "cache read".into(),
                token_cache_write: "cache write".into(),
                token_reasoning: "reasoning".into(),
                token_total: "total".into(),
                ttft: "首字".into(),
                gen_rate: "生成速率".into(),
                cache_hit: "缓存命中".into(),
                spike: "偏慢".into(),
                proxy_disclaimer: "看板不管理 Clash 等节点。通路抖动时请到你自己的代理工具里换节点。".into(),
                open_host_usage: "Host 用量 ↗".into(),
                open_this_run: "打开此 Run 终端 ↗".into(),
                lane_main: "主会话".into(),
                lane_subagent: "子代理".into(),
                lane_switched: "已停用".into(),
                usage_empty: "这段时间没有 Run 用量。".into(),
                close_usage: "返回看板".into(),
                mobile_switch_scope: "切换范围".into(),
                mobile_board: "看板".into(),
                mobile_issue: "票".into(),
                mobile_run: "Run".into(),
                mobile_recent_output: "最近输出".into(),
                mobile_live_terminal: "打开活终端".into(),
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
                updates: "App updates".into(),
                check_for_updates: "Check for updates".into(),
                update_checking: "Checking…".into(),
                update_available: "Update available".into(),
                update_ready: "A new version is ready. Nothing downloads or installs until you confirm.".into(),
                update_notes: "Release notes".into(),
                update_confirm: "Download and install".into(),
                update_later: "Later".into(),
                update_current: "You already have the latest version.".into(),
                update_unavailable_browser: "A browser Client cannot replace a Host installation. Check from the desktop app on that machine.".into(),
                update_active_runs: "An active Run is still running, so the update cannot be installed. End or stop every Run first.".into(),
                update_installing: "Downloading and installing…".into(),
                update_failed: "The update failed. Host data and Client settings were not changed.".into(),
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
                next_step: "Next".into(),
                forget_host: "Forget Host".into(),
                forget_host_confirm_title: "Forget this remote Host?".into(),
                forget_host_confirm_body: "This removes the connection from this Client only. It does not stop the remote Host or revoke other Clients.".into(),
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
                choose_directory: "Choose folder".into(),
                choose_directory_desktop_only: "The system folder picker is only available in the desktop window on this machine. In a browser Client, paste an absolute path on the Host.".into(),
                inferring_from_directory: "Inferring from the local directory…".into(),
                inference_failed: "No usable Git remote was found in this directory. Enter the repository manually.".into(),
                active_project_edit_hint: "This Project has an active Run. Only the display name can be changed; stop all active Runs before changing its directory or GitHub connection.".into(),
                remote_project_hint: "Enter an absolute path on the remote Host. Confirm it on that Host or paste it manually; this Client will not choose a local folder.".into(),
                operation_pending: "Saving…".into(),
                inference_pending: "Inferring…".into(),
                retry_inference: "Retry inference".into(),
                removal_pending: "Removing…".into(),
                github_host: "Git remote host".into(),
                repository: "Repository".into(),
                infer_from_directory: "Infer from local directory".into(),
                use_inference: "Use this inference".into(),
                inference_hint: "After you choose a local directory, the display name defaults to the folder name. A single valid Git remote is filled automatically; candidates appear only when multiple remotes are detected. Manual values always remain valid.".into(),
                save_registration: "Save registration".into(),
                cancel: "Cancel".into(),
                remove_confirm_title: "Remove this Project?".into(),
                remove_confirm_body: "This only unregisters it on this Host. It does not delete the local directory, git repository, or remote Issues.".into(),
                remove_confirm: "Remove registration only".into(),
                cannot_remove_active_run: "Cannot remove now".into(),
                cannot_remove_active_run_body: "This Project has an active Run. Stop or finish the Run first. Closing a Client or switching Project does not stop the Run.".into(),
                remove_keep_claims_body: "This Project has execution-stopped issues. Removing only unregisters it here; Tracker claims are not released.".into(),
                continue_run: "Continue".into(),
                release_claim: "Release claim".into(),
                execution_stopped: "Execution stopped".into(),
                waiting: "Waiting".into(),
                running: "Running".into(),
                inject_line: "Inject".into(),
                inject_placeholder: "Inject a line".into(),
                notify_desktop: "Desktop notifications".into(),
                notify_sound: "Notification sound".into(),
                notify_waiting: "Waiting for you".into(),
                notify_completed: "Run finished".into(),
                notify_abnormal: "Run stopped abnormally".into(),
                notify_crash: "Recovered after Host crash".into(),
                got_it: "Got it".into(),
                auth_failed: "GitHub credentials for this Project are not available.".into(),
                connection_unavailable: "This Project cannot reach GitHub right now.".into(),
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
                view_dependencies: "View dependencies".into(),
                graph_overview: "Open Issue dependency overview".into(),
                graph_return_overview: "Back to dependency overview".into(),
                graph_truncated: "{total} open Issues; showing {shown} at the limit".into(),
                graph_no_dependencies: "No Dependencies in this range. Select any Issue to inspect its upstream and downstream.".into(),
                show_closed_context: "Also show closed context".into(),
                graph_center: "Center Issue: {issue}".into(),
                graph_center_here: "Expand from here".into(),
                graph_show_complete: "View complete upstream and downstream ({count} Issues)".into(),
                graph_show_neighborhood: "Collapse to one hop".into(),
                graph_show_more: "Show more nodes".into(),
                graph_canvas_limit: "Canvas shows {shown}/{total}; search the complete relationship list for the rest.".into(),
                graph_complete_list: "Complete relationship list".into(),
                graph_search_placeholder: "Search upstream and downstream Issues".into(),
                graph_upstream: "Upstream".into(),
                graph_downstream: "Downstream".into(),
                graph_both: "Both".into(),
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
                empty_incomplete: "Issue data could not be read completely. Frontier and the dependency graph are hidden to prevent incorrect decisions.".into(),
                empty_tracker_error: "The Tracker returned a business error. Frontier and the dependency graph are hidden to avoid decisions based on stale data.".into(),
                issue_document: "Issue document".into(),
                issue_document_loading: "Loading the Issue document…".into(),
                issue_document_retry: "Retry load".into(),
                issue_document_stale: "The document could not be refreshed. Showing the last read-only copy from".into(),
                issue_document_failed: "The document has never loaded successfully.".into(),
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
                refresh_interval: "Auto-refresh interval (seconds)".into(),
                refresh_interval_help: "While someone is looking at this board, pull Tracker on this interval. Minimum 15 seconds; there is no maximum.".into(),
                unclear_issue: "The other side is unclear".into(),
                refresh_now: "Refresh".into(),
                refresh_refreshing: "Refreshing".into(),
                refresh_as_of: "Data as of".into(),
                refresh_next: "Next refresh".into(),
                refresh_offline: "Offline".into(),
                refresh_offline_recovery: "Check the network on the computer running the Host, then select Refresh to retry.".into(),
                refresh_never: "Tracker has never been fetched successfully.".into(),
                refresh_rate_limited: "Rate limited".into(),
                refresh_retry: "Can retry around".into(),
                refresh_paused: "Auto-refresh is paused. You can try again manually.".into(),
                refresh_auth: "Credentials unavailable".into(),
                refresh_auth_recovery: "Update the GitHub credentials on the computer running the Host, then select Refresh to retry.".into(),
                refresh_incomplete: "Incomplete data".into(),
                refresh_tracker_error: "Tracker business error".into(),
                new_run: "New".into(),
                execute_run: "Run".into(),
                start_run: "Start".into(),
                start_run_pending: "Starting…".into(),
                switch_agent: "Switch Agent".into(),
                pick_agent: "Choose Agent".into(),
                launch_title: "Launch".into(),
                prefill_current: "Prefill is this Project's last successful launch. You can change it this time.".into(),
                prefill_other: "Prefill is this Agent's memory from another Project. You can change it this time.".into(),
                prefill_seed: "Prefill is the local CLI seed. You can change it this time.".into(),
                isolation: "Isolated work directory".into(),
                isolation_off_reason: "Why it's unavailable".into(),
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
                view_changes: "View changes".into(),
                focus_run: "Focus".into(),
                open_issue: "Open in browser".into(),
                search_title: "Search Issues".into(),
                search_placeholder: "Search titles on Enter".into(),
                search_all_triage: "All triage".into(),
                search_all_states: "open / closed".into(),
                search_open: "open".into(),
                search_closed: "closed".into(),
                search_submit: "Search".into(),
                keyboard_help: "Keyboard help".into(),
                keyboard_help_body: "Use J / K or arrow keys to move between board cards; Enter opens details; / focuses search; ? opens or closes help; Escape closes help. When the terminal is focused, all shortcuts stay with the official TUI.".into(),
                this_round: "This round".into(),
                uncommitted: "Uncommitted".into(),
                add_change_note: "Add a change note".into(),
                change_note_placeholder: "One sentence. It goes into the next opening.".into(),
                delete_change_note: "Delete note".into(),
                auto_advance: "Auto-advance".into(),
                auto_advance_help: "Host master switch. A Project still has to turn it on before the next ready-for-agent is claimed.".into(),
                project_auto_advance: "Auto-advance this Project".into(),
                restore_auto_advance: "Restore auto-advance after cold start".into(),
                restore_delay: "Seconds to wait after cold start".into(),
                pending_confirmation: "Pending confirmation: about to claim the next ready-for-agent".into(),
                veto_advance: "Veto".into(),
                usage: "Usage".into(),
                usage_hint: "Token traffic for every Project on this Host. No dollar estimates and no account quotas.".into(),
                host_overview: "Overview".into(),
                host_overview_hint: "Project status and Runs on the current Host; Runs are grouped by terminal state.".into(),
                host_overview_empty: "No Runs have been started through Agent Taskboard yet; Project status remains visible above.".into(),
                return_to_board: "Back to board".into(),
                show_sidebar: "Show sidebar".into(),
                hide_sidebar: "Hide sidebar".into(),
                show_issue_detail: "Show details".into(),
                hide_issue_detail: "Hide details".into(),
                show_ended_runs: "Show ended Runs".into(),
                run_group_waiting: "Waiting".into(),
                run_group_running: "In progress".into(),
                run_group_stopped: "Execution stopped".into(),
                run_group_ended: "Ended".into(),
                range_24_hours: "24 hours".into(),
                range_today: "Today".into(),
                range_7_days: "7 days".into(),
                range_30_days: "30 days".into(),
                range_custom: "Custom".into(),
                filter_all: "All".into(),
                filter_project: "Project".into(),
                filter_agent: "Agent".into(),
                filter_model: "Model".into(),
                token_input: "input".into(),
                token_output: "output".into(),
                token_cache_read: "cache read".into(),
                token_cache_write: "cache write".into(),
                token_reasoning: "reasoning".into(),
                token_total: "total".into(),
                ttft: "TTFT".into(),
                gen_rate: "Generation rate".into(),
                cache_hit: "Cache hit".into(),
                spike: "Slow".into(),
                proxy_disclaimer: "The board does not manage Clash or other proxies. If the path is jittery, change nodes in your own proxy tool.".into(),
                open_host_usage: "Host usage ↗".into(),
                open_this_run: "Open this Run terminal ↗".into(),
                lane_main: "Main".into(),
                lane_subagent: "Subagent".into(),
                lane_switched: "Retired".into(),
                usage_empty: "No Run usage in this window.".into(),
                close_usage: "Back to board".into(),
                mobile_switch_scope: "Switch scope".into(),
                mobile_board: "Board".into(),
                mobile_issue: "Issue".into(),
                mobile_run: "Run".into(),
                mobile_recent_output: "Recent output".into(),
                mobile_live_terminal: "Open live terminal".into(),
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
    #[serde(default)]
    auto_advance: bool,
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
            auto_advance: false,
        }
    }
}

fn default_refresh_interval_ms() -> u64 {
    refresh::DEFAULT_REFRESH_INTERVAL_MS
}

fn default_tracker_kind() -> TrackerKind {
    TrackerKind::Github
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredProject {
    id: String,
    name: String,
    local_path: PathBuf,
    /// Tracker 类型；旧数据缺失时默认 GitHub。
    #[serde(default = "default_tracker_kind")]
    tracker: TrackerKind,
    github_host: String,
    repository: String,
    #[serde(default)]
    auto_advance: bool,
    #[serde(default)]
    restore_auto_advance: bool,
    #[serde(default = "advance::default_restore_delay_ms")]
    restore_delay_ms: u64,
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
    #[serde(default = "default_true")]
    notify_desktop: bool,
    #[serde(default = "default_true")]
    notify_sound: bool,
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
        bool,
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
                file.notify_desktop,
                file.notify_sound,
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
                true,
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
        notify_desktop: true,
        notify_sound: true,
    };
    write_json(path, &file)?;
    Ok((
        appearance,
        LOCAL_HOST_ID.to_string(),
        Vec::new(),
        board::DEFAULT_RECENT_LIMIT,
        CenterView::Board,
        true,
        true,
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

fn client_local_operation(op: &str) -> bool {
    matches!(
        op,
        "updateInstallGate"
            | "beginUpdateInstall"
            | "cancelUpdateInstall"
            | "hideWindow"
            | "showWindow"
            | "setLanguage"
            | "setTheme"
            | "pairRemoteHost"
            | "focusHost"
            | "setRecentCompletedLimit"
            | "refreshLaunchEnvironment"
            | "setShowCommandPreview"
            | "setNotificationPrefs"
    )
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

fn parse_issue_ref(id: &str) -> Result<IssueRef, KernelError> {
    let (repository, number) = parse_issue_id(id)
        .ok_or_else(|| KernelError::Protocol(format!("invalid issue id: {id}")))?;
    Ok(IssueRef::new(repository, number, ""))
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
    tracker: &dyn TrackerSeam,
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
        tracker: stored.tracker,
        github_host: stored.github_host,
        repository: stored.repository,
        connection: connection_from_probe(outcome, secrets_path, language),
        tracker_synced: false,
        auto_advance: stored.auto_advance,
        restore_auto_advance: stored.restore_auto_advance,
        restore_delay_ms: stored.restore_delay_ms,
        advance_ready_at_ms: None,
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

fn issue_document_body(state: Option<&IssueDocumentState>) -> Option<(String, u64)> {
    match state? {
        IssueDocumentState::Ready {
            body,
            fetched_at_ms,
        }
        | IssueDocumentState::Stale {
            body,
            fetched_at_ms,
            ..
        } => Some((body.clone(), *fetched_at_ms)),
        IssueDocumentState::Loading {
            body: Some(body),
            fetched_at_ms: Some(fetched_at_ms),
        } => Some((body.clone(), *fetched_at_ms)),
        IssueDocumentState::Unloaded
        | IssueDocumentState::Loading { .. }
        | IssueDocumentState::Failed { .. } => None,
    }
}

fn issue_document_failure(error: tracker::TrackerReadError) -> IssueDocumentFailure {
    match error {
        tracker::TrackerReadError::Offline { detail, .. } => IssueDocumentFailure {
            kind: IssueDocumentFailureKind::Offline,
            message: detail.unwrap_or_else(|| "Issue Tracker is offline".into()),
            retry_after_ms: None,
        },
        tracker::TrackerReadError::RateLimited { retry_after_ms } => IssueDocumentFailure {
            kind: IssueDocumentFailureKind::RateLimited,
            message: "Issue Tracker rate limit reached".into(),
            retry_after_ms,
        },
        tracker::TrackerReadError::Auth { detail, .. } => IssueDocumentFailure {
            kind: IssueDocumentFailureKind::Auth,
            message: detail.unwrap_or_else(|| "Issue Tracker authentication failed".into()),
            retry_after_ms: None,
        },
        tracker::TrackerReadError::Failed { detail } => IssueDocumentFailure {
            kind: IssueDocumentFailureKind::Tracker,
            message: detail.unwrap_or_else(|| "Issue Tracker could not load this Issue".into()),
            retry_after_ms: None,
        },
    }
}

#[derive(Debug)]
pub enum KernelError {
    Io(io::Error),
    Json(serde_json::Error),
    Protocol(String),
    Denied(String),
}

fn write_tracker_error(err: tracker::TrackerWriteError) -> KernelError {
    match err {
        tracker::TrackerWriteError::Failed { message } => KernelError::Denied(message),
        tracker::TrackerWriteError::Offline { detail, .. } => {
            KernelError::Denied(write_error_with_detail("offline", detail))
        }
        tracker::TrackerWriteError::Auth { detail, .. } => {
            KernelError::Denied(write_error_with_detail("auth-failed", detail))
        }
        tracker::TrackerWriteError::RateLimited { retry_after_ms } => {
            KernelError::Denied(match retry_after_ms {
                Some(ms) => {
                    format!("cannot write to tracker: rate-limited (retry after {ms}ms)")
                }
                None => "cannot write to tracker: rate-limited".into(),
            })
        }
    }
}

fn write_error_with_detail(kind: &str, detail: Option<String>) -> String {
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
