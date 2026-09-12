use super::*;
use crate::issue::{parse_issue_id, DependencyRef, IssueRecord, IssueRef};
use crate::tracker_seam::TrackerPort;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

fn github_web_issue_url(host: &str, repository: &str, number: u64) -> String {
    if host.eq_ignore_ascii_case("github.com") {
        format!("https://github.com/{repository}/issues/{number}")
    } else {
        format!("https://{host}/{repository}/issues/{number}")
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

#[derive(Clone)]
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
