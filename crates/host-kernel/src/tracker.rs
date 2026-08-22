use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};

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

pub trait TrackerPort: Send + Sync {
    fn probe(&self, ctx: &ProbeContext<'_>) -> ProbeOutcome;
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

pub struct MemoryTracker {
    failures: Mutex<BTreeSet<String>>,
    source: CredentialSource,
}

impl MemoryTracker {
    pub fn new() -> Self {
        Self {
            failures: Mutex::new(BTreeSet::new()),
            source: CredentialSource::Cli,
        }
    }

    pub fn fail_repository(&self, repository: impl Into<String>) {
        self.failures
            .lock()
            .expect("memory tracker")
            .insert(repository.into());
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
}

enum ProbeError {
    Unauthorized,
    Unreachable(String),
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
            }),
        }
    }
}

impl TrackerPort for GitHubTracker {
    fn probe(&self, ctx: &ProbeContext<'_>) -> ProbeOutcome {
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
        let Some((token, source)) = resolved else {
            return ProbeOutcome::Failed {
                source: None,
                kind: AuthFailureKind::MissingCredentials,
                cli_detected,
                detail: None,
            };
        };
        match self.api.probe_repo(ctx.github_host, ctx.repository, &token) {
            Ok(()) => ProbeOutcome::Ready { source },
            Err(ProbeError::Unauthorized) => ProbeOutcome::Failed {
                source: Some(source),
                kind: AuthFailureKind::Rejected,
                cli_detected,
                detail: None,
            },
            Err(ProbeError::Unreachable(detail)) => ProbeOutcome::Failed {
                source: Some(source),
                kind: AuthFailureKind::Unreachable,
                cli_detected,
                detail: Some(detail),
            },
        }
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
            Err(ureq::Error::Status(code, _)) => {
                Err(ProbeError::Unreachable(format!("GitHub HTTP {code}")))
            }
            Err(err) => Err(ProbeError::Unreachable(err.to_string())),
        }
    }
}

fn github_repo_url(host: &str, repository: &str) -> String {
    if host.eq_ignore_ascii_case("github.com") {
        format!("https://api.github.com/repos/{repository}")
    } else {
        format!("https://{host}/api/v3/repos/{repository}")
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
}

impl GitHubApi for MapApi {
    fn probe_repo(&self, _host: &str, _repository: &str, token: &str) -> Result<(), ProbeError> {
        if self.unreachable {
            return Err(ProbeError::Unreachable("unreachable".into()));
        }
        if self.accept.contains(token) {
            Ok(())
        } else {
            Err(ProbeError::Unauthorized)
        }
    }
}
