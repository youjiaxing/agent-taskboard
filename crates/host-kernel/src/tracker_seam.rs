use crate::issue::{DependencyRef, IssueRecord, IssueRef};
use crate::tracker::{
    IssueDocument, IssueEdit, LocalMarkdownTracker, ProbeContext, ProbeOutcome, TrackerPort,
    TrackerReadError, TrackerWriteError,
};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackerWriteOp {
    CreateIssue { title: String, body: String },
    UpdateIssue { title: String, body: String },
    SetOpen { open: bool },
    AddComment { body: String },
    Claim,
    Release,
    SetParent { parent: Option<IssueRef> },
    SetBlockedBy { blocked_by: Vec<IssueRef> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackerReadOutcome {
    Complete {
        issues: Vec<IssueRecord>,
    },
    Incomplete {
        issues: Vec<IssueRecord>,
        detail: String,
    },
}

pub trait TrackerSeam: Send + Sync {
    fn probe(&self, ctx: &ProbeContext<'_>) -> ProbeOutcome;
    fn read_all(&self, ctx: &ProbeContext<'_>) -> Result<TrackerReadOutcome, TrackerReadError>;
    fn read_issue_document(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueDocument, TrackerReadError>;
    fn write_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: Option<&str>,
        op: &TrackerWriteOp,
    ) -> Result<IssueRecord, TrackerWriteError>;
}

/// Routes project operations to the configured tracker without leaking
/// tracker-specific branches into Host refresh and write paths.
pub struct TrackerRouter {
    github: Arc<dyn TrackerSeam>,
    local: LocalMarkdownTracker,
}

impl TrackerRouter {
    pub fn new(github: Arc<dyn TrackerSeam>) -> Self {
        Self {
            github,
            local: LocalMarkdownTracker,
        }
    }

    fn local(ctx: &ProbeContext<'_>) -> bool {
        ctx.github_host == "local"
    }
}

impl TrackerSeam for TrackerRouter {
    fn probe(&self, ctx: &ProbeContext<'_>) -> ProbeOutcome {
        if Self::local(ctx) {
            TrackerPort::probe(&self.local, ctx)
        } else {
            self.github.probe(ctx)
        }
    }

    fn read_all(&self, ctx: &ProbeContext<'_>) -> Result<TrackerReadOutcome, TrackerReadError> {
        if Self::local(ctx) {
            TrackerPort::read_all(&self.local, ctx)
        } else {
            self.github.read_all(ctx)
        }
    }

    fn read_issue_document(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueDocument, TrackerReadError> {
        if Self::local(ctx) {
            TrackerPort::read_issue_document(&self.local, ctx, issue_id)
        } else {
            self.github.read_issue_document(ctx, issue_id)
        }
    }

    fn write_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: Option<&str>,
        op: &TrackerWriteOp,
    ) -> Result<IssueRecord, TrackerWriteError> {
        if Self::local(ctx) {
            <LocalMarkdownTracker as TrackerSeam>::write_issue(&self.local, ctx, issue_id, op)
        } else {
            self.github.write_issue(ctx, issue_id, op)
        }
    }
}

impl<T: TrackerPort> TrackerSeam for T {
    fn probe(&self, ctx: &ProbeContext<'_>) -> ProbeOutcome {
        TrackerPort::probe(self, ctx)
    }

    fn read_all(&self, ctx: &ProbeContext<'_>) -> Result<TrackerReadOutcome, TrackerReadError> {
        TrackerPort::read_all(self, ctx)
    }

    fn read_issue_document(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueDocument, TrackerReadError> {
        TrackerPort::read_issue_document(self, ctx, issue_id)
    }

    fn write_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: Option<&str>,
        op: &TrackerWriteOp,
    ) -> Result<IssueRecord, TrackerWriteError> {
        match op {
            TrackerWriteOp::CreateIssue { title, body } => self.create_issue(ctx, title, body),
            TrackerWriteOp::UpdateIssue { title, body } => self.update_issue(
                ctx,
                required_issue_id(issue_id)?,
                IssueEdit {
                    title: Some(title),
                    body: Some(body),
                },
            ),
            TrackerWriteOp::SetOpen { open: true } => {
                self.reopen_issue(ctx, required_issue_id(issue_id)?)
            }
            TrackerWriteOp::SetOpen { open: false } => {
                TrackerPort::close_issue(self, ctx, required_issue_id(issue_id)?)
            }
            TrackerWriteOp::AddComment { body } => {
                let issue_id = required_issue_id(issue_id)?;
                self.add_comment(ctx, issue_id, body)?;
                read_issue(self, ctx, issue_id)
            }
            TrackerWriteOp::Claim => self.claim_issue(ctx, required_issue_id(issue_id)?),
            TrackerWriteOp::Release => self.release_issue(ctx, required_issue_id(issue_id)?),
            TrackerWriteOp::SetParent { parent } => {
                let issue_id = required_issue_id(issue_id)?;
                self.set_parent(ctx, issue_id, parent.as_ref().map(IssueRef::id).as_deref())?;
                read_issue(self, ctx, issue_id)
            }
            TrackerWriteOp::SetBlockedBy { blocked_by } => {
                let issue_id = required_issue_id(issue_id)?;
                let current = read_issue(self, ctx, issue_id)?;
                let current_ids: Vec<String> = current
                    .blocked_by
                    .iter()
                    .filter_map(|item| match item {
                        DependencyRef::Known(issue) => Some(issue.id()),
                        DependencyRef::Unclear { .. } => None,
                    })
                    .collect();
                let wanted_ids: Vec<String> = blocked_by.iter().map(IssueRef::id).collect();
                self.set_blocked_by(ctx, issue_id, &current_ids, &wanted_ids)?;
                read_issue(self, ctx, issue_id)
            }
        }
    }
}

fn required_issue_id(issue_id: Option<&str>) -> Result<&str, TrackerWriteError> {
    issue_id.ok_or_else(|| TrackerWriteError::Failed {
        message: "issue id is required for this operation".into(),
    })
}

fn read_issue<T: TrackerPort + ?Sized>(
    tracker: &T,
    ctx: &ProbeContext<'_>,
    issue_id: &str,
) -> Result<IssueRecord, TrackerWriteError> {
    tracker
        .read_issues(ctx)
        .map_err(read_as_write_error)?
        .into_iter()
        .find(|issue| issue.id() == issue_id)
        .ok_or_else(|| TrackerWriteError::Failed {
            message: "tracker did not return the updated issue".into(),
        })
}

fn read_as_write_error(error: TrackerReadError) -> TrackerWriteError {
    match error {
        TrackerReadError::Auth {
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
        TrackerReadError::Offline {
            source,
            cli_detected,
            detail,
        } => TrackerWriteError::Offline {
            source,
            cli_detected,
            detail,
        },
        TrackerReadError::RateLimited { retry_after_ms } => {
            TrackerWriteError::RateLimited { retry_after_ms }
        }
        TrackerReadError::Failed { detail } => TrackerWriteError::Failed {
            message: detail.unwrap_or_else(|| "tracker business error".into()),
        },
    }
}
