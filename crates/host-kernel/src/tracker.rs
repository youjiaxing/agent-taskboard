use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::issue::{parse_issue_id, DependencyRef, IssueRecord, IssueRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrackerKind {
    Github,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CredentialSource {
    AppEnv,
    SecretsFile,
    Cli,
    GenericEnv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthFailureKind {
    MissingCredentials,
    Rejected,
    Unreachable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairHint {
    pub cli_detected: bool,
    pub secrets_path: PathBuf,
    pub app_env: String,
    pub generic_env: String,
    pub suggested_scope: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum ProjectConnection {
    Ready {
        source: CredentialSource,
    },
    AuthFailed {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<CredentialSource>,
        kind: AuthFailureKind,
        repair: RepairHint,
        message: String,
    },
    Unreachable {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<CredentialSource>,
        repair: RepairHint,
        message: String,
    },
}

pub struct ProbeContext<'a> {
    pub github_host: &'a str,
    pub repository: &'a str,
    pub secrets_pat: Option<&'a str>,
    pub secrets_path: &'a Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeOutcome {
    Ready {
        source: CredentialSource,
    },
    Failed {
        source: Option<CredentialSource>,
        kind: AuthFailureKind,
        cli_detected: bool,
        detail: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackerReadError {
    Auth {
        source: Option<CredentialSource>,
        kind: AuthFailureKind,
        cli_detected: bool,
        detail: Option<String>,
    },
    Offline {
        source: Option<CredentialSource>,
        cli_detected: bool,
        detail: Option<String>,
    },
    RateLimited {
        retry_after_ms: Option<u64>,
    },
    Failed {
        detail: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackerWriteError {
    Auth {
        source: Option<CredentialSource>,
        kind: AuthFailureKind,
        cli_detected: bool,
        detail: Option<String>,
    },
    Offline {
        source: Option<CredentialSource>,
        cli_detected: bool,
        detail: Option<String>,
    },
    RateLimited {
        retry_after_ms: Option<u64>,
    },
    Failed {
        message: String,
    },
}

/// 只改标题和正文中给出的字段；其余保持原样。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IssueEdit<'a> {
    pub title: Option<&'a str>,
    pub body: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueComment {
    pub id: String,
    pub url: String,
    pub body: String,
}

pub trait TrackerPort: Send + Sync {
    fn probe(&self, ctx: &ProbeContext<'_>) -> ProbeOutcome;
    fn read_issues(&self, ctx: &ProbeContext<'_>) -> Result<Vec<IssueRecord>, TrackerReadError>;
    fn create_issue(
        &self,
        ctx: &ProbeContext<'_>,
        title: &str,
        body: &str,
    ) -> Result<IssueRecord, TrackerWriteError>;
    fn update_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        edit: IssueEdit<'_>,
    ) -> Result<IssueRecord, TrackerWriteError>;
    fn close_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerWriteError>;
    fn reopen_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerWriteError>;
    fn add_comment(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        body: &str,
    ) -> Result<IssueComment, TrackerWriteError>;
    fn claim_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerWriteError>;
    fn release_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerWriteError>;
    /// 把 issue 挂到 parent 之下（None 表示摘除父）。
    /// 走原生边写入；读回依赖下一次 read_issues。
    fn set_parent(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        parent: Option<&str>,
    ) -> Result<(), TrackerWriteError>;
    /// 在原生边上添加 blocked_by 边。
    fn add_blocked_by(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        blocking_issue_id: &str,
    ) -> Result<(), TrackerWriteError>;
    /// 在原生边上移除 blocked_by 边。
    fn remove_blocked_by(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        blocking_issue_id: &str,
    ) -> Result<(), TrackerWriteError>;
}

pub const GITHUB_APP_ENV: &str = "AGENT_TASKBOARD_GITHUB_TOKEN";
pub const GITHUB_GENERIC_ENV: &str = "GH_TOKEN / GITHUB_TOKEN";
pub const GITHUB_SCOPE: &str = "repo";

pub fn repair_hint(cli_detected: bool, secrets_path: &Path) -> RepairHint {
    RepairHint {
        cli_detected,
        secrets_path: secrets_path.to_path_buf(),
        app_env: GITHUB_APP_ENV.to_string(),
        generic_env: GITHUB_GENERIC_ENV.to_string(),
        suggested_scope: GITHUB_SCOPE.to_string(),
    }
}

pub fn resolve_github_token(
    app_env: Option<&str>,
    secrets_pat: Option<&str>,
    gh_token: Option<&str>,
    generic_env: Option<&str>,
) -> Option<(String, CredentialSource)> {
    nonempty(app_env)
        .map(|token| (token, CredentialSource::AppEnv))
        .or_else(|| nonempty(secrets_pat).map(|token| (token, CredentialSource::SecretsFile)))
        .or_else(|| nonempty(gh_token).map(|token| (token, CredentialSource::Cli)))
        .or_else(|| nonempty(generic_env).map(|token| (token, CredentialSource::GenericEnv)))
}

fn nonempty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

#[derive(Debug, Clone)]
enum ReadScript {
    Offline,
    Auth,
    RateLimited { retry_after_ms: Option<u64> },
}

pub const MEMORY_TRACKER_ACTOR: &str = "me";

pub struct MemoryTracker {
    failures: Mutex<BTreeSet<String>>,
    read_scripts: Mutex<BTreeMap<String, ReadScript>>,
    read_counts: Mutex<BTreeMap<String, u64>>,
    issues: Mutex<BTreeMap<String, Vec<IssueRecord>>>,
    write_fail: Mutex<BTreeMap<String, String>>,
    bodies: Mutex<BTreeMap<String, String>>,
    comments: Mutex<BTreeMap<String, Vec<IssueComment>>>,
    actor: String,
    source: CredentialSource,
}

impl MemoryTracker {
    pub fn new() -> Self {
        Self {
            failures: Mutex::new(BTreeSet::new()),
            read_scripts: Mutex::new(BTreeMap::new()),
            read_counts: Mutex::new(BTreeMap::new()),
            issues: Mutex::new(BTreeMap::new()),
            write_fail: Mutex::new(BTreeMap::new()),
            bodies: Mutex::new(BTreeMap::new()),
            comments: Mutex::new(BTreeMap::new()),
            actor: MEMORY_TRACKER_ACTOR.into(),
            source: CredentialSource::Cli,
        }
    }

    pub fn fail_claim(&self, repository: impl Into<String>) {
        self.write_fail
            .lock()
            .expect("memory tracker")
            .insert(repository.into(), "cannot claim issue".into());
    }

    pub fn assignees(&self, repository: &str, number: u64) -> Vec<String> {
        self.issues
            .lock()
            .expect("memory tracker")
            .get(repository)
            .and_then(|items| items.iter().find(|issue| issue.number == number))
            .map(|issue| issue.assignees.clone())
            .unwrap_or_default()
    }

    pub fn close_issue(&self, repository: &str, number: u64) {
        let mut issues = self.issues.lock().expect("memory tracker");
        if let Some(issue) = issues
            .get_mut(repository)
            .and_then(|items| items.iter_mut().find(|issue| issue.number == number))
        {
            issue.open = false;
            if issue.closed_at.is_none() {
                issue.closed_at = Some("2026-08-23T00:00:00Z".into());
            }
        }
    }

    pub fn fail_repository(&self, repository: impl Into<String>) {
        self.failures
            .lock()
            .expect("memory tracker")
            .insert(repository.into());
    }

    pub fn fail_read(&self, repository: impl Into<String>) {
        self.read_scripts
            .lock()
            .expect("memory tracker")
            .insert(repository.into(), ReadScript::Offline);
    }

    pub fn fail_auth(&self, repository: impl Into<String>) {
        self.read_scripts
            .lock()
            .expect("memory tracker")
            .insert(repository.into(), ReadScript::Auth);
    }

    pub fn fail_rate_limited(&self, repository: impl Into<String>, retry_after_ms: Option<u64>) {
        self.read_scripts.lock().expect("memory tracker").insert(
            repository.into(),
            ReadScript::RateLimited { retry_after_ms },
        );
    }

    pub fn clear_read_script(&self, repository: &str) {
        self.read_scripts
            .lock()
            .expect("memory tracker")
            .remove(repository);
    }

    pub fn read_count(&self, repository: &str) -> u64 {
        self.read_counts
            .lock()
            .expect("memory tracker")
            .get(repository)
            .copied()
            .unwrap_or(0)
    }

    pub fn set_issues(&self, repository: impl Into<String>, issues: Vec<IssueRecord>) {
        self.issues
            .lock()
            .expect("memory tracker")
            .insert(repository.into(), issues);
    }

    pub fn add_issue(&self, issue: IssueRecord) {
        self.issues
            .lock()
            .expect("memory tracker")
            .entry(issue.repository.clone())
            .or_default()
            .push(issue);
    }

    fn write_guard(&self, ctx: &ProbeContext<'_>) -> Result<(), TrackerWriteError> {
        if let Some(message) = self
            .write_fail
            .lock()
            .expect("memory tracker")
            .get(ctx.repository)
            .cloned()
        {
            return Err(TrackerWriteError::Failed { message });
        }
        Ok(())
    }

    fn mutate_issue<F>(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        edit: F,
    ) -> Result<IssueRecord, TrackerWriteError>
    where
        F: FnOnce(&mut IssueRecord),
    {
        self.write_guard(ctx)?;
        let Some((repository, number)) = parse_issue_id(issue_id) else {
            return Err(TrackerWriteError::Failed {
                message: "unknown issue".into(),
            });
        };
        let mut issues = self.issues.lock().expect("memory tracker");
        let Some(issue) = issues
            .get_mut(&repository)
            .and_then(|items| items.iter_mut().find(|issue| issue.number == number))
        else {
            return Err(TrackerWriteError::Failed {
                message: "unknown issue".into(),
            });
        };
        edit(issue);
        Ok(issue.clone())
    }
}

impl Default for MemoryTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl TrackerPort for MemoryTracker {
    fn probe(&self, ctx: &ProbeContext<'_>) -> ProbeOutcome {
        let failed = self
            .failures
            .lock()
            .expect("memory tracker")
            .contains(ctx.repository);
        if failed {
            ProbeOutcome::Failed {
                source: Some(self.source),
                kind: AuthFailureKind::Rejected,
                cli_detected: true,
                detail: Some(format!("{} rejected {}", ctx.github_host, ctx.repository)),
            }
        } else {
            ProbeOutcome::Ready {
                source: self.source,
            }
        }
    }

    fn read_issues(&self, ctx: &ProbeContext<'_>) -> Result<Vec<IssueRecord>, TrackerReadError> {
        *self
            .read_counts
            .lock()
            .expect("memory tracker")
            .entry(ctx.repository.to_string())
            .or_default() += 1;
        if self
            .failures
            .lock()
            .expect("memory tracker")
            .contains(ctx.repository)
        {
            return Err(TrackerReadError::Auth {
                source: Some(self.source),
                kind: AuthFailureKind::Rejected,
                cli_detected: true,
                detail: Some(format!("{} rejected {}", ctx.github_host, ctx.repository)),
            });
        }
        if let Some(script) = self
            .read_scripts
            .lock()
            .expect("memory tracker")
            .get(ctx.repository)
            .cloned()
        {
            return Err(match script {
                ReadScript::Offline => TrackerReadError::Offline {
                    source: Some(self.source),
                    cli_detected: true,
                    detail: Some(format!("cannot read {}", ctx.repository)),
                },
                ReadScript::Auth => TrackerReadError::Auth {
                    source: Some(self.source),
                    kind: AuthFailureKind::Rejected,
                    cli_detected: true,
                    detail: Some(format!("{} rejected {}", ctx.github_host, ctx.repository)),
                },
                ReadScript::RateLimited { retry_after_ms } => {
                    TrackerReadError::RateLimited { retry_after_ms }
                }
            });
        }
        Ok(self
            .issues
            .lock()
            .expect("memory tracker")
            .get(ctx.repository)
            .cloned()
            .unwrap_or_default())
    }

    fn create_issue(
        &self,
        ctx: &ProbeContext<'_>,
        title: &str,
        body: &str,
    ) -> Result<IssueRecord, TrackerWriteError> {
        self.write_guard(ctx)?;
        let mut issues = self.issues.lock().expect("memory tracker");
        let items = issues.entry(ctx.repository.to_string()).or_default();
        let number = items.iter().map(|issue| issue.number).max().unwrap_or(0) + 1;
        let issue = IssueRecord {
            repository: ctx.repository.to_string(),
            number,
            title: title.to_string(),
            url: github_web_issue_url(ctx.github_host, ctx.repository, number),
            open: true,
            closed_at: None,
            assignees: Vec::new(),
            labels: Vec::new(),
            parent: None,
            children: Vec::new(),
            blocked_by: Vec::new(),
            blocking: Vec::new(),
        };
        self.bodies
            .lock()
            .expect("memory tracker")
            .insert(issue.id(), body.to_string());
        items.push(issue.clone());
        Ok(issue)
    }

    fn update_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        edit: IssueEdit<'_>,
    ) -> Result<IssueRecord, TrackerWriteError> {
        if let Some(title) = edit.title {
            self.mutate_issue(ctx, issue_id, |issue| issue.title = title.to_string())?;
        }
        if let Some(body) = edit.body {
            self.write_guard(ctx)?;
            self.bodies
                .lock()
                .expect("memory tracker")
                .insert(issue_id.to_string(), body.to_string());
        }
        self.mutate_issue(ctx, issue_id, |_| {})
    }

    fn close_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerWriteError> {
        self.mutate_issue(ctx, issue_id, |issue| {
            issue.open = false;
            if issue.closed_at.is_none() {
                issue.closed_at = Some("2026-08-24T00:00:00Z".into());
            }
        })
    }

    fn reopen_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerWriteError> {
        self.mutate_issue(ctx, issue_id, |issue| {
            issue.open = true;
            issue.closed_at = None;
        })
    }

    fn add_comment(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        body: &str,
    ) -> Result<IssueComment, TrackerWriteError> {
        self.write_guard(ctx)?;
        let issue = self.mutate_issue(ctx, issue_id, |_| {})?;
        let mut comments = self.comments.lock().expect("memory tracker");
        let items = comments.entry(issue_id.to_string()).or_default();
        let number = items.len() + 1;
        let comment = IssueComment {
            id: number.to_string(),
            url: format!("{}#issuecomment-{number}", issue.url),
            body: body.to_string(),
        };
        items.push(comment.clone());
        Ok(comment)
    }

    fn claim_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerWriteError> {
        self.mutate_issue(ctx, issue_id, |issue| {
            if !issue.assignees.iter().any(|login| login == &self.actor) {
                issue.assignees.push(self.actor.clone());
            }
        })
    }

    fn release_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerWriteError> {
        self.mutate_issue(ctx, issue_id, |issue| {
            issue.assignees.retain(|login| login != &self.actor);
        })
    }

    fn set_parent(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        parent: Option<&str>,
    ) -> Result<(), TrackerWriteError> {
        self.write_guard(ctx)?;
        let Some((repository, number)) = parse_issue_id(issue_id) else {
            return Err(TrackerWriteError::Failed {
                message: "unknown issue".into(),
            });
        };
        let mut issues = self.issues.lock().expect("memory tracker");
        let Some(child_pos) = locate_issue(&issues, &repository, number) else {
            return Err(TrackerWriteError::Failed {
                message: "unknown issue".into(),
            });
        };
        let old_parent = issues[&child_pos.0][child_pos.1].parent.clone();
        match parent {
            Some(parent_id) => {
                let Some((parent_repository, parent_number)) = parse_issue_id(parent_id) else {
                    return Err(TrackerWriteError::Failed {
                        message: "unknown parent".into(),
                    });
                };
                let Some(parent_pos) = locate_issue(&issues, &parent_repository, parent_number)
                else {
                    return Err(TrackerWriteError::Failed {
                        message: "unknown parent".into(),
                    });
                };
                if child_pos == parent_pos {
                    return Err(TrackerWriteError::Failed {
                        message: "issue cannot parent itself".into(),
                    });
                }
                remove_child(&mut issues, old_parent.as_ref(), &repository, number);
                let parent_issue = issues[&parent_pos.0][parent_pos.1].clone();
                let child_issue = issue_at_mut(&mut issues, &child_pos.0, child_pos.1);
                child_issue.parent = Some(
                    IssueRef::new(parent_repository.clone(), parent_number, parent_issue.title)
                        .with_open(parent_issue.open),
                );
                let child_ref =
                    IssueRef::new(repository.clone(), number, child_issue.title.clone())
                        .with_open(child_issue.open);
                let parent_node = issue_at_mut(&mut issues, &parent_pos.0, parent_pos.1);
                if !parent_node
                    .children
                    .iter()
                    .any(|child| child.id() == child_ref.id())
                {
                    parent_node.children.push(child_ref);
                }
            }
            None => {
                remove_child(&mut issues, old_parent.as_ref(), &repository, number);
                issue_at_mut(&mut issues, &child_pos.0, child_pos.1).parent = None;
            }
        }
        Ok(())
    }

    fn add_blocked_by(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        blocking_issue_id: &str,
    ) -> Result<(), TrackerWriteError> {
        self.write_guard(ctx)?;
        let Some((repository, number)) = parse_issue_id(issue_id) else {
            return Err(TrackerWriteError::Failed {
                message: "unknown issue".into(),
            });
        };
        let Some((blocking_repository, blocking_number)) = parse_issue_id(blocking_issue_id) else {
            return Err(TrackerWriteError::Failed {
                message: "unknown blocking issue".into(),
            });
        };
        let mut issues = self.issues.lock().expect("memory tracker");
        let Some(blocked_pos) = locate_issue(&issues, &repository, number) else {
            return Err(TrackerWriteError::Failed {
                message: "unknown issue".into(),
            });
        };
        let Some(blocking_pos) = locate_issue(&issues, &blocking_repository, blocking_number)
        else {
            return Err(TrackerWriteError::Failed {
                message: "unknown blocking issue".into(),
            });
        };
        let blocker = issues[&blocking_pos.0][blocking_pos.1].clone();
        let blocked = issues[&blocked_pos.0][blocked_pos.1].clone();
        let blocker_ref = IssueRef::new(blocking_repository, blocking_number, blocker.title)
            .with_open(blocker.open);
        let blocked_ref = IssueRef::new(repository, number, blocked.title).with_open(blocked.open);
        let blocked_node = issue_at_mut(&mut issues, &blocked_pos.0, blocked_pos.1);
        if !blocked_node
            .blocked_by
            .iter()
            .any(|dep| matches!(dep, DependencyRef::Known(known) if known.id() == blocker_ref.id()))
        {
            blocked_node
                .blocked_by
                .push(DependencyRef::Known(blocker_ref));
        }
        let blocking_node = issue_at_mut(&mut issues, &blocking_pos.0, blocking_pos.1);
        if !blocking_node
            .blocking
            .iter()
            .any(|issue| issue.id() == blocked_ref.id())
        {
            blocking_node.blocking.push(blocked_ref);
        }
        Ok(())
    }

    fn remove_blocked_by(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        blocking_issue_id: &str,
    ) -> Result<(), TrackerWriteError> {
        self.write_guard(ctx)?;
        let Some((repository, number)) = parse_issue_id(issue_id) else {
            return Err(TrackerWriteError::Failed {
                message: "unknown issue".into(),
            });
        };
        let Some((blocking_repository, blocking_number)) = parse_issue_id(blocking_issue_id) else {
            return Err(TrackerWriteError::Failed {
                message: "unknown blocking issue".into(),
            });
        };
        let mut issues = self.issues.lock().expect("memory tracker");
        let Some(blocked_pos) = locate_issue(&issues, &repository, number) else {
            return Err(TrackerWriteError::Failed {
                message: "unknown issue".into(),
            });
        };
        let Some(blocking_pos) = locate_issue(&issues, &blocking_repository, blocking_number)
        else {
            return Err(TrackerWriteError::Failed {
                message: "unknown blocking issue".into(),
            });
        };
        let blocked_node = issue_at_mut(&mut issues, &blocked_pos.0, blocked_pos.1);
        blocked_node
            .blocked_by
            .retain(|dep| !matches!(dep, DependencyRef::Known(known) if known.repository == blocking_repository && known.number == blocking_number));
        let blocking_node = issue_at_mut(&mut issues, &blocking_pos.0, blocking_pos.1);
        blocking_node
            .blocking
            .retain(|issue| !(issue.repository == repository && issue.number == number));
        Ok(())
    }
}

fn locate_issue(
    issues: &BTreeMap<String, Vec<IssueRecord>>,
    repository: &str,
    number: u64,
) -> Option<(String, usize)> {
    issues.iter().find_map(|(repo, items)| {
        items
            .iter()
            .position(|issue| issue.repository == repository && issue.number == number)
            .map(|idx| (repo.clone(), idx))
    })
}

fn remove_child(
    issues: &mut BTreeMap<String, Vec<IssueRecord>>,
    parent: Option<&IssueRef>,
    repository: &str,
    number: u64,
) {
    let Some(parent) = parent else { return };
    if let Some(items) = issues.get_mut(&parent.repository) {
        for card in items.iter_mut() {
            if card.number == parent.number {
                card.children
                    .retain(|child| !(child.repository == repository && child.number == number));
            }
        }
    }
}

trait EnvSource: Send + Sync {
    fn var(&self, key: &str) -> Option<String>;
}

trait GhAuth: Send + Sync {
    fn detected(&self) -> bool;
    fn token(&self, hostname: &str) -> Option<String>;
}

#[derive(Debug, Clone)]
struct NodePage {
    nodes: Vec<Value>,
    has_next_page: bool,
    end_cursor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IssueEdges {
    BlockedBy,
    Blocking,
    SubIssues,
}

impl IssueEdges {
    fn field(self) -> &'static str {
        match self {
            Self::BlockedBy => "blockedBy",
            Self::Blocking => "blocking",
            Self::SubIssues => "subIssues",
        }
    }
}

trait GitHubApi: Send + Sync {
    fn probe_repo(&self, host: &str, repository: &str, token: &str) -> Result<(), ProbeError>;
    fn list_issues_page(
        &self,
        host: &str,
        repository: &str,
        token: &str,
        after: Option<&str>,
    ) -> Result<NodePage, ProbeError>;
    fn list_issue_edges_page(
        &self,
        host: &str,
        repository: &str,
        token: &str,
        number: u64,
        edges: IssueEdges,
        after: Option<&str>,
    ) -> Result<NodePage, ProbeError>;
    fn viewer_login(&self, host: &str, token: &str) -> Result<String, ProbeError>;
    fn add_assignees(
        &self,
        host: &str,
        repository: &str,
        number: u64,
        token: &str,
        logins: &[String],
    ) -> Result<Value, ProbeError>;
    fn remove_assignees(
        &self,
        host: &str,
        repository: &str,
        number: u64,
        token: &str,
        logins: &[String],
    ) -> Result<Value, ProbeError>;
    fn create_issue(
        &self,
        host: &str,
        repository: &str,
        token: &str,
        title: &str,
        body: &str,
    ) -> Result<Value, ProbeError>;
    fn update_issue(
        &self,
        host: &str,
        repository: &str,
        token: &str,
        number: u64,
        title: Option<&str>,
        body: Option<&str>,
    ) -> Result<Value, ProbeError>;
    fn set_issue_state(
        &self,
        host: &str,
        repository: &str,
        token: &str,
        number: u64,
        open: bool,
    ) -> Result<Value, ProbeError>;
    fn add_issue_comment(
        &self,
        host: &str,
        repository: &str,
        token: &str,
        number: u64,
        body: &str,
    ) -> Result<Value, ProbeError>;
    fn issue_database_id(
        &self,
        host: &str,
        repository: &str,
        token: &str,
        number: u64,
    ) -> Result<u64, ProbeError>;
    fn issue_parent(
        &self,
        host: &str,
        repository: &str,
        token: &str,
        number: u64,
    ) -> Result<Option<Value>, ProbeError>;
    fn add_sub_issue(
        &self,
        host: &str,
        parent_repository: &str,
        token: &str,
        parent_number: u64,
        sub_issue_id: u64,
    ) -> Result<Value, ProbeError>;
    fn remove_sub_issue(
        &self,
        host: &str,
        parent_repository: &str,
        token: &str,
        parent_number: u64,
        sub_issue_id: u64,
    ) -> Result<Value, ProbeError>;
    fn add_blocked_by(
        &self,
        host: &str,
        repository: &str,
        token: &str,
        number: u64,
        blocking_issue_id: u64,
    ) -> Result<Value, ProbeError>;
    fn remove_blocked_by(
        &self,
        host: &str,
        repository: &str,
        token: &str,
        number: u64,
        blocking_issue_id: u64,
    ) -> Result<Value, ProbeError>;
}

enum ProbeError {
    Unauthorized { detail: Option<String> },
    Unreachable(String),
    RateLimited { retry_after_ms: Option<u64> },
    GraphQl { detail: String },
}

pub struct GitHubTracker {
    env: Box<dyn EnvSource>,
    gh: Box<dyn GhAuth>,
    api: Box<dyn GitHubApi>,
}

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
    pub graphql_auth_error: Option<String>,
    pub graphql_business_error: Option<String>,
}

impl GitHubTracker {
    pub fn live() -> Self {
        Self {
            env: Box::new(ProcessEnv),
            gh: Box::new(ProcessGh),
            api: Box::new(LiveGitHubApi),
        }
    }

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
                graphql_auth_error: script.graphql_auth_error,
                graphql_business_error: script.graphql_business_error,
                comment_seq: Mutex::new(0),
            }),
        }
    }

    fn authorize(
        &self,
        ctx: &ProbeContext<'_>,
    ) -> Result<(String, CredentialSource, bool), ProbeOutcome> {
        let app_env = self.env.var(GITHUB_APP_ENV);
        let generic_env = self
            .env
            .var("GH_TOKEN")
            .or_else(|| self.env.var("GITHUB_TOKEN"));
        let gh_token = self.gh.token(ctx.github_host);
        let cli_detected = self.gh.detected();
        let resolved = resolve_github_token(
            app_env.as_deref(),
            ctx.secrets_pat,
            gh_token.as_deref(),
            generic_env.as_deref(),
        );
        match resolved {
            Some((token, source)) => Ok((token, source, cli_detected)),
            None => Err(ProbeOutcome::Failed {
                source: None,
                kind: AuthFailureKind::MissingCredentials,
                cli_detected,
                detail: None,
            }),
        }
    }

    fn authorized_read(
        &self,
        ctx: &ProbeContext<'_>,
    ) -> Result<(String, CredentialSource, bool), TrackerReadError> {
        self.authorize(ctx).map_err(|outcome| match outcome {
            ProbeOutcome::Failed {
                source,
                kind,
                cli_detected,
                detail,
            } => TrackerReadError::Auth {
                source,
                kind,
                cli_detected,
                detail,
            },
            ProbeOutcome::Ready { .. } => TrackerReadError::Auth {
                source: None,
                kind: AuthFailureKind::MissingCredentials,
                cli_detected: self.gh.detected(),
                detail: None,
            },
        })
    }

    fn authorized_write(
        &self,
        ctx: &ProbeContext<'_>,
    ) -> Result<(String, CredentialSource, bool), TrackerWriteError> {
        self.authorize(ctx).map_err(|outcome| match outcome {
            ProbeOutcome::Failed {
                source,
                kind,
                cli_detected,
                detail,
            } => TrackerWriteError::Auth {
                source,
                kind,
                cli_detected,
                detail,
            },
            ProbeOutcome::Ready { .. } => TrackerWriteError::Auth {
                source: None,
                kind: AuthFailureKind::MissingCredentials,
                cli_detected: self.gh.detected(),
                detail: None,
            },
        })
    }

    fn parse_issue(&self, issue_id: &str) -> Result<(String, u64), TrackerWriteError> {
        parse_issue_id(issue_id).ok_or_else(|| TrackerWriteError::Failed {
            message: "unknown issue".into(),
        })
    }

    fn complete_issue_edges(
        &self,
        ctx: &ProbeContext<'_>,
        token: &str,
        source: CredentialSource,
        cli_detected: bool,
        node: &mut Value,
    ) -> Result<(), TrackerReadError> {
        let Some(number) = node.get("number").and_then(Value::as_u64) else {
            return Ok(());
        };
        for edges in [
            IssueEdges::BlockedBy,
            IssueEdges::Blocking,
            IssueEdges::SubIssues,
        ] {
            if !connection_has_next_page(node, edges.field()) {
                continue;
            }
            let mut pages: Vec<Value> = Vec::new();
            let mut after: Option<String> = None;
            loop {
                let page = self
                    .api
                    .list_issue_edges_page(
                        ctx.github_host,
                        ctx.repository,
                        token,
                        number,
                        edges,
                        after.as_deref(),
                    )
                    .map_err(|err| probe_read_error(err, source, cli_detected))?;
                pages.extend(page.nodes);
                if !page.has_next_page {
                    break;
                }
                let Some(cursor) = page.end_cursor else {
                    break;
                };
                after = Some(cursor);
            }
            set_connection_nodes(node, edges.field(), pages);
        }
        Ok(())
    }
}

impl TrackerPort for GitHubTracker {
    fn probe(&self, ctx: &ProbeContext<'_>) -> ProbeOutcome {
        match self.authorize(ctx) {
            Err(outcome) => outcome,
            Ok((token, source, _)) => {
                match self.api.probe_repo(ctx.github_host, ctx.repository, &token) {
                    Ok(()) => ProbeOutcome::Ready { source },
                    Err(err) => probe_error_outcome(err, source, self.gh.detected()),
                }
            }
        }
    }

    fn read_issues(&self, ctx: &ProbeContext<'_>) -> Result<Vec<IssueRecord>, TrackerReadError> {
        let (token, source, cli_detected) = self.authorized_read(ctx)?;
        let mut nodes: Vec<Value> = Vec::new();
        let mut after: Option<String> = None;
        loop {
            let page = self
                .api
                .list_issues_page(ctx.github_host, ctx.repository, &token, after.as_deref())
                .map_err(|err| probe_read_error(err, source, cli_detected))?;
            nodes.extend(page.nodes);
            if !page.has_next_page {
                break;
            }
            let Some(cursor) = page.end_cursor else {
                break;
            };
            after = Some(cursor);
        }
        for node in nodes.iter_mut() {
            self.complete_issue_edges(ctx, &token, source, cli_detected, node)?;
        }
        Ok(nodes
            .iter()
            .filter_map(|node| map_github_issue_node(node, ctx.repository, ctx.github_host))
            .collect())
    }

    fn create_issue(
        &self,
        ctx: &ProbeContext<'_>,
        title: &str,
        body: &str,
    ) -> Result<IssueRecord, TrackerWriteError> {
        let (token, source, cli_detected) = self.authorized_write(ctx)?;
        let node = self
            .api
            .create_issue(ctx.github_host, ctx.repository, &token, title, body)
            .map_err(|err| probe_write_error(err, source, cli_detected))?;
        map_github_issue_node(&node, ctx.repository, ctx.github_host).ok_or_else(|| {
            TrackerWriteError::Failed {
                message: "cannot map GitHub issue".into(),
            }
        })
    }

    fn update_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        edit: IssueEdit<'_>,
    ) -> Result<IssueRecord, TrackerWriteError> {
        let (token, source, cli_detected) = self.authorized_write(ctx)?;
        let (repository, number) = self.parse_issue(issue_id)?;
        let node = self
            .api
            .update_issue(
                ctx.github_host,
                &repository,
                &token,
                number,
                edit.title,
                edit.body,
            )
            .map_err(|err| probe_write_error(err, source, cli_detected))?;
        map_github_issue_node(&node, &repository, ctx.github_host).ok_or_else(|| {
            TrackerWriteError::Failed {
                message: "cannot map GitHub issue".into(),
            }
        })
    }

    fn close_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerWriteError> {
        self.set_state(ctx, issue_id, false)
    }

    fn reopen_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerWriteError> {
        self.set_state(ctx, issue_id, true)
    }

    fn add_comment(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        body: &str,
    ) -> Result<IssueComment, TrackerWriteError> {
        let (token, source, cli_detected) = self.authorized_write(ctx)?;
        let (repository, number) = self.parse_issue(issue_id)?;
        let node = self
            .api
            .add_issue_comment(ctx.github_host, &repository, &token, number, body)
            .map_err(|err| probe_write_error(err, source, cli_detected))?;
        Ok(map_issue_comment(
            &node,
            ctx.github_host,
            &repository,
            number,
        ))
    }

    fn claim_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerWriteError> {
        self.write_assignees(ctx, issue_id, true)
    }

    fn release_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerWriteError> {
        self.write_assignees(ctx, issue_id, false)
    }

    fn set_parent(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        parent: Option<&str>,
    ) -> Result<(), TrackerWriteError> {
        let (token, source, cli_detected) = self.authorized_write(ctx)?;
        let (child_repository, child_number) = self.parse_issue(issue_id)?;
        match parent {
            Some(parent_id) => {
                let (parent_repository, parent_number) =
                    parse_issue_id(parent_id).ok_or_else(|| TrackerWriteError::Failed {
                        message: "unknown parent".into(),
                    })?;
                let child_database_id = self
                    .api
                    .issue_database_id(ctx.github_host, &child_repository, &token, child_number)
                    .map_err(|err| probe_write_error(err, source, cli_detected))?;
                self.api
                    .add_sub_issue(
                        ctx.github_host,
                        &parent_repository,
                        &token,
                        parent_number,
                        child_database_id,
                    )
                    .map_err(|err| probe_write_error(err, source, cli_detected))?;
            }
            None => {
                if let Some(parent_node) = self
                    .api
                    .issue_parent(ctx.github_host, &child_repository, &token, child_number)
                    .map_err(|err| probe_write_error(err, source, cli_detected))?
                {
                    let parent_number = parent_node
                        .get("number")
                        .and_then(Value::as_u64)
                        .ok_or_else(|| TrackerWriteError::Failed {
                            message: "unknown parent".into(),
                        })?;
                    let parent_repository =
                        ref_repository(&parent_node).unwrap_or(child_repository.clone());
                    let child_database_id = self
                        .api
                        .issue_database_id(ctx.github_host, &child_repository, &token, child_number)
                        .map_err(|err| probe_write_error(err, source, cli_detected))?;
                    self.api
                        .remove_sub_issue(
                            ctx.github_host,
                            &parent_repository,
                            &token,
                            parent_number,
                            child_database_id,
                        )
                        .map_err(|err| probe_write_error(err, source, cli_detected))?;
                }
            }
        }
        Ok(())
    }

    fn add_blocked_by(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        blocking_issue_id: &str,
    ) -> Result<(), TrackerWriteError> {
        let (token, source, cli_detected) = self.authorized_write(ctx)?;
        let (repository, number) = self.parse_issue(issue_id)?;
        let (blocking_repository, blocking_number) =
            parse_issue_id(blocking_issue_id).ok_or_else(|| TrackerWriteError::Failed {
                message: "unknown blocking issue".into(),
            })?;
        let blocking_database_id = self
            .api
            .issue_database_id(
                ctx.github_host,
                &blocking_repository,
                &token,
                blocking_number,
            )
            .map_err(|err| probe_write_error(err, source, cli_detected))?;
        self.api
            .add_blocked_by(
                ctx.github_host,
                &repository,
                &token,
                number,
                blocking_database_id,
            )
            .map_err(|err| probe_write_error(err, source, cli_detected))?;
        Ok(())
    }

    fn remove_blocked_by(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        blocking_issue_id: &str,
    ) -> Result<(), TrackerWriteError> {
        let (token, source, cli_detected) = self.authorized_write(ctx)?;
        let (repository, number) = self.parse_issue(issue_id)?;
        let (blocking_repository, blocking_number) =
            parse_issue_id(blocking_issue_id).ok_or_else(|| TrackerWriteError::Failed {
                message: "unknown blocking issue".into(),
            })?;
        let blocking_database_id = self
            .api
            .issue_database_id(
                ctx.github_host,
                &blocking_repository,
                &token,
                blocking_number,
            )
            .map_err(|err| probe_write_error(err, source, cli_detected))?;
        self.api
            .remove_blocked_by(
                ctx.github_host,
                &repository,
                &token,
                number,
                blocking_database_id,
            )
            .map_err(|err| probe_write_error(err, source, cli_detected))?;
        Ok(())
    }
}

impl GitHubTracker {
    fn set_state(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        open: bool,
    ) -> Result<IssueRecord, TrackerWriteError> {
        let (token, source, cli_detected) = self.authorized_write(ctx)?;
        let (repository, number) = self.parse_issue(issue_id)?;
        let node = self
            .api
            .set_issue_state(ctx.github_host, &repository, &token, number, open)
            .map_err(|err| probe_write_error(err, source, cli_detected))?;
        map_github_issue_node(&node, &repository, ctx.github_host).ok_or_else(|| {
            TrackerWriteError::Failed {
                message: "cannot map GitHub issue".into(),
            }
        })
    }

    fn write_assignees(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        claim: bool,
    ) -> Result<IssueRecord, TrackerWriteError> {
        let (token, source, cli_detected) = self.authorized_write(ctx)?;
        let (repository, number) = self.parse_issue(issue_id)?;
        let login = self
            .api
            .viewer_login(ctx.github_host, &token)
            .map_err(|err| probe_write_error(err, source, cli_detected))?;
        let node = if claim {
            self.api
                .add_assignees(ctx.github_host, &repository, number, &token, &[login])
        } else {
            self.api
                .remove_assignees(ctx.github_host, &repository, number, &token, &[login])
        }
        .map_err(|err| probe_write_error(err, source, cli_detected))?;
        map_github_issue_node(&node, &repository, ctx.github_host).ok_or_else(|| {
            TrackerWriteError::Failed {
                message: "cannot map GitHub issue".into(),
            }
        })
    }
}

fn probe_read_error(
    err: ProbeError,
    source: CredentialSource,
    cli_detected: bool,
) -> TrackerReadError {
    match err {
        ProbeError::Unauthorized { detail } => TrackerReadError::Auth {
            source: Some(source),
            kind: AuthFailureKind::Rejected,
            cli_detected,
            detail,
        },
        ProbeError::Unreachable(detail) => TrackerReadError::Offline {
            source: Some(source),
            cli_detected,
            detail: Some(detail),
        },
        ProbeError::RateLimited { retry_after_ms } => {
            TrackerReadError::RateLimited { retry_after_ms }
        }
        ProbeError::GraphQl { detail } => TrackerReadError::Failed {
            detail: Some(detail),
        },
    }
}

fn probe_write_error(
    err: ProbeError,
    source: CredentialSource,
    cli_detected: bool,
) -> TrackerWriteError {
    match err {
        ProbeError::Unauthorized { detail } => TrackerWriteError::Auth {
            source: Some(source),
            kind: AuthFailureKind::Rejected,
            cli_detected,
            detail,
        },
        ProbeError::Unreachable(detail) => TrackerWriteError::Offline {
            source: Some(source),
            cli_detected,
            detail: Some(detail),
        },
        ProbeError::RateLimited { retry_after_ms } => {
            TrackerWriteError::RateLimited { retry_after_ms }
        }
        ProbeError::GraphQl { detail } => TrackerWriteError::Failed { message: detail },
    }
}

fn probe_error_outcome(
    err: ProbeError,
    source: CredentialSource,
    cli_detected: bool,
) -> ProbeOutcome {
    match err {
        ProbeError::Unauthorized { detail } => ProbeOutcome::Failed {
            source: Some(source),
            kind: AuthFailureKind::Rejected,
            cli_detected,
            detail,
        },
        ProbeError::Unreachable(detail) => ProbeOutcome::Failed {
            source: Some(source),
            kind: AuthFailureKind::Unreachable,
            cli_detected,
            detail: Some(detail),
        },
        ProbeError::RateLimited { .. } => ProbeOutcome::Ready { source },
        // GraphQl 只可能来自 GraphQL 读路径，probe 走 REST，不会出现。
        ProbeError::GraphQl { detail } => ProbeOutcome::Failed {
            source: Some(source),
            kind: AuthFailureKind::Unreachable,
            cli_detected,
            detail: Some(detail),
        },
    }
}

struct ProcessEnv;

impl EnvSource for ProcessEnv {
    fn var(&self, key: &str) -> Option<String> {
        std::env::var(key)
            .ok()
            .filter(|value| !value.trim().is_empty())
    }
}

struct ProcessGh;

impl GhAuth for ProcessGh {
    fn detected(&self) -> bool {
        Command::new("gh")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn token(&self, hostname: &str) -> Option<String> {
        let output = Command::new("gh")
            .args(["auth", "token", "--hostname", hostname])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let token = String::from_utf8(output.stdout).ok()?;
        nonempty(Some(token.as_str()))
    }
}

struct LiveGitHubApi;

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
            Err(ureq::Error::Status(401 | 403 | 404, _)) => {
                Err(ProbeError::Unauthorized { detail: None })
            }
            Err(ureq::Error::Status(429, response)) => Err(ProbeError::RateLimited {
                retry_after_ms: parse_retry_after_ms(response.header("retry-after")),
            }),
            Err(ureq::Error::Status(code, _)) => {
                Err(ProbeError::Unreachable(format!("GitHub HTTP {code}")))
            }
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

fn github_user_url(host: &str) -> String {
    if host.eq_ignore_ascii_case("github.com") {
        "https://api.github.com/user".into()
    } else {
        format!("https://{host}/api/v3/user")
    }
}

fn github_assignees_url(host: &str, repository: &str, number: u64) -> String {
    format!(
        "{}/issues/{number}/assignees",
        github_repo_url(host, repository)
    )
}

fn github_json(
    method: &str,
    url: &str,
    token: &str,
    body: Option<&Value>,
) -> Result<Value, ProbeError> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(8))
        .build();
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
    let response = match response {
        Ok(response) => response,
        Err(ureq::Error::Status(401 | 403 | 404, _)) => {
            return Err(ProbeError::Unauthorized { detail: None });
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
    Ok(payload)
}

fn graphql_post(
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
    let response = match agent
        .post(&url)
        .set("Authorization", &format!("Bearer {token}"))
        .set("User-Agent", "Agent-Taskboard")
        .set("Accept", "application/vnd.github+json")
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
    {
        Ok(response) => response,
        Err(ureq::Error::Status(401 | 403, _)) => {
            return Err(ProbeError::Unauthorized { detail: None })
        }
        Err(ureq::Error::Status(404, _)) => return Err(ProbeError::Unauthorized { detail: None }),
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
    classify_graphql_errors(&payload)?;
    Ok(payload)
}

fn classify_graphql_errors(payload: &Value) -> Result<(), ProbeError> {
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

fn connection_has_next_page(node: &Value, field: &str) -> bool {
    node.pointer(&format!("/{field}/pageInfo/hasNextPage"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn set_connection_nodes(node: &mut Value, field: &str, nodes: Vec<Value>) {
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

fn issue_edges_query(edges: IssueEdges) -> String {
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

fn github_repo_url(host: &str, repository: &str) -> String {
    if host.eq_ignore_ascii_case("github.com") {
        format!("https://api.github.com/repos/{repository}")
    } else {
        format!("https://{host}/api/v3/repos/{repository}")
    }
}

fn github_web_issue_url(host: &str, repository: &str, number: u64) -> String {
    if host.eq_ignore_ascii_case("github.com") {
        format!("https://github.com/{repository}/issues/{number}")
    } else {
        format!("https://{host}/{repository}/issues/{number}")
    }
}

fn parse_retry_after_ms(value: Option<&str>) -> Option<u64> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse::<u64>().ok())
        .map(|seconds| seconds.saturating_mul(1000))
}

fn github_graphql_url(host: &str) -> String {
    if host.eq_ignore_ascii_case("github.com") {
        "https://api.github.com/graphql".into()
    } else {
        format!("https://{host}/api/graphql")
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

fn issue_at_mut<'a>(
    issues: &'a mut BTreeMap<String, Vec<IssueRecord>>,
    repository: &str,
    index: usize,
) -> &'a mut IssueRecord {
    issues
        .get_mut(repository)
        .and_then(|items| items.get_mut(index))
        .expect("memory tracker issue")
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
            end_cursor: has_next_page.then(|| format!("cursor-{next}")),
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
            end_cursor: has_next_page.then(|| format!("cursor-{next}")),
        })
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

fn cursor_offset(cursor: Option<&str>) -> usize {
    cursor
        .and_then(|cursor| cursor.strip_prefix("cursor-"))
        .and_then(|offset| offset.parse::<usize>().ok())
        .unwrap_or(0)
}

fn github_node(
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
        "url": github_web_issue_url(host, repository, number),
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

fn github_ref(node: &Value, repository: &str) -> Value {
    serde_json::json!({
        "number": node.get("number").and_then(Value::as_u64).unwrap_or(0),
        "title": node.get("title").and_then(Value::as_str).unwrap_or(""),
        "state": node.get("state").and_then(Value::as_str).unwrap_or("OPEN"),
        "repository": { "nameWithOwner": repository },
    })
}

fn ref_repository(value: &Value) -> Option<String> {
    value
        .pointer("/repository/full_name")
        .or_else(|| value.pointer("/repository/nameWithOwner"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn push_connection_node(item: &mut Value, field: &str, node_ref: &Value) -> bool {
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

fn remove_node_from_connection(item: &mut Value, field: &str, node_ref: &Value) -> bool {
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

fn bump_blocked_summary(item: &mut Value, field: &str, delta: i64) {
    let path = format!("/issueDependenciesSummary/{field}");
    let Some(current) = item.pointer(&path).and_then(Value::as_u64) else {
        return;
    };
    item["issueDependenciesSummary"][field] =
        serde_json::json!((current as i64 + delta).max(0) as u64);
}

fn mutate_assignee_logins(node: &mut Value, logins: &[String], add: bool) {
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

fn apply_login_delta(items: &mut Vec<Value>, logins: &[String], add: bool) {
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

fn map_issue_comment(node: &Value, host: &str, repository: &str, number: u64) -> IssueComment {
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
        .get("url")
        .or_else(|| node.get("html_url"))
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
    let parent = parse_ref(node.get("parent"), &repository);
    let children = connection_refs(
        node.get("subIssues").or_else(|| node.get("sub_issues")),
        &repository,
    );
    let mut blocked_by = connection_deps(
        node.get("blockedBy")
            .or_else(|| node.get("blocked_by"))
            .or_else(|| node.pointer("/dependencies/blocked_by")),
        &repository,
    );
    let blocking = connection_refs(
        node.get("blocking")
            .or_else(|| node.pointer("/dependencies/blocking")),
        &repository,
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

fn logins(value: Option<&Value>) -> Vec<String> {
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

fn label_names(value: Option<&Value>) -> Vec<String> {
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

fn connection_refs(value: Option<&Value>, fallback_repository: &str) -> Vec<IssueRef> {
    connection_nodes(value)
        .into_iter()
        .filter_map(|node| parse_ref(Some(node), fallback_repository))
        .collect()
}

fn connection_deps(value: Option<&Value>, fallback_repository: &str) -> Vec<DependencyRef> {
    connection_nodes(value)
        .into_iter()
        .map(|node| parse_dep(node, fallback_repository))
        .collect()
}

fn connection_nodes(value: Option<&Value>) -> Vec<&Value> {
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

fn parse_ref(value: Option<&Value>, fallback_repository: &str) -> Option<IssueRef> {
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
    Some(IssueRef {
        repository,
        number,
        title,
        open: value.get("state").and_then(Value::as_str).map(state_open),
    })
}

fn parse_dep(value: &Value, fallback_repository: &str) -> DependencyRef {
    match parse_ref(Some(value), fallback_repository) {
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

fn state_open(state: &str) -> bool {
    !state.eq_ignore_ascii_case("closed")
}
