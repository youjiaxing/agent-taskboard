use super::*;
use serde_json::Value;
use std::time::Duration;

pub(crate) struct LiveGitHubApi;

impl GitHubApi for LiveGitHubApi {
    fn probe_repo(&self, host: &str, repository: &str, token: &str) -> Result<(), ProbeError> {
        let url = github_repo_url(host, repository);
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(8))
            .build();
        match agent
            .get(&url)
            .set("Authorization", &format!("Bearer {token}"))
            .set("User-Agent", "Agent-Taskboard")
            .set("Accept", "application/vnd.github+json")
            .call()
        {
            Ok(_) => Ok(()),
            Err(ureq::Error::Status(code, response)) => Err(classify_rest_status(code, response)),
            Err(err) => Err(ProbeError::Unreachable(err.to_string())),
        }
    }

    fn list_issues_page(
        &self,
        host: &str,
        repository: &str,
        token: &str,
        after: Option<&str>,
    ) -> Result<NodePage, ProbeError> {
        let Some((owner, name)) = repository.split_once('/') else {
            return Err(ProbeError::Unreachable(
                "repository must be owner/name".into(),
            ));
        };
        let payload = graphql_post(
            host,
            token,
            GITHUB_ISSUES_QUERY,
            serde_json::json!({ "owner": owner, "name": name, "after": after }),
        )?;
        let connection = payload
            .pointer("/data/repository/issues")
            .cloned()
            .unwrap_or(Value::Null);
        let nodes = connection
            .get("nodes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let has_next_page = connection
            .pointer("/pageInfo/hasNextPage")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let end_cursor = connection
            .pointer("/pageInfo/endCursor")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        Ok(NodePage {
            nodes,
            has_next_page,
            end_cursor,
        })
    }

    fn list_issue_edges_page(
        &self,
        host: &str,
        repository: &str,
        token: &str,
        number: u64,
        edges: IssueEdges,
        after: Option<&str>,
    ) -> Result<NodePage, ProbeError> {
        let Some((owner, name)) = repository.split_once('/') else {
            return Err(ProbeError::Unreachable(
                "repository must be owner/name".into(),
            ));
        };
        let payload = graphql_post(
            host,
            token,
            &issue_edges_query(edges),
            serde_json::json!({
                "owner": owner,
                "name": name,
                "number": number,
                "after": after,
            }),
        )?;
        let connection = payload
            .pointer(&format!("/data/repository/issue/{}", edges.field()))
            .cloned()
            .unwrap_or(Value::Null);
        let nodes = connection
            .get("nodes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let has_next_page = connection
            .pointer("/pageInfo/hasNextPage")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let end_cursor = connection
            .pointer("/pageInfo/endCursor")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        Ok(NodePage {
            nodes,
            has_next_page,
            end_cursor,
        })
    }

    fn read_issue(
        &self,
        host: &str,
        repository: &str,
        token: &str,
        number: u64,
    ) -> Result<Value, ProbeError> {
        let url = format!("{}/issues/{number}", github_repo_url(host, repository));
        github_json("GET", &url, token, None)
    }

    fn viewer_login(&self, host: &str, token: &str) -> Result<String, ProbeError> {
        let url = github_user_url(host);
        let payload = github_json("GET", &url, token, None)?;
        payload
            .get("login")
            .and_then(Value::as_str)
            .filter(|login| !login.is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| ProbeError::Unreachable("GitHub user has no login".into()))
    }

    fn add_assignees(
        &self,
        host: &str,
        repository: &str,
        number: u64,
        token: &str,
        logins: &[String],
    ) -> Result<Value, ProbeError> {
        let url = github_assignees_url(host, repository, number);
        let body = serde_json::json!({ "assignees": logins });
        github_json("POST", &url, token, Some(&body))
    }

    fn remove_assignees(
        &self,
        host: &str,
        repository: &str,
        number: u64,
        token: &str,
        logins: &[String],
    ) -> Result<Value, ProbeError> {
        let url = github_assignees_url(host, repository, number);
        let body = serde_json::json!({ "assignees": logins });
        github_json("DELETE", &url, token, Some(&body))
    }

    fn create_issue(
        &self,
        host: &str,
        repository: &str,
        token: &str,
        title: &str,
        body: &str,
    ) -> Result<Value, ProbeError> {
        let url = format!("{}/issues", github_repo_url(host, repository));
        let body = serde_json::json!({ "title": title, "body": body });
        github_json("POST", &url, token, Some(&body))
    }

    fn update_issue(
        &self,
        host: &str,
        repository: &str,
        token: &str,
        number: u64,
        title: Option<&str>,
        body: Option<&str>,
    ) -> Result<Value, ProbeError> {
        let url = format!("{}/issues/{number}", github_repo_url(host, repository));
        let mut edit = serde_json::Map::new();
        if let Some(title) = title {
            edit.insert("title".into(), Value::String(title.to_string()));
        }
        if let Some(body) = body {
            edit.insert("body".into(), Value::String(body.to_string()));
        }
        github_json("PATCH", &url, token, Some(&Value::Object(edit)))
    }

    fn set_issue_state(
        &self,
        host: &str,
        repository: &str,
        token: &str,
        number: u64,
        open: bool,
    ) -> Result<Value, ProbeError> {
        let url = format!("{}/issues/{number}", github_repo_url(host, repository));
        let state = if open { "open" } else { "closed" };
        github_json(
            "PATCH",
            &url,
            token,
            Some(&serde_json::json!({ "state": state })),
        )
    }

    fn add_issue_comment(
        &self,
        host: &str,
        repository: &str,
        token: &str,
        number: u64,
        body: &str,
    ) -> Result<Value, ProbeError> {
        let url = format!(
            "{}/issues/{number}/comments",
            github_repo_url(host, repository)
        );
        let body = serde_json::json!({ "body": body });
        github_json("POST", &url, token, Some(&body))
    }

    fn issue_database_id(
        &self,
        host: &str,
        repository: &str,
        token: &str,
        number: u64,
    ) -> Result<u64, ProbeError> {
        let url = format!("{}/issues/{number}", github_repo_url(host, repository));
        let payload = github_json("GET", &url, token, None)?;
        payload
            .get("id")
            .and_then(Value::as_u64)
            .ok_or_else(|| ProbeError::Unreachable("GitHub issue has no id".into()))
    }

    fn issue_parent(
        &self,
        host: &str,
        repository: &str,
        token: &str,
        number: u64,
    ) -> Result<Option<Value>, ProbeError> {
        let url = format!(
            "{}/issues/{number}/parent",
            github_repo_url(host, repository)
        );
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(8))
            .build();
        let response = match agent
            .get(&url)
            .set("Authorization", &format!("Bearer {token}"))
            .set("User-Agent", "Agent-Taskboard")
            .set("Accept", "application/vnd.github+json")
            .call()
        {
            Ok(response) => response,
            Err(ureq::Error::Status(404, _)) => return Ok(None),
            Err(ureq::Error::Status(401 | 403, _)) => {
                return Err(ProbeError::Unauthorized { detail: None })
            }
            Err(ureq::Error::Status(429, response)) => {
                return Err(ProbeError::RateLimited {
                    retry_after_ms: parse_retry_after_ms(response.header("retry-after")),
                });
            }
            Err(ureq::Error::Status(code, _)) => {
                return Err(ProbeError::Unreachable(format!("GitHub HTTP {code}")));
            }
            Err(err) => return Err(ProbeError::Unreachable(err.to_string())),
        };
        let payload: Value = serde_json::from_str(
            &response
                .into_string()
                .map_err(|err| ProbeError::Unreachable(err.to_string()))?,
        )
        .map_err(|err| ProbeError::Unreachable(err.to_string()))?;
        Ok(Some(payload))
    }

    fn add_sub_issue(
        &self,
        host: &str,
        parent_repository: &str,
        token: &str,
        parent_number: u64,
        sub_issue_id: u64,
    ) -> Result<Value, ProbeError> {
        let url = format!(
            "{}/issues/{parent_number}/sub_issues",
            github_repo_url(host, parent_repository)
        );
        let body = serde_json::json!({ "sub_issue_id": sub_issue_id });
        github_json("POST", &url, token, Some(&body))
    }

    fn remove_sub_issue(
        &self,
        host: &str,
        parent_repository: &str,
        token: &str,
        parent_number: u64,
        sub_issue_id: u64,
    ) -> Result<Value, ProbeError> {
        let url = format!(
            "{}/issues/{parent_number}/sub_issue",
            github_repo_url(host, parent_repository)
        );
        let body = serde_json::json!({ "sub_issue_id": sub_issue_id });
        github_json("DELETE", &url, token, Some(&body))
    }

    fn add_blocked_by(
        &self,
        host: &str,
        repository: &str,
        token: &str,
        number: u64,
        blocking_issue_id: u64,
    ) -> Result<Value, ProbeError> {
        let url = format!(
            "{}/issues/{number}/dependencies/blocked_by",
            github_repo_url(host, repository)
        );
        let body = serde_json::json!({ "issue_id": blocking_issue_id });
        github_json("POST", &url, token, Some(&body))
    }

    fn remove_blocked_by(
        &self,
        host: &str,
        repository: &str,
        token: &str,
        number: u64,
        blocking_issue_id: u64,
    ) -> Result<Value, ProbeError> {
        let url = format!(
            "{}/issues/{number}/dependencies/blocked_by/{blocking_issue_id}",
            github_repo_url(host, repository)
        );
        github_json("DELETE", &url, token, None)
    }
}

pub(crate) fn github_user_url(host: &str) -> String {
    if host.eq_ignore_ascii_case("github.com") {
        "https://api.github.com/user".into()
    } else {
        format!("https://{host}/api/v3/user")
    }
}

pub(crate) fn github_assignees_url(host: &str, repository: &str, number: u64) -> String {
    format!(
        "{}/issues/{number}/assignees",
        github_repo_url(host, repository)
    )
}

pub(crate) fn github_json(
    method: &str,
    url: &str,
    token: &str,
    body: Option<&Value>,
) -> Result<Value, ProbeError> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(8))
        .build();
    let mut attempt = 0;
    let response = loop {
        let request = agent
            .request(method, url)
            .set("Authorization", &format!("Bearer {token}"))
            .set("User-Agent", "Agent-Taskboard")
            .set("Accept", "application/vnd.github+json");
        let response = match body {
            Some(body) => request
                .set("Content-Type", "application/json")
                .send_string(&body.to_string()),
            None => request.call(),
        };
        match response {
            Ok(response) => break response,
            Err(ureq::Error::Status(code, response)) => {
                return Err(classify_rest_status(code, response));
            }
            Err(err)
                if method.eq_ignore_ascii_case("GET")
                    && attempt < TRANSIENT_NETWORK_RETRIES
                    && is_transient_transport_error(&err) =>
            {
                attempt += 1;
                std::thread::sleep(transient_retry_delay(attempt));
            }
            Err(err) => return Err(ProbeError::Unreachable(err.to_string())),
        }
    };
    let payload: Value = serde_json::from_str(
        &response
            .into_string()
            .map_err(|err| ProbeError::Unreachable(err.to_string()))?,
    )
    .map_err(|err| ProbeError::Unreachable(err.to_string()))?;
    Ok(payload)
}

pub(crate) fn classify_rest_status(code: u16, response: ureq::Response) -> ProbeError {
    let retry_after_ms = parse_retry_after_ms(response.header("retry-after"));
    let remaining = response
        .header("x-ratelimit-remaining")
        .and_then(|value| value.parse::<u64>().ok());
    let detail = response.into_string().ok().and_then(|body| {
        serde_json::from_str::<Value>(&body)
            .ok()
            .and_then(|value| {
                value
                    .get("message")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
            .or_else(|| (!body.trim().is_empty()).then(|| body.trim().to_string()))
    });
    if code == 429 || (code == 403 && (retry_after_ms.is_some() || remaining == Some(0))) {
        return ProbeError::RateLimited { retry_after_ms };
    }
    if matches!(code, 401 | 403) {
        return ProbeError::Unauthorized { detail };
    }
    if matches!(code, 400 | 404 | 409 | 422) {
        return ProbeError::GraphQl {
            detail: detail.unwrap_or_else(|| format!("GitHub HTTP {code}")),
        };
    }
    ProbeError::Unreachable(
        detail
            .map(|detail| format!("GitHub HTTP {code}: {detail}"))
            .unwrap_or_else(|| format!("GitHub HTTP {code}")),
    )
}

pub(crate) fn graphql_post(
    host: &str,
    token: &str,
    query: &str,
    variables: Value,
) -> Result<Value, ProbeError> {
    let url = github_graphql_url(host);
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(20))
        .build();
    let body = serde_json::json!({ "query": query, "variables": variables });
    let mut attempt = 0;
    let response = loop {
        let response = agent
            .post(&url)
            .set("Authorization", &format!("Bearer {token}"))
            .set("User-Agent", "Agent-Taskboard")
            .set("Accept", "application/vnd.github+json")
            .set("Content-Type", "application/json")
            .send_string(&body.to_string());
        match response {
            Ok(response) => break response,
            Err(ureq::Error::Status(401 | 403, _)) => {
                return Err(ProbeError::Unauthorized { detail: None })
            }
            Err(ureq::Error::Status(404, _)) => {
                return Err(ProbeError::Unauthorized { detail: None })
            }
            Err(ureq::Error::Status(429, response)) => {
                return Err(ProbeError::RateLimited {
                    retry_after_ms: parse_retry_after_ms(response.header("retry-after")),
                });
            }
            Err(ureq::Error::Status(code, _)) => {
                return Err(ProbeError::Unreachable(format!("GitHub HTTP {code}")));
            }
            Err(err)
                if attempt < TRANSIENT_NETWORK_RETRIES && is_transient_transport_error(&err) =>
            {
                attempt += 1;
                std::thread::sleep(transient_retry_delay(attempt));
            }
            Err(err) => return Err(ProbeError::Unreachable(err.to_string())),
        }
    };
    let payload: Value = serde_json::from_str(
        &response
            .into_string()
            .map_err(|err| ProbeError::Unreachable(err.to_string()))?,
    )
    .map_err(|err| ProbeError::Unreachable(err.to_string()))?;
    classify_graphql_errors(&payload)?;
    Ok(payload)
}

const TRANSIENT_NETWORK_RETRIES: u8 = 2;

pub(crate) fn transient_retry_delay(attempt: u8) -> Duration {
    Duration::from_millis(100 * u64::from(attempt))
}

pub(crate) fn is_transient_transport_error(error: &ureq::Error) -> bool {
    if matches!(error, ureq::Error::Status(..)) {
        return false;
    }
    is_transient_transport_detail(&error.to_string())
}

pub(crate) fn is_transient_transport_detail(detail: &str) -> bool {
    let detail = detail.to_ascii_lowercase();
    [
        "unexpected end of file",
        "connection reset",
        "connection closed",
        "timed out",
        "timeout",
        "tls connection init failed",
    ]
    .iter()
    .any(|needle| detail.contains(needle))
}

pub(crate) fn classify_graphql_errors(payload: &Value) -> Result<(), ProbeError> {
    let Some(errors) = payload.get("errors").and_then(Value::as_array) else {
        return Ok(());
    };
    if errors.is_empty() {
        return Ok(());
    }
    let detail = errors
        .iter()
        .filter_map(|error| error.get("message").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("; ");
    let error_type = |error: &Value| {
        error
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    if errors.iter().any(|error| {
        matches!(
            error_type(error).as_str(),
            "UNAUTHENTICATED" | "INSUFFICIENT_SCOPES" | "FORBIDDEN"
        )
    }) {
        return Err(ProbeError::Unauthorized {
            detail: Some(detail),
        });
    }
    if errors
        .iter()
        .any(|error| error_type(error) == "RATE_LIMITED")
    {
        return Err(ProbeError::RateLimited {
            retry_after_ms: None,
        });
    }
    Err(ProbeError::GraphQl { detail })
}

pub(crate) fn connection_has_next_page(node: &Value, field: &str) -> bool {
    node.pointer(&format!("/{field}/pageInfo/hasNextPage"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

pub(crate) fn set_connection_nodes(node: &mut Value, field: &str, nodes: Vec<Value>) {
    node[field] = serde_json::json!({
        "pageInfo": { "hasNextPage": false },
        "nodes": nodes,
    });
}

const GITHUB_ISSUES_QUERY: &str = r#"
query($owner: String!, $name: String!, $after: String) {
  repository(owner: $owner, name: $name) {
    issues(first: 100, after: $after, states: [OPEN, CLOSED], orderBy: {field: UPDATED_AT, direction: DESC}) {
      pageInfo { hasNextPage endCursor }
      nodes {
        number
        title
        state
        closedAt
        url
        repository { nameWithOwner }
        parent { number title state repository { nameWithOwner } }
        assignees(first: 10) { nodes { login } }
        labels(first: 30) { nodes { name } }
        issueDependenciesSummary { blockedBy }
        blockedBy(first: 100) { pageInfo { hasNextPage endCursor } nodes { number title state repository { nameWithOwner } } }
        blocking(first: 100) { pageInfo { hasNextPage endCursor } nodes { number title state repository { nameWithOwner } } }
        subIssues(first: 100) { pageInfo { hasNextPage endCursor } nodes { number title state repository { nameWithOwner } } }
      }
    }
  }
}
"#;

pub(crate) fn issue_edges_query(edges: IssueEdges) -> String {
    format!(
        r#"query($owner: String!, $name: String!, $number: Int!, $after: String) {{
  repository(owner: $owner, name: $name) {{
    issue(number: $number) {{
      {}(first: 100, after: $after) {{ pageInfo {{ hasNextPage endCursor }} nodes {{ number title state repository {{ nameWithOwner }} }} }}
    }}
  }}
}}"#,
        edges.field()
    )
}

pub(crate) fn github_repo_url(host: &str, repository: &str) -> String {
    if host.eq_ignore_ascii_case("github.com") {
        format!("https://api.github.com/repos/{repository}")
    } else {
        format!("https://{host}/api/v3/repos/{repository}")
    }
}

pub(crate) fn github_web_issue_url(host: &str, repository: &str, number: u64) -> String {
    if host.eq_ignore_ascii_case("github.com") {
        format!("https://github.com/{repository}/issues/{number}")
    } else {
        format!("https://{host}/{repository}/issues/{number}")
    }
}

pub(crate) fn parse_retry_after_ms(value: Option<&str>) -> Option<u64> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse::<u64>().ok())
        .map(|seconds| seconds.saturating_mul(1000))
}

pub(crate) fn github_graphql_url(host: &str) -> String {
    if host.eq_ignore_ascii_case("github.com") {
        "https://api.github.com/graphql".into()
    } else {
        format!("https://{host}/api/graphql")
    }
}

#[cfg(test)]
mod tests {
    use super::is_transient_transport_detail;

    #[test]
    fn transient_tls_disconnects_are_retryable_but_business_errors_are_not() {
        assert!(is_transient_transport_detail(
            "tls connection init failed: unexpected end of file"
        ));
        assert!(is_transient_transport_detail("connection reset by peer"));
        assert!(!is_transient_transport_detail(
            "GitHub HTTP 422: validation failed"
        ));
    }
}

pub(crate) fn cursor_offset(cursor: Option<&str>) -> usize {
    cursor
        .and_then(|cursor| cursor.strip_prefix("cursor-"))
        .and_then(|offset| offset.parse::<usize>().ok())
        .unwrap_or(0)
}

pub(crate) fn github_node(
    host: &str,
    repository: &str,
    number: u64,
    title: &str,
    body: Option<&str>,
) -> Value {
    let mut node = serde_json::json!({
        "number": number,
        "title": title,
        "state": "OPEN",
        "url": format!("{}/issues/{number}", github_repo_url(host, repository)),
        "html_url": github_web_issue_url(host, repository, number),
        "repository": { "nameWithOwner": repository },
        "assignees": { "nodes": [] },
        "labels": { "nodes": [] },
        "parent": null,
        "subIssues": { "nodes": [] },
        "issueDependenciesSummary": { "blockedBy": 0 },
        "blockedBy": { "nodes": [] },
        "blocking": { "nodes": [] },
    });
    if let Some(body) = body {
        node["body"] = Value::String(body.to_string());
    }
    node
}

pub(crate) fn github_ref(node: &Value, repository: &str) -> Value {
    serde_json::json!({
        "number": node.get("number").and_then(Value::as_u64).unwrap_or(0),
        "title": node.get("title").and_then(Value::as_str).unwrap_or(""),
        "state": node.get("state").and_then(Value::as_str).unwrap_or("OPEN"),
        "repository": { "nameWithOwner": repository },
    })
}

pub(crate) fn ref_repository(value: &Value) -> Option<String> {
    value
        .pointer("/repository/full_name")
        .or_else(|| value.pointer("/repository/nameWithOwner"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

pub(crate) fn push_connection_node(item: &mut Value, field: &str, node_ref: &Value) -> bool {
    if let Some(connection) = item.get_mut(field).and_then(Value::as_object_mut) {
        let nodes = connection
            .entry("nodes".to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        if let Some(items) = nodes.as_array_mut() {
            let target_repository = ref_repository(node_ref);
            let target_number = node_ref.get("number").and_then(Value::as_u64);
            if items.iter().any(|item| {
                item.get("number").and_then(Value::as_u64) == target_number
                    && ref_repository(item) == target_repository
            }) {
                return false;
            }
            items.push(node_ref.clone());
            return true;
        }
    } else {
        item[field] = serde_json::json!({ "nodes": [node_ref.clone()] });
        return true;
    }
    false
}

pub(crate) fn remove_node_from_connection(item: &mut Value, field: &str, node_ref: &Value) -> bool {
    let Some(items) = item
        .get_mut(field)
        .and_then(|connection| connection.get_mut("nodes"))
        .and_then(Value::as_array_mut)
    else {
        return false;
    };
    let target_repository = ref_repository(node_ref);
    let target_number = node_ref.get("number").and_then(Value::as_u64);
    let before = items.len();
    items.retain(|item| {
        !(item.get("number").and_then(Value::as_u64) == target_number
            && ref_repository(item) == target_repository)
    });
    before != items.len()
}

pub(crate) fn bump_blocked_summary(item: &mut Value, field: &str, delta: i64) {
    let path = format!("/issueDependenciesSummary/{field}");
    let Some(current) = item.pointer(&path).and_then(Value::as_u64) else {
        return;
    };
    item["issueDependenciesSummary"][field] =
        serde_json::json!((current as i64 + delta).max(0) as u64);
}

pub(crate) fn mutate_assignee_logins(node: &mut Value, logins: &[String], add: bool) {
    let Some(assignees) = node.get_mut("assignees") else {
        node["assignees"] = serde_json::json!({ "nodes": [] });
        return mutate_assignee_logins(node, logins, add);
    };
    if let Some(items) = assignees.as_array_mut() {
        apply_login_delta(items, logins, add);
        return;
    }
    if let Some(items) = assignees.get_mut("nodes").and_then(Value::as_array_mut) {
        apply_login_delta(items, logins, add);
    }
}

pub(crate) fn apply_login_delta(items: &mut Vec<Value>, logins: &[String], add: bool) {
    for login in logins {
        if add {
            if !items
                .iter()
                .any(|item| item.get("login").and_then(Value::as_str) == Some(login))
            {
                items.push(serde_json::json!({ "login": login }));
            }
        } else {
            items.retain(|item| item.get("login").and_then(Value::as_str) != Some(login.as_str()));
        }
    }
}

pub(crate) fn map_github_nodes(nodes: &[Value], ctx: &ProbeContext<'_>) -> Vec<IssueRecord> {
    nodes
        .iter()
        .filter_map(|node| map_github_issue_node(node, ctx.repository, ctx.github_host))
        .collect()
}

pub(crate) fn map_issue_comment(
    node: &Value,
    host: &str,
    repository: &str,
    number: u64,
) -> IssueComment {
    let body = node
        .get("body")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let url = node
        .get("html_url")
        .or_else(|| node.get("url"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| github_web_issue_url(host, repository, number));
    let id = node
        .get("id")
        .and_then(Value::as_u64)
        .map(|id| id.to_string())
        .or_else(|| {
            node.get("id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| url.clone());
    IssueComment { id, url, body }
}
