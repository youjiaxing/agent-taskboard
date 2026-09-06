use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::issue::{
    issue_id, label_mapping_active, DependencyRef, IssueRecord, IssueRef, TriageRole,
};

mod graph;
use graph::*;

pub const DEFAULT_RECENT_LIMIT: u32 = 5;
pub const MAX_RECENT_LIMIT: u32 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BoardEmptyReason {
    NoData,
    /// 有数据但读取不完整（截断等），不能当作全量数据绘制列。
    IncompleteRead,
    /// Tracker 返回业务错误；保留已知详情，但不能用旧数据计算 Frontier。
    TrackerError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FrontierEmptyReason {
    AllBlocked,
    AllClaimed,
    NoOpen,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueCard {
    pub id: String,
    pub repository: String,
    pub number: u64,
    pub title: String,
    pub url: String,
    pub claimed_by: Vec<String>,
    #[serde(default)]
    pub labels: Vec<String>,
    pub triage_role: Option<TriageRole>,
    pub open: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity: Option<IssueActivity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IssueActivity {
    Running,
    Waiting,
    ExecutionStopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueLink {
    pub id: String,
    pub repository: String,
    pub number: Option<u64>,
    pub title: String,
    pub open: Option<bool>,
    pub visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IssueDocumentFailureKind {
    Offline,
    RateLimited,
    Auth,
    Tracker,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueDocumentFailure {
    pub kind: IssueDocumentFailureKind,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum IssueDocumentState {
    Unloaded,
    Loading {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        body: Option<String>,
        #[serde(
            rename = "fetchedAtMs",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        fetched_at_ms: Option<u64>,
    },
    Ready {
        body: String,
        #[serde(rename = "fetchedAtMs")]
        fetched_at_ms: u64,
    },
    Stale {
        body: String,
        #[serde(rename = "fetchedAtMs")]
        fetched_at_ms: u64,
        failure: IssueDocumentFailure,
    },
    Failed {
        failure: IssueDocumentFailure,
    },
}

impl Default for IssueDocumentState {
    fn default() -> Self {
        Self::Unloaded
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueDetail {
    pub id: String,
    pub repository: String,
    pub number: u64,
    pub title: String,
    pub url: String,
    pub open: bool,
    pub claimed_by: Vec<String>,
    pub triage_role: Option<TriageRole>,
    pub labels: Vec<String>,
    pub parent: Option<IssueLink>,
    pub children: Vec<IssueLink>,
    pub blocked_by: Vec<IssueLink>,
    pub blocking: Vec<IssueLink>,
    #[serde(default)]
    pub document: IssueDocumentState,
    #[serde(default)]
    pub execution_stopped: bool,
    #[serde(default)]
    pub waiting_for_user: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_run_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardColumns {
    pub blocked: Vec<IssueCard>,
    pub frontier: Vec<IssueCard>,
    pub in_progress: Vec<IssueCard>,
    pub recently_completed: Vec<IssueCard>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum RefreshStatus {
    Refreshing {
        #[serde(rename = "fetchedAtMs")]
        fetched_at_ms: Option<u64>,
    },
    Ready {
        #[serde(rename = "fetchedAtMs")]
        fetched_at_ms: u64,
        #[serde(rename = "nextRefreshInMs")]
        next_refresh_in_ms: Option<u64>,
    },
    Offline {
        #[serde(rename = "fetchedAtMs")]
        fetched_at_ms: u64,
        #[serde(rename = "nextRefreshInMs")]
        next_refresh_in_ms: Option<u64>,
    },
    NeverFetched,
    RateLimited {
        #[serde(rename = "fetchedAtMs")]
        fetched_at_ms: Option<u64>,
        #[serde(rename = "retryAtMs")]
        retry_at_ms: Option<u64>,
    },
    AuthFailed {
        #[serde(rename = "fetchedAtMs")]
        fetched_at_ms: Option<u64>,
    },
    /// 拿到数据但不完整（分页截断等）；不能当作全量数据计算 Frontier/依赖图。
    Incomplete {
        #[serde(rename = "fetchedAtMs")]
        fetched_at_ms: Option<u64>,
        #[serde(
            rename = "nextRefreshInMs",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        next_refresh_in_ms: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    TrackerError {
        #[serde(rename = "fetchedAtMs")]
        fetched_at_ms: Option<u64>,
        #[serde(rename = "dataComplete")]
        data_complete: bool,
        #[serde(
            rename = "nextRefreshInMs",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        next_refresh_in_ms: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
}

impl RefreshStatus {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Refreshing { .. } => "refreshing",
            Self::Ready { .. } => "ready",
            Self::Offline { .. } => "offline",
            Self::NeverFetched => "never-fetched",
            Self::RateLimited { .. } => "rate-limited",
            Self::AuthFailed { .. } => "auth-failed",
            Self::Incomplete { .. } => "incomplete",
            Self::TrackerError { .. } => "tracker-error",
        }
    }

    pub fn fetched_at_ms(&self) -> Option<u64> {
        match self {
            Self::Refreshing { fetched_at_ms } => *fetched_at_ms,
            Self::Ready { fetched_at_ms, .. } => Some(*fetched_at_ms),
            Self::Offline { fetched_at_ms, .. } => Some(*fetched_at_ms),
            Self::NeverFetched => None,
            Self::RateLimited { fetched_at_ms, .. } => *fetched_at_ms,
            Self::AuthFailed { fetched_at_ms } => *fetched_at_ms,
            Self::Incomplete { fetched_at_ms, .. } => *fetched_at_ms,
            Self::TrackerError { fetched_at_ms, .. } => *fetched_at_ms,
        }
    }

    /// 当前数据能否当作全量数据使用；不完整时禁止计算 Frontier/依赖图。
    pub fn complete(&self) -> bool {
        match self {
            Self::Incomplete { .. } => false,
            Self::TrackerError { data_complete, .. } => *data_complete,
            _ => true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CenterView {
    #[default]
    Board,
    Graph,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: String,
    pub repository: String,
    pub number: u64,
    pub title: String,
    pub open: bool,
    pub rank: u32,
    #[serde(default)]
    pub distance: u32,
    #[serde(default)]
    pub relation: GraphRelation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GraphRelation {
    #[default]
    Center,
    Upstream,
    Downstream,
    Both,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    #[serde(default)]
    pub mode: DependencyGraphMode,
    #[serde(default)]
    pub center_id: Option<String>,
    #[serde(default)]
    pub total_count: usize,
    #[serde(default)]
    pub complete: bool,
    #[serde(default)]
    pub max_distance: u32,
    #[serde(default)]
    pub truncated: bool,
    /// Legacy protocol field retained for older Clients.
    pub closed_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DependencyGraphMode {
    Overview,
    #[default]
    Focused,
}

const DEPENDENCY_OVERVIEW_LIMIT: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IssueStateFilter {
    #[default]
    All,
    Open,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueSearch {
    pub title: String,
    pub triage_role: Option<TriageRole>,
    pub state: IssueStateFilter,
}

impl IssueSearch {
    fn active(&self) -> bool {
        !self.title.trim().is_empty()
            || self.triage_role.is_some()
            || self.state != IssueStateFilter::All
    }

    fn matches(&self, issue: &IssueRecord, mapping_active: bool) -> bool {
        let title_matches = self.title.trim().is_empty()
            || issue
                .title
                .to_lowercase()
                .contains(&self.title.trim().to_lowercase());
        let triage_matches = self
            .triage_role
            .is_none_or(|role| mapping_active && issue.triage_role() == Some(role));
        let state_matches = match self.state {
            IssueStateFilter::All => true,
            IssueStateFilter::Open => issue.open,
            IssueStateFilter::Closed => !issue.open,
        };
        title_matches && triage_matches && state_matches
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardSnapshot {
    pub project_id: String,
    pub columns: Option<BoardColumns>,
    pub empty: Option<BoardEmptyReason>,
    pub frontier_empty: Option<FrontierEmptyReason>,
    pub parent_filter: Option<IssueCard>,
    pub selected: Option<IssueDetail>,
    /// All Issues loaded for this Project, independent of the current board
    /// search/filter. The Client uses these stable links for relationship
    /// editing without having to infer options from the projected columns.
    #[serde(default)]
    pub issue_options: Vec<IssueLink>,
    pub label_mapping_active: bool,
    pub recent_limit: u32,
    pub refresh: RefreshStatus,
    pub graph: Option<DependencyGraph>,
    pub show_closed_graph_context: bool,
    pub search: IssueSearch,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectIssueCounts {
    pub data_available: bool,
    pub total: usize,
    pub open: usize,
    pub closed: usize,
    pub blocked: usize,
    pub frontier: usize,
    pub in_progress: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lane {
    Blocked,
    Frontier,
    InProgress,
}

pub fn clamp_recent_limit(limit: u32) -> u32 {
    limit.clamp(1, MAX_RECENT_LIMIT)
}

pub fn project_issue_counts(
    loaded: Option<&[IssueRecord]>,
    refresh: &RefreshStatus,
) -> ProjectIssueCounts {
    let Some(issues) = loaded else {
        return ProjectIssueCounts::default();
    };
    let mut counts = ProjectIssueCounts {
        data_available: refresh.complete(),
        total: issues.len(),
        open: issues.iter().filter(|issue| issue.open).count(),
        closed: issues.iter().filter(|issue| !issue.open).count(),
        ..ProjectIssueCounts::default()
    };
    if !refresh.complete() {
        return counts;
    }
    for issue in issues.iter().filter(|issue| issue.open) {
        match lane(issue) {
            Lane::Blocked => counts.blocked += 1,
            Lane::Frontier => counts.frontier += 1,
            Lane::InProgress => counts.in_progress += 1,
        }
    }
    counts
}

pub fn project_board(
    project_id: &str,
    loaded: Option<&[IssueRecord]>,
    parent_filter: Option<&str>,
    selected_id: Option<&str>,
    recent_limit: u32,
    refresh: RefreshStatus,
    graph_center_id: Option<&str>,
    complete_dependency_graph: bool,
    search: IssueSearch,
) -> BoardSnapshot {
    let recent_limit = clamp_recent_limit(recent_limit);
    let Some(issues) = loaded else {
        return BoardSnapshot {
            project_id: project_id.to_string(),
            columns: None,
            empty: Some(BoardEmptyReason::NoData),
            frontier_empty: None,
            parent_filter: None,
            selected: None,
            issue_options: Vec::new(),
            label_mapping_active: false,
            recent_limit,
            refresh,
            graph: None,
            show_closed_graph_context: complete_dependency_graph,
            search,
        };
    };

    let mapping_active = label_mapping_active(issues);
    // 数据不完整时不能当作全量数据计算 Frontier 与依赖图：
    // 不画四列、不画图，但保留已知数据的详情与父过滤视图。
    if !refresh.complete() {
        let empty = if matches!(refresh, RefreshStatus::TrackerError { .. }) {
            BoardEmptyReason::TrackerError
        } else {
            BoardEmptyReason::IncompleteRead
        };
        return BoardSnapshot {
            project_id: project_id.to_string(),
            columns: None,
            empty: Some(empty),
            frontier_empty: None,
            parent_filter: parent_filter
                .and_then(|id| issues.iter().find(|issue| issue.id() == id))
                .map(|issue| card(issue, mapping_active)),
            selected: selected_id.and_then(|id| select_issue(issues, id, mapping_active)),
            issue_options: issue_options(issues),
            label_mapping_active: mapping_active,
            recent_limit,
            refresh,
            graph: None,
            show_closed_graph_context: complete_dependency_graph,
            search,
        };
    }

    let filter = parent_filter.and_then(|id| issues.iter().find(|issue| issue.id() == id));
    let visible: Vec<&IssueRecord> = issues
        .iter()
        .filter(|issue| {
            filter.is_none_or(|parent| {
                issue
                    .parent
                    .as_ref()
                    .is_some_and(|item| item.id() == parent.id())
                    || parent.children.iter().any(|child| child.id() == issue.id())
            }) && search.matches(issue, mapping_active)
        })
        .collect();

    let mut blocked = Vec::new();
    let mut frontier = Vec::new();
    let mut in_progress = Vec::new();
    let mut closed = Vec::new();
    for issue in &visible {
        if !issue.open {
            closed.push(*issue);
            continue;
        }
        match lane(issue) {
            Lane::InProgress => in_progress.push(card(issue, mapping_active)),
            Lane::Blocked => blocked.push(card(issue, mapping_active)),
            Lane::Frontier => frontier.push(card(issue, mapping_active)),
        }
    }
    closed.sort_by(|a, b| {
        b.closed_at
            .as_deref()
            .unwrap_or("")
            .cmp(a.closed_at.as_deref().unwrap_or(""))
            .then_with(|| b.number.cmp(&a.number))
    });
    let recently_completed = closed
        .into_iter()
        .take(recent_limit as usize)
        .map(|issue| card(issue, mapping_active))
        .collect();

    let frontier_empty = if frontier.is_empty() && !search.active() {
        Some(frontier_empty_reason(&visible))
    } else {
        None
    };

    BoardSnapshot {
        project_id: project_id.to_string(),
        columns: Some(BoardColumns {
            blocked,
            frontier,
            in_progress,
            recently_completed,
        }),
        empty: None,
        frontier_empty,
        parent_filter: filter.map(|issue| card(issue, mapping_active)),
        selected: selected_id.and_then(|id| select_issue(issues, id, mapping_active)),
        issue_options: issue_options(issues),
        label_mapping_active: mapping_active,
        recent_limit,
        refresh,
        graph: Some(dependency_graph(
            issues,
            graph_center_id,
            complete_dependency_graph,
        )),
        show_closed_graph_context: complete_dependency_graph,
        search,
    }
}
