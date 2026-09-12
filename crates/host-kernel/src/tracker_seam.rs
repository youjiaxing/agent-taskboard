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
    fn read_issue_content(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueDocument, TrackerReadError> {
        self.read_issue_document(ctx, issue_id)
    }
    fn read_issue_relations(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerReadError> {
        self.read_issue_document(ctx, issue_id)
            .map(|document| document.issue)
    }
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
use crate::issue::{DependencyRef, IssueRecord, IssueRef};
use crate::tracker::{
    IssueComment, IssueDocument, IssueEdit, LocalMarkdownTracker, ProbeContext, ProbeOutcome,
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
    fn read_issue_content(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueDocument, TrackerReadError> {
        self.read_issue_document(ctx, issue_id)
    }
    fn read_issue_relations(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerReadError>;
    fn write_issue(
        &self,
        ctx: &ProbeContext<'_>,
        current_issue: Option<&IssueRecord>,
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
        ctx.tracker == crate::tracker::TrackerKind::LocalMarkdown
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

    fn read_issue_content(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueDocument, TrackerReadError> {
        if Self::local(ctx) {
            TrackerPort::read_issue_content(&self.local, ctx, issue_id)
        } else {
            self.github.read_issue_content(ctx, issue_id)
        }
    }

    fn read_issue_relations(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerReadError> {
        if Self::local(ctx) {
            TrackerPort::read_issue_relations(&self.local, ctx, issue_id)
        } else {
            self.github.read_issue_relations(ctx, issue_id)
        }
    }

    fn write_issue(
        &self,
        ctx: &ProbeContext<'_>,
        current_issue: Option<&IssueRecord>,
        issue_id: Option<&str>,
        op: &TrackerWriteOp,
    ) -> Result<IssueRecord, TrackerWriteError> {
        if Self::local(ctx) {
            <LocalMarkdownTracker as TrackerSeam>::write_issue(
                &self.local,
                ctx,
                current_issue,
                issue_id,
                op,
            )
        } else {
            self.github.write_issue(ctx, current_issue, issue_id, op)
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

    fn read_issue_content(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueDocument, TrackerReadError> {
        TrackerPort::read_issue_content(self, ctx, issue_id)
    }

    fn read_issue_relations(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerReadError> {
        TrackerPort::read_issue_relations(self, ctx, issue_id)
    }

    fn write_issue(
        &self,
        ctx: &ProbeContext<'_>,
        current_issue: Option<&IssueRecord>,
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
                current_issue
                    .cloned()
                    .ok_or_else(|| TrackerWriteError::Failed {
                        message: "current issue is required after adding a comment".into(),
                    })
            }
            TrackerWriteOp::Claim => self.claim_issue(ctx, required_issue_id(issue_id)?),
            TrackerWriteOp::Release => self.release_issue(ctx, required_issue_id(issue_id)?),
            TrackerWriteOp::SetParent { parent } => {
                let issue_id = required_issue_id(issue_id)?;
                self.set_parent(ctx, issue_id, parent.as_ref().map(IssueRef::id).as_deref())?;
                let mut updated =
                    current_issue
                        .cloned()
                        .ok_or_else(|| TrackerWriteError::Failed {
                            message: "current issue is required after changing its parent".into(),
                        })?;
                updated.parent = parent.clone();
                Ok(updated)
            }
            TrackerWriteOp::SetBlockedBy { blocked_by } => {
                let issue_id = required_issue_id(issue_id)?;
                let current = current_issue.ok_or_else(|| TrackerWriteError::Failed {
                    message: "current issue is required before changing dependencies".into(),
                })?;
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
                let mut updated = current.clone();
                updated.blocked_by = blocked_by
                    .iter()
                    .cloned()
                    .map(DependencyRef::Known)
                    .collect();
                Ok(updated)
            }
        }
    }
}

fn required_issue_id(issue_id: Option<&str>) -> Result<&str, TrackerWriteError> {
    issue_id.ok_or_else(|| TrackerWriteError::Failed {
        message: "issue id is required for this operation".into(),
    })
}
