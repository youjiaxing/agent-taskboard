use std::collections::{BTreeMap, BTreeSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::agent::{home_dir, prepare_launch_env, probe_binary, ProbeResult};
use crate::issue::{parse_issue_id, DependencyRef, IssueRecord, IssueRef};
use crate::launch_env::{LaunchEnvPort, LaunchEnvironment};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrackerKind {
    Github,
    LocalMarkdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CredentialSource {
    AppEnv,
    SecretsFile,
    Cli,
    GenericEnv,
    LocalFile,
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
    pub tracker: TrackerKind,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueDocument {
    pub issue: IssueRecord,
    /// Tracker 原文；Host 和 Client 不从中推导 Dependency 或父子关系。
    pub body: String,
}

pub trait TrackerPort: Send + Sync {
    fn probe(&self, ctx: &ProbeContext<'_>) -> ProbeOutcome;
    fn read_issues(&self, ctx: &ProbeContext<'_>) -> Result<Vec<IssueRecord>, TrackerReadError>;
    fn read_all(
        &self,
        ctx: &ProbeContext<'_>,
    ) -> Result<crate::tracker_seam::TrackerReadOutcome, TrackerReadError> {
        self.read_issues(ctx)
            .map(|issues| crate::tracker_seam::TrackerReadOutcome::Complete { issues })
    }
    fn read_issue_document(
        &self,
        _ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueDocument, TrackerReadError>;
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
    /// Replace the complete blocked_by set. Trackers with a transactional or
    /// single-document representation should override this to avoid partial writes.
    fn set_blocked_by(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        current_issue_ids: &[String],
        blocking_issue_ids: &[String],
    ) -> Result<(), TrackerWriteError> {
        for blocker in current_issue_ids
            .iter()
            .filter(|id| !blocking_issue_ids.contains(id))
        {
            self.remove_blocked_by(ctx, issue_id, blocker)?;
        }
        for blocker in blocking_issue_ids
            .iter()
            .filter(|id| !current_issue_ids.contains(id))
        {
            self.add_blocked_by(ctx, issue_id, blocker)?;
        }
        Ok(())
    }
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
    detail_read_scripts: Mutex<BTreeMap<String, ReadScript>>,
    read_counts: Mutex<BTreeMap<String, u64>>,
    issues: Mutex<BTreeMap<String, Vec<IssueRecord>>>,
    write_fail: Mutex<BTreeMap<String, String>>,
    bodies: Mutex<BTreeMap<String, String>>,
    comments: Mutex<BTreeMap<String, Vec<IssueComment>>>,
    actor: String,
    source: CredentialSource,
}

/// Tracker adapter for the repository-local Markdown convention used by Matt's
/// local tracker. The project `repository` field contains the absolute checkout
/// path; files are discovered below `.scratch/*/issues/*.md`.
#[derive(Debug, Default, Clone, Copy)]
pub struct LocalMarkdownTracker;

impl LocalMarkdownTracker {
    fn root(ctx: &ProbeContext<'_>) -> PathBuf {
        PathBuf::from(ctx.repository)
    }

    fn issue_files(root: &Path) -> Vec<PathBuf> {
        let scratch = root.join(".scratch");
        let Ok(features) = std::fs::read_dir(scratch) else {
            return Vec::new();
        };
        let mut files = Vec::new();
        for feature in features.flatten() {
            let issues = feature.path().join("issues");
            let Ok(entries) = std::fs::read_dir(issues) else {
                continue;
            };
            files.extend(entries.flatten().map(|entry| entry.path()).filter(|path| {
                path.extension().is_some_and(|ext| ext == "md")
                    && path
                        .file_stem()
                        .and_then(|name| name.to_str())
                        .and_then(|name| name.split_once('-'))
                        .and_then(|(n, _)| n.parse::<u64>().ok())
                        .is_some()
            }));
        }
        files.sort();
        files
    }

    pub fn content_revision(root: &Path) -> std::io::Result<u64> {
        let mut hasher = DefaultHasher::new();
        for path in Self::issue_files(root) {
            path.hash(&mut hasher);
            std::fs::read(&path)?.hash(&mut hasher);
        }
        Ok(hasher.finish())
    }

    fn validate_relation_update<F>(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        replacement: Vec<String>,
        relation: &str,
        edges: F,
    ) -> Result<(), TrackerWriteError>
    where
        F: Fn(&IssueRecord) -> Vec<String>,
    {
        let outcome = self
            .read_all(ctx)
            .map_err(|error| TrackerWriteError::Failed {
                message: format!("cannot validate {relation} graph: {error:?}"),
            })?;
        let issues = match outcome {
            crate::tracker_seam::TrackerReadOutcome::Complete { issues } => issues,
            crate::tracker_seam::TrackerReadOutcome::Incomplete { detail, .. } => {
                return Err(TrackerWriteError::Failed {
                    message: format!(
                        "cannot update {relation}: tracker data is incomplete: {detail}"
                    ),
                });
            }
        };
        let mut adjacency = issues
            .iter()
            .map(|issue| (issue.id(), edges(issue)))
            .collect::<BTreeMap<_, _>>();
        adjacency.insert(issue_id.to_string(), replacement);
        if graph_cycle_node(&adjacency).is_some() {
            return Err(TrackerWriteError::Failed {
                message: format!("{relation} update would create a cycle"),
            });
        }
        Ok(())
    }

    fn parse_file(
        root: &Path,
        path: &Path,
    ) -> Result<(IssueRecord, String, Vec<LocalReference>, Vec<String>), String> {
        let body =
            std::fs::read_to_string(path).map_err(|err| format!("{}: {err}", path.display()))?;
        Self::parse_document(root, path, body)
    }

    fn parse_document(
        root: &Path,
        path: &Path,
        body: String,
    ) -> Result<(IssueRecord, String, Vec<LocalReference>, Vec<String>), String> {
        let stem = path
            .file_stem()
            .ok_or_else(|| format!("{} has no filename", path.display()))?
            .to_string_lossy();
        let (number_text, _) = stem
            .split_once('-')
            .ok_or_else(|| format!("{} must be named NN-slug.md", path.display()))?;
        let number = number_text
            .parse::<u64>()
            .map_err(|_| format!("{} has an invalid issue number", path.display()))?;
        let fields = header_fields(&body);
        let mut metadata_errors = Vec::new();
        let status_values = fields.get("status").cloned().unwrap_or_default();
        let status = status_values.last().cloned().unwrap_or_default();
        let normalized_status = normalize_metadata(&status);
        if status_values.len() > 1
            && status_values
                .iter()
                .map(|value| normalize_metadata(value))
                .collect::<BTreeSet<_>>()
                .len()
                > 1
        {
            metadata_errors.push("conflicting Status metadata".into());
        }
        let type_values = fields.get("type").cloned().unwrap_or_default();
        let issue_type = type_values.last().map(|value| normalize_metadata(value));
        if type_values.len() > 1
            && type_values
                .iter()
                .map(|value| normalize_metadata(value))
                .collect::<BTreeSet<_>>()
                .len()
                > 1
        {
            metadata_errors.push("conflicting Type metadata".into());
        }
        let is_wayfinder = issue_type.is_some();
        let valid_wayfinder_type = issue_type
            .as_deref()
            .is_some_and(|value| matches!(value, "research" | "prototype" | "grilling" | "task"));
        if is_wayfinder && !valid_wayfinder_type {
            metadata_errors.push(format!(
                "invalid Type: {}",
                issue_type.as_deref().unwrap_or("<empty>")
            ));
        }
        let implementation_statuses = [
            "needs-triage",
            "needs-info",
            "ready-for-agent",
            "ready-for-human",
            "wontfix",
            "claimed",
            "resolved",
        ];
        let wayfinder_statuses = ["open", "ready-for-agent", "claimed", "resolved", "wontfix"];
        if !is_wayfinder {
            if status_values.is_empty() {
                metadata_errors.push("missing Status metadata".into());
            } else if !implementation_statuses.contains(&normalized_status.as_str()) {
                metadata_errors.push(format!("invalid Status: {status}"));
            }
        } else if !normalized_status.is_empty()
            && !wayfinder_statuses.contains(&normalized_status.as_str())
        {
            metadata_errors.push(format!("invalid Status: {status}"));
        }
        let closed_values = fields.get("closed").cloned().unwrap_or_default();
        let closed_legacy = match closed_values.last().map(|value| normalize_metadata(value)) {
            None => None,
            Some(value) if value == "true" => Some(true),
            Some(value) if value == "false" => Some(false),
            Some(value) => {
                metadata_errors.push(format!("invalid Closed: {value}"));
                None
            }
        };
        let terminal_status = matches!(normalized_status.as_str(), "resolved" | "wontfix");
        if terminal_status && closed_legacy == Some(false) {
            metadata_errors.push("Status terminal value conflicts with Closed: false".into());
        }
        let closed = terminal_status && closed_legacy != Some(false) || closed_legacy == Some(true);
        let open = !closed;
        if closed_legacy == Some(true) {
            metadata_errors.retain(|error| !error.starts_with("missing Status"));
        }
        let repository = root.to_string_lossy().to_string();
        let title = body
            .lines()
            .find_map(|line| {
                let heading = line.trim().strip_prefix('#')?.trim();
                let heading = heading
                    .strip_prefix(&format!("{number_text} "))
                    .unwrap_or(heading);
                let heading = heading
                    .strip_prefix('—')
                    .or_else(|| heading.strip_prefix('–'))
                    .or_else(|| heading.strip_prefix('-'))
                    .or_else(|| heading.strip_prefix(':'))
                    .unwrap_or(heading)
                    .trim();
                (!heading.is_empty()).then(|| heading.to_string())
            })
            .unwrap_or_else(|| stem.to_string());
        let mut labels = Vec::new();
        if implementation_statuses.contains(&normalized_status.as_str())
            && !labels.iter().any(|label| label == &normalized_status)
        {
            labels.push(normalized_status.clone());
        }
        if let Some(issue_type) = issue_type.as_deref().filter(|_| valid_wayfinder_type) {
            labels.push(format!("type:{issue_type}"));
        }
        if !normalized_status.is_empty() {
            labels.push(format!("status:{normalized_status}"));
        }
        labels.sort();
        labels.dedup();
        let assignee_values = fields
            .get("assignee")
            .or_else(|| fields.get("assignees"))
            .into_iter()
            .flatten()
            .flat_map(|value| value.split([',', ';']).map(|item| item.trim().to_string()))
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        let mut assignees = assignee_values;
        if normalized_status == "claimed" && assignees.is_empty() {
            assignees.push(MEMORY_TRACKER_ACTOR.into());
        }
        let parent_raw = fields
            .get("part of")
            .or_else(|| fields.get("parent"))
            .and_then(|values| values.last())
            .cloned()
            .or_else(|| {
                let section = section_lines(&body, "Parent");
                (!section.is_empty()).then(|| section.join(" "))
            });
        let parent = parent_raw
            .as_deref()
            .and_then(parse_reference)
            .and_then(|reference| {
                reference.number.map(|parent_number| {
                    IssueRef::new(repository.clone(), parent_number, reference.title)
                        .with_open(true)
                })
            });
        let references = blocked_by_reference_lines(&body)
            .into_iter()
            .flat_map(|line| parse_reference_line(&line))
            .collect::<Vec<_>>();
        let issue = IssueRecord {
            repository,
            number,
            title,
            url: format!("file://{}", path.display()),
            open,
            closed_at: closed.then(|| "local-markdown".into()),
            assignees,
            labels,
            parent,
            children: Vec::new(),
            blocked_by: Vec::new(),
            blocking: Vec::new(),
        };
        Ok((issue, body, references, metadata_errors))
    }

    fn locate_issue(root: &Path, issue_id: &str) -> Result<PathBuf, TrackerWriteError> {
        let (repository, number) =
            parse_issue_id(issue_id).ok_or_else(|| TrackerWriteError::Failed {
                message: "invalid issue id".into(),
            })?;
        if repository != root.to_string_lossy() {
            return Err(TrackerWriteError::Failed {
                message: "issue belongs to another local tracker".into(),
            });
        }
        let matches = Self::issue_files(root)
            .into_iter()
            .filter(|path| {
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .and_then(|stem| stem.split_once('-'))
                    .and_then(|(n, _)| n.parse::<u64>().ok())
                    == Some(number)
            })
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [path] => Ok(path.clone()),
            [] => Err(TrackerWriteError::Failed {
                message: "unknown issue".into(),
            }),
            _ => Err(TrackerWriteError::Failed {
                message: format!("issue #{number} is ambiguous"),
            }),
        }
    }

    fn atomic_write(path: &Path, contents: &str) -> Result<(), TrackerWriteError> {
        let parent = path.parent().ok_or_else(|| TrackerWriteError::Failed {
            message: "issue has no parent directory".into(),
        })?;
        let tmp = parent.join(format!(
            ".{}.tmp-{}",
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("issue"),
            std::process::id()
        ));
        std::fs::write(&tmp, contents).map_err(|err| TrackerWriteError::Failed {
            message: err.to_string(),
        })?;
        if let Err(err) = std::fs::rename(&tmp, path) {
            let _ = std::fs::remove_file(&tmp);
            return Err(TrackerWriteError::Failed {
                message: err.to_string(),
            });
        }
        Ok(())
    }

    fn write_status(
        root: &Path,
        issue_id: &str,
        status: &str,
        clear_assignees: bool,
        clear_closed: bool,
    ) -> Result<IssueRecord, TrackerWriteError> {
        let path = Self::locate_issue(root, issue_id)?;
        let body = std::fs::read_to_string(&path).map_err(|err| TrackerWriteError::Failed {
            message: err.to_string(),
        })?;
        let (_, _, _, existing_errors) = Self::parse_document(root, &path, body.clone())
            .map_err(|message| TrackerWriteError::Failed { message })?;
        if !existing_errors.is_empty() {
            return Err(TrackerWriteError::Failed {
                message: existing_errors.join(", "),
            });
        }
        let mut lines: Vec<String> = body.lines().map(ToOwned::to_owned).collect();
        let mut replaced = false;
        for line in &mut lines {
            if line.trim_start().starts_with("## ") {
                break;
            }
            if clear_assignees
                && parse_field_line(line)
                    .is_some_and(|(name, _)| name == "assignee" || name == "assignees")
            {
                *line = String::new();
                continue;
            }
            if clear_closed && parse_field_line(line).is_some_and(|(name, _)| name == "closed") {
                *line = String::new();
                continue;
            }
            if let Some((name, _)) = parse_field_line(line) {
                if name == "status" {
                    *line = format!("Status: {status}");
                    replaced = true;
                }
            }
        }
        if !replaced {
            lines.insert(1.min(lines.len()), format!("Status: {status}"));
        }
        let contents = format!("{}\n", lines.join("\n"));
        let (issue, _, _, errors) = Self::parse_document(root, &path, contents.clone())
            .map_err(|message| TrackerWriteError::Failed { message })?;
        if !errors.is_empty() {
            return Err(TrackerWriteError::Failed {
                message: errors.join(", "),
            });
        }
        Self::atomic_write(&path, &contents)?;
        Ok(issue)
    }

    fn transition_open_status(
        root: &Path,
        issue_id: &str,
        clear_assignees: bool,
    ) -> Result<IssueRecord, TrackerWriteError> {
        let path = Self::locate_issue(root, issue_id)?;
        let body = std::fs::read_to_string(&path).map_err(|err| TrackerWriteError::Failed {
            message: err.to_string(),
        })?;
        let wayfinder = header_fields(&body)
            .get("type")
            .and_then(|values| values.last())
            .is_some_and(|value| {
                matches!(
                    normalize_metadata(value).as_str(),
                    "research" | "prototype" | "grilling" | "task"
                )
            });
        Self::write_status(
            root,
            issue_id,
            if wayfinder { "open" } else { "ready-for-agent" },
            clear_assignees,
            true,
        )
    }

    fn rewrite_parent(
        root: &Path,
        issue_id: &str,
        value: Option<&str>,
    ) -> Result<(), TrackerWriteError> {
        let path = Self::locate_issue(root, issue_id)?;
        let body = std::fs::read_to_string(&path).map_err(|err| TrackerWriteError::Failed {
            message: err.to_string(),
        })?;
        let lines = body.lines().map(ToOwned::to_owned).collect::<Vec<_>>();
        let mut rewritten = Vec::with_capacity(lines.len());
        let mut in_header = true;
        let mut index = 0;
        while index < lines.len() {
            let line = &lines[index];
            if let Some(section) = line.trim().strip_prefix("## ") {
                in_header = false;
                if normalize_metadata(section) == "parent" {
                    index += 1;
                    while index < lines.len() && !lines[index].trim_start().starts_with("## ") {
                        index += 1;
                    }
                    continue;
                }
            }
            if in_header
                && parse_field_line(line)
                    .is_some_and(|(name, _)| matches!(name.as_str(), "part of" | "parent"))
            {
                index += 1;
                continue;
            }
            rewritten.push(line.clone());
            index += 1;
        }
        if let Some(value) = value {
            rewritten.insert(1.min(rewritten.len()), format!("Part of: {value}"));
        }
        Self::atomic_write(&path, &format!("{}\n", rewritten.join("\n")))
    }

    fn rewrite_blocked_by(
        root: &Path,
        issue_id: &str,
        blocking_issue_id: &str,
        add: bool,
    ) -> Result<(), TrackerWriteError> {
        let path = Self::locate_issue(root, issue_id)?;
        let (_, blocker_number) =
            parse_issue_id(blocking_issue_id).ok_or_else(|| TrackerWriteError::Failed {
                message: "unknown blocker".into(),
            })?;
        Self::locate_issue(root, blocking_issue_id)?;
        let body = std::fs::read_to_string(&path).map_err(|err| TrackerWriteError::Failed {
            message: err.to_string(),
        })?;
        let refs = blocked_by_reference_lines(&body).join(", ");
        let mut references = parse_reference_line(&refs)
            .into_iter()
            .filter(|reference| reference.number != Some(blocker_number))
            .map(|reference| {
                reference
                    .number
                    .map(|number| number.to_string())
                    .unwrap_or(reference.raw)
            })
            .collect::<Vec<_>>();
        if add {
            references.push(blocker_number.to_string());
        }
        Self::write_blocked_by_references(&path, &body, &references)
    }

    fn rewrite_blocked_by_set(
        root: &Path,
        issue_id: &str,
        blocking_issue_ids: &[String],
    ) -> Result<(), TrackerWriteError> {
        let path = Self::locate_issue(root, issue_id)?;
        let mut references = Vec::new();
        for blocking_issue_id in blocking_issue_ids {
            let (_, blocker_number) =
                parse_issue_id(blocking_issue_id).ok_or_else(|| TrackerWriteError::Failed {
                    message: "unknown blocker".into(),
                })?;
            let blocker_path = Self::locate_issue(root, blocking_issue_id)?;
            if blocker_path == path {
                return Err(TrackerWriteError::Failed {
                    message: "issue cannot block itself".into(),
                });
            }
            let number = blocker_number.to_string();
            if !references.contains(&number) {
                references.push(number);
            }
        }
        let body = std::fs::read_to_string(&path).map_err(|err| TrackerWriteError::Failed {
            message: err.to_string(),
        })?;
        Self::write_blocked_by_references(&path, &body, &references)
    }

    fn write_blocked_by_references(
        path: &Path,
        body: &str,
        references: &[String],
    ) -> Result<(), TrackerWriteError> {
        let replacement = if references.is_empty() {
            "Blocked by: None".to_string()
        } else {
            format!("Blocked by: {}", references.join(", "))
        };
        let mut lines: Vec<String> = body.lines().map(ToOwned::to_owned).collect();
        if let Some(line) = lines
            .iter_mut()
            .find(|line| parse_field_line(line).is_some_and(|(name, _)| name == "blocked by"))
        {
            *line = replacement;
        } else if let Some(index) = lines
            .iter()
            .position(|line| line.trim().eq_ignore_ascii_case("## blocked by"))
        {
            let mut end = index + 1;
            while end < lines.len() && !lines[end].trim_start().starts_with("## ") {
                end += 1;
            }
            let rendered = if references.is_empty() {
                vec!["- None".to_string()]
            } else {
                references
                    .iter()
                    .map(|reference| format!("- {reference}"))
                    .collect()
            };
            lines.splice(index + 1..end, rendered);
        } else {
            lines.insert(1.min(lines.len()), replacement);
        }
        Self::atomic_write(&path, &format!("{}\n", lines.join("\n")))
    }
}

#[derive(Debug, Clone)]
struct LocalReference {
    raw: String,
    file: Option<String>,
    number: Option<u64>,
    title: String,
}

fn normalize_metadata(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_field_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    let trimmed = trimmed.strip_prefix("**").unwrap_or(trimmed);
    let (name, value) = trimmed.split_once(':')?;
    let name = name.strip_suffix("**").unwrap_or(name).trim();
    if name.is_empty() || !name.chars().next()?.is_ascii_alphabetic() {
        return None;
    }
    Some((
        normalize_metadata(name),
        value.trim().trim_matches('*').trim().to_string(),
    ))
}

fn is_local_metadata_field(name: &str) -> bool {
    matches!(
        name,
        "status"
            | "type"
            | "assignee"
            | "assignees"
            | "part of"
            | "parent"
            | "blocked by"
            | "closed"
    )
}

fn header_fields(text: &str) -> BTreeMap<String, Vec<String>> {
    let mut fields = BTreeMap::<String, Vec<String>>::new();
    for line in text.lines() {
        if line.trim_start().starts_with("## ") {
            break;
        }
        if let Some((name, value)) = parse_field_line(line) {
            fields.entry(name).or_default().push(value);
        }
    }
    fields
}

fn section_lines(text: &str, heading: &str) -> Vec<String> {
    let wanted = normalize_metadata(heading);
    let mut in_section = false;
    let mut result = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(section) = trimmed.strip_prefix("## ") {
            let current = normalize_metadata(section);
            if in_section && current != wanted {
                break;
            }
            in_section = current == wanted;
            continue;
        }
        if in_section {
            result.push(line.to_string());
        }
    }
    result
}

fn blocked_by_reference_lines(text: &str) -> Vec<String> {
    let section = section_lines(text, "Blocked by");
    if !section.is_empty() {
        return section;
    }
    header_fields(text)
        .get("blocked by")
        .cloned()
        .unwrap_or_default()
}

fn normalize_reference_title(value: &str) -> String {
    normalize_metadata(
        value
            .trim()
            .trim_start_matches(['-', '*', '+'])
            .trim()
            .trim_matches(['`', '*', '_', '.', ',', ';', ':'])
            .trim_start_matches(['—', '–', ':', '-'])
            .trim(),
    )
}

fn parse_reference(raw: &str) -> Option<LocalReference> {
    let raw = raw.trim();
    if raw.is_empty() || normalize_metadata(raw).starts_with("none") {
        return None;
    }
    let cleaned = raw
        .trim_start_matches(['-', '*', '+'])
        .trim()
        .trim_matches(['`', '*', '_'])
        .trim();
    if cleaned.is_empty() || normalize_metadata(cleaned).starts_with("none") {
        return None;
    }
    if let Some(start) = cleaned.find('(') {
        if let Some(end) = cleaned[start + 1..].find(')') {
            let target = &cleaned[start + 1..start + 1 + end];
            let file = target
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(target)
                .to_string();
            return Some(LocalReference {
                raw: raw.into(),
                file: Some(file),
                number: None,
                title: String::new(),
            });
        }
    }
    let digits = cleaned.strip_prefix('#').unwrap_or(cleaned);
    let digit_end = digits
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(digits.len());
    if digit_end == 0 {
        return Some(LocalReference {
            raw: raw.into(),
            file: None,
            number: None,
            title: String::new(),
        });
    }
    let number = digits[..digit_end].parse::<u64>().ok();
    let rest = digits[digit_end..].trim();
    let title = rest
        .strip_prefix(['—', '–', ':', '-'])
        .unwrap_or(rest)
        .trim();
    let file = cleaned
        .split_whitespace()
        .find(|part| part.to_ascii_lowercase().ends_with(".md"))
        .map(|part| part.trim_matches(['`', '*', '_', ',', ';']).to_string());
    Some(LocalReference {
        raw: raw.into(),
        file,
        number,
        title: normalize_reference_title(title),
    })
}

fn parse_reference_line(raw: &str) -> Vec<LocalReference> {
    let raw = raw.trim();
    if raw.is_empty() || normalize_metadata(raw).starts_with("none") {
        return Vec::new();
    }
    let mut parts = raw
        .split([',', ';'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() == 1 {
        // A markdown link may contain commas in its label; parse it as one token.
        parts = vec![raw];
    }
    parts.into_iter().filter_map(parse_reference).collect()
}

fn local_slug(title: &str) -> String {
    let mut slug = String::new();
    for character in title.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "issue".into()
    } else {
        slug.chars().take(48).collect()
    }
}

fn graph_cycle_node(adjacency: &BTreeMap<String, Vec<String>>) -> Option<String> {
    fn visit(
        node: &str,
        adjacency: &BTreeMap<String, Vec<String>>,
        visiting: &mut BTreeSet<String>,
        visited: &mut BTreeSet<String>,
    ) -> bool {
        if visiting.contains(node) {
            return true;
        }
        if !visited.insert(node.to_string()) {
            return false;
        }
        visiting.insert(node.to_string());
        let cycle = adjacency
            .get(node)
            .into_iter()
            .flatten()
            .any(|neighbor| visit(neighbor, adjacency, visiting, visited));
        visiting.remove(node);
        cycle
    }

    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    adjacency
        .keys()
        .find(|node| visit(node, adjacency, &mut visiting, &mut visited))
        .cloned()
}

impl TrackerPort for LocalMarkdownTracker {
    fn probe(&self, ctx: &ProbeContext<'_>) -> ProbeOutcome {
        let root = Self::root(ctx);
        if root.is_dir() && (root.join(".scratch").is_dir() || !Self::issue_files(&root).is_empty())
        {
            ProbeOutcome::Ready {
                source: CredentialSource::LocalFile,
            }
        } else {
            ProbeOutcome::Failed {
                source: Some(CredentialSource::LocalFile),
                kind: AuthFailureKind::Unreachable,
                cli_detected: false,
                detail: Some(".scratch/*/issues/*.md not found".into()),
            }
        }
    }

    fn read_issues(&self, ctx: &ProbeContext<'_>) -> Result<Vec<IssueRecord>, TrackerReadError> {
        match self.read_all(ctx)? {
            crate::tracker_seam::TrackerReadOutcome::Complete { issues }
            | crate::tracker_seam::TrackerReadOutcome::Incomplete { issues, .. } => Ok(issues),
        }
    }

    fn read_all(
        &self,
        ctx: &ProbeContext<'_>,
    ) -> Result<crate::tracker_seam::TrackerReadOutcome, TrackerReadError> {
        let root = Self::root(ctx);
        if !root.is_dir() {
            return Err(TrackerReadError::Offline {
                source: Some(CredentialSource::LocalFile),
                cli_detected: false,
                detail: Some("local project directory does not exist".into()),
            });
        }
        let mut parsed = Vec::new();
        let mut problems = Vec::new();
        for path in Self::issue_files(&root) {
            match Self::parse_file(&root, &path) {
                Ok((issue, body, references, errors)) => {
                    if !errors.is_empty() {
                        problems.push(format!("{}: {}", path.display(), errors.join(", ")));
                    }
                    parsed.push((issue, body, references));
                }
                Err(error) => problems.push(error),
            }
        }
        let repository = root.to_string_lossy().to_string();
        let mut by_number = BTreeMap::<u64, Vec<IssueRecord>>::new();
        for (issue, _, _) in &parsed {
            by_number
                .entry(issue.number)
                .or_default()
                .push(issue.clone());
        }
        for (number, candidates) in &by_number {
            if candidates.len() > 1 {
                let locations = candidates
                    .iter()
                    .map(|issue| issue.url.trim_start_matches("file://"))
                    .collect::<Vec<_>>()
                    .join(", ");
                problems.push(format!("duplicate issue number #{number}: {locations}"));
            }
        }
        let mut issues = parsed
            .iter()
            .map(|(issue, _, _)| issue.clone())
            .collect::<Vec<_>>();
        for (index, (_, _, references)) in parsed.iter().enumerate() {
            let issue_id = issues[index].id();
            for reference in references {
                let resolved = reference.file.as_deref().and_then(|file| {
                    parsed
                        .iter()
                        .find(|(issue, _, _)| issue.url.rsplit('/').next() == Some(file))
                        .map(|(issue, _, _)| issue)
                });
                let resolved = if resolved.is_some() {
                    resolved
                } else {
                    let candidates = reference.number.and_then(|number| by_number.get(&number));
                    candidates.and_then(|candidates| {
                        if candidates.len() == 1 {
                            candidates.first()
                        } else if !reference.title.is_empty() {
                            candidates.iter().find(|issue| {
                                normalize_reference_title(&issue.title) == reference.title
                            })
                        } else {
                            None
                        }
                    })
                };
                if let Some(target) = resolved {
                    if target.id() == issue_id {
                        problems.push(format!("{} has a self dependency", issue_id));
                        issues[index].blocked_by.push(DependencyRef::Unclear {
                            repository: Some(repository.clone()),
                            number: Some(target.number),
                        });
                    } else {
                        issues[index].blocked_by.push(DependencyRef::Known(
                            IssueRef::new(
                                target.repository.clone(),
                                target.number,
                                target.title.clone(),
                            )
                            .with_open(target.open),
                        ));
                    }
                } else {
                    if reference.number.is_none() && reference.file.is_none() {
                        problems.push(format!(
                            "{issue_id} has invalid dependency reference: {}",
                            reference.raw
                        ));
                    }
                    issues[index].blocked_by.push(DependencyRef::Unclear {
                        repository: Some(repository.clone()),
                        number: reference.number,
                    });
                }
            }
            let issue_number = issues[index].number;
            if let Some(parent) = issues[index].parent.as_mut() {
                if parent.number == issue_number {
                    problems.push(format!("{issue_id} has a self parent"));
                    issues[index].parent = None;
                    continue;
                }
                if let Some(candidates) = by_number.get(&parent.number) {
                    if candidates.len() == 1 {
                        parent.title = candidates[0].title.clone();
                        parent.open = Some(candidates[0].open);
                    } else {
                        problems.push(format!(
                            "{issue_id} has an ambiguous parent #{}",
                            parent.number
                        ));
                        issues[index].parent = None;
                    }
                } else {
                    problems.push(format!(
                        "{issue_id} references missing parent #{}",
                        parent.number
                    ));
                    issues[index].parent = None;
                }
            }
        }
        let parent_adjacency = issues
            .iter()
            .map(|issue| {
                (
                    issue.id(),
                    issue
                        .parent
                        .as_ref()
                        .map(IssueRef::id)
                        .into_iter()
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        if let Some(issue_id) = graph_cycle_node(&parent_adjacency) {
            problems.push(format!("parent cycle includes {issue_id}"));
        }
        let mut index_by_id = BTreeMap::new();
        for (index, issue) in issues.iter().enumerate() {
            index_by_id.insert(issue.id(), index);
        }
        let relations = issues
            .iter()
            .enumerate()
            .flat_map(|(child_index, issue)| {
                issue
                    .parent
                    .as_ref()
                    .map(|parent| (child_index, parent.id()))
            })
            .collect::<Vec<_>>();
        for (child_index, parent_id) in relations {
            if let Some(parent_index) = index_by_id.get(&parent_id).copied() {
                let child = IssueRef::new(
                    issues[child_index].repository.clone(),
                    issues[child_index].number,
                    issues[child_index].title.clone(),
                )
                .with_open(issues[child_index].open);
                if !issues[parent_index]
                    .children
                    .iter()
                    .any(|item| item.id() == child.id())
                {
                    issues[parent_index].children.push(child);
                }
            }
        }
        let dependency_relations = issues
            .iter()
            .enumerate()
            .flat_map(|(blocked_index, issue)| {
                issue
                    .blocked_by
                    .iter()
                    .filter_map(move |dependency| match dependency {
                        DependencyRef::Known(blocker) => Some((blocked_index, blocker.id())),
                        DependencyRef::Unclear { .. } => None,
                    })
            })
            .collect::<Vec<_>>();
        for (blocked_index, blocker_id) in dependency_relations {
            if let Some(blocker_index) = index_by_id.get(&blocker_id).copied() {
                let blocked = IssueRef::new(
                    issues[blocked_index].repository.clone(),
                    issues[blocked_index].number,
                    issues[blocked_index].title.clone(),
                )
                .with_open(issues[blocked_index].open);
                if !issues[blocker_index]
                    .blocking
                    .iter()
                    .any(|item| item.id() == blocked.id())
                {
                    issues[blocker_index].blocking.push(blocked);
                }
            }
        }
        let adjacency = issues
            .iter()
            .map(|issue| {
                (
                    issue.id(),
                    issue
                        .blocked_by
                        .iter()
                        .filter_map(|dependency| match dependency {
                            DependencyRef::Known(blocker) => Some(blocker.id()),
                            DependencyRef::Unclear { .. } => None,
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        if let Some(issue_id) = graph_cycle_node(&adjacency) {
            problems.push(format!("dependency cycle includes {issue_id}"));
        }
        issues.sort_by_key(|issue| issue.number);
        if problems.is_empty() {
            Ok(crate::tracker_seam::TrackerReadOutcome::Complete { issues })
        } else {
            Ok(crate::tracker_seam::TrackerReadOutcome::Incomplete {
                issues,
                detail: problems.join("; "),
            })
        }
    }

    fn read_issue_document(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueDocument, TrackerReadError> {
        let root = Self::root(ctx);
        let path = Self::locate_issue(&root, issue_id).map_err(|error| match error {
            TrackerWriteError::Failed { message } => TrackerReadError::Failed {
                detail: Some(message),
            },
            _ => TrackerReadError::Failed {
                detail: Some("unknown issue".into()),
            },
        })?;
        let (issue, body, _, errors) =
            Self::parse_file(&root, &path).map_err(|error| TrackerReadError::Failed {
                detail: Some(error),
            })?;
        if !errors.is_empty() {
            return Err(TrackerReadError::Failed {
                detail: Some(errors.join(", ")),
            });
        }
        Ok(IssueDocument { issue, body })
    }

    fn create_issue(
        &self,
        ctx: &ProbeContext<'_>,
        title: &str,
        body: &str,
    ) -> Result<IssueRecord, TrackerWriteError> {
        let root = Self::root(ctx);
        let files = Self::issue_files(&root);
        let issues_dir = files
            .first()
            .and_then(|path| path.parent())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| root.join(".scratch").join("taskboard").join("issues"));
        std::fs::create_dir_all(&issues_dir).map_err(|err| TrackerWriteError::Failed {
            message: err.to_string(),
        })?;
        let next = files
            .iter()
            .filter_map(|path| {
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .and_then(|stem| stem.split_once('-'))
                    .and_then(|(n, _)| n.parse::<u64>().ok())
            })
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let slug = local_slug(title);
        let path = issues_dir.join(format!("{next:02}-{slug}.md"));
        let document = format!(
            "# {next:02} — {}\n\nStatus: ready-for-agent\n\n{}\n",
            title.trim(),
            body.trim()
        );
        let (issue, _, _, errors) = Self::parse_document(&root, &path, document.clone())
            .map_err(|message| TrackerWriteError::Failed { message })?;
        if !errors.is_empty() {
            return Err(TrackerWriteError::Failed {
                message: errors.join(", "),
            });
        }
        Self::atomic_write(&path, &document)?;
        Ok(issue)
    }
    fn update_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        edit: IssueEdit<'_>,
    ) -> Result<IssueRecord, TrackerWriteError> {
        let root = Self::root(ctx);
        let path = Self::locate_issue(&root, issue_id)?;
        let original = std::fs::read_to_string(&path).map_err(|err| TrackerWriteError::Failed {
            message: err.to_string(),
        })?;
        let (current, _, _, current_errors) = Self::parse_file(&root, &path)
            .map_err(|message| TrackerWriteError::Failed { message })?;
        if !current_errors.is_empty() {
            return Err(TrackerWriteError::Failed {
                message: current_errors.join(", "),
            });
        }
        let title = edit.title.unwrap_or(&current.title).trim();
        if title.is_empty() {
            return Err(TrackerWriteError::Failed {
                message: "title cannot be empty".into(),
            });
        }
        let issue_number_text = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .and_then(|stem| stem.split_once('-'))
            .map(|(number, _)| number.to_string())
            .unwrap_or_else(|| current.number.to_string());
        let mut lines = original.lines().map(ToOwned::to_owned).collect::<Vec<_>>();
        if let Some(index) = lines
            .iter()
            .position(|line| line.trim_start().starts_with('#'))
        {
            lines[index] = format!("# {} — {}", issue_number_text, title);
        }
        if let Some(new_body) = edit.body {
            let metadata = lines
                .iter()
                .skip(1)
                .take_while(|line| !line.trim_start().starts_with("## "))
                .filter(|line| {
                    parse_field_line(line).is_some_and(|(name, _)| is_local_metadata_field(&name))
                })
                .cloned()
                .collect::<Vec<_>>();
            let comments = section_lines(&original, "Comments");
            let mut rebuilt = vec![
                format!("# {} — {}", issue_number_text, title),
                String::new(),
            ];
            rebuilt.extend(metadata);
            rebuilt.push(String::new());
            rebuilt.extend(new_body.lines().map(ToOwned::to_owned));
            if !comments.is_empty() {
                rebuilt.push(String::new());
                rebuilt.push("## Comments".into());
                rebuilt.extend(comments);
            }
            lines = rebuilt;
        }
        let contents = format!("{}\n", lines.join("\n"));
        let (issue, _, _, errors) = Self::parse_document(&root, &path, contents.clone())
            .map_err(|message| TrackerWriteError::Failed { message })?;
        if !errors.is_empty() {
            return Err(TrackerWriteError::Failed {
                message: errors.join(", "),
            });
        }
        Self::atomic_write(&path, &contents)?;
        Ok(issue)
    }
    fn close_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerWriteError> {
        Self::write_status(&Self::root(ctx), issue_id, "resolved", false, true)
    }
    fn reopen_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerWriteError> {
        let root = Self::root(ctx);
        Self::transition_open_status(&root, issue_id, false)
    }
    fn add_comment(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        body: &str,
    ) -> Result<IssueComment, TrackerWriteError> {
        let body = body.trim();
        if body.is_empty() {
            return Err(TrackerWriteError::Failed {
                message: "comment cannot be empty".into(),
            });
        }
        let root = Self::root(ctx);
        let path = Self::locate_issue(&root, issue_id)?;
        let original = std::fs::read_to_string(&path).map_err(|err| TrackerWriteError::Failed {
            message: err.to_string(),
        })?;
        let mut lines = original.lines().map(ToOwned::to_owned).collect::<Vec<_>>();
        let comment_id = section_lines(&original, "Comments")
            .iter()
            .filter(|line| !line.trim().is_empty())
            .count()
            + 1;
        if let Some(index) = lines
            .iter()
            .position(|line| line.trim().eq_ignore_ascii_case("## comments"))
        {
            let mut end = index + 1;
            while end < lines.len() && !lines[end].trim_start().starts_with("## ") {
                end += 1;
            }
            lines.splice(end..end, [format!("- {body}")]);
        } else {
            lines.extend([String::new(), "## Comments".into(), format!("- {body}")]);
        }
        Self::atomic_write(&path, &format!("{}\n", lines.join("\n")))?;
        Ok(IssueComment {
            id: comment_id.to_string(),
            url: format!("file://{}#comment-{comment_id}", path.display()),
            body: body.into(),
        })
    }
    fn claim_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerWriteError> {
        Self::write_status(&Self::root(ctx), issue_id, "claimed", false, false)
    }
    fn release_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerWriteError> {
        let root = Self::root(ctx);
        Self::transition_open_status(&root, issue_id, true)
    }
    fn set_parent(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        parent: Option<&str>,
    ) -> Result<(), TrackerWriteError> {
        let root = Self::root(ctx);
        Self::locate_issue(&root, issue_id)?;
        if let Some(parent) = parent {
            let parent_path = Self::locate_issue(&root, parent)?;
            if parent_path == Self::locate_issue(&root, issue_id)? {
                return Err(TrackerWriteError::Failed {
                    message: "issue cannot parent itself".into(),
                });
            }
        }
        self.validate_relation_update(
            ctx,
            issue_id,
            parent.into_iter().map(ToOwned::to_owned).collect(),
            "parent",
            |issue| {
                issue
                    .parent
                    .as_ref()
                    .map(IssueRef::id)
                    .into_iter()
                    .collect()
            },
        )?;
        Self::rewrite_parent(
            &root,
            issue_id,
            parent.map(|id| id.rsplit_once('#').map(|(_, number)| number).unwrap_or(id)),
        )
    }
    fn add_blocked_by(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        blocking_issue_id: &str,
    ) -> Result<(), TrackerWriteError> {
        Self::rewrite_blocked_by(&Self::root(ctx), issue_id, blocking_issue_id, true)
    }
    fn remove_blocked_by(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        blocking_issue_id: &str,
    ) -> Result<(), TrackerWriteError> {
        Self::rewrite_blocked_by(&Self::root(ctx), issue_id, blocking_issue_id, false)
    }
    fn set_blocked_by(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        _current_issue_ids: &[String],
        blocking_issue_ids: &[String],
    ) -> Result<(), TrackerWriteError> {
        let root = Self::root(ctx);
        Self::locate_issue(&root, issue_id)?;
        let mut wanted = Vec::new();
        for blocking_issue_id in blocking_issue_ids {
            Self::locate_issue(&root, blocking_issue_id)?;
            if blocking_issue_id == issue_id {
                return Err(TrackerWriteError::Failed {
                    message: "issue cannot block itself".into(),
                });
            }
            if !wanted.contains(blocking_issue_id) {
                wanted.push(blocking_issue_id.clone());
            }
        }
        self.validate_relation_update(ctx, issue_id, wanted, "dependency", |issue| {
            issue
                .blocked_by
                .iter()
                .filter_map(|dependency| match dependency {
                    DependencyRef::Known(blocker) => Some(blocker.id()),
                    DependencyRef::Unclear { .. } => None,
                })
                .collect()
        })?;
        Self::rewrite_blocked_by_set(&root, issue_id, blocking_issue_ids)
    }
}

impl MemoryTracker {
    pub fn new() -> Self {
        Self {
            failures: Mutex::new(BTreeSet::new()),
            read_scripts: Mutex::new(BTreeMap::new()),
            detail_read_scripts: Mutex::new(BTreeMap::new()),
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

    pub fn set_issue_body(&self, issue_id: impl Into<String>, body: impl Into<String>) {
        self.bodies
            .lock()
            .expect("memory tracker")
            .insert(issue_id.into(), body.into());
    }

    pub fn fail_issue_document_offline(&self, issue_id: impl Into<String>) {
        self.detail_read_scripts
            .lock()
            .expect("memory tracker")
            .insert(issue_id.into(), ReadScript::Offline);
    }

    pub fn fail_issue_document_rate_limited(
        &self,
        issue_id: impl Into<String>,
        retry_after_ms: Option<u64>,
    ) {
        self.detail_read_scripts
            .lock()
            .expect("memory tracker")
            .insert(issue_id.into(), ReadScript::RateLimited { retry_after_ms });
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

    fn read_issue_document(
        &self,
        _ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueDocument, TrackerReadError> {
        if let Some(script) = self
            .detail_read_scripts
            .lock()
            .expect("memory tracker")
            .get(issue_id)
            .cloned()
        {
            return Err(match script {
                ReadScript::Offline => TrackerReadError::Offline {
                    source: Some(self.source),
                    cli_detected: true,
                    detail: Some(format!("cannot read {issue_id}")),
                },
                ReadScript::Auth => TrackerReadError::Auth {
                    source: Some(self.source),
                    kind: AuthFailureKind::Rejected,
                    cli_detected: true,
                    detail: Some(format!("credentials rejected for {issue_id}")),
                },
                ReadScript::RateLimited { retry_after_ms } => {
                    TrackerReadError::RateLimited { retry_after_ms }
                }
            });
        }
        let issue = self
            .issues
            .lock()
            .expect("memory tracker")
            .values()
            .flat_map(|issues| issues.iter())
            .find(|issue| issue.id() == issue_id)
            .cloned()
            .ok_or_else(|| TrackerReadError::Failed {
                detail: Some("unknown issue".into()),
            })?;
        let body = self
            .bodies
            .lock()
            .expect("memory tracker")
            .get(issue_id)
            .cloned()
            .unwrap_or_default();
        Ok(IssueDocument { issue, body })
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
        Err(ureq::Error::Status(code, response)) => {
            return Err(classify_rest_status(code, response));
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
