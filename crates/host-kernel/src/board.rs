use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::issue::{
    issue_id, label_mapping_active, DependencyRef, IssueRecord, IssueRef, TriageRole,
};

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

fn dependency_graph(
    issues: &[IssueRecord],
    requested_center_id: Option<&str>,
    complete: bool,
) -> DependencyGraph {
    let by_id: BTreeMap<String, &IssueRecord> =
        issues.iter().map(|issue| (issue.id(), issue)).collect();
    let mut refs: BTreeMap<String, &IssueRef> = BTreeMap::new();
    for issue in issues {
        for blocker in &issue.blocked_by {
            if let DependencyRef::Known(known) = blocker {
                refs.entry(known.id()).or_insert(known);
            }
        }
        for blocked in &issue.blocking {
            refs.entry(blocked.id()).or_insert(blocked);
        }
    }
    let mut all_node_ids: BTreeSet<String> = issues.iter().map(IssueRecord::id).collect();
    all_node_ids.extend(refs.keys().cloned());

    let mut all_edges: BTreeSet<(String, String)> = BTreeSet::new();
    for issue in issues {
        for blocker in &issue.blocked_by {
            if let DependencyRef::Known(known) = blocker {
                all_edges.insert((known.id(), issue.id()));
            }
        }
        for blocked in &issue.blocking {
            all_edges.insert((issue.id(), blocked.id()));
        }
    }

    if requested_center_id.is_none() {
        return dependency_overview(issues, &by_id, &refs, &all_edges);
    }

    let center_id = requested_center_id
        .filter(|id| all_node_ids.contains(*id))
        .map(str::to_string)
        .or_else(|| issues.iter().find(|issue| issue.open).map(IssueRecord::id))
        .or_else(|| issues.first().map(IssueRecord::id));
    let Some(center_id) = center_id else {
        return DependencyGraph {
            nodes: Vec::new(),
            edges: Vec::new(),
            mode: DependencyGraphMode::Focused,
            center_id: None,
            total_count: 0,
            complete,
            max_distance: 0,
            truncated: false,
            closed_count: 0,
        };
    };

    let mut outgoing: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut incoming: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (from, to) in &all_edges {
        outgoing.entry(from.clone()).or_default().push(to.clone());
        incoming.entry(to.clone()).or_default().push(from.clone());
    }
    let upstream = graph_distances(&center_id, &incoming);
    let downstream = graph_distances(&center_id, &outgoing);
    let closure_ids: BTreeSet<String> = upstream.keys().chain(downstream.keys()).cloned().collect();
    let node_ids: BTreeSet<String> = closure_ids
        .iter()
        .filter(|id| {
            complete
                || upstream.get(*id).is_some_and(|distance| *distance <= 1)
                || downstream.get(*id).is_some_and(|distance| *distance <= 1)
        })
        .cloned()
        .collect();
    let edge_set: BTreeSet<(String, String)> = all_edges
        .into_iter()
        .filter(|(from, to)| node_ids.contains(from) && node_ids.contains(to))
        .collect();

    let ranks = graph_ranks(&node_ids, &edge_set);
    let mut nodes: Vec<GraphNode> = node_ids
        .iter()
        .map(|id| {
            let upstream_distance = upstream.get(id).copied();
            let downstream_distance = downstream.get(id).copied();
            let relation = if id == &center_id {
                GraphRelation::Center
            } else {
                match (upstream_distance.is_some(), downstream_distance.is_some()) {
                    (true, true) => GraphRelation::Both,
                    (true, false) => GraphRelation::Upstream,
                    (false, true) => GraphRelation::Downstream,
                    (false, false) => GraphRelation::Center,
                }
            };
            let distance = upstream_distance
                .into_iter()
                .chain(downstream_distance)
                .min()
                .unwrap_or(0);
            graph_node(
                id,
                by_id.get(id).copied(),
                refs.get(id).copied(),
                ranks.get(id).copied().unwrap_or(0),
                distance,
                relation,
            )
        })
        .collect();
    nodes.sort_by(|a, b| a.rank.cmp(&b.rank).then_with(|| a.number.cmp(&b.number)));
    let edges = edge_set
        .into_iter()
        .map(|(from, to)| GraphEdge { from, to })
        .collect();
    let closed_count = closure_ids
        .iter()
        .filter(|id| {
            by_id
                .get(*id)
                .map(|issue| !issue.open)
                .or_else(|| refs.get(*id).and_then(|issue| issue.open.map(|open| !open)))
                .unwrap_or(false)
        })
        .count();
    let max_distance = upstream
        .values()
        .chain(downstream.values())
        .copied()
        .max()
        .unwrap_or(0);
    DependencyGraph {
        nodes,
        edges,
        mode: DependencyGraphMode::Focused,
        center_id: Some(center_id),
        total_count: closure_ids.len(),
        complete,
        max_distance,
        truncated: false,
        closed_count,
    }
}

fn dependency_overview(
    issues: &[IssueRecord],
    by_id: &BTreeMap<String, &IssueRecord>,
    refs: &BTreeMap<String, &IssueRef>,
    all_edges: &BTreeSet<(String, String)>,
) -> DependencyGraph {
    let participants: BTreeSet<&str> = all_edges
        .iter()
        .flat_map(|(from, to)| [from.as_str(), to.as_str()])
        .collect();
    let mut open_issues: Vec<&IssueRecord> = issues.iter().filter(|issue| issue.open).collect();
    open_issues.sort_by(|a, b| {
        participants
            .contains(b.id().as_str())
            .cmp(&participants.contains(a.id().as_str()))
            .then_with(|| b.number.cmp(&a.number))
            .then_with(|| a.id().cmp(&b.id()))
    });
    let total_count = open_issues.len();
    let node_ids: BTreeSet<String> = open_issues
        .into_iter()
        .take(DEPENDENCY_OVERVIEW_LIMIT)
        .map(IssueRecord::id)
        .collect();
    let edge_set: BTreeSet<(String, String)> = all_edges
        .iter()
        .filter(|(from, to)| node_ids.contains(from) && node_ids.contains(to))
        .cloned()
        .collect();
    let ranks = graph_ranks(&node_ids, &edge_set);
    let mut nodes: Vec<GraphNode> = node_ids
        .iter()
        .map(|id| {
            graph_node(
                id,
                by_id.get(id).copied(),
                refs.get(id).copied(),
                ranks.get(id).copied().unwrap_or(0),
                0,
                GraphRelation::Center,
            )
        })
        .collect();
    nodes.sort_by(|a, b| a.rank.cmp(&b.rank).then_with(|| b.number.cmp(&a.number)));
    let edges = edge_set
        .into_iter()
        .map(|(from, to)| GraphEdge { from, to })
        .collect();
    DependencyGraph {
        nodes,
        edges,
        mode: DependencyGraphMode::Overview,
        center_id: None,
        total_count,
        complete: false,
        max_distance: 0,
        truncated: total_count > DEPENDENCY_OVERVIEW_LIMIT,
        closed_count: 0,
    }
}

fn graph_distances(
    center_id: &str,
    adjacency: &BTreeMap<String, Vec<String>>,
) -> BTreeMap<String, u32> {
    let mut distances = BTreeMap::from([(center_id.to_string(), 0)]);
    let mut pending = std::collections::VecDeque::from([(center_id.to_string(), 0)]);
    while let Some((id, distance)) = pending.pop_front() {
        for next in adjacency.get(&id).into_iter().flatten() {
            if distances.contains_key(next) {
                continue;
            }
            distances.insert(next.clone(), distance + 1);
            pending.push_back((next.clone(), distance + 1));
        }
    }
    distances
}

fn graph_node(
    id: &str,
    issue: Option<&IssueRecord>,
    fallback: Option<&IssueRef>,
    rank: u32,
    distance: u32,
    relation: GraphRelation,
) -> GraphNode {
    if let Some(issue) = issue {
        return GraphNode {
            id: issue.id(),
            repository: issue.repository.clone(),
            number: issue.number,
            title: issue.title.clone(),
            open: issue.open,
            rank,
            distance,
            relation,
        };
    }
    if let Some(issue) = fallback {
        return GraphNode {
            id: issue.id(),
            repository: issue.repository.clone(),
            number: issue.number,
            title: issue.title.clone(),
            open: issue.open.unwrap_or(false),
            rank,
            distance,
            relation,
        };
    }
    let (repository, number) = split_issue_id(id);
    GraphNode {
        id: id.to_string(),
        repository,
        number,
        title: String::new(),
        open: false,
        rank,
        distance,
        relation,
    }
}

fn split_issue_id(id: &str) -> (String, u64) {
    match id.rsplit_once('#') {
        Some((repository, number)) => (repository.to_string(), number.parse().unwrap_or(0)),
        None => (id.to_string(), 0),
    }
}

fn graph_ranks(
    node_ids: &BTreeSet<String>,
    edges: &BTreeSet<(String, String)>,
) -> BTreeMap<String, u32> {
    let mut incoming: BTreeMap<String, u32> = node_ids.iter().map(|id| (id.clone(), 0)).collect();
    let mut outgoing: BTreeMap<String, Vec<String>> =
        node_ids.iter().map(|id| (id.clone(), Vec::new())).collect();
    for (from, to) in edges {
        if node_ids.contains(from) && node_ids.contains(to) {
            *incoming.entry(to.clone()).or_default() += 1;
            outgoing.entry(from.clone()).or_default().push(to.clone());
        }
    }
    let mut ranks: BTreeMap<String, u32> = BTreeMap::new();
    let mut pending: Vec<String> = incoming
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(id, _)| id.clone())
        .collect();
    pending.sort();
    for id in &pending {
        ranks.insert(id.clone(), 0);
    }
    let mut remaining = incoming;
    while let Some(id) = pending.pop() {
        let rank = ranks.get(&id).copied().unwrap_or(0);
        let nexts = outgoing.get(&id).cloned().unwrap_or_default();
        for next in nexts {
            let next_rank = ranks.get(&next).copied().unwrap_or(0).max(rank + 1);
            ranks.insert(next.clone(), next_rank);
            if let Some(count) = remaining.get_mut(&next) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    pending.push(next);
                }
            }
        }
    }
    for id in node_ids {
        ranks.entry(id.clone()).or_insert(0);
    }
    ranks
}

fn lane(issue: &IssueRecord) -> Lane {
    if issue.claimed() {
        Lane::InProgress
    } else if issue.unfinished_blocker() {
        Lane::Blocked
    } else {
        Lane::Frontier
    }
}

fn frontier_empty_reason(visible: &[&IssueRecord]) -> FrontierEmptyReason {
    let open: Vec<_> = visible.iter().copied().filter(|issue| issue.open).collect();
    if open.is_empty() {
        FrontierEmptyReason::NoOpen
    } else if open.iter().all(|issue| issue.claimed()) {
        FrontierEmptyReason::AllClaimed
    } else {
        FrontierEmptyReason::AllBlocked
    }
}

fn card(issue: &IssueRecord, mapping_active: bool) -> IssueCard {
    IssueCard {
        id: issue.id(),
        repository: issue.repository.clone(),
        number: issue.number,
        title: issue.title.clone(),
        url: issue.url.clone(),
        claimed_by: issue.assignees.clone(),
        labels: issue.labels.clone(),
        triage_role: if mapping_active {
            issue.triage_role()
        } else {
            None
        },
        open: issue.open,
        activity: None,
        run_id: None,
    }
}

fn issue_options(issues: &[IssueRecord]) -> Vec<IssueLink> {
    let mut options: Vec<IssueLink> = issues.iter().map(known_link_from_record).collect();
    options.sort_by(|a, b| {
        a.number
            .cmp(&b.number)
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
            .then_with(|| a.id.cmp(&b.id))
    });
    options
}

fn known_link_from_record(issue: &IssueRecord) -> IssueLink {
    IssueLink {
        id: issue.id(),
        repository: issue.repository.clone(),
        number: Some(issue.number),
        title: issue.title.clone(),
        open: Some(issue.open),
        visible: true,
    }
}

fn select_issue(issues: &[IssueRecord], id: &str, mapping_active: bool) -> Option<IssueDetail> {
    if let Some(issue) = issues.iter().find(|issue| issue.id() == id) {
        return Some(detail(issue, mapping_active));
    }
    issues.iter().find_map(|issue| {
        if issue
            .parent
            .as_ref()
            .is_some_and(|parent| parent.id() == id)
        {
            return Some(link_detail(issue.parent.as_ref().unwrap()));
        }
        if let Some(child) = issue.children.iter().find(|child| child.id() == id) {
            return Some(link_detail(child));
        }
        if let Some(blocked) = issue.blocking.iter().find(|blocked| blocked.id() == id) {
            return Some(link_detail(blocked));
        }
        issue.blocked_by.iter().find_map(|blocker| match blocker {
            DependencyRef::Known(known) if known.id() == id => Some(link_detail(known)),
            DependencyRef::Unclear { repository, number }
                if unclear_id(repository.as_deref(), *number).as_deref() == Some(id) =>
            {
                Some(unclear_detail(repository.as_deref(), *number))
            }
            _ => None,
        })
    })
}

fn detail(issue: &IssueRecord, mapping_active: bool) -> IssueDetail {
    IssueDetail {
        id: issue.id(),
        repository: issue.repository.clone(),
        number: issue.number,
        title: issue.title.clone(),
        url: issue.url.clone(),
        open: issue.open,
        claimed_by: issue.assignees.clone(),
        triage_role: if mapping_active {
            issue.triage_role()
        } else {
            None
        },
        labels: issue.labels.clone(),
        parent: issue.parent.as_ref().map(known_link),
        children: issue.children.iter().map(known_link).collect(),
        blocked_by: issue.blocked_by.iter().map(dependency_link).collect(),
        blocking: issue.blocking.iter().map(known_link).collect(),
        document: IssueDocumentState::Unloaded,
        execution_stopped: false,
        waiting_for_user: false,
        active_run_id: None,
    }
}

fn link_detail(issue: &IssueRef) -> IssueDetail {
    IssueDetail {
        id: issue.id(),
        repository: issue.repository.clone(),
        number: issue.number,
        title: issue.title.clone(),
        url: issue.url.clone(),
        open: issue.open.unwrap_or(true),
        claimed_by: Vec::new(),
        triage_role: None,
        labels: Vec::new(),
        parent: None,
        children: Vec::new(),
        blocked_by: Vec::new(),
        blocking: Vec::new(),
        document: IssueDocumentState::Unloaded,
        execution_stopped: false,
        waiting_for_user: false,
        active_run_id: None,
    }
}

fn unclear_detail(repository: Option<&str>, number: Option<u64>) -> IssueDetail {
    let repository = repository.unwrap_or("").to_string();
    IssueDetail {
        id: unclear_id(Some(&repository), number).unwrap_or_else(|| "unclear".into()),
        repository: repository.clone(),
        number: number.unwrap_or(0),
        title: String::new(),
        url: String::new(),
        open: true,
        claimed_by: Vec::new(),
        triage_role: None,
        labels: Vec::new(),
        parent: None,
        children: Vec::new(),
        blocked_by: Vec::new(),
        blocking: Vec::new(),
        document: IssueDocumentState::Unloaded,
        execution_stopped: false,
        waiting_for_user: false,
        active_run_id: None,
    }
}

fn known_link(issue: &IssueRef) -> IssueLink {
    IssueLink {
        id: issue.id(),
        repository: issue.repository.clone(),
        number: Some(issue.number),
        title: issue.title.clone(),
        open: issue.open,
        visible: true,
    }
}

fn dependency_link(blocker: &DependencyRef) -> IssueLink {
    match blocker {
        DependencyRef::Known(issue) => known_link(issue),
        DependencyRef::Unclear { repository, number } => IssueLink {
            id: unclear_id(repository.as_deref(), *number).unwrap_or_else(|| "unclear".into()),
            repository: repository.clone().unwrap_or_default(),
            number: *number,
            title: String::new(),
            open: None,
            visible: false,
        },
    }
}

fn unclear_id(repository: Option<&str>, number: Option<u64>) -> Option<String> {
    match (repository, number) {
        (Some(repository), Some(number)) if !repository.is_empty() => {
            Some(issue_id(repository, number))
        }
        _ => None,
    }
}
