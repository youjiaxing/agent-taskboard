mod common;
mod github;
mod local_markdown;
mod memory;

pub use crate::tracker_seam::TrackerPort;
pub use common::{
    AuthFailureKind, CredentialSource, IssueComment, IssueDocument, IssueEdit, ProbeContext,
    ProbeOutcome, ProjectConnection, RepairHint, TrackerKind, TrackerReadError, TrackerWriteError,
};
#[allow(unused_imports)]
pub use github::{
    gh_known_install_locations, map_github_issue_node, repair_hint, resolve_gh,
    resolve_github_token, GitHubTracker, ScriptedGitHub, GITHUB_APP_ENV, GITHUB_GENERIC_ENV,
    GITHUB_SCOPE,
};
pub(crate) use local_markdown::editable_body;
pub use local_markdown::LocalMarkdownTracker;
pub use memory::{MemoryTracker, MEMORY_TRACKER_ACTOR};

#[allow(unused_imports)]
pub(crate) use common::*;
