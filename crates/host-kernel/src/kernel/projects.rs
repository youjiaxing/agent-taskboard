use super::super::*;

impl HostKernel {
    pub(crate) fn start_bound_run_with_agent(
        &mut self,
        issue_id: &str,
        agent_id: &str,
    ) -> Result<(), KernelError> {
        let project_id = self.project_id_for_issue(issue_id)?;
        let issue = self
            .issue_by_id(issue_id)
            .ok_or_else(|| KernelError::Protocol("unknown issue".into()))?;
        let agent = self
            .agents
            .iter()
            .find(|agent| agent.id() == agent_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown Agent Adapter".into()))?;
        let mut values = self
            .launch_defaults
            .get(&project_id)
            .and_then(|agents| agents.get(agent.id()))
            .cloned()
            .unwrap_or_else(|| agent.seed_config());
        let opening = launch::bound_opening(&issue, agent.as_ref());
        values.insert(launch::INITIAL_INSTRUCTION.into(), opening.clone());
        self.start_unbound_run(
            &project_id,
            RunLaunchConfig {
                agent_id: agent.id().to_string(),
                values,
                opening_text: opening,
            },
            Some(issue_id.to_string()),
            false,
            None,
        )
    }

    pub(crate) fn forward_if_remote(
        &mut self,
        request: &serde_json::Value,
    ) -> Result<Option<CommandOutcome>, KernelError> {
        if self.focused_host_id == LOCAL_HOST_ID {
            return Ok(None);
        }
        let remote = self
            .remote_hosts
            .iter()
            .find(|host| host.id == self.focused_host_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown host".into()))?;
        let response =
            pairing::post_rpc(&remote.address, Some(&remote.token), request).map_err(|err| {
                match err {
                    KernelError::Io(_) => KernelError::Protocol("address is not reachable".into()),
                    other => other,
                }
            })?;
        let host_id = self.focused_host_id.clone();
        self.apply_remote_view(&host_id, &response)?;
        let mut outcome = self.outcome();
        if let Some(inference) = response.get("inference").cloned() {
            outcome.inference = serde_json::from_value(inference).ok();
        }
        if let Some(view) = response.get("viewChanges").cloned() {
            outcome.view_changes = serde_json::from_value(view).ok();
        }
        Ok(Some(outcome))
    }

    pub(crate) fn register_project(
        &mut self,
        name: &str,
        local_path: &str,
        github_host: &str,
        repository: &str,
    ) -> Result<(), KernelError> {
        let name = project::require_name(name).map_err(KernelError::Protocol)?;
        let local_path =
            project::require_local_directory(local_path).map_err(KernelError::Protocol)?;
        let github_host =
            project::normalize_github_host(github_host).map_err(KernelError::Protocol)?;
        let repository =
            project::normalize_repository(repository).map_err(KernelError::Protocol)?;
        if self
            .projects
            .iter()
            .any(|project| project::same_local_directory(&project.local_path, &local_path))
        {
            return Err(KernelError::Protocol(
                "a Project is already registered for this directory".into(),
            ));
        }
        let tracker_kind = tracker_kind_for_host(&github_host);
        let connection = self.probe_tracker(tracker_kind, &github_host, &repository);
        let record = ProjectRecord {
            id: pairing::random_id(),
            name,
            local_path,
            tracker: tracker_kind,
            github_host,
            repository,
            connection,
            tracker_synced: false,
            auto_advance: false,
            restore_auto_advance: false,
            restore_delay_ms: advance::DEFAULT_RESTORE_DELAY_MS,
            advance_ready_at_ms: None,
        };
        let project_id = record.id.clone();
        let mut projects = self.projects.clone();
        projects.push(record);
        self.persist_host_settings_state(&projects, Some(&project_id))?;
        self.projects = projects;
        self.focused_project_id = Some(project_id.clone());
        self.selected_issue_id = None;
        self.graph_center_issue_id = None;
        self.complete_dependency_graph = false;
        self.parent_filter = None;
        self.refresh_project(&project_id, RefreshTrigger::Immediate);
        Ok(())
    }

    pub(crate) fn edit_project(
        &mut self,
        project_id: &str,
        name: &str,
        local_path: &str,
        github_host: &str,
        repository: &str,
    ) -> Result<(), KernelError> {
        let name = project::require_name(name).map_err(KernelError::Protocol)?;
        let local_path =
            project::require_local_directory(local_path).map_err(KernelError::Protocol)?;
        let github_host =
            project::normalize_github_host(github_host).map_err(KernelError::Protocol)?;
        let repository =
            project::normalize_repository(repository).map_err(KernelError::Protocol)?;
        if self.project_has_active_run(project_id) {
            let current = self
                .projects
                .iter()
                .find(|project| project.id == project_id)
                .ok_or_else(|| KernelError::Protocol("unknown project".into()))?;
            if current.local_path != local_path
                || current.github_host != github_host
                || current.repository != repository
            {
                return Err(KernelError::Denied(
                    "cannot change the directory or GitHub connection while a Project has an active Run".into(),
                ));
            }
        }
        if self.projects.iter().any(|project| {
            project.id != project_id
                && project::same_local_directory(&project.local_path, &local_path)
        }) {
            return Err(KernelError::Protocol(
                "a Project is already registered for this directory".into(),
            ));
        }
        let current = self
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .ok_or_else(|| KernelError::Protocol("unknown project".into()))?;
        let registration_changed = current.local_path != local_path
            || current.github_host != github_host
            || current.repository != repository;
        let tracker_kind = tracker_kind_for_host(&github_host);
        let connection = registration_changed
            .then(|| self.probe_tracker(tracker_kind, &github_host, &repository));
        let mut projects = self.projects.clone();
        let project = projects
            .iter_mut()
            .find(|project| project.id == project_id)
            .expect("validated project");
        project.name = name;
        if registration_changed {
            project.local_path = local_path;
            project.tracker = tracker_kind;
            project.github_host = github_host;
            project.repository = repository;
            project.connection = connection.expect("changed registration connection");
            project.tracker_synced = false;
        }
        self.persist_host_settings_state(&projects, self.focused_project_id.as_deref())?;
        self.projects = projects;
        if registration_changed {
            self.loaded_issues.remove(project_id);
            self.issue_documents.remove(project_id);
            self.issue_document_in_flight
                .retain(|(pending_project, _), _| pending_project != project_id);
            self.refresh.remove(project_id);
            self.issue_write_generations.remove(project_id);
            self.local_tracker_revisions.remove(project_id);
            refresh::remove_project_data(&self.data.host_dir, project_id)?;
            if self.focused_project_id.as_deref() == Some(project_id) {
                self.selected_issue_id = None;
                self.graph_center_issue_id = None;
                self.complete_dependency_graph = false;
                self.parent_filter = None;
                self.refresh_project(project_id, RefreshTrigger::Immediate);
            }
        }
        Ok(())
    }

    pub(crate) fn remove_project(&mut self, project_id: &str) -> Result<(), KernelError> {
        let index = self
            .projects
            .iter()
            .position(|project| project.id == project_id)
            .ok_or_else(|| KernelError::Protocol("unknown project".into()))?;
        if self.project_has_active_run(project_id) {
            return Err(KernelError::Denied(
                "cannot remove a Project with an active Run".into(),
            ));
        }
        let was_current = self.focused_project_id.as_deref() == Some(project_id);
        let mut projects = self.projects.clone();
        projects.remove(index);
        let focused_project_id = if was_current {
            if projects.is_empty() {
                None
            } else {
                Some(projects[index.min(projects.len() - 1)].id.clone())
            }
        } else {
            self.focused_project_id.clone()
        };
        self.persist_host_settings_state(&projects, focused_project_id.as_deref())?;

        self.projects = projects;
        self.focused_project_id = focused_project_id;
        self.refresh.remove(project_id);
        self.local_tracker_revisions.remove(project_id);
        self.loaded_issues.remove(project_id);
        self.issue_documents.remove(project_id);
        self.issue_document_in_flight
            .retain(|(pending_project, _), _| pending_project != project_id);
        self.clear_pending(project_id, false);
        if was_current {
            self.selected_issue_id = None;
            self.graph_center_issue_id = None;
            self.complete_dependency_graph = false;
            self.parent_filter = None;
            if let Some(next_id) = self.focused_project_id.clone() {
                self.refresh_project(&next_id, RefreshTrigger::Immediate);
            }
        }
        Ok(())
    }

    pub(crate) fn focus_project(&mut self, project_id: &str) -> Result<(), KernelError> {
        if !self.projects.iter().any(|project| project.id == project_id) {
            return Err(KernelError::Protocol("unknown project".into()));
        }
        self.focused_project_id = Some(project_id.to_string());
        self.selected_issue_id = None;
        self.graph_center_issue_id = None;
        self.complete_dependency_graph = false;
        self.parent_filter = None;
        self.refresh_project(project_id, RefreshTrigger::Immediate);
        self.persist_host_settings()
    }

    pub(crate) fn infer_project(
        &self,
        local_path: &str,
    ) -> Result<Option<ProjectInference>, KernelError> {
        let local_path =
            project::require_local_directory(local_path).map_err(KernelError::Protocol)?;
        Ok(project::infer_github_project(&local_path))
    }
}

pub(crate) fn probe_record(
    stored: StoredProject,
    tracker: &dyn TrackerSeam,
    secrets_path: &Path,
    language: Language,
) -> ProjectRecord {
    let pat = read_github_pat(secrets_path, &stored.github_host);
    let outcome = tracker.probe(&tracker::ProbeContext {
        tracker: stored.tracker,
        github_host: &stored.github_host,
        repository: &stored.repository,
        secrets_pat: pat.as_deref(),
        secrets_path,
    });
    let connection = connection_from_probe(outcome, secrets_path, language, &stored.github_host);
    ProjectRecord {
        id: stored.id,
        name: stored.name,
        local_path: stored.local_path,
        tracker: stored.tracker,
        github_host: stored.github_host,
        repository: stored.repository,
        connection,
        tracker_synced: false,
        auto_advance: stored.auto_advance,
        restore_auto_advance: stored.restore_auto_advance,
        restore_delay_ms: stored.restore_delay_ms,
        advance_ready_at_ms: None,
    }
}

pub(crate) fn connection_from_probe(
    outcome: tracker::ProbeOutcome,
    secrets_path: &Path,
    language: Language,
    github_host: &str,
) -> ProjectConnection {
    match outcome {
        tracker::ProbeOutcome::Ready { source } => ProjectConnection::Ready { source },
        tracker::ProbeOutcome::Failed {
            source,
            kind: AuthFailureKind::Unreachable,
            cli_detected,
            detail,
        } => ProjectConnection::Unreachable {
            source,
            repair: tracker::repair_hint(cli_detected, secrets_path),
            message: auth_failure_message(
                language,
                AuthFailureKind::Unreachable,
                detail.as_deref(),
                github_host,
            ),
        },
        tracker::ProbeOutcome::Failed {
            source,
            kind,
            cli_detected,
            detail,
        } => ProjectConnection::AuthFailed {
            source,
            kind,
            repair: tracker::repair_hint(cli_detected, secrets_path),
            message: auth_failure_message(language, kind, detail.as_deref(), github_host),
        },
    }
}

pub(crate) fn auth_failure_message(
    language: Language,
    kind: AuthFailureKind,
    detail: Option<&str>,
    github_host: &str,
) -> String {
    if github_host == "local" {
        let base = match language {
            Language::ZhCn => "本地 Markdown tracker 不可用。".to_string(),
            Language::En => "Local Markdown tracker is unavailable.".to_string(),
        };
        return match detail {
            Some(detail) if !detail.is_empty() => format!("{base} {detail}"),
            _ => base,
        };
    }
    let base = match (language, kind) {
        (Language::ZhCn, AuthFailureKind::MissingCredentials) => {
            "没有可用的 GitHub 凭据。".to_string()
        }
        (Language::En, AuthFailureKind::MissingCredentials) => {
            "No GitHub credentials are available.".to_string()
        }
        (Language::ZhCn, AuthFailureKind::Rejected) => "GitHub 拒绝了当前凭据。".to_string(),
        (Language::En, AuthFailureKind::Rejected) => {
            "GitHub rejected the current credentials.".to_string()
        }
        (Language::ZhCn, AuthFailureKind::Unreachable) => "连不上这个 GitHub host。".to_string(),
        (Language::En, AuthFailureKind::Unreachable) => {
            "This GitHub host could not be reached.".to_string()
        }
    };
    match detail {
        Some(detail) if !detail.is_empty() => format!("{base} {detail}"),
        _ => base,
    }
}
