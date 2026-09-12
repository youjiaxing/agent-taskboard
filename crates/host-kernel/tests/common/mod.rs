//! Host 接缝测试共享替身：按扩展后的 TrackerSeam 实现，供测试覆盖写操作与不完整读取。

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use host_kernel::{
    AuthFailureKind, CredentialSource, DependencyRef, IssueDocument, IssueRecord, IssueRef,
    ProbeContext, ProbeOutcome, TrackerReadError, TrackerReadOutcome, TrackerSeam,
    TrackerWriteError, TrackerWriteOp,
};

pub fn browser_e2e_guard() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub const BOARD_TEST_NOW_MS: u64 = 1_787_748_507_000;

pub fn boot_req(root: &Path) -> host_kernel::BootRequest {
    host_kernel::BootRequest {
        app_local_data_dir: root.to_path_buf(),
        app_log_dir: root.join("logs"),
        system_locale: "zh-Hans-CN".into(),
        system_appearance: host_kernel::SystemAppearance::Light,
        host_display_name: "Studio".into(),
    }
}

pub fn make_dir(root: &Path, name: &str) -> PathBuf {
    let dir = root.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

pub fn boot(root: &Path, tracker: Arc<host_kernel::MemoryTracker>) -> host_kernel::HostKernel {
    host_kernel::HostKernel::boot_with(boot_req(root), tracker).unwrap()
}

pub fn boot_local(root: &Path) -> host_kernel::HostKernel {
    let tracker = Arc::new(host_kernel::TrackerRouter::new(Arc::new(
        host_kernel::MemoryTracker::new(),
    )));
    host_kernel::HostKernel::boot_with(boot_req(root), tracker).unwrap()
}

pub fn boot_seam(root: &Path, tracker: Arc<SeamTracker>) -> host_kernel::HostKernel {
    host_kernel::HostKernel::boot_with(boot_req(root), tracker).unwrap()
}

pub fn boot_board(
    root: &Path,
    tracker: Arc<host_kernel::MemoryTracker>,
) -> host_kernel::HostKernel {
    let mut host = boot(root, tracker);
    pin_board_test_time(&mut host);
    host
}

pub fn boot_board_seam(root: &Path, tracker: Arc<SeamTracker>) -> host_kernel::HostKernel {
    let mut host = boot_seam(root, tracker);
    pin_board_test_time(&mut host);
    host
}

pub fn pin_board_test_time(host: &mut host_kernel::HostKernel) {
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": BOARD_TEST_NOW_MS,
    }))
    .unwrap();
}

pub fn register_project(
    host: &mut host_kernel::HostKernel,
    name: &str,
    dir: &Path,
    repository: &str,
) -> String {
    host.handle(serde_json::json!({
        "op": "registerProject",
        "name": name,
        "localPath": dir,
        "repository": repository,
    }))
    .unwrap()
    .snapshot
    .projects
    .iter()
    .find(|project| project.name == name)
    .unwrap()
    .id
    .clone()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadMode {
    Complete,
    Incomplete(String),
    Auth,
    Offline,
    RateLimited(Option<u64>),
    Failed(String),
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
    read_starts: Mutex<BTreeMap<String, u64>>,
    read_delay_ms: AtomicU64,
    read_document_delay_ms: AtomicU64,
    document_response_gate:
        Mutex<Option<(std::sync::mpsc::Sender<()>, std::sync::mpsc::Receiver<()>)>>,
    relation_reads: AtomicU64,
    write_delay_ms: AtomicU64,
    comments: Mutex<BTreeMap<String, Vec<String>>>,
    bodies: Mutex<BTreeMap<String, String>>,
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

    pub fn set_issue_body(&self, issue_id: &str, body: &str) {
        self.bodies
            .lock()
            .expect("seam tracker")
            .insert(issue_id.to_string(), body.to_string());
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

    pub fn read_start_count(&self, repository: &str) -> u64 {
        self.read_starts
            .lock()
            .expect("seam tracker")
            .get(repository)
            .copied()
            .unwrap_or(0)
    }

    pub fn set_read_delay_ms(&self, delay_ms: u64) {
        self.read_delay_ms.store(delay_ms, Ordering::Relaxed);
    }

    pub fn set_read_document_delay_ms(&self, delay_ms: u64) {
        self.read_document_delay_ms
            .store(delay_ms, Ordering::Relaxed);
    }

    pub fn set_write_delay_ms(&self, delay_ms: u64) {
        self.write_delay_ms.store(delay_ms, Ordering::Relaxed);
    }

    pub fn hold_next_document_response(
        &self,
    ) -> (std::sync::mpsc::Receiver<()>, std::sync::mpsc::Sender<()>) {
        let (captured_tx, captured_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        *self.document_response_gate.lock().unwrap() = Some((captured_tx, release_rx));
        (captured_rx, release_tx)
    }

    pub fn relation_read_count(&self) -> u64 {
        self.relation_reads.load(Ordering::Relaxed)
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
            .read_starts
            .lock()
            .expect("seam tracker")
            .entry(ctx.repository.to_string())
            .or_default() += 1;
        let delay_ms = self.read_delay_ms.load(Ordering::Relaxed);
        if delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(delay_ms));
        }
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
            Some(ReadMode::Failed(detail)) => Err(TrackerReadError::Failed {
                detail: Some(detail),
            }),
            Some(ReadMode::Incomplete(detail)) => Ok(TrackerReadOutcome::Incomplete {
                issues: issues(),
                detail,
            }),
            Some(ReadMode::Complete) | None => {
                Ok(TrackerReadOutcome::Complete { issues: issues() })
            }
        }
    }

    fn read_issue_document(
        &self,
        _ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueDocument, TrackerReadError> {
        let delay_ms = self.read_document_delay_ms.load(Ordering::Relaxed);
        if delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(delay_ms));
        }
        let issue = self
            .issues
            .lock()
            .expect("seam tracker")
            .values()
            .flat_map(|issues| issues.iter())
            .find(|issue| issue.id() == issue_id)
            .cloned()
            .ok_or_else(|| TrackerReadError::Failed {
                detail: Some("unknown issue".into()),
            })?;
        let document = IssueDocument {
            issue,
            body: self
                .bodies
                .lock()
                .expect("seam tracker")
                .get(issue_id)
                .cloned()
                .unwrap_or_default(),
        };
        let gate = self.document_response_gate.lock().unwrap().take();
        if let Some((captured, release)) = gate {
            captured.send(()).unwrap();
            release.recv_timeout(Duration::from_secs(10)).unwrap();
        }
        Ok(document)
    }

    fn read_issue_relations(
        &self,
        _ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerReadError> {
        self.relation_reads.fetch_add(1, Ordering::Relaxed);
        self.issues
            .lock()
            .expect("seam tracker")
            .values()
            .flat_map(|issues| issues.iter())
            .find(|issue| issue.id() == issue_id)
            .cloned()
            .ok_or_else(|| TrackerReadError::Failed {
                detail: Some("unknown issue".into()),
            })
    }

    fn write_issue(
        &self,
        ctx: &ProbeContext<'_>,
        _current_issue: Option<&IssueRecord>,
        issue_id: Option<&str>,
        op: &TrackerWriteOp,
    ) -> Result<IssueRecord, TrackerWriteError> {
        let delay_ms = self.write_delay_ms.load(Ordering::Relaxed);
        if delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(delay_ms));
        }
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

pub fn start_unbound_grok(
    host: &mut host_kernel::HostKernel,
    project_id: &str,
) -> host_kernel::CommandOutcome {
    host.handle(serde_json::json!({
        "op": "startUnboundRun",
        "projectId": project_id,
        "agentId": "grok-build",
        "values": {
            "model": "grok-4.6",
            "effort": "high",
            "permission-mode": "default",
            "always-approve": "false",
            "sandbox": "off",
            "initial-instruction": "",
            "additional-args": ""
        },
        "openingText": "project integration",
    }))
    .unwrap()
}

pub fn start_bound_grok(
    host: &mut host_kernel::HostKernel,
    project_id: &str,
    issue_id: &str,
) -> host_kernel::CommandOutcome {
    host.handle(serde_json::json!({
        "op": "startUnboundRun",
        "projectId": project_id,
        "issueId": issue_id,
        "agentId": "grok-build",
        "values": {
            "model": "grok-4.6",
            "effort": "high",
            "permission-mode": "default",
            "always-approve": "false",
            "sandbox": "off",
            "initial-instruction": "",
            "additional-args": ""
        },
        "openingText": "browser board integration",
    }))
    .unwrap()
}
