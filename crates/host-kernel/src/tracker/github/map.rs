use super::*;
use crate::issue::{DependencyRef, IssueRecord, IssueRef};
use serde_json::Value;

pub fn map_github_issue_node(
    node: &Value,
    fallback_repository: &str,
    github_host: &str,
) -> Option<IssueRecord> {
    if node.get("pull_request").is_some() {
        return None;
    }
    let number = node.get("number")?.as_u64()?;
    let title = node
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let repository = node
        .pointer("/repository/nameWithOwner")
        .and_then(Value::as_str)
        .unwrap_or(fallback_repository)
        .to_string();
    let state = node.get("state").and_then(Value::as_str).unwrap_or("OPEN");
    let open = !state.eq_ignore_ascii_case("closed");
    let url = node
        .get("html_url")
        .or_else(|| node.get("url"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| github_web_issue_url(github_host, &repository, number));
    let closed_at = node
        .get("closedAt")
        .or_else(|| node.get("closed_at"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let assignees = logins(node.get("assignees"));
    let labels = label_names(node.get("labels"));
    let parent = parse_ref(node.get("parent"), &repository, github_host);
    let children = connection_refs(
        node.get("subIssues").or_else(|| node.get("sub_issues")),
        &repository,
        github_host,
    );
    let mut blocked_by = connection_deps(
        node.get("blockedBy")
            .or_else(|| node.get("blocked_by"))
            .or_else(|| node.pointer("/dependencies/blocked_by")),
        &repository,
        github_host,
    );
    let blocking = connection_refs(
        node.get("blocking")
            .or_else(|| node.pointer("/dependencies/blocking")),
        &repository,
        github_host,
    );
    let summary_open = node
        .pointer("/issueDependenciesSummary/blockedBy")
        .or_else(|| node.pointer("/issue_dependencies_summary/blocked_by"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let known_open = blocked_by
        .iter()
        .filter(|blocker| blocker.unfinished())
        .count() as u64;
    if summary_open > known_open {
        for _ in 0..(summary_open - known_open) {
            blocked_by.push(DependencyRef::Unclear {
                repository: None,
                number: None,
            });
        }
    }
    Some(IssueRecord {
        repository,
        number,
        title,
        url,
        open,
        closed_at,
        assignees,
        labels,
        parent,
        children,
        blocked_by,
        blocking,
    })
}

pub(crate) fn logins(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| {
                item.get("login")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
            .collect(),
        Some(object) => object
            .get("nodes")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        item.get("login")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned)
                    })
                    .collect()
            })
            .unwrap_or_default(),
        None => Vec::new(),
    }
}

pub(crate) fn label_names(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| {
                item.get("name")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
            .collect(),
        Some(object) => object
            .get("nodes")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        item.get("name")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned)
                    })
                    .collect()
            })
            .unwrap_or_default(),
        None => Vec::new(),
    }
}

pub(crate) fn connection_refs(
    value: Option<&Value>,
    fallback_repository: &str,
    github_host: &str,
) -> Vec<IssueRef> {
    connection_nodes(value)
        .into_iter()
        .filter_map(|node| parse_ref(Some(node), fallback_repository, github_host))
        .collect()
}

pub(crate) fn connection_deps(
    value: Option<&Value>,
    fallback_repository: &str,
    github_host: &str,
) -> Vec<DependencyRef> {
    connection_nodes(value)
        .into_iter()
        .map(|node| parse_dep(node, fallback_repository, github_host))
        .collect()
}

pub(crate) fn connection_nodes(value: Option<&Value>) -> Vec<&Value> {
    match value {
        Some(Value::Array(items)) => items.iter().collect(),
        Some(object) => object
            .get("nodes")
            .and_then(Value::as_array)
            .map(|items| items.iter().collect())
            .unwrap_or_default(),
        None => Vec::new(),
    }
}

pub(crate) fn parse_ref(
    value: Option<&Value>,
    fallback_repository: &str,
    github_host: &str,
) -> Option<IssueRef> {
    let value = value?;
    if value.is_null() {
        return None;
    }
    let number = value.get("number")?.as_u64()?;
    let repository = value
        .pointer("/repository/nameWithOwner")
        .and_then(Value::as_str)
        .unwrap_or(fallback_repository)
        .to_string();
    let title = value
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let url = value
        .get("url")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| github_web_issue_url(github_host, &repository, number));
    Some(IssueRef {
        repository,
        number,
        title,
        open: value.get("state").and_then(Value::as_str).map(state_open),
        url,
    })
}

pub(crate) fn parse_dep(
    value: &Value,
    fallback_repository: &str,
    github_host: &str,
) -> DependencyRef {
    match parse_ref(Some(value), fallback_repository, github_host) {
        Some(issue) if issue.open.is_some() && !issue.repository.is_empty() => {
            DependencyRef::Known(issue)
        }
        Some(issue) => DependencyRef::Unclear {
            repository: Some(issue.repository),
            number: Some(issue.number),
        },
        None => DependencyRef::Unclear {
            repository: value
                .pointer("/repository/nameWithOwner")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            number: value.get("number").and_then(Value::as_u64),
        },
    }
}

pub(crate) fn state_open(state: &str) -> bool {
    !state.eq_ignore_ascii_case("closed")
}
