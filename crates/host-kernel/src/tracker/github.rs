mod scripted;
pub use scripted::ScriptedGitHub;

use super::*;
use crate::agent::{home_dir, prepare_launch_env, probe_binary, ProbeResult};
use crate::issue::{parse_issue_id, DependencyRef, IssueRecord, IssueRef};
use crate::launch_env::{LaunchEnvPort, LaunchEnvironment};
use crate::tracker_seam::TrackerPort;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

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

pub fn gh_known_install_locations() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    dirs.push(PathBuf::from("/opt/homebrew/bin"));
    dirs.push(PathBuf::from("/usr/local/bin"));
    if let Some(home) = home_dir() {
        dirs.push(home.join(".local").join("bin"));
        dirs.push(home.join("scoop").join("shims"));
        dirs.push(
            home.join("AppData")
                .join("Local")
                .join("Programs")
                .join("GitHub CLI"),
        );
    }
    for key in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
        if let Some(root) = std::env::var_os(key).filter(|value| !value.is_empty()) {
            dirs.push(PathBuf::from(root).join("GitHub CLI"));
        }
    }
    dirs.push(PathBuf::from(r"C:\Program Files\GitHub CLI"));
    dirs.push(PathBuf::from(r"C:\Program Files (x86)\GitHub CLI"));
    dirs
}

fn process_environment(cwd: &Path) -> LaunchEnvironment {
    LaunchEnvironment::from_vars(cwd.to_path_buf(), std::env::vars().collect())
}

pub fn resolve_gh(
    launch_env: Arc<dyn LaunchEnvPort>,
    cwd: &Path,
    known_locations: &[PathBuf],
) -> Option<PathBuf> {
    let captured = launch_env
        .capture(cwd)
        .unwrap_or_else(|_| process_environment(cwd));
    let env = prepare_launch_env(captured, &[], known_locations);
    match probe_binary("gh", &env, known_locations) {
        ProbeResult::Found { executable } => Some(executable),
        ProbeResult::Missing { .. } => None,
    }
}

fn capture_for_gh(
    launch_env: &dyn LaunchEnvPort,
    cwd: &Path,
    known_locations: &[PathBuf],
) -> LaunchEnvironment {
    let captured = launch_env
        .capture(cwd)
        .unwrap_or_else(|_| process_environment(cwd));
    prepare_launch_env(captured, &[], known_locations)
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
    fn read_issue(
        &self,
        host: &str,
        repository: &str,
        token: &str,
        number: u64,
    ) -> Result<Value, ProbeError>;
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

impl GitHubTracker {
    pub fn live(launch_env: Arc<dyn LaunchEnvPort>) -> Self {
        Self::live_with(launch_env, gh_known_install_locations())
    }

    pub fn live_with(launch_env: Arc<dyn LaunchEnvPort>, known_locations: Vec<PathBuf>) -> Self {
        Self::assembled(launch_env, known_locations, Box::new(LiveGitHubApi))
    }

    pub fn live_with_script(
        launch_env: Arc<dyn LaunchEnvPort>,
        cwd: PathBuf,
        known_locations: Vec<PathBuf>,
        script: ScriptedGitHub,
    ) -> Self {
        let scripted = Self::scripted(script);
        Self {
            env: Box::new(LaunchEnvSource {
                launch_env: launch_env.clone(),
                cwd: cwd.clone(),
                known_locations: known_locations.clone(),
            }),
            gh: Box::new(ResolvedGh::new(launch_env, cwd, known_locations)),
            api: scripted.api,
        }
    }

    fn assembled(
        launch_env: Arc<dyn LaunchEnvPort>,
        known_locations: Vec<PathBuf>,
        api: Box<dyn GitHubApi>,
    ) -> Self {
        let cwd = home_dir().unwrap_or_else(|| PathBuf::from("."));
        Self {
            env: Box::new(LaunchEnvSource {
                launch_env: launch_env.clone(),
                cwd: cwd.clone(),
                known_locations: known_locations.clone(),
            }),
            gh: Box::new(ResolvedGh::new(launch_env, cwd, known_locations)),
            api,
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
    ) -> Result<Option<String>, TrackerReadError> {
        let Some(number) = node.get("number").and_then(Value::as_u64) else {
            return Ok(None);
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
                    return Ok(Some(format!(
                        "GitHub {} pagination for Issue #{} ended without a cursor",
                        edges.field(),
                        number
                    )));
                };
                after = Some(cursor);
            }
            set_connection_nodes(node, edges.field(), pages);
        }
        Ok(None)
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
        match self.read_all(ctx)? {
            crate::tracker_seam::TrackerReadOutcome::Complete { issues } => Ok(issues),
            crate::tracker_seam::TrackerReadOutcome::Incomplete { detail, .. } => {
                Err(TrackerReadError::Failed {
                    detail: Some(detail),
                })
            }
        }
    }

    fn read_all(
        &self,
        ctx: &ProbeContext<'_>,
    ) -> Result<crate::tracker_seam::TrackerReadOutcome, TrackerReadError> {
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
                return Ok(crate::tracker_seam::TrackerReadOutcome::Incomplete {
                    issues: map_github_nodes(&nodes, ctx),
                    detail: "GitHub Issue pagination ended without a cursor".into(),
                });
            };
            after = Some(cursor);
        }
        for node in nodes.iter_mut() {
            if let Some(detail) =
                self.complete_issue_edges(ctx, &token, source, cli_detected, node)?
            {
                return Ok(crate::tracker_seam::TrackerReadOutcome::Incomplete {
                    issues: map_github_nodes(&nodes, ctx),
                    detail,
                });
            }
        }
        Ok(crate::tracker_seam::TrackerReadOutcome::Complete {
            issues: map_github_nodes(&nodes, ctx),
        })
    }

    fn read_issue_document(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueDocument, TrackerReadError> {
        let (token, source, cli_detected) = self.authorized_read(ctx)?;
        let (repository, number) =
            parse_issue_id(issue_id).ok_or_else(|| TrackerReadError::Failed {
                detail: Some("unknown issue".into()),
            })?;
        let node = self
            .api
            .read_issue(ctx.github_host, &repository, &token, number)
            .map_err(|err| probe_read_error(err, source, cli_detected))?;
        let issue =
            map_github_issue_node(&node, &repository, ctx.github_host).ok_or_else(|| {
                TrackerReadError::Failed {
                    detail: Some("cannot map GitHub issue".into()),
                }
            })?;
        let body = node
            .get("body")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        Ok(IssueDocument { issue, body })
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

struct LaunchEnvSource {
    launch_env: Arc<dyn LaunchEnvPort>,
    cwd: PathBuf,
    known_locations: Vec<PathBuf>,
}

impl EnvSource for LaunchEnvSource {
    fn var(&self, key: &str) -> Option<String> {
        capture_for_gh(self.launch_env.as_ref(), &self.cwd, &self.known_locations)
            .vars
            .get(key)
            .cloned()
            .and_then(|value| nonempty(Some(&value)))
            .or_else(|| {
                std::env::var(key)
                    .ok()
                    .and_then(|value| nonempty(Some(&value)))
            })
    }
}

struct ResolvedGh {
    executable: Option<PathBuf>,
    env: LaunchEnvironment,
}

impl ResolvedGh {
    fn new(
        launch_env: Arc<dyn LaunchEnvPort>,
        cwd: PathBuf,
        known_locations: Vec<PathBuf>,
    ) -> Self {
        let env = capture_for_gh(launch_env.as_ref(), &cwd, &known_locations);
        let executable = match probe_binary("gh", &env, &known_locations) {
            ProbeResult::Found { executable } => Some(executable),
            ProbeResult::Missing { .. } => None,
        };
        Self { executable, env }
    }

    fn command(&self) -> Option<Command> {
        let executable = self.executable.as_ref()?;
        let mut command = Command::new(executable);
        command.current_dir(&self.env.cwd);
        command.env_clear();
        for (key, value) in &self.env.vars {
            command.env(key, value);
        }
        if !self.env.vars.contains_key("HOME") && !self.env.vars.contains_key("USERPROFILE") {
            if let Some(home) = home_dir() {
                command.env("HOME", &home);
                command.env("USERPROFILE", &home);
            }
        }
        Some(command)
    }
}

impl GhAuth for ResolvedGh {
    fn detected(&self) -> bool {
        let mut command = match self.command() {
            Some(command) => command,
            None => return false,
        };
        command
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn token(&self, hostname: &str) -> Option<String> {
        let output = self
            .command()?
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

fn classify_rest_status(code: u16, response: ureq::Response) -> ProbeError {
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

fn transient_retry_delay(attempt: u8) -> Duration {
    Duration::from_millis(100 * u64::from(attempt))
}

fn is_transient_transport_error(error: &ureq::Error) -> bool {
    if matches!(error, ureq::Error::Status(..)) {
        return false;
    }
    is_transient_transport_detail(&error.to_string())
}

fn is_transient_transport_detail(detail: &str) -> bool {
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

fn map_github_nodes(nodes: &[Value], ctx: &ProbeContext<'_>) -> Vec<IssueRecord> {
    nodes
        .iter()
        .filter_map(|node| map_github_issue_node(node, ctx.repository, ctx.github_host))
        .collect()
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

fn connection_refs(
    value: Option<&Value>,
    fallback_repository: &str,
    github_host: &str,
) -> Vec<IssueRef> {
    connection_nodes(value)
        .into_iter()
        .filter_map(|node| parse_ref(Some(node), fallback_repository, github_host))
        .collect()
}

fn connection_deps(
    value: Option<&Value>,
    fallback_repository: &str,
    github_host: &str,
) -> Vec<DependencyRef> {
    connection_nodes(value)
        .into_iter()
        .map(|node| parse_dep(node, fallback_repository, github_host))
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

fn parse_ref(
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

fn parse_dep(value: &Value, fallback_repository: &str, github_host: &str) -> DependencyRef {
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

fn state_open(state: &str) -> bool {
    !state.eq_ignore_ascii_case("closed")
}
