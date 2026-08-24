//! Host 接缝：Tracker 扩展能力的最小公共类型。
//!
//! Issue #87 会把 `TrackerPort` 扩展为 create/update/set_open/add_comment/claim/release/
//! parent/dependency，并把读取结果改为能表达 complete/incomplete 及详情。
//! 在 `tracker.rs` 完成扩展前，这里先定义 Host 内核消费的最小公共类型与接缝 trait，
//! 主代理随后统一冲突（届时可直接把这些方法并入 `TrackerPort` 的扩展）。

use crate::issue::{IssueRecord, IssueRef};
use crate::tracker::{
    ProbeContext, ProbeOutcome, TrackerPort, TrackerReadError, TrackerWriteError,
};

/// 一次 Issue 写操作，与 TrackerPort 即将扩展的方法一一对应。
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

/// 读取结果：必须能表达完整 / 不完整（分页截断等），不完整时保留可读详情。
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

/// Host 消费的 Tracker 接缝。
///
/// 现在的 `TrackerPort` 只有 probe/read/claim/release，`TrackerSeam` 把读取改为可表达
/// 完整性的 `read_all`，并补齐全部写操作；`TrackerPort` 的实现通过下方默认实现兼容，
/// 未实现的写操作明确报错而不是静默成功。
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
        TrackerPort::read_issues(self, ctx).map(|issues| TrackerReadOutcome::Complete { issues })
    }

    fn write_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: Option<&str>,
        op: &TrackerWriteOp,
    ) -> Result<IssueRecord, TrackerWriteError> {
        let issue_id = issue_id.ok_or_else(|| TrackerWriteError::Failed {
            message: "issue id is required for this operation".into(),
        })?;
        match op {
            TrackerWriteOp::Claim => self.claim_issue(ctx, issue_id),
            TrackerWriteOp::Release => self.release_issue(ctx, issue_id),
            other => Err(TrackerWriteError::Failed {
                message: format!("tracker does not support {other:?} yet"),
            }),
        }
    }
}
