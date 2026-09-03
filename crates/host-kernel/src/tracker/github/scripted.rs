use super::*;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

#[derive(Clone, Default)]
pub struct ScriptedGitHub {
    pub env: BTreeMap<String, String>,
    pub gh_detected: bool,
    pub gh_tokens: BTreeMap<String, String>,
    pub accept_tokens: BTreeSet<String>,
    pub unreachable: bool,
    pub issues: BTreeMap<String, Vec<Value>>,
    pub read_unauthorized: bool,
    pub rate_limited: bool,
    pub retry_after_ms: Option<u64>,
    pub viewer_login: String,
    pub write_fail: bool,
    pub issue_page_size: usize,
    pub edge_page_size: usize,
    pub missing_issue_cursor: bool,
    pub missing_edge_cursor: bool,
    pub graphql_auth_error: Option<String>,
    pub graphql_business_error: Option<String>,
}

impl GitHubTracker {
    pub fn scripted(script: ScriptedGitHub) -> Self {
        Self {
            env: Box::new(MapEnv(script.env.clone())),
            gh: Box::new(MapGh {
                detected: script.gh_detected,
                tokens: script.gh_tokens.clone(),
            }),
            api: Box::new(MapApi {
                accept: script.accept_tokens.clone(),
                unreachable: script.unreachable,
                issues: Mutex::new(script.issues.clone()),
                read_unauthorized: script.read_unauthorized,
                rate_limited: script.rate_limited,
                retry_after_ms: script.retry_after_ms,
                viewer_login: if script.viewer_login.trim().is_empty() {
                    MEMORY_TRACKER_ACTOR.to_string()
                } else {
                    script.viewer_login
                },
                write_fail: script.write_fail,
                issue_page_size: script.issue_page_size,
                edge_page_size: script.edge_page_size,
                missing_issue_cursor: script.missing_issue_cursor,
                missing_edge_cursor: script.missing_edge_cursor,
                graphql_auth_error: script.graphql_auth_error,
                graphql_business_error: script.graphql_business_error,
                comment_seq: Mutex::new(0),
            }),
        }
    }
}

struct MapEnv(BTreeMap<String, String>);

impl EnvSource for MapEnv {
    fn var(&self, key: &str) -> Option<String> {
        self.0
            .get(key)
            .cloned()
            .and_then(|value| nonempty(Some(&value)))
    }
}

struct MapGh {
    detected: bool,
    tokens: BTreeMap<String, String>,
}

impl GhAuth for MapGh {
    fn detected(&self) -> bool {
        self.detected
    }

    fn token(&self, hostname: &str) -> Option<String> {
        self.tokens
            .get(hostname)
            .cloned()
            .and_then(|value| nonempty(Some(&value)))
    }
}

struct MapApi {
    accept: BTreeSet<String>,
    unreachable: bool,
    issues: Mutex<BTreeMap<String, Vec<Value>>>,
    read_unauthorized: bool,
    rate_limited: bool,
    retry_after_ms: Option<u64>,
    viewer_login: String,
    write_fail: bool,
    issue_page_size: usize,
    edge_page_size: usize,
    missing_issue_cursor: bool,
    missing_edge_cursor: bool,
    graphql_auth_error: Option<String>,
    graphql_business_error: Option<String>,
    comment_seq: Mutex<u64>,
}

impl MapApi {
    fn guard_read(&self, token: &str) -> Result<(), ProbeError> {
        if self.unreachable {
            return Err(ProbeError::Unreachable("unreachable".into()));
        }
        if self.rate_limited {
            return Err(ProbeError::RateLimited {
                retry_after_ms: self.retry_after_ms,
            });
        }
        if self.read_unauthorized || !self.accept.contains(token) {
            return Err(ProbeError::Unauthorized { detail: None });
        }
        Ok(())
    }

    fn guard_write(&self, token: &str) -> Result<(), ProbeError> {
        if self.unreachable {
            return Err(ProbeError::Unreachable("unreachable".into()));
        }
        if self.rate_limited {
            return Err(ProbeError::RateLimited {
                retry_after_ms: self.retry_after_ms,
            });
        }
        if self.write_fail || self.read_unauthorized || !self.accept.contains(token) {
            return Err(ProbeError::Unauthorized { detail: None });
        }
        Ok(())
    }

    fn guard_graphql(&self) -> Result<(), ProbeError> {
        if let Some(detail) = &self.graphql_auth_error {
            return Err(ProbeError::Unauthorized {
                detail: Some(detail.clone()),
            });
        }
        if let Some(detail) = &self.graphql_business_error {
            return Err(ProbeError::GraphQl {
                detail: detail.clone(),
            });
        }
        Ok(())
    }

    fn page_size(&self) -> usize {
        self.issue_page_size.max(1)
    }

    fn edge_size(&self) -> usize {
        self.edge_page_size.max(100)
    }

    fn patch_connection_pages(&self, node: &mut Value) {
        let edge_size = self.edge_size();
        for field in ["blockedBy", "blocking", "subIssues"] {
            let Some(connection) = node.get_mut(field).and_then(Value::as_object_mut) else {
                continue;
            };
            let Some(items) = connection.get_mut("nodes").and_then(Value::as_array_mut) else {
                continue;
            };
            let total = items.len();
            if total > edge_size {
                items.truncate(edge_size);
            }
            let has_next = total > edge_size;
            connection.insert(
                "pageInfo".into(),
                serde_json::json!({
                    "hasNextPage": has_next,
                    "endCursor": has_next.then(|| format!("cursor-{edge_size}")),
                }),
            );
        }
    }

    fn mutate_assignees(
        &self,
        repository: &str,
        number: u64,
        token: &str,
        logins: &[String],
        add: bool,
    ) -> Result<Value, ProbeError> {
        self.guard_write(token)?;
        let mut issues = self.issues.lock().expect("scripted github");
        let Some(node) = issues.get_mut(repository).and_then(|items| {
            items
                .iter_mut()
                .find(|item| item.get("number").and_then(Value::as_u64) == Some(number))
        }) else {
            return Err(ProbeError::Unreachable("unknown issue".into()));
        };
        mutate_assignee_logins(node, logins, add);
        Ok(node.clone())
    }

    fn locate(
        &self,
        issues: &BTreeMap<String, Vec<Value>>,
        repository: &str,
        number: u64,
    ) -> Option<(String, usize)> {
        issues.iter().find_map(|(repo, items)| {
            items
                .iter()
                .position(|item| {
                    item.get("number").and_then(Value::as_u64) == Some(number)
                        && ref_repository(item).as_deref() == Some(repository)
                })
                .map(|index| (repo.clone(), index))
        })
    }

    fn locate_database_id(
        &self,
        issues: &BTreeMap<String, Vec<Value>>,
        database_id: u64,
    ) -> Option<(String, usize)> {
        issues.iter().find_map(|(repo, items)| {
            items
                .iter()
                .position(|item| {
                    item.get("id").and_then(Value::as_u64) == Some(database_id)
                        || item.get("number").and_then(Value::as_u64) == Some(database_id)
                })
                .map(|index| (repo.clone(), index))
        })
    }
}

fn node_at_mut<'a>(
    issues: &'a mut BTreeMap<String, Vec<Value>>,
    repository: &str,
    index: usize,
) -> &'a mut Value {
    issues
        .get_mut(repository)
        .and_then(|items| items.get_mut(index))
        .expect("scripted github issue")
}

impl GitHubApi for MapApi {
    fn probe_repo(&self, _host: &str, _repository: &str, token: &str) -> Result<(), ProbeError> {
        if self.unreachable {
            return Err(ProbeError::Unreachable("unreachable".into()));
        }
        if self.rate_limited {
            return Ok(());
        }
        if self.accept.contains(token) {
            Ok(())
        } else {
            Err(ProbeError::Unauthorized { detail: None })
        }
    }

    fn list_issues_page(
        &self,
        _host: &str,
        repository: &str,
        token: &str,
        after: Option<&str>,
    ) -> Result<NodePage, ProbeError> {
        self.guard_read(token)?;
        self.guard_graphql()?;
        let all = self
            .issues
            .lock()
            .expect("scripted github")
            .get(repository)
            .cloned()
            .unwrap_or_default();
        let (start, size) = (cursor_offset(after), self.page_size());
        let mut nodes: Vec<Value> = all.iter().skip(start).take(size).cloned().collect();
        let next = start + nodes.len();
        for node in nodes.iter_mut() {
            self.patch_connection_pages(node);
        }
        let has_next_page = next < all.len();
        Ok(NodePage {
            nodes,
            has_next_page,
            end_cursor: (has_next_page && !self.missing_issue_cursor)
                .then(|| format!("cursor-{next}")),
        })
    }

    fn list_issue_edges_page(
        &self,
        _host: &str,
        repository: &str,
        token: &str,
        number: u64,
        edges: IssueEdges,
        after: Option<&str>,
    ) -> Result<NodePage, ProbeError> {
        self.guard_read(token)?;
        self.guard_graphql()?;
        let all = self
            .issues
            .lock()
            .expect("scripted github")
            .get(repository)
            .and_then(|items| {
                items
                    .iter()
                    .find(|item| item.get("number").and_then(Value::as_u64) == Some(number))
            })
            .and_then(|node| node.get(edges.field()).and_then(|conn| conn.get("nodes")))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let (start, size) = (cursor_offset(after), self.edge_size());
        let nodes: Vec<Value> = all.iter().skip(start).take(size).cloned().collect();
        let next = start + nodes.len();
        let has_next_page = next < all.len();
        Ok(NodePage {
            nodes,
            has_next_page,
            end_cursor: (has_next_page && !self.missing_edge_cursor)
                .then(|| format!("cursor-{next}")),
        })
    }

    fn read_issue(
        &self,
        _host: &str,
        repository: &str,
        token: &str,
        number: u64,
    ) -> Result<Value, ProbeError> {
        self.guard_read(token)?;
        self.issues
            .lock()
            .expect("scripted github")
            .get(repository)
            .and_then(|items| {
                items
                    .iter()
                    .find(|item| item.get("number").and_then(Value::as_u64) == Some(number))
            })
            .cloned()
            .ok_or_else(|| ProbeError::Unreachable("unknown issue".into()))
    }

    fn viewer_login(&self, _host: &str, token: &str) -> Result<String, ProbeError> {
        self.guard_write(token)?;
        Ok(self.viewer_login.clone())
    }

    fn add_assignees(
        &self,
        _host: &str,
        repository: &str,
        number: u64,
        token: &str,
        logins: &[String],
    ) -> Result<Value, ProbeError> {
        self.mutate_assignees(repository, number, token, logins, true)
    }

    fn remove_assignees(
        &self,
        _host: &str,
        repository: &str,
        number: u64,
        token: &str,
        logins: &[String],
    ) -> Result<Value, ProbeError> {
        self.mutate_assignees(repository, number, token, logins, false)
    }

    fn create_issue(
        &self,
        host: &str,
        repository: &str,
        token: &str,
        title: &str,
        body: &str,
    ) -> Result<Value, ProbeError> {
        self.guard_write(token)?;
        let mut issues = self.issues.lock().expect("scripted github");
        let items = issues.entry(repository.to_string()).or_default();
        let number = items
            .iter()
            .filter_map(|item| item.get("number").and_then(Value::as_u64))
            .max()
            .unwrap_or(0)
            + 1;
        let node = github_node(host, repository, number, title, Some(body));
        items.push(node.clone());
        Ok(node)
    }

    fn update_issue(
        &self,
        _host: &str,
        repository: &str,
        token: &str,
        number: u64,
        title: Option<&str>,
        body: Option<&str>,
    ) -> Result<Value, ProbeError> {
        self.guard_write(token)?;
        let mut issues = self.issues.lock().expect("scripted github");
        let Some(node) = issues.get_mut(repository).and_then(|items| {
            items
                .iter_mut()
                .find(|item| item.get("number").and_then(Value::as_u64) == Some(number))
        }) else {
            return Err(ProbeError::Unreachable("unknown issue".into()));
        };
        if let Some(title) = title {
            node["title"] = Value::String(title.to_string());
        }
        if let Some(body) = body {
            node["body"] = Value::String(body.to_string());
        }
        Ok(node.clone())
    }

    fn set_issue_state(
        &self,
        _host: &str,
        repository: &str,
        token: &str,
        number: u64,
        open: bool,
    ) -> Result<Value, ProbeError> {
        self.guard_write(token)?;
        let mut issues = self.issues.lock().expect("scripted github");
        let Some(node) = issues.get_mut(repository).and_then(|items| {
            items
                .iter_mut()
                .find(|item| item.get("number").and_then(Value::as_u64) == Some(number))
        }) else {
            return Err(ProbeError::Unreachable("unknown issue".into()));
        };
        if open {
            node["state"] = Value::String("OPEN".into());
            node.as_object_mut().unwrap().remove("closedAt");
        } else {
            node["state"] = Value::String("CLOSED".into());
            node["closedAt"] = Value::String("2026-08-24T00:00:00Z".into());
        }
        Ok(node.clone())
    }

    fn add_issue_comment(
        &self,
        host: &str,
        repository: &str,
        token: &str,
        number: u64,
        body: &str,
    ) -> Result<Value, ProbeError> {
        self.guard_write(token)?;
        let issues = self.issues.lock().expect("scripted github");
        let exists = issues.get(repository).is_some_and(|items| {
            items
                .iter()
                .any(|item| item.get("number").and_then(Value::as_u64) == Some(number))
        });
        if !exists {
            return Err(ProbeError::Unreachable("unknown issue".into()));
        }
        let seq = {
            let mut counter = self.comment_seq.lock().expect("scripted github");
            *counter += 1;
            *counter
        };
        Ok(serde_json::json!({
            "id": seq,
            "html_url": format!("{}#issuecomment-{seq}", github_web_issue_url(host, repository, number)),
            "body": body,
        }))
    }

    fn issue_database_id(
        &self,
        _host: &str,
        repository: &str,
        token: &str,
        number: u64,
    ) -> Result<u64, ProbeError> {
        self.guard_write(token)?;
        let issues = self.issues.lock().expect("scripted github");
        let Some(node) = issues.get(repository).and_then(|items| {
            items
                .iter()
                .find(|item| item.get("number").and_then(Value::as_u64) == Some(number))
        }) else {
            return Err(ProbeError::Unreachable("unknown issue".into()));
        };
        Ok(node.get("id").and_then(Value::as_u64).unwrap_or(number))
    }

    fn issue_parent(
        &self,
        _host: &str,
        repository: &str,
        token: &str,
        number: u64,
    ) -> Result<Option<Value>, ProbeError> {
        self.guard_write(token)?;
        let issues = self.issues.lock().expect("scripted github");
        let Some(node) = issues.get(repository).and_then(|items| {
            items
                .iter()
                .find(|item| item.get("number").and_then(Value::as_u64) == Some(number))
        }) else {
            return Err(ProbeError::Unreachable("unknown issue".into()));
        };
        match node.get("parent") {
            Some(parent) if !parent.is_null() => Ok(Some(parent.clone())),
            _ => Ok(None),
        }
    }

    fn add_sub_issue(
        &self,
        _host: &str,
        parent_repository: &str,
        token: &str,
        parent_number: u64,
        sub_issue_id: u64,
    ) -> Result<Value, ProbeError> {
        self.guard_write(token)?;
        let mut issues = self.issues.lock().expect("scripted github");
        let Some(parent_pos) = self.locate(&issues, parent_repository, parent_number) else {
            return Err(ProbeError::Unreachable("unknown parent".into()));
        };
        let Some(child_pos) = self.locate_database_id(&issues, sub_issue_id) else {
            return Err(ProbeError::Unreachable("unknown sub-issue".into()));
        };
        let child_ref = github_ref(&issues[&child_pos.0][child_pos.1], &child_pos.0);
        // 摘掉旧父：GitHub 原生行为是 sub-issue 至多一个父。
        let old_parent = issues[&child_pos.0][child_pos.1]
            .get("parent")
            .cloned()
            .filter(|parent| !parent.is_null());
        if let Some(old_parent) = old_parent {
            if let (Some(old_repository), Some(old_number)) = (
                ref_repository(&old_parent),
                old_parent.get("number").and_then(Value::as_u64),
            ) {
                if let Some(old_pos) = self.locate(&issues, &old_repository, old_number) {
                    remove_node_from_connection(
                        node_at_mut(&mut issues, &old_pos.0, old_pos.1),
                        "subIssues",
                        &child_ref,
                    );
                }
            }
        }
        push_connection_node(
            node_at_mut(&mut issues, &parent_pos.0, parent_pos.1),
            "subIssues",
            &child_ref,
        );
        let parent_ref = github_ref(&issues[&parent_pos.0][parent_pos.1], &parent_pos.0);
        node_at_mut(&mut issues, &child_pos.0, child_pos.1)["parent"] = parent_ref;
        Ok(issues[&parent_pos.0][parent_pos.1].clone())
    }

    fn remove_sub_issue(
        &self,
        _host: &str,
        parent_repository: &str,
        token: &str,
        parent_number: u64,
        sub_issue_id: u64,
    ) -> Result<Value, ProbeError> {
        self.guard_write(token)?;
        let mut issues = self.issues.lock().expect("scripted github");
        let Some(parent_pos) = self.locate(&issues, parent_repository, parent_number) else {
            return Err(ProbeError::Unreachable("unknown parent".into()));
        };
        let Some(child_pos) = self.locate_database_id(&issues, sub_issue_id) else {
            return Err(ProbeError::Unreachable("unknown sub-issue".into()));
        };
        let child_ref = github_ref(&issues[&child_pos.0][child_pos.1], &child_pos.0);
        remove_node_from_connection(
            node_at_mut(&mut issues, &parent_pos.0, parent_pos.1),
            "subIssues",
            &child_ref,
        );
        node_at_mut(&mut issues, &child_pos.0, child_pos.1)["parent"] = Value::Null;
        Ok(issues[&parent_pos.0][parent_pos.1].clone())
    }

    fn add_blocked_by(
        &self,
        _host: &str,
        repository: &str,
        token: &str,
        number: u64,
        blocking_issue_id: u64,
    ) -> Result<Value, ProbeError> {
        self.guard_write(token)?;
        let mut issues = self.issues.lock().expect("scripted github");
        let Some(blocked_pos) = self.locate(&issues, repository, number) else {
            return Err(ProbeError::Unreachable("unknown issue".into()));
        };
        let Some(blocking_pos) = self.locate_database_id(&issues, blocking_issue_id) else {
            return Err(ProbeError::Unreachable("unknown blocking issue".into()));
        };
        let blocker_ref = github_ref(&issues[&blocking_pos.0][blocking_pos.1], &blocking_pos.0);
        let blocked_ref = github_ref(&issues[&blocked_pos.0][blocked_pos.1], &blocked_pos.0);
        let added = {
            let blocked_node = node_at_mut(&mut issues, &blocked_pos.0, blocked_pos.1);
            push_connection_node(blocked_node, "blockedBy", &blocker_ref)
        };
        {
            let blocking_node = node_at_mut(&mut issues, &blocking_pos.0, blocking_pos.1);
            push_connection_node(blocking_node, "blocking", &blocked_ref);
        }
        if added {
            bump_blocked_summary(
                node_at_mut(&mut issues, &blocked_pos.0, blocked_pos.1),
                "blockedBy",
                1,
            );
        }
        Ok(issues[&blocking_pos.0][blocking_pos.1].clone())
    }

    fn remove_blocked_by(
        &self,
        _host: &str,
        repository: &str,
        token: &str,
        number: u64,
        blocking_issue_id: u64,
    ) -> Result<Value, ProbeError> {
        self.guard_write(token)?;
        let mut issues = self.issues.lock().expect("scripted github");
        let Some(blocked_pos) = self.locate(&issues, repository, number) else {
            return Err(ProbeError::Unreachable("unknown issue".into()));
        };
        let Some(blocking_pos) = self.locate_database_id(&issues, blocking_issue_id) else {
            return Err(ProbeError::Unreachable("unknown blocking issue".into()));
        };
        let blocker_ref = github_ref(&issues[&blocking_pos.0][blocking_pos.1], &blocking_pos.0);
        let blocked_ref = github_ref(&issues[&blocked_pos.0][blocked_pos.1], &blocked_pos.0);
        let removed = {
            let blocked_node = node_at_mut(&mut issues, &blocked_pos.0, blocked_pos.1);
            remove_node_from_connection(blocked_node, "blockedBy", &blocker_ref)
        };
        {
            let blocking_node = node_at_mut(&mut issues, &blocking_pos.0, blocking_pos.1);
            remove_node_from_connection(blocking_node, "blocking", &blocked_ref);
        }
        if removed {
            bump_blocked_summary(
                node_at_mut(&mut issues, &blocked_pos.0, blocked_pos.1),
                "blockedBy",
                -1,
            );
        }
        Ok(issues[&blocking_pos.0][blocking_pos.1].clone())
    }
}
