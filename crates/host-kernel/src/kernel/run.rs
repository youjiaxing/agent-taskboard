//! Run lifecycle, launch, and PTY orchestration.

use super::super::*;

pub(crate) struct PreviousRun {
    pub(crate) id: String,
    pub(crate) native_session_id: Option<String>,
    pub(crate) working_directory: String,
    pub(crate) isolated: bool,
    pub(crate) self_check: bool,
}

impl HostKernel {
    pub fn pty_session(&self, run_id: &str) -> Result<Arc<dyn AgentSession>, KernelError> {
        if self
            .runs
            .iter()
            .any(|run| run.id == run_id && run.is_archived())
        {
            return Err(KernelError::Denied("run is archived".into()));
        }
        self.live
            .get(run_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown run".into()))
    }

    pub fn pty_output(
        &self,
        run_id: &str,
        after: usize,
        wait: Duration,
    ) -> Result<PtyChunk, KernelError> {
        Ok(self.pty_session(run_id)?.read_after(after, wait))
    }

    pub fn note_run_exit(&mut self, run_id: &str, code: i32) {
        let stopped = self
            .live
            .get(run_id)
            .is_some_and(|session| session.was_stopped());
        let _ = self.mark_run_ended(run_id, RunEndedReason::from_exit(code, stopped));
    }

    pub fn update_install_gate(&mut self) -> UpdateInstallGate {
        self.observe_live_runs();
        let active_run_count = self.active_run_count();
        UpdateInstallGate {
            allowed: active_run_count == 0,
            active_run_count,
        }
    }

    pub fn begin_client_only_switch(&mut self) -> UpdateInstallGate {
        let gate = self.update_install_gate();
        if gate.allowed {
            self.update_installing = true;
        }
        gate
    }

    pub fn write_pty(&self, run_id: &str, data: &[u8]) -> Result<(), KernelError> {
        let session = self.pty_session(run_id)?;
        session.write(data).map_err(KernelError::Io)
    }

    pub fn resize_pty(&self, run_id: &str, cols: u16, rows: u16) -> Result<(), KernelError> {
        let session = self.pty_session(run_id)?;
        session.resize(cols, rows);
        Ok(())
    }

    pub(crate) fn runs_for_focus(
        &self,
    ) -> (Vec<RunSummary>, String, WorkspaceView, Option<QuitOffer>) {
        if self.focused_host_id != LOCAL_HOST_ID {
            if let Some(view) = &self.remote_view {
                if view.host_id == self.focused_host_id {
                    return (
                        view.runs.clone(),
                        view.focused_run_id.clone(),
                        view.workspace_view,
                        view.quit_offer.clone(),
                    );
                }
            }
        }
        (
            self.decorate_runs(
                &self
                    .runs
                    .iter()
                    .filter(|run| !run.is_archived())
                    .cloned()
                    .collect::<Vec<_>>(),
            ),
            self.focused_run_id.clone().unwrap_or_default(),
            self.workspace_view,
            self.quit_offer.clone(),
        )
    }

    pub(crate) fn default_agent_for_project(
        &self,
        project_id: &str,
    ) -> Result<Arc<dyn AgentPort>, KernelError> {
        let project = self
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .ok_or_else(|| KernelError::Protocol("unknown project".into()))?;
        let summaries = launch::summarize_agents(
            &self.agents,
            self.launch_env.as_ref(),
            &project.local_path,
            self.appearance.language,
        );
        let last = self.last_successful_agent.get(project_id).cloned();
        let selected = launch::default_agent_id(&summaries, last.as_deref(), None);
        self.agents
            .iter()
            .find(|agent| agent.id() == selected)
            .cloned()
            .ok_or_else(|| KernelError::Denied("choose an Agent before starting".into()))
    }

    pub(crate) fn refresh_launch_environment(
        &mut self,
    ) -> Result<LaunchEnvironmentStatus, KernelError> {
        let directories = if self.projects.is_empty() {
            std::env::var_os("HOME")
                .or_else(|| std::env::var_os("USERPROFILE"))
                .map(PathBuf::from)
                .into_iter()
                .collect::<Vec<_>>()
        } else {
            self.projects
                .iter()
                .map(|project| project.local_path.clone())
                .collect::<Vec<_>>()
        };
        if directories.is_empty() {
            return Ok(LaunchEnvironmentStatus {
                status: "ready",
                refreshed_directories: 0,
                message: None,
            });
        }

        let mut failures = Vec::new();
        let mut refreshed = 0;
        for directory in directories {
            match self.launch_env.refresh(&directory) {
                Ok(_) => refreshed += 1,
                Err(err) => failures.push(format!("{}: {err}", directory.display())),
            }
        }
        self.agent_config_cache.clear();
        if failures.is_empty() {
            Ok(LaunchEnvironmentStatus {
                status: "ready",
                refreshed_directories: refreshed,
                message: None,
            })
        } else {
            Err(KernelError::Denied(failures.join("\n")))
        }
    }

    pub(crate) fn prepare_run_launch(
        &mut self,
        project_id: &str,
        issue_id: Option<String>,
        agent_id: Option<String>,
        pick_agent: bool,
        defer_discovery: bool,
        language: Language,
    ) -> Result<(), KernelError> {
        let project = self
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown project".into()))?;
        let agents = launch::summarize_agents(
            &self.agents,
            self.launch_env.as_ref(),
            &project.local_path,
            language,
        );
        let last = self.last_successful_agent.get(project_id).cloned();
        let selected = launch::default_agent_id(&agents, last.as_deref(), agent_id.as_deref());
        if selected.is_empty() {
            self.launch_form = Some(RunLaunchForm {
                project_id: project_id.to_string(),
                issue_id,
                agents,
                selected_agent_id: String::new(),
                skip_agent_picker: false,
                fields: Vec::new(),
                values: BTreeMap::new(),
                prefill_source: PrefillSource::CliSeed,
                working_directory: project.local_path.display().to_string(),
                isolation_supported: false,
                isolation_reason: String::new(),
                opening_text: String::new(),
                change_notes_text: String::new(),
                command_preview: String::new(),
                intents: launch::intent_options(language),
                warnings: Vec::new(),
                error: None,
                option_discovery_pending: None,
                option_discovery_error: None,
            });
            return Ok(());
        }
        let agent = self
            .agents
            .iter()
            .find(|agent| agent.id() == selected)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown Agent Adapter".into()))?;
        let (discovery, option_discovery_pending, option_discovery_error) = if defer_discovery {
            (
                AgentConfigDiscovery {
                    fields: agent.config_fields(),
                    seed: agent.seed_config(),
                },
                Some(launch::option_discovery_pending(language)),
                None,
            )
        } else {
            let (discovery, error) =
                self.agent_config_for(&project.local_path, agent.as_ref(), language);
            (discovery, None, error)
        };
        let current = self
            .launch_defaults
            .get(project_id)
            .and_then(|agents| agents.get(&selected));
        let other = launch::other_project_memory(&self.launch_defaults, project_id, &selected);
        let (mut values, prefill_source) = launch::merge_prefill(&discovery.seed, current, other);
        let mut opening_text = String::new();
        if let Some(issue_id) = issue_id.as_deref() {
            if let Some(issue) = self.issue_by_id(issue_id) {
                let instruction = launch::bound_opening(&issue, agent.as_ref());
                values.insert(launch::INITIAL_INSTRUCTION.into(), instruction.clone());
                opening_text = instruction;
            }
        }
        let pending = changes::pending_notes(&self.change_notes, project_id, issue_id.as_deref());
        let change_notes_text = changes::format_notes(&pending);
        opening_text = changes::append_notes(&opening_text, &pending);
        let fields = launch::localize_fields(discovery.fields, language);
        values.insert(launch::ISOLATION_FIELD.into(), "false".into());
        launch::apply_option_defaults(&fields, &mut values);
        let (isolation_supported, isolation_reason) =
            launch::isolation_availability(agent.as_ref(), &project.local_path, language);
        let preview = launch::command_preview(&launch::preview_argv(agent.as_ref(), &values));
        let skip_agent_picker = !pick_agent && (last.is_some() || agent_id.is_some());
        let mut warnings = launch::unknown_enum_warnings(&fields, &values, language);
        warnings.extend(launch::side_effect_warnings(
            &project.local_path,
            self.runs
                .iter()
                .any(|run| run.project_id == project_id && run.is_active()),
            language,
        ));
        self.launch_form = Some(RunLaunchForm {
            project_id: project_id.to_string(),
            issue_id,
            agents,
            selected_agent_id: selected,
            skip_agent_picker,
            fields: fields.clone(),
            values: values.clone(),
            prefill_source,
            working_directory: project.local_path.display().to_string(),
            isolation_supported,
            isolation_reason,
            opening_text,
            change_notes_text,
            command_preview: preview,
            intents: launch::intent_options(language),
            warnings,
            error: None,
            option_discovery_pending,
            option_discovery_error,
        });
        Ok(())
    }

    pub(crate) fn remember_launch(
        &mut self,
        project_id: &str,
        config: &RunLaunchConfig,
    ) -> Result<(), KernelError> {
        let remembered = launch::remembered_values(&config.values);
        self.launch_defaults
            .entry(project_id.to_string())
            .or_default()
            .insert(config.agent_id.clone(), remembered);
        self.last_successful_agent
            .insert(project_id.to_string(), config.agent_id.clone());
        self.persist_host_settings()
    }

    pub(crate) fn stop_run(&mut self, run_id: &str) -> Result<(), KernelError> {
        self.ensure_run_persistence_writable()?;
        let run = self
            .runs
            .iter()
            .find(|run| run.id == run_id)
            .ok_or_else(|| KernelError::Protocol("unknown run".into()))?;
        if run.is_archived() {
            return Err(KernelError::Denied("run is archived".into()));
        }
        if let Some(session) = self.live.get(run_id).cloned() {
            session.stop();
        }
        self.mark_run_ended(run_id, RunEndedReason::Stopped)
    }

    pub(crate) fn focus_run(&mut self, run_id: &str) -> Result<(), KernelError> {
        let run = self
            .runs
            .iter()
            .find(|run| run.id == run_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown run".into()))?;
        if run.is_archived() {
            return Err(KernelError::Denied("run is archived".into()));
        }
        self.focused_run_id = Some(run.id);
        if self
            .projects
            .iter()
            .any(|project| project.id == run.project_id)
        {
            self.focused_project_id = Some(run.project_id);
        }
        if let Some(issue_id) = run.issue_id {
            self.selected_issue_id = Some(issue_id);
        }
        Ok(())
    }

    pub(crate) fn set_run_pinned(&mut self, run_id: &str, pinned: bool) -> Result<(), KernelError> {
        self.ensure_run_persistence_writable()?;
        let position = self
            .runs
            .iter()
            .position(|run| run.id == run_id)
            .ok_or_else(|| KernelError::Protocol("unknown run".into()))?;
        let run = &self.runs[position];
        if pinned && run.is_archived() {
            return Err(KernelError::Denied("archived run cannot be pinned".into()));
        }
        if (pinned && run.pinned_at_ms.is_some()) || (!pinned && run.pinned_at_ms.is_none()) {
            return Ok(());
        }
        let mut runs = self.runs.clone();
        runs[position].pinned_at_ms = pinned.then_some(self.now_ms);
        self.commit_run_records(runs)
    }

    pub(crate) fn archive_run(&mut self, run_id: &str) -> Result<(), KernelError> {
        self.ensure_run_persistence_writable()?;
        let position = self
            .runs
            .iter()
            .position(|run| run.id == run_id)
            .ok_or_else(|| KernelError::Protocol("unknown run".into()))?;
        let run = &self.runs[position];
        if run.is_archived() {
            return Ok(());
        }
        if run.status != RunStatus::Ended {
            return Err(KernelError::Denied(
                "only ended runs can be archived".into(),
            ));
        }
        let mut runs = self.runs.clone();
        runs[position].pinned_at_ms = None;
        runs[position].archived_at_ms = Some(self.now_ms);
        self.commit_run_records(runs)?;
        self.clear_run_navigation(run_id);
        Ok(())
    }

    pub(crate) fn prepare_restore_run(
        &self,
        run_id: &str,
    ) -> Result<RunRestoreResult, KernelError> {
        self.ensure_run_persistence_writable()?;
        let run = self
            .runs
            .iter()
            .find(|run| run.id == run_id)
            .ok_or_else(|| KernelError::Protocol("unknown run".into()))?;
        if !run.is_archived() {
            return Ok(RunRestoreResult::Restored {
                run_id: run.id.clone(),
                project_id: run.project_id.clone(),
                project_recreated: false,
            });
        }
        Ok(self.restore_run_preflight(run))
    }

    pub(crate) fn restore_run(
        &mut self,
        run_id: &str,
        confirm_project_recreate: bool,
        expected_tombstone_revision: Option<&str>,
    ) -> Result<RunRestoreResult, KernelError> {
        self.ensure_run_persistence_writable()?;
        let position = self
            .runs
            .iter()
            .position(|run| run.id == run_id)
            .ok_or_else(|| KernelError::Protocol("unknown run".into()))?;
        let run = self.runs[position].clone();
        if !run.is_archived() {
            return Ok(RunRestoreResult::Restored {
                run_id: run.id,
                project_id: run.project_id,
                project_recreated: false,
            });
        }

        let preflight = self.restore_run_preflight(&run);
        match preflight {
            RunRestoreResult::Ready { .. } => {
                self.unarchive_run(position)?;
                Ok(RunRestoreResult::Restored {
                    run_id: run.id,
                    project_id: run.project_id,
                    project_recreated: false,
                })
            }
            RunRestoreResult::ProjectRecreateRequired { tombstone, .. } => {
                if !confirm_project_recreate {
                    return Ok(RunRestoreResult::ProjectRecreateRequired {
                        run_id: run.id,
                        project_id: run.project_id,
                        tombstone,
                    });
                }
                if expected_tombstone_revision != Some(tombstone.revision.as_str()) {
                    return Ok(RunRestoreResult::Conflict {
                        run_id: run.id,
                        project_id: run.project_id,
                        reason: RunRestoreConflictReason::TombstoneRevisionChanged,
                        tombstone: Some(tombstone),
                        conflicting_project_id: None,
                    });
                }

                self.recreate_project_from_tombstone(
                    &run.project_id,
                    expected_tombstone_revision.expect("matched revision"),
                )?;

                self.unarchive_run(position)?;
                Ok(RunRestoreResult::Restored {
                    run_id: run.id,
                    project_id: run.project_id,
                    project_recreated: true,
                })
            }
            conflict @ RunRestoreResult::Conflict { .. } => Ok(conflict),
            RunRestoreResult::Restored { .. } => unreachable!("archived run preflight"),
        }
    }

    fn restore_run_preflight(&self, run: &RunSummary) -> RunRestoreResult {
        let project = self
            .projects
            .iter()
            .find(|project| project.id == run.project_id);
        let tombstone = self
            .project_tombstones
            .iter()
            .find(|tombstone| tombstone.id == run.project_id);
        if project.is_some() && tombstone.is_some() {
            return RunRestoreResult::Conflict {
                run_id: run.id.clone(),
                project_id: run.project_id.clone(),
                reason: RunRestoreConflictReason::ActiveProjectTombstoneConflict,
                tombstone: tombstone.map(project_tombstone_summary),
                conflicting_project_id: Some(run.project_id.clone()),
            };
        }
        if project.is_some() {
            return RunRestoreResult::Ready {
                run_id: run.id.clone(),
                project_id: run.project_id.clone(),
            };
        }
        let Some(tombstone) = tombstone else {
            return RunRestoreResult::Conflict {
                run_id: run.id.clone(),
                project_id: run.project_id.clone(),
                reason: RunRestoreConflictReason::TombstoneMissing,
                tombstone: None,
                conflicting_project_id: None,
            };
        };
        let summary = project_tombstone_summary(tombstone);
        if !tombstone.local_path.is_dir() {
            return RunRestoreResult::Conflict {
                run_id: run.id.clone(),
                project_id: run.project_id.clone(),
                reason: RunRestoreConflictReason::DirectoryMissing,
                tombstone: Some(summary),
                conflicting_project_id: None,
            };
        }
        if let Some(conflicting) = self.projects.iter().find(|project_record| {
            project::same_local_directory(&project_record.local_path, &tombstone.local_path)
        }) {
            return RunRestoreResult::Conflict {
                run_id: run.id.clone(),
                project_id: run.project_id.clone(),
                reason: RunRestoreConflictReason::DirectoryInUse,
                tombstone: Some(summary),
                conflicting_project_id: Some(conflicting.id.clone()),
            };
        }
        RunRestoreResult::ProjectRecreateRequired {
            run_id: run.id.clone(),
            project_id: run.project_id.clone(),
            tombstone: summary,
        }
    }

    fn unarchive_run(&mut self, position: usize) -> Result<(), KernelError> {
        let mut runs = self.runs.clone();
        runs[position].archived_at_ms = None;
        self.commit_run_records(runs)
    }

    pub(crate) fn list_archived_runs(&self, project_id: Option<&str>) -> Vec<RunSummary> {
        let mut runs = self
            .runs
            .iter()
            .filter(|run| {
                run.is_archived()
                    && project_id.is_none_or(|project_id| run.project_id == project_id)
            })
            .cloned()
            .collect::<Vec<_>>();
        runs.sort_by(|left, right| {
            right
                .archived_at_ms
                .cmp(&left.archived_at_ms)
                .then(right.started_at_ms.cmp(&left.started_at_ms))
                .then(right.id.cmp(&left.id))
        });
        self.decorate_runs(&runs)
    }

    pub(crate) fn clear_run_navigation(&mut self, run_id: &str) {
        clear_local_run_navigation(
            &mut self.focused_run_id,
            &mut self.workspace_view,
            &mut self.usage_query,
            run_id,
        );
        clear_local_client_navigation(&mut self.client_navigation_seed, run_id);
        for state in self.client_navigation.values_mut() {
            clear_local_client_navigation(state, run_id);
        }
        if self.open_view_changes_run_id.as_deref() == Some(run_id) {
            self.open_view_changes_run_id = None;
        }
        self.pending_events.retain(|event| {
            !matches!(event, HostEvent::Notification { run_id: event_run_id, .. } if event_run_id == run_id)
        });
    }

    pub(crate) fn stop_all_runs(&mut self) {
        let ids = self
            .runs
            .iter()
            .filter(|run| run.is_active())
            .map(|run| run.id.clone())
            .collect::<Vec<_>>();
        for id in ids {
            let _ = self.stop_run(&id);
        }
    }

    pub(crate) fn reap_runs(&mut self) {
        let ended = self
            .live
            .iter()
            .filter_map(|(id, session)| {
                session.exit_code().map(|code| {
                    (
                        id.clone(),
                        RunEndedReason::from_exit(code, session.was_stopped()),
                    )
                })
            })
            .collect::<Vec<_>>();
        for (id, reason) in ended {
            let _ = self.mark_run_ended(&id, reason);
        }
    }

    pub(crate) fn observe_live_runs(&mut self) {
        self.discover_isolated_directories();
        self.ingest_telemetry();
        self.harvest_live_signals();
        let stop_failures = self
            .runs
            .iter()
            .filter(|run| run.is_active() && run.stop_failure && !run.self_check_attempted)
            .map(|run| run.id.clone())
            .collect::<Vec<_>>();
        for run_id in stop_failures {
            self.maybe_inject_self_check(&run_id);
        }
        self.reap_runs();
        let waiting = self
            .live
            .iter()
            .map(|(id, session)| (id.clone(), session.waiting_for_user()))
            .collect::<Vec<_>>();
        let mut runs = self.runs.clone();
        let mut became_waiting = Vec::new();
        let mut changed = false;
        for (id, is_waiting) in waiting {
            let Some(run) = runs.iter_mut().find(|run| run.id == id) else {
                continue;
            };
            if !run.is_active() || run.waiting_for_user == is_waiting {
                continue;
            }
            run.waiting_for_user = is_waiting;
            changed = true;
            if is_waiting {
                became_waiting.push(id);
            }
        }
        if changed {
            if self.commit_run_records(runs).is_err() {
                return;
            }
        }
        for id in became_waiting {
            self.pending_events
                .push(HostEvent::Waiting { run_id: id.clone() });
            self.push_notification(NotificationKind::Waiting, &id);
        }
    }

    pub(crate) fn push_notification(&mut self, kind: NotificationKind, run_id: &str) {
        let Some(run) = self
            .runs
            .iter()
            .find(|run| run.id == run_id && !run.is_archived())
        else {
            return;
        };
        self.pending_events.push(HostEvent::Notification {
            kind,
            run_id: run.id.clone(),
            issue_id: run.issue_id.clone(),
            project_id: run.project_id.clone(),
        });
    }

    pub(crate) fn issue_waiting(&self, issue_id: &str) -> bool {
        self.runs.iter().any(|run| {
            run.issue_id.as_deref() == Some(issue_id) && run.is_active() && run.waiting_for_user
        })
    }

    pub(crate) fn run_for_issue(&self, issue_id: &str) -> Option<&RunSummary> {
        self.runs
            .iter()
            .rev()
            .find(|run| {
                run.issue_id.as_deref() == Some(issue_id) && run.is_active() && !run.is_archived()
            })
            .or_else(|| {
                self.runs
                    .iter()
                    .rev()
                    .find(|run| run.issue_id.as_deref() == Some(issue_id) && !run.is_archived())
            })
    }

    pub(crate) fn issue_activity(&self, issue_id: &str) -> Option<IssueActivity> {
        if self.issue_waiting(issue_id) {
            Some(IssueActivity::Waiting)
        } else if self.active_run_id_for_issue(issue_id).is_some() {
            Some(IssueActivity::Running)
        } else if self.execution_stopped(issue_id) {
            Some(IssueActivity::ExecutionStopped)
        } else {
            None
        }
    }

    pub(crate) fn mark_run_ended(
        &mut self,
        run_id: &str,
        reason: RunEndedReason,
    ) -> Result<(), KernelError> {
        self.ensure_run_persistence_writable()?;
        self.harvest_run_signals(run_id)?;
        let recent_output = self.live.get(run_id).map(|session| session.recent_output());
        let mut issue_id = None;
        let mut project_id = None;
        let mut newly_ended = false;
        let mut runs = self.runs.clone();
        if let Some(run) = runs.iter_mut().find(|run| run.id == run_id) {
            if run.status != RunStatus::Ended {
                run.status = RunStatus::Ended;
                run.waiting_for_user = false;
                run.ended_reason = Some(reason);
                if let Some(output) = &recent_output {
                    run.recent_output = output.clone();
                }
                issue_id = run.issue_id.clone();
                project_id = Some(run.project_id.clone());
                newly_ended = true;
            }
        }
        if newly_ended {
            self.commit_run_records(runs)?;
        }
        if newly_ended {
            self.pending_events.push(HostEvent::RunStatusChanged {
                run_id: run_id.to_string(),
                status: RunStatus::Ended,
            });
            match reason {
                RunEndedReason::Exited => {
                    self.push_notification(NotificationKind::Completed, run_id);
                }
                RunEndedReason::Abnormal => {
                    self.push_notification(NotificationKind::AbnormalStop, run_id);
                }
                RunEndedReason::Stopped | RunEndedReason::Crash => {}
            }
        }
        if !newly_ended && !self.runs.iter().any(|run| run.id == run_id) {
            return Err(KernelError::Protocol("unknown run".into()));
        }
        self.live.remove(run_id);
        if let Some(issue_id) = issue_id {
            if self.execution_stopped(&issue_id) {
                self.pending_events.push(HostEvent::ExecutionStopped {
                    issue_id,
                    run_id: run_id.to_string(),
                });
            }
        }
        let active = self.active_run_count();
        if active == 0 {
            self.quit_offer = None;
        } else if let Some(offer) = &mut self.quit_offer {
            offer.active_run_count = active;
        }
        if newly_ended {
            if let Some(project_id) = project_id {
                let live = self.refresh_project(&project_id, RefreshTrigger::RunEnded);
                if live {
                    self.consider_auto_advance(run_id);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn project_has_active_run(&self, project_id: &str) -> bool {
        self.project_active_run_count(project_id) > 0
    }

    pub(crate) fn project_active_run_count(&self, project_id: &str) -> u32 {
        self.runs
            .iter()
            .filter(|run| run.project_id == project_id && run.is_active())
            .count() as u32
    }

    pub(crate) fn project_has_execution_stopped(&self, project_id: &str) -> bool {
        self.loaded_issues
            .get(project_id)
            .into_iter()
            .flatten()
            .any(|issue| self.execution_stopped(&issue.id()))
    }

    pub(crate) fn active_run_id_for_issue(&self, issue_id: &str) -> Option<String> {
        self.runs
            .iter()
            .find(|run| run.issue_id.as_deref() == Some(issue_id) && run.is_active())
            .map(|run| run.id.clone())
    }

    pub(crate) fn last_bound_run(&self, issue_id: &str) -> Option<&RunSummary> {
        self.runs
            .iter()
            .rev()
            .find(|run| run.issue_id.as_deref() == Some(issue_id))
    }

    pub(crate) fn execution_stopped(&self, issue_id: &str) -> bool {
        let claimed = self
            .issue_by_id(issue_id)
            .is_some_and(|issue| issue.claimed());
        if !claimed || self.active_run_id_for_issue(issue_id).is_some() {
            return false;
        }
        self.last_bound_run(issue_id)
            .and_then(|run| run.ended_reason)
            .is_some_and(RunEndedReason::execution_stopped)
    }

    pub(crate) fn active_run_count(&self) -> u32 {
        self.runs.iter().filter(|run| run.is_active()).count() as u32
    }

    pub(crate) fn update_run_launch(
        &mut self,
        project_id: &str,
        config: RunLaunchConfig,
        language: Language,
    ) -> Result<(), KernelError> {
        let form = self
            .launch_form
            .as_ref()
            .filter(|form| form.project_id == project_id)
            .ok_or_else(|| KernelError::Protocol("no launch form".into()))?;
        if form.selected_agent_id != config.agent_id {
            return Err(KernelError::Protocol("launch form Agent changed".into()));
        }
        let fields = form.fields.clone();
        let project_dir = self
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .map(|project| project.local_path.clone())
            .ok_or_else(|| KernelError::Protocol("unknown project".into()))?;
        let agent = self
            .agents
            .iter()
            .find(|agent| agent.id() == config.agent_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown Agent Adapter".into()))?;
        let mut warnings = launch::unknown_enum_warnings(&fields, &config.values, language);
        warnings.extend(launch::side_effect_warnings(
            &project_dir,
            self.runs
                .iter()
                .any(|run| run.project_id == project_id && run.is_active()),
            language,
        ));
        let preview =
            launch::command_preview(&launch::preview_argv(agent.as_ref(), &config.values));
        let form = self.launch_form.as_mut().expect("checked launch form");
        launch::apply_submitted_form(form, &config);
        form.warnings = warnings;
        form.command_preview = preview;
        Ok(())
    }

    pub(crate) fn start_unbound_run(
        &mut self,
        project_id: &str,
        mut config: RunLaunchConfig,
        issue_id: Option<String>,
        from_form: bool,
        previous: Option<PreviousRun>,
    ) -> Result<(), KernelError> {
        self.ensure_run_persistence_writable()?;
        if self.update_installing {
            return Err(KernelError::Denied("update install is starting".into()));
        }
        let project_dir = self
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .map(|project| project.local_path.clone())
            .ok_or_else(|| KernelError::Protocol("unknown project".into()))?;
        let agent = self
            .agents
            .iter()
            .find(|agent| agent.id() == config.agent_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown Agent Adapter".into()))?;
        let language = self.appearance.language;
        let (supported, _) = launch::isolation_availability(agent.as_ref(), &project_dir, language);
        let mut cwd = project_dir.clone();
        let mut isolation_note = None;
        let mut isolate = false;
        if let Some(previous) = &previous {
            if self
                .runs
                .iter()
                .any(|run| run.id == previous.id && self.isolation_directory_unconfirmed(run))
            {
                return Err(KernelError::Denied(
                    super::isolation::pending_directory_note(language),
                ));
            }
            config.values.remove(launch::ISOLATION_FIELD);
            if previous.isolated {
                let recorded = PathBuf::from(&previous.working_directory);
                if !previous.working_directory.is_empty() && recorded.exists() {
                    cwd = recorded;
                } else {
                    isolation_note = Some(launch::isolation_missing_tree_note(language));
                }
            }
        } else {
            isolate = launch::isolation_requested(&config.values) && supported;
            if !isolate {
                config
                    .values
                    .insert(launch::ISOLATION_FIELD.into(), "false".into());
            }
        }
        if isolate
            && self.runs.iter().any(|run| {
                run.is_active()
                    && run.isolation_pending.is_some()
                    && self.projects.iter().any(|project| {
                        project.id == run.project_id
                            && launch::same_path(&project.local_path, &project_dir)
                    })
            })
        {
            return Err(KernelError::Denied(
                super::isolation::pending_directory_note(language),
            ));
        }
        let fields = self
            .launch_form
            .as_ref()
            .filter(|form| form.selected_agent_id == config.agent_id)
            .map(|form| form.fields.clone())
            .unwrap_or_else(|| launch::localize_fields(agent.config_fields(), language));
        if let Some(form) = &mut self.launch_form {
            launch::apply_submitted_form(form, &config);
            let mut warnings = launch::unknown_enum_warnings(&fields, &config.values, language);
            warnings.extend(launch::side_effect_warnings(
                &project_dir,
                self.runs
                    .iter()
                    .any(|run| run.project_id == project_id && run.is_active()),
                language,
            ));
            form.warnings = warnings;
            form.command_preview =
                launch::command_preview(&launch::preview_argv(agent.as_ref(), &config.values));
        }
        if from_form {
            if let Some(err) = launch::missing_required(&fields, &config.values, language)
                .or_else(|| launch::opening_required(&config.opening_text, language))
            {
                if let Some(form) = &mut self.launch_form {
                    form.error = Some(err);
                    return Ok(());
                }
                return Err(KernelError::Protocol(err));
            }
        }
        if !from_form {
            let pending =
                changes::pending_notes(&self.change_notes, project_id, issue_id.as_deref());
            config.opening_text = changes::append_notes(&config.opening_text, &pending);
        }
        let (previous_run_id, resume_session_id) = match &previous {
            Some(previous) => (
                Some(previous.id.clone()),
                previous.native_session_id.clone(),
            ),
            None => (None, None),
        };
        if let Some(issue_id) = issue_id.as_deref() {
            if self.active_run_id_for_issue(issue_id).is_some() {
                return Err(KernelError::Denied(
                    "issue already has an active Run".into(),
                ));
            }
            if previous_run_id.is_none() {
                if let Err(err) = self.require_live_tracker_for_issue(issue_id) {
                    if let Some(form) = &mut self.launch_form {
                        form.error = Some(err.to_string());
                        return Ok(());
                    }
                    return Err(err);
                }
                if let Err(err) = self.claim_issue(issue_id) {
                    if let Some(form) = &mut self.launch_form {
                        form.error = Some(err.to_string());
                        return Ok(());
                    }
                    return Err(err);
                }
            }
        }
        let provisional_claim = issue_id
            .as_deref()
            .filter(|_| previous_run_id.is_none())
            .map(ToOwned::to_owned);
        let before = if isolate {
            launch::git_worktrees(&project_dir)
        } else {
            Vec::new()
        };
        let early_baselines = if !cwd.exists() {
            None
        } else {
            Some(changes::record_baselines(&cwd))
        };
        let mut hook_plan = None;
        let mut hook_dir = None;
        if issue_id.is_some() && agent.completion_hooks_supported() {
            let dir = self
                .data
                .host_dir
                .join("projects")
                .join(project_id)
                .join("hooks")
                .join(pairing::random_id());
            if let Ok(mut plan) = agent.attach_completion_hooks(&dir, &project_dir) {
                plan.extra_env
                    .entry("AGENT_TASKBOARD_HOOK_SINK".into())
                    .or_insert_with(|| dir.to_string_lossy().into_owned());
                hook_dir = Some(dir);
                hook_plan = Some(plan);
            }
        }
        let mut result = run::start_unbound(
            project_id,
            &cwd,
            agent.as_ref(),
            self.launch_env.as_ref(),
            self.sessions.as_ref(),
            language,
            &[],
            &config,
            issue_id.as_deref(),
            previous_run_id.as_deref(),
            resume_session_id.as_deref(),
            hook_plan.as_ref(),
        );
        result.record.hook_dir = hook_dir;
        result.record.hooks_attached = hook_plan.is_some();
        let using_recorded_tree = previous.as_ref().is_some_and(|previous| previous.isolated)
            && isolation_note.is_none()
            && cwd != project_dir;
        result.record.working_directory = cwd.display().to_string();
        result.record.isolated = (isolate && result.session.is_some()) || using_recorded_tree;
        result.record.isolation_note = isolation_note;
        if isolate && result.session.is_some() {
            if let Some(tree) = agent
                .isolation_tree_after_launch(&project_dir, &before)
                .or_else(|| launch::new_git_worktree(&project_dir, &before))
                .filter(|tree| !self.isolation_tree_conflicts(&result.record, tree))
            {
                result.record.working_directory = tree.display().to_string();
            } else {
                result.record.isolation_pending = Some(before);
                result.record.isolation_note =
                    Some(super::isolation::pending_directory_note(language));
            }
        }
        result.record.started_at_ms = self.now_ms;
        result.record.git_baselines = early_baselines.unwrap_or_default();
        if isolate {
            result
                .record
                .git_baselines
                .retain(|baseline| launch::same_path(Path::new(&baseline.path), &project_dir));
            for baseline in &mut result.record.git_baselines {
                baseline.path = result.record.working_directory.clone();
                baseline.display_path = result.record.working_directory.clone();
            }
        }
        let mut runs = self.runs.clone();
        if previous
            .as_ref()
            .is_some_and(|previous| previous.self_check)
        {
            if let Some(previous) = runs
                .iter_mut()
                .find(|run| Some(run.id.as_str()) == previous_run_id.as_deref())
            {
                previous.self_check_attempted = true;
            }
            result.record.self_check = true;
            result.record.self_check_attempted = true;
        }
        runs.push(result.record.clone());
        if let Err(err) = self.persist_run_records(&runs) {
            self.note_run_persistence_write_error(&err);
            if let Some(session) = result.session.as_ref() {
                session.stop();
            }
            if let Some(issue_id) = provisional_claim.as_deref() {
                let _ = self.release_issue(issue_id);
            }
            if let Some(form) = &mut self.launch_form {
                form.error = Some(err.to_string());
            }
            return Err(err);
        }
        self.runs = runs;
        self.run_persistence_write_error = None;
        self.focused_run_id = Some(result.record.id.clone());
        self.pending_events.push(HostEvent::RunStatusChanged {
            run_id: result.record.id.clone(),
            status: result.record.status,
        });
        if let Some(session) = result.session {
            self.live.insert(result.record.id.clone(), session);
            self.remember_launch(project_id, &config)?;
            self.launch_form = None;
            self.clear_pending_notes(project_id, issue_id.as_deref())?;
        } else {
            if let Some(issue_id) = provisional_claim.as_deref() {
                let _ = self.release_issue(issue_id);
            }
            if let Some(form) = &mut self.launch_form {
                form.error = result.record.failure.clone();
            }
        }
        Ok(())
    }

    pub(crate) fn start_bound_run(&mut self, issue_id: &str) -> Result<(), KernelError> {
        self.ensure_run_persistence_writable()?;
        let project_id = self.project_id_for_issue(issue_id)?;
        let issue = self
            .issue_by_id(issue_id)
            .ok_or_else(|| KernelError::Protocol("unknown issue".into()))?;
        let agent = self.default_agent_for_project(&project_id)?;
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

    pub(crate) fn continue_run(&mut self, issue_id: &str) -> Result<(), KernelError> {
        self.ensure_run_persistence_writable()?;
        if !self.execution_stopped(issue_id) {
            return Err(KernelError::Denied("issue is not execution-stopped".into()));
        }
        let last = self
            .last_bound_run(issue_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown run".into()))?;
        let agent = self
            .agents
            .iter()
            .find(|agent| agent.id() == last.agent_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown Agent Adapter".into()))?;
        let values = self
            .launch_defaults
            .get(&last.project_id)
            .and_then(|agents| agents.get(&last.agent_id))
            .cloned()
            .unwrap_or_else(|| agent.seed_config());
        self.start_unbound_run(
            &last.project_id,
            RunLaunchConfig {
                agent_id: last.agent_id.clone(),
                values,
                opening_text: String::new(),
            },
            Some(issue_id.to_string()),
            false,
            Some(PreviousRun {
                id: last.id.clone(),
                native_session_id: last.native_session_id.clone(),
                working_directory: last.working_directory.clone(),
                isolated: last.isolated,
                self_check: false,
            }),
        )
    }

    pub(crate) fn load_persisted_runs(&mut self) -> Vec<String> {
        let was_recovering = self.run_persistence_recovery.is_some();
        let mut runs = match self.read_persisted_runs() {
            Ok(Some(runs)) => runs,
            Ok(None) if !was_recovering => {
                self.run_persistence_recovery = None;
                return Vec::new();
            }
            Ok(None) => {
                self.set_run_persistence_recovery(
                    RunPersistenceFailureKind::Unreadable,
                    "runs.json is missing".into(),
                );
                return Vec::new();
            }
            Err(recovery) => {
                self.run_persistence_recovery = Some(recovery);
                return Vec::new();
            }
        };
        let mut crashed_ids = Vec::new();
        let mut persisted_changes = false;
        for run in &mut runs {
            if run.recent_output.contains('\x1b') {
                run.recent_output = session::readable_pty_output(run.recent_output.as_bytes());
            }
            if run.is_active() {
                run.status = RunStatus::Ended;
                run.waiting_for_user = false;
                run.ended_reason = Some(RunEndedReason::Crash);
                crashed_ids.push(run.id.clone());
                persisted_changes = true;
            }
            if !run.is_archived()
                && !self
                    .projects
                    .iter()
                    .any(|project| project.id == run.project_id)
            {
                run.pinned_at_ms = None;
                run.archived_at_ms = Some(self.now_ms);
                persisted_changes = true;
            }
        }
        if persisted_changes {
            if let Err(err) = self.persist_run_records(&runs) {
                self.set_run_persistence_recovery(
                    RunPersistenceFailureKind::WriteFailed,
                    err.to_string(),
                );
                return Vec::new();
            }
        }
        self.runs = runs;
        self.run_persistence_recovery = None;
        self.run_persistence_write_error = None;
        crashed_ids
    }

    pub(crate) fn retry_run_persistence_load(&mut self) -> Vec<String> {
        if self.run_persistence_recovery.is_none() {
            return Vec::new();
        }
        self.load_persisted_runs()
    }

    fn read_persisted_runs(&self) -> Result<Option<Vec<RunSummary>>, RunPersistenceRecovery> {
        let path = self.runs_path();
        let raw = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(err) => {
                return Err(run_persistence_recovery(
                    RunPersistenceFailureKind::Unreadable,
                    err.to_string(),
                ))
            }
        };
        let value = serde_json::from_str::<serde_json::Value>(&raw).map_err(|err| {
            run_persistence_recovery(RunPersistenceFailureKind::InvalidJson, err.to_string())
        })?;
        let runs = serde_json::from_value::<Vec<RunSummary>>(value).map_err(|err| {
            run_persistence_recovery(RunPersistenceFailureKind::InvalidRunRecord, err.to_string())
        })?;
        Ok(Some(runs))
    }

    fn set_run_persistence_recovery(&mut self, kind: RunPersistenceFailureKind, detail: String) {
        self.run_persistence_recovery = Some(run_persistence_recovery(kind, detail));
    }

    pub(crate) fn note_crash_recovery(&mut self, crashed_ids: Vec<String>) {
        if crashed_ids.is_empty() {
            return;
        }
        self.pending_events.push(HostEvent::HostCrashedRecovered {
            run_ids: crashed_ids.clone(),
        });
        for run_id in crashed_ids {
            self.push_notification(NotificationKind::CrashRecovered, &run_id);
        }
    }
}

fn run_persistence_recovery(
    kind: RunPersistenceFailureKind,
    detail: String,
) -> RunPersistenceRecovery {
    RunPersistenceRecovery {
        kind,
        detail,
        retry_operation: "retryRunPersistenceLoad".into(),
        writes_blocked: true,
    }
}

fn project_tombstone_summary(tombstone: &StoredProjectTombstone) -> ProjectTombstoneSummary {
    ProjectTombstoneSummary {
        project_id: tombstone.id.clone(),
        name: tombstone.name.clone(),
        local_path: tombstone.local_path.clone(),
        tracker: tombstone.tracker,
        github_host: tombstone.github_host.clone(),
        repository: tombstone.repository.clone(),
        revision: tombstone.revision.clone(),
    }
}

fn clear_local_client_navigation(state: &mut ClientNavigationState, run_id: &str) {
    if state.focused_host_id != LOCAL_HOST_ID {
        return;
    }
    clear_local_run_navigation(
        &mut state.focused_run_id,
        &mut state.workspace_view,
        &mut state.usage_query,
        run_id,
    );
}

fn clear_local_run_navigation(
    focused_run_id: &mut Option<String>,
    workspace_view: &mut WorkspaceView,
    usage_query: &mut usage::UsageQuery,
    run_id: &str,
) {
    if focused_run_id.as_deref() == Some(run_id) {
        *focused_run_id = None;
        if *workspace_view == WorkspaceView::Run {
            *workspace_view = WorkspaceView::Project;
        }
    }
    if usage_query.highlighted_run_id.as_deref() == Some(run_id) {
        usage_query.highlighted_run_id = None;
    }
}
