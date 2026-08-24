use crate::issue::{DependencyRef, IssueRecord, IssueRef};
use crate::tracker::{
    IssueEdit, ProbeContext, ProbeOutcome, TrackerPort, TrackerReadError, TrackerWriteError,
};

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
    fn write_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: Option<&str>,
        op: &TrackerWriteOp,
    ) -> Result<IssueRecord, TrackerWriteError>;
}

impl<T: TrackerPort> TrackerSeam for T {
    fn probe(&self, ctx: &ProbeContext<'_>) -> ProbeOutcome {
        TrackerPort::probe(self, ctx)
    }

    fn read_all(&self, ctx: &ProbeContext<'_>) -> Result<TrackerReadOutcome, TrackerReadError> {
        TrackerPort::read_all(self, ctx)
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
                for blocker in current_ids.iter().filter(|id| !wanted_ids.contains(id)) {
                    self.remove_blocked_by(ctx, issue_id, blocker)?;
                }
                for blocker in wanted_ids.iter().filter(|id| !current_ids.contains(id)) {
                    self.add_blocked_by(ctx, issue_id, blocker)?;
                }
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
