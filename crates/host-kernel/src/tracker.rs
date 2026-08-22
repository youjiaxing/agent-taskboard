use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::issue::{DependencyRef, IssueRecord, IssueRef};

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
}

pub trait TrackerPort: Send + Sync {
    fn probe(&self, ctx: &ProbeContext<'_>) -> ProbeOutcome;
    fn read_issues(&self, ctx: &ProbeContext<'_>) -> Result<Vec<IssueRecord>, TrackerReadError>;
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

pub struct MemoryTracker {
    failures: Mutex<BTreeSet<String>>,
    read_scripts: Mutex<BTreeMap<String, ReadScript>>,
    read_counts: Mutex<BTreeMap<String, u64>>,
    issues: Mutex<BTreeMap<String, Vec<IssueRecord>>>,
    source: CredentialSource,
}

impl MemoryTracker {
    pub fn new() -> Self {
        Self {
            failures: Mutex::new(BTreeSet::new()),
            read_scripts: Mutex::new(BTreeMap::new()),
            read_counts: Mutex::new(BTreeMap::new()),
            issues: Mutex::new(BTreeMap::new()),
            source: CredentialSource::Cli,
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
}

trait EnvSource: Send + Sync {
    fn var(&self, key: &str) -> Option<String>;
}

trait GhAuth: Send + Sync {
    fn detected(&self) -> bool;
    fn token(&self, hostname: &str) -> Option<String>;
}

trait GitHubApi: Send + Sync {
    fn probe_repo(&self, host: &str, repository: &str, token: &str) -> Result<(), ProbeError>;
    fn list_issue_nodes(
        &self,
        host: &str,
        repository: &str,
        token: &str,
    ) -> Result<Vec<Value>, ProbeError>;
}

enum ProbeError {
    Unauthorized,
    Unreachable(String),
    RateLimited { retry_after_ms: Option<u64> },
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
                issues: script.issues.clone(),
                read_unauthorized: script.read_unauthorized,
                rate_limited: script.rate_limited,
                retry_after_ms: script.retry_after_ms,
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
        let (token, source, cli_detected) =
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
            })?;
        match self
            .api
            .list_issue_nodes(ctx.github_host, ctx.repository, &token)
        {
            Ok(nodes) => Ok(nodes
                .iter()
                .filter_map(|node| map_github_issue_node(node, ctx.repository))
                .collect()),
            Err(err) => Err(match err {
                ProbeError::Unauthorized => TrackerReadError::Auth {
                    source: Some(source),
                    kind: AuthFailureKind::Rejected,
                    cli_detected,
                    detail: None,
                },
                ProbeError::Unreachable(detail) => TrackerReadError::Offline {
                    source: Some(source),
                    cli_detected,
                    detail: Some(detail),
                },
                ProbeError::RateLimited { retry_after_ms } => {
                    TrackerReadError::RateLimited { retry_after_ms }
                }
            }),
        }
    }
}

fn probe_error_outcome(
    err: ProbeError,
    source: CredentialSource,
    cli_detected: bool,
) -> ProbeOutcome {
    match err {
        ProbeError::Unauthorized => ProbeOutcome::Failed {
            source: Some(source),
            kind: AuthFailureKind::Rejected,
            cli_detected,
            detail: None,
        },
        ProbeError::Unreachable(detail) => ProbeOutcome::Failed {
            source: Some(source),
            kind: AuthFailureKind::Unreachable,
            cli_detected,
            detail: Some(detail),
        },
        ProbeError::RateLimited { .. } => ProbeOutcome::Ready { source },
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
            Err(ureq::Error::Status(401 | 403 | 404, _)) => Err(ProbeError::Unauthorized),
            Err(ureq::Error::Status(429, response)) => Err(ProbeError::RateLimited {
                retry_after_ms: parse_retry_after_ms(response.header("retry-after")),
            }),
            Err(ureq::Error::Status(code, _)) => {
                Err(ProbeError::Unreachable(format!("GitHub HTTP {code}")))
            }
            Err(err) => Err(ProbeError::Unreachable(err.to_string())),
        }
    }

    fn list_issue_nodes(
        &self,
        host: &str,
        repository: &str,
        token: &str,
    ) -> Result<Vec<Value>, ProbeError> {
        let Some((owner, name)) = repository.split_once('/') else {
            return Err(ProbeError::Unreachable(
                "repository must be owner/name".into(),
            ));
        };
        let url = github_graphql_url(host);
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(20))
            .build();
        let mut after: Option<String> = None;
        let mut nodes = Vec::new();
        loop {
            let body = serde_json::json!({
                "query": GITHUB_ISSUES_QUERY,
                "variables": { "owner": owner, "name": name, "after": after },
            });
            let response = match agent
                .post(&url)
                .set("Authorization", &format!("Bearer {token}"))
                .set("User-Agent", "Agent-Taskboard")
                .set("Accept", "application/vnd.github+json")
                .set("Content-Type", "application/json")
                .send_string(&body.to_string())
            {
                Ok(response) => response,
                Err(ureq::Error::Status(401 | 403, _)) => return Err(ProbeError::Unauthorized),
                Err(ureq::Error::Status(404, _)) => return Err(ProbeError::Unauthorized),
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
            if payload
                .get("errors")
                .and_then(Value::as_array)
                .is_some_and(|errors| !errors.is_empty())
            {
                return Err(ProbeError::Unreachable("GitHub GraphQL error".into()));
            }
            let connection = payload
                .pointer("/data/repository/issues")
                .cloned()
                .unwrap_or(Value::Null);
            if let Some(page) = connection.get("nodes").and_then(Value::as_array) {
                nodes.extend(page.iter().cloned());
            }
            let has_next = connection
                .pointer("/pageInfo/hasNextPage")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            after = connection
                .pointer("/pageInfo/endCursor")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            if !has_next || after.is_none() || nodes.len() >= 500 {
                break;
            }
        }
        Ok(nodes)
    }
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
        blockedBy(first: 50) { nodes { number title state repository { nameWithOwner } } }
        blocking(first: 50) { nodes { number title state repository { nameWithOwner } } }
        subIssues(first: 100) { nodes { number title state repository { nameWithOwner } } }
      }
    }
  }
}
"#;

fn github_repo_url(host: &str, repository: &str) -> String {
    if host.eq_ignore_ascii_case("github.com") {
        format!("https://api.github.com/repos/{repository}")
    } else {
        format!("https://{host}/api/v3/repos/{repository}")
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
    issues: BTreeMap<String, Vec<Value>>,
    read_unauthorized: bool,
    rate_limited: bool,
    retry_after_ms: Option<u64>,
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
            Err(ProbeError::Unauthorized)
        }
    }

    fn list_issue_nodes(
        &self,
        _host: &str,
        repository: &str,
        token: &str,
    ) -> Result<Vec<Value>, ProbeError> {
        if self.unreachable {
            return Err(ProbeError::Unreachable("unreachable".into()));
        }
        if self.rate_limited {
            return Err(ProbeError::RateLimited {
                retry_after_ms: self.retry_after_ms,
            });
        }
        if self.read_unauthorized || !self.accept.contains(token) {
            return Err(ProbeError::Unauthorized);
        }
        Ok(self.issues.get(repository).cloned().unwrap_or_default())
    }
}

pub fn map_github_issue_node(node: &Value, fallback_repository: &str) -> Option<IssueRecord> {
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
        .unwrap_or_else(|| format!("https://github.com/{repository}/issues/{number}"));
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
