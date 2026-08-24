//! Host 接缝测试共享替身：按扩展后的 TrackerSeam 实现，供测试覆盖写操作与不完整读取。

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::sync::Mutex;

use host_kernel::{
    AuthFailureKind, CredentialSource, DependencyRef, IssueRecord, IssueRef, ProbeContext,
    ProbeOutcome, TrackerReadError, TrackerReadOutcome, TrackerSeam, TrackerWriteError,
    TrackerWriteOp,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadMode {
    Complete,
    Incomplete(String),
    Auth,
    Offline,
    RateLimited(Option<u64>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteMode {
    Ok,
    Fail(String),
    Auth,
    Offline,
    RateLimited(Option<u64>),
}

#[derive(Default)]
pub struct SeamTracker {
    issues: Mutex<BTreeMap<String, Vec<IssueRecord>>>,
    read_modes: Mutex<BTreeMap<String, ReadMode>>,
    write_modes: Mutex<BTreeMap<String, WriteMode>>,
    reads: Mutex<BTreeMap<String, u64>>,
    comments: Mutex<BTreeMap<String, Vec<String>>>,
    write_log: Mutex<Vec<(String, Option<String>, TrackerWriteOp)>>,
}

impl SeamTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_issues(&self, repository: &str, issues: Vec<IssueRecord>) {
        self.issues
            .lock()
            .expect("seam tracker")
            .insert(repository.to_string(), issues);
    }

    pub fn add_issue(&self, issue: IssueRecord) {
        self.issues
            .lock()
            .expect("seam tracker")
            .entry(issue.repository.clone())
            .or_default()
            .push(issue);
    }

    pub fn set_read_mode(&self, repository: &str, mode: ReadMode) {
        self.read_modes
            .lock()
            .expect("seam tracker")
            .insert(repository.to_string(), mode);
    }

    pub fn set_write_mode(&self, repository: &str, mode: WriteMode) {
        self.write_modes
            .lock()
            .expect("seam tracker")
            .insert(repository.to_string(), mode);
    }

    pub fn read_count(&self, repository: &str) -> u64 {
        self.reads
            .lock()
            .expect("seam tracker")
            .get(repository)
            .copied()
            .unwrap_or(0)
    }

    pub fn comments(&self, repository: &str) -> Vec<String> {
        self.comments
            .lock()
            .expect("seam tracker")
            .get(repository)
            .cloned()
            .unwrap_or_default()
    }

    pub fn log(&self) -> Vec<(String, Option<String>, TrackerWriteOp)> {
        self.write_log.lock().expect("seam tracker").clone()
    }

    fn apply_write(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: Option<&str>,
        op: &TrackerWriteOp,
    ) -> Result<IssueRecord, TrackerWriteError> {
        let repository = ctx.repository.to_string();
        let mut issues = self.issues.lock().expect("seam tracker");
        if let TrackerWriteOp::CreateIssue { title, .. } = op {
            let list = issues.entry(repository.clone()).or_default();
            let number = list.iter().map(|issue| issue.number).max().unwrap_or(0) + 1;
            let issue = IssueRecord::open(repository, number, title);
            list.push(issue.clone());
            return Ok(issue);
        }
        let (repository, number) = issue_id
            .and_then(|id| id.rsplit_once('#'))
            .and_then(|(repository, number)| {
                number
                    .parse::<u64>()
                    .ok()
                    .map(|number| (repository, number))
            })
            .ok_or_else(|| TrackerWriteError::Failed {
                message: "unknown issue".into(),
            })?;
        if let TrackerWriteOp::SetBlockedBy { blocked_by } = op {
            // 先按仓库内已知状态解析阻塞边的 open，再整体替换
            let list_ref = issues.get(repository).map(Vec::as_slice).unwrap_or(&[]);
            let resolved: Vec<IssueRef> = blocked_by
                .iter()
                .map(|item| {
                    let open = list_ref
                        .iter()
                        .find(|known| {
                            known.repository == item.repository && known.number == item.number
                        })
                        .map(|known| known.open);
                    IssueRef {
                        repository: item.repository.clone(),
                        number: item.number,
                        title: item.title.clone(),
                        open,
                        url: item.url.clone(),
                    }
                })
                .collect();
            let list = issues
                .get_mut(repository)
                .ok_or_else(|| TrackerWriteError::Failed {
                    message: "unknown issue".into(),
                })?;
            let issue = list
                .iter_mut()
                .find(|issue| issue.number == number)
                .ok_or_else(|| TrackerWriteError::Failed {
                    message: "unknown issue".into(),
                })?;
            issue.blocked_by = resolved.into_iter().map(DependencyRef::Known).collect();
            return Ok(issue.clone());
        }
        let list = issues
            .get_mut(repository)
            .ok_or_else(|| TrackerWriteError::Failed {
                message: "unknown issue".into(),
            })?;
        let issue = list
            .iter_mut()
            .find(|issue| issue.number == number)
            .ok_or_else(|| TrackerWriteError::Failed {
                message: "unknown issue".into(),
            })?;
        match op {
            TrackerWriteOp::UpdateIssue { title, .. } => {
                issue.title = title.clone();
            }
            TrackerWriteOp::SetOpen { open } => {
                issue.open = *open;
                if !*open {
                    issue
                        .closed_at
                        .get_or_insert_with(|| "2026-08-24T00:00:00Z".into());
                } else {
                    issue.closed_at = None;
                }
            }
            TrackerWriteOp::AddComment { body } => {
                self.comments
                    .lock()
                    .expect("seam tracker")
                    .entry(ctx.repository.to_string())
                    .or_default()
                    .push(body.clone());
            }
            TrackerWriteOp::Claim => {
                if !issue.assignees.iter().any(|login| login == "me") {
                    issue.assignees.push("me".into());
                }
            }
            TrackerWriteOp::Release => {
                issue.assignees.retain(|login| login != "me");
            }
            TrackerWriteOp::SetParent { parent } => {
                issue.parent = parent.as_ref().map(|item| IssueRef {
                    repository: item.repository.clone(),
                    number: item.number,
                    title: item.title.clone(),
                    open: None,
                    url: item.url.clone(),
                });
            }
            TrackerWriteOp::SetBlockedBy { .. } | TrackerWriteOp::CreateIssue { .. } => {
                unreachable!()
            }
        }
        Ok(issue.clone())
    }
}

impl TrackerSeam for SeamTracker {
    fn probe(&self, _ctx: &ProbeContext<'_>) -> ProbeOutcome {
        ProbeOutcome::Ready {
            source: CredentialSource::Cli,
        }
    }

    fn read_all(&self, ctx: &ProbeContext<'_>) -> Result<TrackerReadOutcome, TrackerReadError> {
        *self
            .reads
            .lock()
            .expect("seam tracker")
            .entry(ctx.repository.to_string())
            .or_default() += 1;
        let issues = || {
            self.issues
                .lock()
                .expect("seam tracker")
                .get(ctx.repository)
                .cloned()
                .unwrap_or_default()
        };
        match self
            .read_modes
            .lock()
            .expect("seam tracker")
            .get(ctx.repository)
            .cloned()
        {
            Some(ReadMode::Auth) => Err(TrackerReadError::Auth {
                source: Some(CredentialSource::Cli),
                kind: AuthFailureKind::Rejected,
                cli_detected: true,
                detail: Some("token rejected".into()),
            }),
            Some(ReadMode::Offline) => Err(TrackerReadError::Offline {
                source: Some(CredentialSource::Cli),
                cli_detected: true,
                detail: Some("network down".into()),
            }),
            Some(ReadMode::RateLimited(retry_after_ms)) => {
                Err(TrackerReadError::RateLimited { retry_after_ms })
            }
            Some(ReadMode::Incomplete(detail)) => Ok(TrackerReadOutcome::Incomplete {
                issues: issues(),
                detail,
            }),
            Some(ReadMode::Complete) | None => {
                Ok(TrackerReadOutcome::Complete { issues: issues() })
            }
        }
    }

    fn write_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: Option<&str>,
        op: &TrackerWriteOp,
    ) -> Result<IssueRecord, TrackerWriteError> {
        self.write_log.lock().expect("seam tracker").push((
            ctx.repository.to_string(),
            issue_id.map(ToOwned::to_owned),
            op.clone(),
        ));
        match self
            .write_modes
            .lock()
            .expect("seam tracker")
            .get(ctx.repository)
            .cloned()
        {
            Some(WriteMode::Auth) => {
                return Err(TrackerWriteError::Auth {
                    source: Some(CredentialSource::Cli),
                    kind: AuthFailureKind::Rejected,
                    cli_detected: true,
                    detail: Some("token rejected".into()),
                });
            }
            Some(WriteMode::Offline) => {
                return Err(TrackerWriteError::Offline {
                    source: Some(CredentialSource::Cli),
                    cli_detected: true,
                    detail: Some("network down".into()),
                });
            }
            Some(WriteMode::RateLimited(retry_after_ms)) => {
                return Err(TrackerWriteError::RateLimited { retry_after_ms });
            }
            Some(WriteMode::Fail(message)) => {
                return Err(TrackerWriteError::Failed { message });
            }
            Some(WriteMode::Ok) | None => {}
        }
        self.apply_write(ctx, issue_id, op)
    }
}
