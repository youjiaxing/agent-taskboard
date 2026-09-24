//! Host 内核：桌面窗口、浏览器和以后的远程 Client 都走这一条接缝。

mod advance;
mod agent;
mod board;
mod changes;
mod copy;
mod issue;
mod kernel;
mod launch;
mod launch_env;
mod local_rpc;
mod owner;
mod pairing;
mod persist;
mod project;
mod protocol;
mod refresh;
mod rpc_handle;
mod run;
mod session;
mod tracker;
mod tracker_seam;
mod usage;

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

pub use advance::{PendingConfirmation, DEFAULT_RESTORE_DELAY_MS, PENDING_CONFIRM_MS};
pub use agent::{
    builtin_agents, intent_prefix, probe_binary, AgentConfigDiscovery, AgentField, AgentFieldKind,
    AgentFieldOptionFilter, AgentPort, AgentSummary, AntigravityAdapter, ClaudeAdapter,
    CodexAdapter, CompletionHookPlan, CompletionSignals, GrokAdapter, IntentOption, MemoryAgent,
    PrefillSource, ProbeResult, RunIntent, RunLaunchConfig, RunLaunchForm, ANTIGRAVITY_BIN,
    ANTIGRAVITY_ID, ANTIGRAVITY_NAME, CLAUDE_BIN, CLAUDE_CODE_ID, CLAUDE_CODE_NAME, CODEX_BIN,
    CODEX_ID, CODEX_NAME, GROK_BIN, GROK_BUILD_ID, GROK_BUILD_NAME,
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
pub use copy::ShellCopy;
pub use issue::{parse_issue_id, DependencyRef, IssueRecord, IssueRef, TriageRole};
pub use launch_env::{LaunchEnvPort, LaunchEnvironment, MemoryLaunchEnv, ShellLaunchEnv};
pub use local_rpc::{
    bind_local_rpc, local_client_origin_allowed, spawn_local_rpc, LoopbackAssets, LoopbackServer,
    LOCAL_RPC_PORT,
};
pub use pairing::{IssuedPairing, PairedClient, PairingOffer};
pub use persist::DataLayout;
pub use project::ProjectInference;
pub use protocol::*;
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

pub(crate) const LOCAL_HOST_ID: &str = "local";

pub(crate) use kernel::issue_documents::issue_document_body;
pub(crate) use kernel::projects::{auth_failure_message, connection_from_probe, probe_record};
pub(crate) use persist::{
    host_not_running_reason, load_client_tokens, load_or_init_appearance,
    load_or_init_host_settings, load_paired_clients, occupied_reason, read_github_pat,
    read_github_pats, write_json, write_json_inner, ClientSecretsFile, ClientSettingsFile,
    HostSecretsFile, HostSettingsFile, StoredProject, StoredProjectTombstone,
};
pub(crate) use protocol::{
    write_tracker_error, AppearanceSelection, ClientNavigationState, ClientView, LoopbackKind,
    ProjectRecord, RefreshTrigger, RemoteView,
};
pub(crate) use rpc_handle::{parse_issue_ref, tracker_kind_for_host};

pub struct HostKernel {
    running: bool,
    window_visible: bool,
    host_mode: HostMode,
    exiting: bool,
    host_display_name: String,
    data: DataLayout,
    appearance: AppearanceSelection,
    projects: Vec<ProjectRecord>,
    project_tombstones: Vec<StoredProjectTombstone>,
    focused_project_id: Option<String>,
    tracker: Arc<dyn TrackerSeam>,
    agents: Vec<Arc<dyn AgentPort>>,
    launch_env: Arc<dyn LaunchEnvPort>,
    sessions: Arc<dyn SessionFactory>,
    runs: Vec<RunSummary>,
    live: BTreeMap<String, Arc<dyn AgentSession>>,
    isolation_probes: BTreeMap<String, kernel::isolation::IsolationProbe>,
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
    deferred_remote_calls: Option<kernel::remote::RemoteCallBatch>,
    remote_call_generations: BTreeMap<(String, String), u64>,
    next_remote_call_generation: u64,
    loaded_issues: BTreeMap<String, Vec<IssueRecord>>,
    issue_documents: BTreeMap<String, BTreeMap<String, IssueDocumentState>>,
    issue_document_in_flight: BTreeMap<(String, String), u64>,
    next_issue_document_generation: u64,
    refresh: BTreeMap<String, kernel::refresh::ProjectRefreshState>,
    refresh_in_flight: BTreeMap<String, u64>,
    next_refresh_generation: u64,
    issue_write_generations: BTreeMap<String, u64>,
    defer_refreshes: bool,
    deferred_refreshes: Vec<kernel::refresh::PreparedRefresh>,
    defer_issue_documents: bool,
    deferred_issue_documents: Vec<kernel::issue_documents::PreparedIssueDocument>,
    defer_issue_writes: bool,
    deferred_issue_writes: Vec<kernel::issue_documents::PreparedIssueWrite>,
    local_tracker_revisions: BTreeMap<String, u64>,
    client_views: BTreeMap<String, ClientView>,
    client_navigation: BTreeMap<String, ClientNavigationState>,
    client_navigation_seed: ClientNavigationState,
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
    agent_config_cache: BTreeMap<(PathBuf, String), kernel::agent_config::CachedAgentConfig>,
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
        let project_tombstones = if host_mode == HostMode::HostAndClient {
            settings.project_tombstones.clone()
        } else {
            Vec::new()
        };
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

        let client_navigation_seed = ClientNavigationState {
            focused_host_id: focused_host_id.clone(),
            remote_view: None,
            focused_project_id: focused_project_id.clone(),
            selected_issue_id: None,
            parent_filter: None,
            issue_search: BTreeMap::new(),
            center_view,
            workspace_view: WorkspaceView::Project,
            graph_center_issue_id: None,
            complete_dependency_graph: false,
            focused_run_id: None,
            launch_form: None,
            usage_open: false,
            usage_query: usage::UsageQuery::default(),
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
            project_tombstones,
            focused_project_id,
            tracker,
            agents,
            launch_env,
            sessions,
            runs: Vec::new(),
            live: BTreeMap::new(),
            isolation_probes: BTreeMap::new(),
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
            deferred_remote_calls: None,
            remote_call_generations: BTreeMap::new(),
            next_remote_call_generation: 0,
            loaded_issues: BTreeMap::new(),
            issue_documents: BTreeMap::new(),
            issue_document_in_flight: BTreeMap::new(),
            next_issue_document_generation: 0,
            refresh: BTreeMap::new(),
            refresh_in_flight: BTreeMap::new(),
            next_refresh_generation: 0,
            issue_write_generations: BTreeMap::new(),
            defer_refreshes: false,
            deferred_refreshes: Vec::new(),
            defer_issue_documents: false,
            deferred_issue_documents: Vec::new(),
            defer_issue_writes: false,
            deferred_issue_writes: Vec::new(),
            local_tracker_revisions: BTreeMap::new(),
            client_views: BTreeMap::new(),
            client_navigation: BTreeMap::new(),
            client_navigation_seed,
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
            agent_config_cache: BTreeMap::new(),
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

    pub fn process_alive(&self) -> bool {
        !self.exiting
    }
}
