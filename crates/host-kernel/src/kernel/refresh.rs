//! Tracker refresh, cached board data, and live-read gates.

use super::super::*;

#[derive(Debug, Clone)]
pub(crate) struct ProjectRefreshState {
    pub(crate) fetched_at_ms: Option<u64>,
    pub(crate) last_attempt_ms: u64,
    pub(crate) kind: StoredRefreshKind,
    pub(crate) retry_at_ms: Option<u64>,
    /// 最近一次成功读取是否完整；不完整时不能当作全量数据计算 Frontier/依赖图。
    pub(crate) complete: bool,
    /// 不完整读取的可读详情。
    pub(crate) detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StoredRefreshKind {
    Ready,
    Offline,
    NeverFetched,
    RateLimited,
    AuthFailed,
    Incomplete,
    TrackerError,
}

pub(crate) struct PreparedRefresh {
    tracker: Arc<dyn TrackerSeam>,
    project_id: String,
    github_host: String,
    repository: String,
    tracker_kind: TrackerKind,
    secrets_pat: Option<String>,
    secrets_path: PathBuf,
    previous: Option<kernel::refresh::ProjectRefreshState>,
    issues_at_prepare: Vec<IssueRecord>,
    local_revision_at_prepare: Option<u64>,
    now_ms: u64,
    generation: u64,
    issue_write_generation: u64,
}

pub(crate) struct CompletedRefresh {
    prepared: PreparedRefresh,
    result: Result<tracker_seam::TrackerReadOutcome, tracker::TrackerReadError>,
}

fn preserve_written_issues(
    issues_at_prepare: &[IssueRecord],
    current_issues: &[IssueRecord],
    mut refreshed_issues: Vec<IssueRecord>,
) -> Vec<IssueRecord> {
    for current in current_issues {
        let changed_after_prepare = issues_at_prepare
            .iter()
            .find(|issue| issue.id() == current.id())
            != Some(current);
        if !changed_after_prepare {
            continue;
        }
        if let Some(refreshed) = refreshed_issues
            .iter_mut()
            .find(|issue| issue.id() == current.id())
        {
            *refreshed = current.clone();
        } else {
            refreshed_issues.push(current.clone());
        }
    }
    refreshed_issues
}

impl HostKernel {
    pub(crate) fn record_local_tracker_revision(&mut self, prepared: &PreparedRefresh) {
        if prepared.tracker_kind == TrackerKind::LocalMarkdown {
            if let Some(revision) = prepared.local_revision_at_prepare {
                self.local_tracker_revisions
                    .insert(prepared.project_id.clone(), revision);
            }
        }
    }

    pub(crate) fn refresh_project(&mut self, project_id: &str, trigger: RefreshTrigger) -> bool {
        let Some(prepared) = self.prepare_refresh(project_id, trigger) else {
            return false;
        };
        if self.defer_refreshes && trigger != RefreshTrigger::Action {
            self.deferred_refreshes.push(prepared);
            return true;
        }
        let completed = Self::execute_prepared_refresh(prepared);
        self.finish_prepared_refresh(completed)
    }

    pub(crate) fn prepare_refresh(
        &mut self,
        project_id: &str,
        trigger: RefreshTrigger,
    ) -> Option<PreparedRefresh> {
        if !self.should_attempt_refresh(project_id, trigger) {
            return None;
        }
        let index = self
            .projects
            .iter()
            .position(|project| project.id == project_id)?;
        let previous = self.refresh.get(project_id).cloned();
        let previous_fetched = previous.as_ref().and_then(|state| state.fetched_at_ms);
        self.next_refresh_generation = self.next_refresh_generation.saturating_add(1);
        let generation = self.next_refresh_generation;
        self.refresh_in_flight
            .insert(project_id.to_string(), generation);
        self.pending_events.push(HostEvent::RefreshStatusChanged {
            project_id: project_id.to_string(),
            status: RefreshStatus::Refreshing {
                fetched_at_ms: previous_fetched,
            },
        });
        let github_host = self.projects[index].github_host.clone();
        let repository = self.projects[index].repository.clone();
        let tracker_kind = self.projects[index].tracker;
        let local_revision_at_prepare = (tracker_kind == TrackerKind::LocalMarkdown)
            .then(|| LocalMarkdownTracker::content_revision(Path::new(&repository)).ok())
            .flatten();
        let local_documents_may_be_stale = tracker_kind == TrackerKind::LocalMarkdown
            && match local_revision_at_prepare {
                Some(revision) => {
                    self.local_tracker_revisions.get(project_id).copied() != Some(revision)
                }
                None => self.issue_documents.contains_key(project_id),
            };
        if local_documents_may_be_stale {
            self.invalidate_issue_documents(project_id);
        }
        Some(PreparedRefresh {
            tracker: Arc::clone(&self.tracker),
            project_id: project_id.to_string(),
            github_host: github_host.clone(),
            repository,
            tracker_kind,
            secrets_pat: read_github_pat(&self.data.host_secrets_path, &github_host),
            secrets_path: self.data.host_secrets_path.clone(),
            previous,
            issues_at_prepare: self
                .loaded_issues
                .get(project_id)
                .cloned()
                .unwrap_or_default(),
            local_revision_at_prepare,
            now_ms: self.now_ms,
            generation,
            issue_write_generation: self
                .issue_write_generations
                .get(project_id)
                .copied()
                .unwrap_or(0),
        })
    }

    pub(crate) fn apply_read(
        &mut self,
        project_id: &str,
        index: usize,
        github_host: &str,
        repository: &str,
        now: u64,
        issues: Vec<IssueRecord>,
        complete: bool,
        detail: Option<String>,
    ) -> bool {
        let accepted_issues = if complete {
            issues
        } else {
            let mut merged = self
                .loaded_issues
                .get(project_id)
                .cloned()
                .unwrap_or_default();
            for issue in issues {
                if let Some(existing) = merged
                    .iter_mut()
                    .find(|existing| existing.id() == issue.id())
                {
                    *existing = issue;
                } else {
                    merged.push(issue);
                }
            }
            merged
        };
        let connection_changed = !matches!(
            self.projects[index].connection,
            ProjectConnection::Ready { .. }
        );
        let content_changed = connection_changed
            || self.loaded_issues.get(project_id) != Some(&accepted_issues)
            || self.refresh.get(project_id).map(|state| state.complete) != Some(complete);
        if connection_changed {
            self.projects[index].connection =
                self.probe_tracker(self.projects[index].tracker, github_host, repository);
        }
        let snapshot = refresh::StoredTrackerSnapshot {
            fetched_at_ms: now,
            complete,
            detail: detail.clone(),
            issues: accepted_issues.clone(),
            documents: self.stored_issue_documents(project_id),
        };
        if let Err(err) = refresh::save_snapshot(
            &refresh::snapshot_path(&self.data.host_dir, project_id),
            &snapshot,
        ) {
            self.projects[index].tracker_synced = false;
            self.loaded_issues
                .insert(project_id.to_string(), accepted_issues);
            self.refresh.insert(
                project_id.to_string(),
                ProjectRefreshState {
                    fetched_at_ms: Some(now),
                    last_attempt_ms: now,
                    kind: StoredRefreshKind::TrackerError,
                    retry_at_ms: None,
                    complete: false,
                    detail: Some(format!("tracker snapshot could not be persisted: {err}")),
                },
            );
            return true;
        }
        self.projects[index].tracker_synced = complete;
        self.loaded_issues
            .insert(project_id.to_string(), accepted_issues);
        self.refresh.insert(
            project_id.to_string(),
            ProjectRefreshState {
                fetched_at_ms: Some(now),
                last_attempt_ms: now,
                kind: if complete {
                    StoredRefreshKind::Ready
                } else {
                    StoredRefreshKind::Incomplete
                },
                retry_at_ms: None,
                complete,
                detail,
            },
        );
        content_changed
    }

    pub(crate) fn maybe_auto_refresh(&mut self) {
        let due: Vec<String> = self
            .projects
            .iter()
            .map(|project| project.id.clone())
            .filter(|id| self.should_auto_refresh(id))
            .collect();
        for project_id in due {
            self.refresh_project(&project_id, RefreshTrigger::Interval);
        }
    }

    pub(crate) fn maybe_refresh_local_markdown(&mut self) {
        let changed: Vec<String> = self
            .projects
            .iter()
            .filter(|project| project.tracker == TrackerKind::LocalMarkdown)
            .filter_map(|project| {
                let revision =
                    LocalMarkdownTracker::content_revision(Path::new(&project.repository)).ok();
                let changed = revision.is_none()
                    || self.local_tracker_revisions.get(&project.id).copied() != revision;
                changed.then(|| project.id.clone())
            })
            .collect();
        for project_id in changed {
            self.refresh_project(&project_id, RefreshTrigger::Immediate);
        }
    }

    pub(crate) fn invalidate_issue_documents(&mut self, project_id: &str) {
        self.issue_documents.remove(project_id);
        self.issue_document_in_flight
            .retain(|(pending_project, _), _| pending_project != project_id);
    }

    pub(crate) fn should_attempt_refresh(&self, project_id: &str, trigger: RefreshTrigger) -> bool {
        if self.refresh_in_flight.contains_key(project_id) && trigger != RefreshTrigger::Action {
            return false;
        }
        let Some(state) = self.refresh.get(project_id) else {
            return true;
        };
        match state.kind {
            StoredRefreshKind::RateLimited => match trigger {
                RefreshTrigger::Immediate | RefreshTrigger::Action | RefreshTrigger::RunEnded => {
                    true
                }
                RefreshTrigger::Interval => state
                    .retry_at_ms
                    .is_some_and(|retry_at| self.now_ms >= retry_at),
            },
            StoredRefreshKind::AuthFailed => {
                matches!(trigger, RefreshTrigger::Immediate | RefreshTrigger::Action)
            }
            _ => true,
        }
    }

    pub(crate) fn expire_stale_client_views(&mut self) {
        let ttl = self.refresh_interval_ms.saturating_mul(3).max(90_000);
        let now = self.now_ms;
        self.client_views
            .retain(|_, view| now.saturating_sub(view.last_seen_ms) <= ttl);
    }

    pub(crate) fn should_auto_refresh(&self, project_id: &str) -> bool {
        if !self.project_watched(project_id) {
            return false;
        }
        let Some(state) = self.refresh.get(project_id) else {
            return true;
        };
        match state.kind {
            StoredRefreshKind::RateLimited => state
                .retry_at_ms
                .is_some_and(|retry_at| self.now_ms >= retry_at),
            StoredRefreshKind::AuthFailed => false,
            StoredRefreshKind::Ready
            | StoredRefreshKind::Offline
            | StoredRefreshKind::Incomplete
            | StoredRefreshKind::TrackerError
            | StoredRefreshKind::NeverFetched => {
                self.now_ms
                    >= state
                        .last_attempt_ms
                        .saturating_add(self.refresh_interval_ms)
            }
        }
    }

    pub(crate) fn project_watched(&self, project_id: &str) -> bool {
        if self.window_visible
            && self.focused_host_id == LOCAL_HOST_ID
            && self.focused_project_id.as_deref() == Some(project_id)
        {
            return true;
        }
        self.client_views
            .values()
            .any(|view| view.visible && view.project_id == project_id)
    }

    pub(crate) fn refresh_status_for(&self, project_id: &str) -> RefreshStatus {
        if self.refresh_in_flight.contains_key(project_id) {
            return RefreshStatus::Refreshing {
                fetched_at_ms: self
                    .refresh
                    .get(project_id)
                    .and_then(|state| state.fetched_at_ms),
            };
        }
        let Some(state) = self.refresh.get(project_id) else {
            return RefreshStatus::NeverFetched;
        };
        let next = self.next_refresh_in_ms(project_id, state);
        // 拿到的是不完整数据时优先表达 incomplete：看板不能当作全量数据计算 Frontier/依赖图；
        // 从未读到过数据（complete=false 且无已加载数据）则按错误状态表达。
        if !state.complete && self.loaded_issues.contains_key(project_id) {
            match state.kind {
                StoredRefreshKind::Offline
                | StoredRefreshKind::RateLimited
                | StoredRefreshKind::AuthFailed => {}
                StoredRefreshKind::TrackerError => {
                    return RefreshStatus::TrackerError {
                        fetched_at_ms: state.fetched_at_ms,
                        data_complete: false,
                        next_refresh_in_ms: next,
                        detail: state.detail.clone(),
                    };
                }
                _ => {
                    return RefreshStatus::Incomplete {
                        fetched_at_ms: state.fetched_at_ms,
                        next_refresh_in_ms: next,
                        detail: state.detail.clone(),
                    };
                }
            }
        }
        match state.kind {
            StoredRefreshKind::Ready => match state.fetched_at_ms {
                Some(fetched_at_ms) => RefreshStatus::Ready {
                    fetched_at_ms,
                    next_refresh_in_ms: next,
                },
                None => RefreshStatus::NeverFetched,
            },
            StoredRefreshKind::Offline => match state.fetched_at_ms {
                Some(fetched_at_ms) => RefreshStatus::Offline {
                    fetched_at_ms,
                    next_refresh_in_ms: next,
                },
                None => RefreshStatus::NeverFetched,
            },
            StoredRefreshKind::NeverFetched => RefreshStatus::NeverFetched,
            StoredRefreshKind::RateLimited => RefreshStatus::RateLimited {
                fetched_at_ms: state.fetched_at_ms,
                retry_at_ms: state.retry_at_ms,
            },
            StoredRefreshKind::AuthFailed => RefreshStatus::AuthFailed {
                fetched_at_ms: state.fetched_at_ms,
            },
            StoredRefreshKind::Incomplete => RefreshStatus::Incomplete {
                fetched_at_ms: state.fetched_at_ms,
                next_refresh_in_ms: next,
                detail: state.detail.clone(),
            },
            StoredRefreshKind::TrackerError => RefreshStatus::TrackerError {
                fetched_at_ms: state.fetched_at_ms,
                data_complete: state.complete,
                next_refresh_in_ms: next,
                detail: state.detail.clone(),
            },
        }
    }

    pub(crate) fn next_refresh_in_ms(
        &self,
        project_id: &str,
        state: &ProjectRefreshState,
    ) -> Option<u64> {
        if !self.project_watched(project_id) {
            return None;
        }
        match state.kind {
            StoredRefreshKind::RateLimited => state
                .retry_at_ms
                .map(|retry_at| retry_at.saturating_sub(self.now_ms)),
            StoredRefreshKind::Ready
            | StoredRefreshKind::Offline
            | StoredRefreshKind::Incomplete
            | StoredRefreshKind::TrackerError => Some(
                (state
                    .last_attempt_ms
                    .saturating_add(self.refresh_interval_ms))
                .saturating_sub(self.now_ms),
            ),
            StoredRefreshKind::NeverFetched | StoredRefreshKind::AuthFailed => None,
        }
    }

    pub(crate) fn set_client_view(&mut self, client_id: &str, project_id: &str, visible: bool) {
        if !visible || project_id.is_empty() {
            self.client_views.remove(client_id);
            return;
        }
        let previous = self.client_views.insert(
            client_id.to_string(),
            ClientView {
                project_id: project_id.to_string(),
                visible: true,
                last_seen_ms: self.now_ms,
            },
        );
        let changed = previous
            .map(|view| !view.visible || view.project_id != project_id)
            .unwrap_or(true);
        if changed {
            self.refresh_project(project_id, RefreshTrigger::Immediate);
        }
    }

    pub(crate) fn require_live_tracker(&mut self, project_id: &str) -> Result<(), KernelError> {
        if !self.projects.iter().any(|project| project.id == project_id) {
            return Err(KernelError::Protocol("unknown project".into()));
        }
        if self.refresh_project(project_id, RefreshTrigger::Action) {
            Ok(())
        } else {
            Err(KernelError::Denied(self.write_block_reason(project_id)))
        }
    }

    pub(crate) fn require_live_tracker_for_issue(
        &mut self,
        issue_id: &str,
    ) -> Result<(), KernelError> {
        let project_id = match self.project_id_for_issue(issue_id) {
            Ok(project_id) => project_id,
            Err(_) => self
                .focused_project_id
                .clone()
                .ok_or_else(|| KernelError::Protocol("unknown issue".into()))?,
        };
        self.require_live_tracker(&project_id)
    }

    pub(crate) fn write_block_reason(&self, project_id: &str) -> String {
        match self.refresh.get(project_id).map(|state| state.kind) {
            Some(StoredRefreshKind::RateLimited) => {
                match self
                    .refresh
                    .get(project_id)
                    .and_then(|state| state.retry_at_ms)
                {
                    Some(retry_at_ms) => format!(
                        "cannot write to tracker: rate-limited (retry after {retry_at_ms}ms)"
                    ),
                    None => "cannot write to tracker: rate-limited".into(),
                }
            }
            Some(StoredRefreshKind::AuthFailed) => "cannot write to tracker: auth-failed".into(),
            Some(StoredRefreshKind::NeverFetched) | None => {
                "cannot write to tracker: never-fetched".into()
            }
            Some(StoredRefreshKind::Incomplete) => "cannot write to tracker: incomplete".into(),
            Some(StoredRefreshKind::TrackerError) => self
                .refresh
                .get(project_id)
                .and_then(|state| state.detail.as_deref())
                .map(|detail| format!("cannot write to tracker: tracker-error ({detail})"))
                .unwrap_or_else(|| "cannot write to tracker: tracker-error".into()),
            _ => "cannot write to tracker: offline".into(),
        }
    }

    pub(crate) fn probe_tracker(
        &self,
        tracker_kind: TrackerKind,
        github_host: &str,
        repository: &str,
    ) -> ProjectConnection {
        let pat = read_github_pat(&self.data.host_secrets_path, github_host);
        let outcome = self.tracker.probe(&tracker::ProbeContext {
            tracker: tracker_kind,
            github_host,
            repository,
            secrets_pat: pat.as_deref(),
            secrets_path: &self.data.host_secrets_path,
        });
        connection_from_probe(
            outcome,
            &self.data.host_secrets_path,
            self.appearance.language,
            github_host,
        )
    }

    pub(crate) fn persist_tracker_snapshot(&self, project_id: &str) {
        let Some(state) = self.refresh.get(project_id) else {
            return;
        };
        let Some(fetched_at_ms) = state.fetched_at_ms else {
            return;
        };
        let Some(issues) = self.loaded_issues.get(project_id) else {
            return;
        };
        let _ = refresh::save_snapshot(
            &refresh::snapshot_path(&self.data.host_dir, project_id),
            &refresh::StoredTrackerSnapshot {
                fetched_at_ms,
                complete: state.complete,
                detail: state.detail.clone(),
                issues: issues.clone(),
                documents: self.stored_issue_documents(project_id),
            },
        );
    }

    pub(crate) fn load_persisted_snapshot(&mut self, project_id: &str) {
        let Some(stored) =
            refresh::load_snapshot(&refresh::snapshot_path(&self.data.host_dir, project_id))
        else {
            return;
        };
        let documents = self
            .issue_documents
            .entry(project_id.to_string())
            .or_default();
        for (issue_id, document) in &stored.documents {
            documents.insert(
                issue_id.clone(),
                IssueDocumentState::Loading {
                    body: Some(document.body.clone()),
                    fetched_at_ms: Some(document.fetched_at_ms),
                },
            );
        }
        self.loaded_issues
            .insert(project_id.to_string(), stored.issues);
        if let Some(project) = self
            .projects
            .iter_mut()
            .find(|project| project.id == project_id)
        {
            project.tracker_synced = stored.complete;
        }
        self.refresh.insert(
            project_id.to_string(),
            ProjectRefreshState {
                fetched_at_ms: Some(stored.fetched_at_ms),
                last_attempt_ms: stored.fetched_at_ms,
                kind: if stored.complete {
                    StoredRefreshKind::Ready
                } else {
                    StoredRefreshKind::Incomplete
                },
                retry_at_ms: None,
                complete: stored.complete,
                detail: stored.detail,
            },
        );
    }

    pub(crate) fn begin_deferred_refreshes(&mut self) {
        debug_assert!(!self.defer_refreshes);
        debug_assert!(self.deferred_refreshes.is_empty());
        self.defer_refreshes = true;
    }

    pub(crate) fn take_deferred_refreshes(&mut self) -> Vec<PreparedRefresh> {
        self.defer_refreshes = false;
        std::mem::take(&mut self.deferred_refreshes)
    }

    pub(crate) fn cancel_prepared_refreshes(&mut self, prepared: &[PreparedRefresh]) {
        for refresh in prepared {
            if self.refresh_in_flight.get(&refresh.project_id).copied() == Some(refresh.generation)
            {
                self.refresh_in_flight.remove(&refresh.project_id);
            }
        }
    }

    pub(crate) fn execute_prepared_refresh(prepared: PreparedRefresh) -> CompletedRefresh {
        let result = prepared.tracker.read_all(&tracker::ProbeContext {
            tracker: prepared.tracker_kind,
            github_host: &prepared.github_host,
            repository: &prepared.repository,
            secrets_pat: prepared.secrets_pat.as_deref(),
            secrets_path: &prepared.secrets_path,
        });
        CompletedRefresh { prepared, result }
    }

    pub(crate) fn finish_prepared_refresh(&mut self, completed: CompletedRefresh) -> bool {
        let CompletedRefresh { prepared, result } = completed;
        if self.refresh_in_flight.get(&prepared.project_id).copied() != Some(prepared.generation) {
            return false;
        }
        self.refresh_in_flight.remove(&prepared.project_id);
        let current_write_generation = self
            .issue_write_generations
            .get(&prepared.project_id)
            .copied()
            .unwrap_or(0);
        let result = if current_write_generation != prepared.issue_write_generation {
            match result {
                Ok(tracker_seam::TrackerReadOutcome::Complete { issues }) => {
                    Ok(tracker_seam::TrackerReadOutcome::Complete {
                        issues: preserve_written_issues(
                            &prepared.issues_at_prepare,
                            self.loaded_issues
                                .get(&prepared.project_id)
                                .map(Vec::as_slice)
                                .unwrap_or_default(),
                            issues,
                        ),
                    })
                }
                Ok(tracker_seam::TrackerReadOutcome::Incomplete { issues, detail }) => {
                    Ok(tracker_seam::TrackerReadOutcome::Incomplete {
                        issues: preserve_written_issues(
                            &prepared.issues_at_prepare,
                            self.loaded_issues
                                .get(&prepared.project_id)
                                .map(Vec::as_slice)
                                .unwrap_or_default(),
                            issues,
                        ),
                        detail,
                    })
                }
                Err(error) => Err(error),
            }
        } else {
            result
        };
        let Some(index) = self.projects.iter().position(|project| {
            project.id == prepared.project_id
                && project.tracker == prepared.tracker_kind
                && project.github_host == prepared.github_host
                && project.repository == prepared.repository
        }) else {
            return false;
        };
        let previous_fetched = prepared
            .previous
            .as_ref()
            .and_then(|state| state.fetched_at_ms);
        match result {
            Ok(tracker_seam::TrackerReadOutcome::Complete { issues }) => {
                let content_changed = self.apply_read(
                    &prepared.project_id,
                    index,
                    &prepared.github_host,
                    &prepared.repository,
                    prepared.now_ms,
                    issues,
                    true,
                    None,
                );
                self.record_local_tracker_revision(&prepared);
                let status = self.refresh_status_for(&prepared.project_id);
                self.pending_events.push(HostEvent::RefreshStatusChanged {
                    project_id: prepared.project_id.clone(),
                    status,
                });
                if content_changed {
                    self.pending_events.push(HostEvent::BoardUpdated {
                        project_id: prepared.project_id,
                    });
                }
                true
            }
            Ok(tracker_seam::TrackerReadOutcome::Incomplete { issues, detail }) => {
                let content_changed = self.apply_read(
                    &prepared.project_id,
                    index,
                    &prepared.github_host,
                    &prepared.repository,
                    prepared.now_ms,
                    issues,
                    false,
                    Some(detail),
                );
                self.record_local_tracker_revision(&prepared);
                let status = self.refresh_status_for(&prepared.project_id);
                self.pending_events.push(HostEvent::RefreshStatusChanged {
                    project_id: prepared.project_id.clone(),
                    status,
                });
                if content_changed {
                    self.pending_events.push(HostEvent::BoardUpdated {
                        project_id: prepared.project_id,
                    });
                }
                true
            }
            Err(tracker::TrackerReadError::RateLimited { retry_after_ms }) => {
                self.refresh.insert(
                    prepared.project_id.clone(),
                    ProjectRefreshState {
                        fetched_at_ms: previous_fetched,
                        last_attempt_ms: prepared.now_ms,
                        kind: StoredRefreshKind::RateLimited,
                        retry_at_ms: retry_after_ms.map(|ms| prepared.now_ms.saturating_add(ms)),
                        complete: prepared
                            .previous
                            .as_ref()
                            .map(|state| state.complete)
                            .unwrap_or(false),
                        detail: prepared
                            .previous
                            .as_ref()
                            .and_then(|state| state.detail.clone()),
                    },
                );
                let status = self.refresh_status_for(&prepared.project_id);
                self.pending_events.push(HostEvent::RefreshStatusChanged {
                    project_id: prepared.project_id,
                    status,
                });
                false
            }
            Err(tracker::TrackerReadError::Offline {
                source,
                cli_detected,
                detail,
            }) => {
                self.projects[index].connection = ProjectConnection::Unreachable {
                    source,
                    repair: tracker::repair_hint(cli_detected, &prepared.secrets_path),
                    message: auth_failure_message(
                        self.appearance.language,
                        AuthFailureKind::Unreachable,
                        detail.as_deref(),
                        &prepared.github_host,
                    ),
                };
                let has_data = self.loaded_issues.contains_key(&prepared.project_id);
                self.refresh.insert(
                    prepared.project_id.clone(),
                    ProjectRefreshState {
                        fetched_at_ms: previous_fetched,
                        last_attempt_ms: prepared.now_ms,
                        kind: if has_data {
                            StoredRefreshKind::Offline
                        } else {
                            StoredRefreshKind::NeverFetched
                        },
                        retry_at_ms: None,
                        complete: prepared
                            .previous
                            .as_ref()
                            .map(|state| state.complete)
                            .unwrap_or(false),
                        detail: prepared
                            .previous
                            .as_ref()
                            .and_then(|state| state.detail.clone()),
                    },
                );
                let status = self.refresh_status_for(&prepared.project_id);
                self.pending_events.push(HostEvent::RefreshStatusChanged {
                    project_id: prepared.project_id,
                    status,
                });
                false
            }
            Err(tracker::TrackerReadError::Auth {
                source,
                kind,
                cli_detected,
                detail,
            }) => {
                self.projects[index].connection = ProjectConnection::AuthFailed {
                    source,
                    kind,
                    repair: tracker::repair_hint(cli_detected, &prepared.secrets_path),
                    message: auth_failure_message(
                        self.appearance.language,
                        kind,
                        detail.as_deref(),
                        &prepared.github_host,
                    ),
                };
                self.refresh.insert(
                    prepared.project_id.clone(),
                    ProjectRefreshState {
                        fetched_at_ms: previous_fetched,
                        last_attempt_ms: prepared.now_ms,
                        kind: StoredRefreshKind::AuthFailed,
                        retry_at_ms: None,
                        complete: prepared
                            .previous
                            .as_ref()
                            .map(|state| state.complete)
                            .unwrap_or(false),
                        detail: prepared
                            .previous
                            .as_ref()
                            .and_then(|state| state.detail.clone()),
                    },
                );
                let status = self.refresh_status_for(&prepared.project_id);
                self.pending_events.push(HostEvent::RefreshStatusChanged {
                    project_id: prepared.project_id,
                    status,
                });
                false
            }
            Err(tracker::TrackerReadError::Failed { detail }) => {
                let detail = detail.unwrap_or_else(|| "tracker business error".into());
                let complete = prepared
                    .previous
                    .as_ref()
                    .map(|state| state.complete)
                    .unwrap_or(false);
                self.refresh.insert(
                    prepared.project_id.clone(),
                    ProjectRefreshState {
                        fetched_at_ms: previous_fetched,
                        last_attempt_ms: prepared.now_ms,
                        kind: StoredRefreshKind::TrackerError,
                        retry_at_ms: None,
                        complete,
                        detail: Some(detail),
                    },
                );
                let status = self.refresh_status_for(&prepared.project_id);
                self.pending_events.push(HostEvent::RefreshStatusChanged {
                    project_id: prepared.project_id,
                    status,
                });
                false
            }
        }
    }
}
