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
    pub triage_role: Option<TriageRole>,
    pub open: bool,
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
        }
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
    pub label_mapping_active: bool,
    pub recent_limit: u32,
    pub refresh: RefreshStatus,
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

pub fn project_board(
    project_id: &str,
    loaded: Option<&[IssueRecord]>,
    parent_filter: Option<&str>,
    selected_id: Option<&str>,
    recent_limit: u32,
    refresh: RefreshStatus,
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
            label_mapping_active: false,
            recent_limit,
            refresh,
        };
    };

    let mapping_active = label_mapping_active(issues);
    let filter = parent_filter.and_then(|id| issues.iter().find(|issue| issue.id() == id));
    let visible: Vec<&IssueRecord> = if let Some(parent) = filter {
        issues
            .iter()
            .filter(|issue| {
                issue
                    .parent
                    .as_ref()
                    .is_some_and(|item| item.id() == parent.id())
                    || parent.children.iter().any(|child| child.id() == issue.id())
            })
            .collect()
    } else {
        issues.iter().collect()
    };

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

    let frontier_empty = if frontier.is_empty() {
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
        label_mapping_active: mapping_active,
        recent_limit,
        refresh,
    }
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
        triage_role: if mapping_active {
            issue.triage_role()
        } else {
            None
        },
        open: issue.open,
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
    }
}

fn link_detail(issue: &IssueRef) -> IssueDetail {
    IssueDetail {
        id: issue.id(),
        repository: issue.repository.clone(),
        number: issue.number,
        title: issue.title.clone(),
        url: format!(
            "https://github.com/{}/issues/{}",
            issue.repository, issue.number
        ),
        open: issue.open.unwrap_or(true),
        claimed_by: Vec::new(),
        triage_role: None,
        labels: Vec::new(),
        parent: None,
        children: Vec::new(),
        blocked_by: Vec::new(),
        blocking: Vec::new(),
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
