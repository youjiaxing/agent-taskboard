mod api;
mod map;
mod scripted;
pub(crate) use api::*;
pub use map::map_github_issue_node;
pub(crate) use map::*;
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

    fn read_one_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        complete_relations: bool,
    ) -> Result<IssueDocument, TrackerReadError> {
        let (token, source, cli_detected) = self.authorized_read(ctx)?;
        let (repository, number) =
            parse_issue_id(issue_id).ok_or_else(|| TrackerReadError::Failed {
                detail: Some("unknown issue".into()),
            })?;
        let mut node = self
            .api
            .read_issue(ctx.github_host, &repository, &token, number)
            .map_err(|err| probe_read_error(err, source, cli_detected))?;
        if complete_relations {
            if let Some(detail) =
                self.complete_issue_edges(ctx, &token, source, cli_detected, &mut node)?
            {
                return Err(TrackerReadError::Failed {
                    detail: Some(detail),
                });
            }
        }
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
        self.read_one_issue(ctx, issue_id, false)
    }

    fn read_issue_content(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueDocument, TrackerReadError> {
        self.read_one_issue(ctx, issue_id, false)
    }

    fn read_issue_relations(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerReadError> {
        self.read_one_issue(ctx, issue_id, true)
            .map(|document| document.issue)
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
