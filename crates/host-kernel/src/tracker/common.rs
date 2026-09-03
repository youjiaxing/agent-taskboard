use crate::issue::IssueRecord;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
