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

pub(crate) struct PreparedIssueWrite {
    pub(crate) tracker: Arc<dyn TrackerSeam>,
    pub(crate) project_id: String,
    pub(crate) issue_id: Option<String>,
    pub(crate) github_host: String,
    pub(crate) repository: String,
    pub(crate) tracker_kind: TrackerKind,
    pub(crate) secrets_pat: Option<String>,
    pub(crate) secrets_path: PathBuf,
    pub(crate) op: tracker_seam::TrackerWriteOp,
}

pub(crate) struct CompletedIssueWrite {
    pub(crate) prepared: PreparedIssueWrite,
    pub(crate) result: Result<IssueRecord, tracker::TrackerWriteError>,
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
                IssueDocumentState::Ready {
                    body: document.body,
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
