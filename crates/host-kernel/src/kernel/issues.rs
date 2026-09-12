use super::super::*;

impl HostKernel {
    pub(crate) fn issue_by_id(&self, issue_id: &str) -> Option<IssueRecord> {
        self.loaded_issues
            .values()
            .flat_map(|issues| issues.iter())
            .find(|issue| issue.id() == issue_id)
            .cloned()
    }

    pub(crate) fn claim_issue(&mut self, issue_id: &str) -> Result<(), KernelError> {
        self.write_claim(issue_id, true)
    }

    pub(crate) fn release_issue(&mut self, issue_id: &str) -> Result<(), KernelError> {
        self.write_claim(issue_id, false)
    }

    pub(crate) fn write_claim(&mut self, issue_id: &str, claim: bool) -> Result<(), KernelError> {
        let project_id = self.project_id_for_issue(issue_id)?;
        let op = if claim {
            tracker_seam::TrackerWriteOp::Claim
        } else {
            tracker_seam::TrackerWriteOp::Release
        };
        self.write_issue_op(&project_id, Some(issue_id), op)?;
        Ok(())
    }

    pub(crate) fn create_issue(
        &mut self,
        project_id: &str,
        title: &str,
        body: &str,
    ) -> Result<(), KernelError> {
        let title = title.trim();
        if title.is_empty() {
            return Err(KernelError::Protocol("missing title".into()));
        }
        if !self.projects.iter().any(|project| project.id == project_id) {
            return Err(KernelError::Protocol("unknown project".into()));
        }
        let op = tracker_seam::TrackerWriteOp::CreateIssue {
            title: title.to_string(),
            body: body.to_string(),
        };
        self.write_issue_op(project_id, None, op)?;
        Ok(())
    }

    pub(crate) fn update_issue(
        &mut self,
        issue_id: &str,
        title: &str,
        body: &str,
        base_title: Option<String>,
        base_body: Option<String>,
        overwrite_conflict: bool,
    ) -> Result<(), KernelError> {
        let title = title.trim();
        if title.is_empty() {
            return Err(KernelError::Protocol("missing title".into()));
        }
        let project_id = self.project_id_for_issue(issue_id)?;
        let op = tracker_seam::TrackerWriteOp::UpdateIssue {
            title: title.to_string(),
            body: body.to_string(),
        };
        match (base_title, base_body) {
            (Some(base_title), Some(base_body)) => {
                self.write_issue_op_checked(
                    &project_id,
                    issue_id,
                    op,
                    kernel::issue_documents::IssueWriteExpectation::Content {
                        title: base_title,
                        body: base_body,
                    },
                    overwrite_conflict,
                )?;
            }
            _ => {
                self.write_issue_op(&project_id, Some(issue_id), op)?;
            }
        }
        Ok(())
    }

    pub(crate) fn set_issue_open(&mut self, issue_id: &str, open: bool) -> Result<(), KernelError> {
        self.write_issue_op(
            &self.project_id_for_issue(issue_id)?,
            Some(issue_id),
            tracker_seam::TrackerWriteOp::SetOpen { open },
        )?;
        Ok(())
    }

    pub(crate) fn add_issue_comment(
        &mut self,
        issue_id: &str,
        body: &str,
    ) -> Result<(), KernelError> {
        let body = body.trim();
        if body.is_empty() {
            return Err(KernelError::Protocol("missing body".into()));
        }
        self.write_issue_op(
            &self.project_id_for_issue(issue_id)?,
            Some(issue_id),
            tracker_seam::TrackerWriteOp::AddComment {
                body: body.to_string(),
            },
        )?;
        Ok(())
    }

    pub(crate) fn set_issue_parent(
        &mut self,
        issue_id: &str,
        parent: Option<&str>,
        base_parent: Option<Option<String>>,
        overwrite_conflict: bool,
    ) -> Result<(), KernelError> {
        let parent = parent.map(parse_issue_ref).transpose()?;
        let project_id = self.project_id_for_issue(issue_id)?;
        let op = tracker_seam::TrackerWriteOp::SetParent { parent };
        if let Some(base_parent) = base_parent {
            self.write_issue_op_checked(
                &project_id,
                issue_id,
                op,
                kernel::issue_documents::IssueWriteExpectation::Parent {
                    parent: base_parent,
                },
                overwrite_conflict,
            )?;
        } else {
            self.write_issue_op(&project_id, Some(issue_id), op)?;
        }
        Ok(())
    }

    pub(crate) fn set_issue_blocked_by(
        &mut self,
        issue_id: &str,
        blocked_by: &[String],
        base_blocked_by: Option<Vec<String>>,
        overwrite_conflict: bool,
    ) -> Result<(), KernelError> {
        let blocked_by = blocked_by
            .iter()
            .map(|id| parse_issue_ref(id))
            .collect::<Result<Vec<_>, _>>()?;
        let project_id = self.project_id_for_issue(issue_id)?;
        let op = tracker_seam::TrackerWriteOp::SetBlockedBy { blocked_by };
        if let Some(base_blocked_by) = base_blocked_by {
            self.write_issue_op_checked(
                &project_id,
                issue_id,
                op,
                kernel::issue_documents::IssueWriteExpectation::BlockedBy {
                    blocked_by: base_blocked_by,
                },
                overwrite_conflict,
            )?;
        } else {
            self.write_issue_op(&project_id, Some(issue_id), op)?;
        }
        Ok(())
    }

    pub(crate) fn write_issue_op(
        &mut self,
        project_id: &str,
        issue_id: Option<&str>,
        op: tracker_seam::TrackerWriteOp,
    ) -> Result<(), KernelError> {
        let prepared = self.prepare_issue_write(project_id, issue_id, op, None, false)?;
        if self.defer_issue_writes {
            self.deferred_issue_writes.push(prepared);
            return Ok(());
        }
        let completed = Self::execute_prepared_issue_write(prepared);
        self.finish_prepared_issue_write(completed).map(drop)
    }

    pub(crate) fn write_issue_op_checked(
        &mut self,
        project_id: &str,
        issue_id: &str,
        op: tracker_seam::TrackerWriteOp,
        expectation: kernel::issue_documents::IssueWriteExpectation,
        overwrite_conflict: bool,
    ) -> Result<(), KernelError> {
        let prepared = self.prepare_issue_write(
            project_id,
            Some(issue_id),
            op,
            Some(expectation),
            overwrite_conflict,
        )?;
        if self.defer_issue_writes {
            self.deferred_issue_writes.push(prepared);
            return Ok(());
        }
        let completed = Self::execute_prepared_issue_write(prepared);
        self.finish_prepared_issue_write(completed).map(drop)
    }

    pub(crate) fn prepare_issue_write(
        &self,
        project_id: &str,
        issue_id: Option<&str>,
        op: tracker_seam::TrackerWriteOp,
        expectation: Option<kernel::issue_documents::IssueWriteExpectation>,
        overwrite_conflict: bool,
    ) -> Result<kernel::issue_documents::PreparedIssueWrite, KernelError> {
        let project = self
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown project".into()))?;
        let pat = read_github_pat(&self.data.host_secrets_path, &project.github_host);
        Ok(kernel::issue_documents::PreparedIssueWrite {
            tracker: Arc::clone(&self.tracker),
            project_id: project_id.to_string(),
            issue_id: issue_id.map(ToOwned::to_owned),
            github_host: project.github_host,
            repository: project.repository,
            tracker_kind: project.tracker,
            current_issue: issue_id.and_then(|id| self.issue_by_id(id)),
            expectation,
            overwrite_conflict,
            secrets_pat: pat,
            secrets_path: self.data.host_secrets_path.clone(),
            op,
        })
    }

    pub(crate) fn merge_issue(
        &mut self,
        project_id: &str,
        updated: IssueRecord,
        op: &tracker_seam::TrackerWriteOp,
    ) {
        let issues = self
            .loaded_issues
            .entry(project_id.to_string())
            .or_default();
        if let Some(existing) = issues.iter_mut().find(|issue| issue.id() == updated.id()) {
            // 按操作合并对应字段，避免用适配器响应里缺失的关系字段覆盖本地已知数据。
            match op {
                tracker_seam::TrackerWriteOp::CreateIssue { .. } => {}
                tracker_seam::TrackerWriteOp::UpdateIssue { .. } => {
                    existing.title = updated.title;
                    existing.url = updated.url;
                }
                tracker_seam::TrackerWriteOp::SetOpen { .. } => {
                    existing.open = updated.open;
                    existing.closed_at = updated.closed_at;
                    existing.labels = updated.labels;
                }
                tracker_seam::TrackerWriteOp::AddComment { .. } => {}
                tracker_seam::TrackerWriteOp::Claim | tracker_seam::TrackerWriteOp::Release => {
                    existing.assignees = updated.assignees;
                    existing.open = updated.open;
                    existing.title = updated.title;
                    existing.url = updated.url;
                    existing.closed_at = updated.closed_at;
                    existing.labels = updated.labels;
                }
                tracker_seam::TrackerWriteOp::SetParent { .. } => {
                    existing.parent = updated.parent;
                }
                tracker_seam::TrackerWriteOp::SetBlockedBy { .. } => {
                    existing.blocked_by = updated.blocked_by;
                }
            }
        } else {
            issues.push(updated);
        }
        self.persist_tracker_snapshot(project_id);
        self.pending_events.push(HostEvent::BoardUpdated {
            project_id: project_id.to_string(),
        });
    }

    pub(crate) fn focus_issue(&mut self, issue_id: &str) {
        self.selected_issue_id = Some(issue_id.to_string());
        if let Some(project_id) = self.focused_project_id.clone() {
            let previous_body = self
                .issue_documents
                .get(&project_id)
                .and_then(|documents| documents.get(issue_id))
                .and_then(|state| issue_document_body(Some(state)));
            let should_start_loading = self
                .issue_documents
                .get(&project_id)
                .and_then(|documents| documents.get(issue_id))
                .is_none_or(|state| !matches!(state, IssueDocumentState::Ready { .. }));
            if should_start_loading {
                self.issue_documents.entry(project_id).or_default().insert(
                    issue_id.to_string(),
                    IssueDocumentState::Loading {
                        body: previous_body.as_ref().map(|(body, _)| body.clone()),
                        fetched_at_ms: previous_body.map(|(_, fetched_at_ms)| fetched_at_ms),
                    },
                );
            }
        }
        self.focused_run_id = self.active_run_id_for_issue(issue_id);
        self.workspace_view = WorkspaceView::Project;
    }

    /// 记录一次成功读取（完整或不完整）的结果并持久化快照。
    pub(crate) fn project_id_for_issue(&self, issue_id: &str) -> Result<String, KernelError> {
        self.loaded_issues
            .iter()
            .find_map(|(project_id, issues)| {
                issues
                    .iter()
                    .any(|issue| issue.id() == issue_id)
                    .then(|| project_id.clone())
            })
            .ok_or_else(|| KernelError::Protocol("unknown issue".into()))
    }
}
