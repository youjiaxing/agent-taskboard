//! On-demand Issue document reads.

use super::super::*;

pub(crate) struct PreparedIssueDocument {
    tracker: Arc<dyn TrackerSeam>,
    pub(crate) project_id: String,
    pub(crate) issue_id: String,
    github_host: String,
    repository: String,
    tracker_kind: TrackerKind,
    secrets_pat: Option<String>,
    secrets_path: PathBuf,
    previous_body: Option<(String, u64)>,
    now_ms: u64,
    pub(crate) generation: u64,
}

pub(crate) struct CompletedIssueDocument {
    prepared: PreparedIssueDocument,
    result: Result<tracker::IssueDocument, tracker::TrackerReadError>,
}

pub(crate) enum IssueWriteExpectation {
    Content { title: String, body: String },
    Parent { parent: Option<String> },
    BlockedBy { blocked_by: Vec<String> },
}

pub(crate) enum IssueWriteFailure {
    Tracker(tracker::TrackerWriteError),
    Conflict(IssueConflict),
}

pub(crate) struct PreparedIssueWrite {
    pub(crate) tracker: Arc<dyn TrackerSeam>,
    pub(crate) project_id: String,
    pub(crate) issue_id: Option<String>,
    pub(crate) github_host: String,
    pub(crate) repository: String,
    pub(crate) tracker_kind: TrackerKind,
    pub(crate) current_issue: Option<IssueRecord>,
    pub(crate) expectation: Option<IssueWriteExpectation>,
    pub(crate) overwrite_conflict: bool,
    pub(crate) secrets_pat: Option<String>,
    pub(crate) secrets_path: PathBuf,
    pub(crate) op: tracker_seam::TrackerWriteOp,
}

pub(crate) struct CompletedIssueWrite {
    pub(crate) prepared: PreparedIssueWrite,
    pub(crate) result: Result<IssueRecord, IssueWriteFailure>,
}

impl HostKernel {
    pub(crate) fn load_issue_document(&mut self, issue_id: &str) -> Result<(), KernelError> {
        let prepared = self.prepare_issue_document(issue_id)?;
        if self.defer_issue_documents {
            self.deferred_issue_documents.push(prepared);
            return Ok(());
        }
        let completed = Self::execute_prepared_issue_document(prepared);
        self.finish_prepared_issue_document(completed);
        Ok(())
    }

    fn prepare_issue_document(
        &mut self,
        issue_id: &str,
    ) -> Result<PreparedIssueDocument, KernelError> {
        let project_id = self
            .focused_project_id
            .clone()
            .ok_or_else(|| KernelError::Protocol("no focused project".into()))?;
        if self.selected_issue_id.as_deref() != Some(issue_id) {
            return Err(KernelError::Protocol(
                "Issue document can only be loaded for the selected Issue".into(),
            ));
        }
        let project = self
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown project".into()))?;
        let previous = self
            .issue_documents
            .get(&project_id)
            .and_then(|documents| documents.get(issue_id))
            .cloned();
        let previous_body = issue_document_body(previous.as_ref());
        self.issue_documents
            .entry(project_id.clone())
            .or_default()
            .insert(
                issue_id.to_string(),
                IssueDocumentState::Loading {
                    body: previous_body.as_ref().map(|(body, _)| body.clone()),
                    fetched_at_ms: previous_body
                        .as_ref()
                        .map(|(_, fetched_at_ms)| *fetched_at_ms),
                },
            );
        self.next_issue_document_generation = self.next_issue_document_generation.saturating_add(1);
        let generation = self.next_issue_document_generation;
        self.issue_document_in_flight
            .insert((project_id.clone(), issue_id.to_string()), generation);
        let pat = read_github_pat(&self.data.host_secrets_path, &project.github_host);
        Ok(PreparedIssueDocument {
            tracker: Arc::clone(&self.tracker),
            project_id,
            issue_id: issue_id.to_string(),
            github_host: project.github_host,
            repository: project.repository,
            tracker_kind: project.tracker,
            secrets_pat: pat,
            secrets_path: self.data.host_secrets_path.clone(),
            previous_body,
            now_ms: self.now_ms,
            generation,
        })
    }

    pub(crate) fn execute_prepared_issue_document(
        prepared: PreparedIssueDocument,
    ) -> CompletedIssueDocument {
        let result = prepared.tracker.read_issue_document(
            &tracker::ProbeContext {
                tracker: prepared.tracker_kind,
                github_host: &prepared.github_host,
                repository: &prepared.repository,
                secrets_pat: prepared.secrets_pat.as_deref(),
                secrets_path: &prepared.secrets_path,
            },
            &prepared.issue_id,
        );
        CompletedIssueDocument { prepared, result }
    }

    pub(crate) fn finish_prepared_issue_document(
        &mut self,
        completed: CompletedIssueDocument,
    ) -> bool {
        let CompletedIssueDocument { prepared, result } = completed;
        let key = (prepared.project_id.clone(), prepared.issue_id.clone());
        if self.issue_document_in_flight.get(&key).copied() != Some(prepared.generation) {
            return false;
        }
        self.issue_document_in_flight.remove(&key);
        let state = match result {
            Ok(document) => {
                if let Some(issues) = self.loaded_issues.get_mut(&prepared.project_id) {
                    if let Some(existing) = issues
                        .iter_mut()
                        .find(|issue| issue.id() == prepared.issue_id)
                    {
                        // 单 Issue REST 响应不保证带原生父子与 Dependency；详情刷新只合并
                        // 该响应确实拥有的基础字段，关系仍由列表/GraphQL 真源维护。
                        existing.title = document.issue.title;
                        existing.url = document.issue.url;
                        existing.open = document.issue.open;
                        existing.closed_at = document.issue.closed_at;
                        existing.assignees = document.issue.assignees;
                        existing.labels = document.issue.labels;
                    }
                }
                let editable_body = if prepared.tracker_kind == TrackerKind::LocalMarkdown {
                    tracker::editable_body(&document.body)
                } else {
                    document.body.clone()
                };
                IssueDocumentState::Ready {
                    body: document.body,
                    editable_body: Some(editable_body),
                    fetched_at_ms: prepared.now_ms,
                }
            }
            Err(error) => {
                let failure = issue_document_failure(error);
                match prepared.previous_body {
                    Some((body, fetched_at_ms)) => IssueDocumentState::Stale {
                        body,
                        fetched_at_ms,
                        failure,
                    },
                    None => IssueDocumentState::Failed { failure },
                }
            }
        };
        self.issue_documents
            .entry(prepared.project_id.clone())
            .or_default()
            .insert(prepared.issue_id, state);
        self.persist_tracker_snapshot(&prepared.project_id);
        self.pending_events.push(HostEvent::BoardUpdated {
            project_id: prepared.project_id,
        });
        true
    }
}

impl HostKernel {
    pub(crate) fn begin_deferred_issue_documents(&mut self) {
        debug_assert!(!self.defer_issue_documents);
        debug_assert!(self.deferred_issue_documents.is_empty());
        self.defer_issue_documents = true;
    }

    pub(crate) fn take_deferred_issue_documents(
        &mut self,
    ) -> Vec<kernel::issue_documents::PreparedIssueDocument> {
        self.defer_issue_documents = false;
        std::mem::take(&mut self.deferred_issue_documents)
    }

    pub(crate) fn cancel_prepared_issue_documents(
        &mut self,
        prepared: &[kernel::issue_documents::PreparedIssueDocument],
    ) {
        for document in prepared {
            let key = (document.project_id.clone(), document.issue_id.clone());
            if self.issue_document_in_flight.get(&key).copied() == Some(document.generation) {
                self.issue_document_in_flight.remove(&key);
            }
        }
    }

    pub(crate) fn begin_deferred_issue_writes(&mut self) {
        debug_assert!(!self.defer_issue_writes);
        debug_assert!(self.deferred_issue_writes.is_empty());
        self.defer_issue_writes = true;
    }

    pub(crate) fn take_deferred_issue_writes(
        &mut self,
    ) -> Vec<kernel::issue_documents::PreparedIssueWrite> {
        self.defer_issue_writes = false;
        std::mem::take(&mut self.deferred_issue_writes)
    }

    pub(crate) fn execute_prepared_issue_write(
        mut prepared: kernel::issue_documents::PreparedIssueWrite,
    ) -> kernel::issue_documents::CompletedIssueWrite {
        let context = tracker::ProbeContext {
            tracker: prepared.tracker_kind,
            github_host: &prepared.github_host,
            repository: &prepared.repository,
            secrets_pat: prepared.secrets_pat.as_deref(),
            secrets_path: &prepared.secrets_path,
        };
        let result = (|| {
            if !prepared.overwrite_conflict {
                if let Some(expectation) = prepared.expectation.as_ref() {
                    let latest = match expectation {
                        IssueWriteExpectation::Content { .. } => prepared
                            .tracker
                            .read_issue_content(
                                &context,
                                prepared.issue_id.as_deref().unwrap_or_default(),
                            )
                            .map_err(read_error_as_write_failure)?,
                        IssueWriteExpectation::Parent { .. }
                        | IssueWriteExpectation::BlockedBy { .. } => IssueDocument {
                            issue: prepared
                                .tracker
                                .read_issue_relations(
                                    &context,
                                    prepared.issue_id.as_deref().unwrap_or_default(),
                                )
                                .map_err(read_error_as_write_failure)?,
                            body: String::new(),
                        },
                    };
                    let mut fields = Vec::new();
                    let mut skip_write = false;
                    match (expectation, &mut prepared.op) {
                        (
                            IssueWriteExpectation::Content { title, body },
                            TrackerWriteOp::UpdateIssue {
                                title: wanted_title,
                                body: wanted_body,
                            },
                        ) => {
                            if wanted_title == title {
                                *wanted_title = latest.issue.title.clone();
                            } else if latest.issue.title != *title
                                && latest.issue.title != *wanted_title
                            {
                                fields.push("title".to_string());
                            }
                            if wanted_body == body {
                                *wanted_body = latest.body.clone();
                            } else if latest.body != *body && latest.body != *wanted_body {
                                fields.push("body".to_string());
                            }
                        }
                        (
                            IssueWriteExpectation::Parent {
                                parent: base_parent,
                            },
                            TrackerWriteOp::SetParent {
                                parent: wanted_parent,
                            },
                        ) => {
                            let latest_parent = latest.issue.parent.as_ref().map(IssueRef::id);
                            let wanted_id = wanted_parent.as_ref().map(IssueRef::id);
                            if wanted_id == *base_parent {
                                *wanted_parent = latest.issue.parent.clone();
                                skip_write = true;
                            } else if latest_parent != *base_parent && latest_parent != wanted_id {
                                fields.push("parent".to_string());
                            }
                        }
                        (
                            IssueWriteExpectation::BlockedBy {
                                blocked_by: base_blocked_by,
                            },
                            TrackerWriteOp::SetBlockedBy {
                                blocked_by: wanted_blocked_by,
                            },
                        ) => {
                            let latest_ids = dependency_ids(&latest.issue);
                            let wanted_ids =
                                sorted_ids(wanted_blocked_by.iter().map(IssueRef::id).collect());
                            let base_ids = sorted_ids(base_blocked_by.clone());
                            if wanted_ids == base_ids {
                                skip_write = true;
                            } else if latest_ids != base_ids && latest_ids != wanted_ids {
                                fields.push("blockedBy".to_string());
                            }
                        }
                        _ => {
                            return Err(IssueWriteFailure::Tracker(TrackerWriteError::Failed {
                                message: "Issue write expectation does not match the operation"
                                    .into(),
                            }));
                        }
                    }
                    if !fields.is_empty() {
                        return Err(IssueWriteFailure::Conflict(IssueConflict {
                            issue_id: prepared.issue_id.clone().unwrap_or_default(),
                            fields,
                            latest: IssueConflictLatest {
                                title: Some(latest.issue.title.clone()),
                                body: Some(latest.body),
                                parent: latest.issue.parent.as_ref().map(IssueRef::id),
                                blocked_by: Some(dependency_ids(&latest.issue)),
                            },
                        }));
                    }
                    prepared.current_issue = Some(latest.issue.clone());
                    if skip_write {
                        return Ok(latest.issue);
                    }
                }
            }
            if matches!(&prepared.op, TrackerWriteOp::SetBlockedBy { .. })
                && (prepared.overwrite_conflict || prepared.expectation.is_none())
            {
                prepared.current_issue = Some(
                    prepared
                        .tracker
                        .read_issue_relations(
                            &context,
                            prepared.issue_id.as_deref().unwrap_or_default(),
                        )
                        .map_err(read_error_as_write_failure)?,
                );
            }
            prepared
                .tracker
                .write_issue(
                    &context,
                    prepared.current_issue.as_ref(),
                    prepared.issue_id.as_deref(),
                    &prepared.op,
                )
                .map_err(IssueWriteFailure::Tracker)
        })();
        kernel::issue_documents::CompletedIssueWrite { prepared, result }
    }

    pub(crate) fn finish_prepared_issue_write(
        &mut self,
        completed: kernel::issue_documents::CompletedIssueWrite,
    ) -> Result<IssueRecord, KernelError> {
        let kernel::issue_documents::CompletedIssueWrite { prepared, result } = completed;
        let updated = result.map_err(|failure| match failure {
            IssueWriteFailure::Tracker(error) => write_tracker_error(error),
            IssueWriteFailure::Conflict(conflict) => KernelError::Conflict(conflict),
        })?;
        let returned = updated.clone();
        let generation = self
            .issue_write_generations
            .entry(prepared.project_id.clone())
            .or_default();
        *generation = generation.saturating_add(1);
        self.merge_issue(&prepared.project_id, updated, &prepared.op);
        Ok(returned)
    }
}

fn sorted_ids(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids.dedup();
    ids
}

fn dependency_ids(issue: &IssueRecord) -> Vec<String> {
    sorted_ids(
        issue
            .blocked_by
            .iter()
            .filter_map(|dependency| match dependency {
                DependencyRef::Known(reference) => Some(reference.id()),
                DependencyRef::Unclear { .. } => None,
            })
            .collect(),
    )
}

fn read_error_as_write_failure(error: TrackerReadError) -> IssueWriteFailure {
    IssueWriteFailure::Tracker(match error {
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
    })
}

pub(crate) fn issue_document_body(state: Option<&IssueDocumentState>) -> Option<(String, u64)> {
    match state? {
        IssueDocumentState::Ready {
            body,
            fetched_at_ms,
            ..
        }
        | IssueDocumentState::Stale {
            body,
            fetched_at_ms,
            ..
        } => Some((body.clone(), *fetched_at_ms)),
        IssueDocumentState::Loading {
            body: Some(body),
            fetched_at_ms: Some(fetched_at_ms),
        } => Some((body.clone(), *fetched_at_ms)),
        IssueDocumentState::Unloaded
        | IssueDocumentState::Loading { .. }
        | IssueDocumentState::Failed { .. } => None,
    }
}

pub(crate) fn issue_document_failure(error: tracker::TrackerReadError) -> IssueDocumentFailure {
    match error {
        tracker::TrackerReadError::Offline { detail, .. } => IssueDocumentFailure {
            kind: IssueDocumentFailureKind::Offline,
            message: detail.unwrap_or_else(|| "Issue Tracker is offline".into()),
            retry_after_ms: None,
        },
        tracker::TrackerReadError::RateLimited { retry_after_ms } => IssueDocumentFailure {
            kind: IssueDocumentFailureKind::RateLimited,
            message: "Issue Tracker rate limit reached".into(),
            retry_after_ms,
        },
        tracker::TrackerReadError::Auth { detail, .. } => IssueDocumentFailure {
            kind: IssueDocumentFailureKind::Auth,
            message: detail.unwrap_or_else(|| "Issue Tracker authentication failed".into()),
            retry_after_ms: None,
        },
        tracker::TrackerReadError::Failed { detail } => IssueDocumentFailure {
            kind: IssueDocumentFailureKind::Tracker,
            message: detail.unwrap_or_else(|| "Issue Tracker could not load this Issue".into()),
            retry_after_ms: None,
        },
    }
}

impl HostKernel {
    pub(crate) fn stored_issue_documents(
        &self,
        project_id: &str,
    ) -> BTreeMap<String, refresh::StoredIssueDocument> {
        self.issue_documents
            .get(project_id)
            .into_iter()
            .flat_map(|documents| documents.iter())
            .filter_map(|(issue_id, state)| {
                issue_document_body(Some(state)).map(|(body, fetched_at_ms)| {
                    (
                        issue_id.clone(),
                        refresh::StoredIssueDocument {
                            body,
                            fetched_at_ms,
                        },
                    )
                })
            })
            .collect()
    }
}
