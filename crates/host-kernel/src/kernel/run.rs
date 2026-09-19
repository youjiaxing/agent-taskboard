//! Run lifecycle, launch, and PTY orchestration.

use super::super::*;

pub(crate) struct PreviousRun {
    pub(crate) id: String,
    pub(crate) native_session_id: Option<String>,
    pub(crate) working_directory: String,
    pub(crate) isolated: bool,
}

impl HostKernel {
    pub fn pty_session(&self, run_id: &str) -> Result<Arc<dyn AgentSession>, KernelError> {
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
        self.mark_run_ended(run_id, RunEndedReason::from_exit(code, stopped));
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
        let session = self
            .live
            .get(run_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown run".into()))?;
        session.write(data).map_err(KernelError::Io)
    }

    pub fn resize_pty(&self, run_id: &str, cols: u16, rows: u16) -> Result<(), KernelError> {
        let session = self
            .live
            .get(run_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown run".into()))?;
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
            self.decorate_runs(&self.runs),
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
        if let Some(session) = self.live.get(run_id).cloned() {
            session.stop();
        }
        if self.runs.iter().any(|run| run.id == run_id) {
            self.mark_run_ended(run_id, RunEndedReason::Stopped);
        }
        Ok(())
    }

    pub(crate) fn focus_run(&mut self, run_id: &str) -> Result<(), KernelError> {
        let run = self
            .runs
            .iter()
            .find(|run| run.id == run_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown run".into()))?;
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
            self.mark_run_ended(&id, reason);
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
        let mut became_waiting = Vec::new();
        for (id, is_waiting) in waiting {
            let Some(run) = self.runs.iter_mut().find(|run| run.id == id) else {
                continue;
            };
            if !run.is_active() || run.waiting_for_user == is_waiting {
                continue;
            }
            run.waiting_for_user = is_waiting;
            if is_waiting {
                became_waiting.push(id);
            }
        }
        for id in became_waiting {
            self.pending_events
                .push(HostEvent::Waiting { run_id: id.clone() });
            self.push_notification(NotificationKind::Waiting, &id);
        }
    }

    pub(crate) fn push_notification(&mut self, kind: NotificationKind, run_id: &str) {
        let Some(run) = self.runs.iter().find(|run| run.id == run_id) else {
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
            .find(|run| run.issue_id.as_deref() == Some(issue_id) && run.is_active())
            .or_else(|| {
                self.runs
                    .iter()
                    .rev()
                    .find(|run| run.issue_id.as_deref() == Some(issue_id))
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

    pub(crate) fn mark_run_ended(&mut self, run_id: &str, reason: RunEndedReason) {
        self.harvest_run_signals(run_id);
        let recent_output = self.live.get(run_id).map(|session| session.recent_output());
        let mut issue_id = None;
        let mut project_id = None;
        let mut newly_ended = false;
        if let Some(run) = self.runs.iter_mut().find(|run| run.id == run_id) {
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
        let _ = self.persist_runs();
        if newly_ended {
            if let Some(project_id) = project_id {
                let live = self.refresh_project(&project_id, RefreshTrigger::RunEnded);
                if live {
                    self.consider_auto_advance(run_id);
                }
            }
        }
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
        self.runs.push(result.record);
        self.persist_runs()?;
        Ok(())
    }

    pub(crate) fn start_bound_run(&mut self, issue_id: &str) -> Result<(), KernelError> {
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
            }),
        )
    }

    pub(crate) fn load_persisted_runs(&mut self) -> Vec<String> {
        let path = self.data.host_dir.join("runs.json");
        let Ok(raw) = fs::read_to_string(&path) else {
            return Vec::new();
        };
        let Ok(mut runs) = serde_json::from_str::<Vec<RunSummary>>(&raw) else {
            return Vec::new();
        };
        let mut crashed_ids = Vec::new();
        for run in &mut runs {
            if run.recent_output.contains('\x1b') {
                run.recent_output = session::readable_pty_output(run.recent_output.as_bytes());
            }
            if run.is_active() {
                run.status = RunStatus::Ended;
                run.waiting_for_user = false;
                run.ended_reason = Some(RunEndedReason::Crash);
                crashed_ids.push(run.id.clone());
            }
        }
        self.runs = runs;
        if !crashed_ids.is_empty() {
            let _ = self.persist_runs();
        }
        crashed_ids
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
