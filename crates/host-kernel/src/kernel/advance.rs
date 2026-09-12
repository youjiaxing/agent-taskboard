//! Completion signals, self-checks, and optional automatic advancement.

use super::super::*;

impl HostKernel {
    fn auto_advance_allowed(&self, project_id: &str) -> bool {
        if !self.host_auto_advance {
            return false;
        }
        self.projects
            .iter()
            .find(|project| project.id == project_id)
            .is_some_and(|project| {
                project.auto_advance
                    && project
                        .advance_ready_at_ms
                        .is_some_and(|ready| self.now_ms >= ready)
            })
    }

    pub(crate) fn arm_cold_start(&mut self) {
        let boot = self.now_ms;
        let host_on = self.host_auto_advance;
        for project in &mut self.projects {
            project.advance_ready_at_ms =
                if host_on && project.auto_advance && project.restore_auto_advance {
                    Some(boot.saturating_add(project.restore_delay_ms))
                } else {
                    None
                };
        }
    }

    fn arm_project_now(&mut self, project_id: &str) {
        let now = self.now_ms;
        let allowed = self.host_auto_advance
            && self
                .projects
                .iter()
                .find(|project| project.id == project_id)
                .is_some_and(|project| project.auto_advance);
        if let Some(project) = self
            .projects
            .iter_mut()
            .find(|project| project.id == project_id)
        {
            project.advance_ready_at_ms = allowed.then_some(now);
        }
        if !allowed {
            self.clear_pending(project_id, false);
        }
    }

    pub(crate) fn set_host_auto_advance(&mut self, enabled: bool) -> Result<(), KernelError> {
        self.host_auto_advance = enabled;
        let ids: Vec<String> = self
            .projects
            .iter()
            .map(|project| project.id.clone())
            .collect();
        for id in ids {
            self.arm_project_now(&id);
        }
        self.persist_host_settings()
    }

    pub(crate) fn set_project_auto_advance(
        &mut self,
        project_id: &str,
        enabled: bool,
    ) -> Result<(), KernelError> {
        let project = self
            .projects
            .iter_mut()
            .find(|project| project.id == project_id)
            .ok_or_else(|| KernelError::Protocol("unknown project".into()))?;
        project.auto_advance = enabled;
        self.arm_project_now(project_id);
        self.persist_host_settings()
    }

    pub(crate) fn set_project_restore_auto_advance(
        &mut self,
        project_id: &str,
        enabled: bool,
    ) -> Result<(), KernelError> {
        let project = self
            .projects
            .iter_mut()
            .find(|project| project.id == project_id)
            .ok_or_else(|| KernelError::Protocol("unknown project".into()))?;
        project.restore_auto_advance = enabled;
        self.persist_host_settings()
    }

    pub(crate) fn set_project_restore_delay(
        &mut self,
        project_id: &str,
        delay_ms: u64,
    ) -> Result<(), KernelError> {
        let project = self
            .projects
            .iter_mut()
            .find(|project| project.id == project_id)
            .ok_or_else(|| KernelError::Protocol("unknown project".into()))?;
        project.restore_delay_ms = advance::clamp_restore_delay_ms(delay_ms);
        self.persist_host_settings()
    }

    pub(crate) fn veto_pending(&mut self, project_id: &str) {
        self.clear_pending(project_id, false);
    }

    pub(crate) fn clear_pending(&mut self, project_id: &str, advanced: bool) {
        if let Some(pending) = self.pending_advance.remove(project_id) {
            self.pending_events
                .push(HostEvent::PendingConfirmationEnded {
                    project_id: pending.project_id,
                    issue_id: pending.issue_id,
                    run_id: pending.run_id,
                    advanced,
                });
        }
    }

    fn begin_pending(&mut self, run: &RunSummary, issue_id: &str) {
        let pending = advance::PendingAdvance {
            project_id: run.project_id.clone(),
            issue_id: issue_id.to_string(),
            run_id: run.id.clone(),
            agent_id: run.agent_id.clone(),
            deadline_ms: self.now_ms.saturating_add(advance::PENDING_CONFIRM_MS),
        };
        self.pending_events
            .push(HostEvent::PendingConfirmationStarted {
                project_id: pending.project_id.clone(),
                issue_id: pending.issue_id.clone(),
                run_id: pending.run_id.clone(),
            });
        self.pending_advance.insert(run.project_id.clone(), pending);
    }

    pub(crate) fn finish_due_pending(&mut self) {
        let due: Vec<String> = self
            .pending_advance
            .iter()
            .filter(|(_, pending)| pending.deadline_ms <= self.now_ms)
            .map(|(project_id, _)| project_id.clone())
            .collect();
        for project_id in due {
            self.finish_pending_if_due(&project_id);
        }
    }

    pub(crate) fn finish_pending_if_due(&mut self, project_id: &str) {
        let Some(pending) = self.pending_advance.get(project_id).cloned() else {
            return;
        };
        if pending.deadline_ms > self.now_ms {
            return;
        }
        if !self.refresh_project(project_id, RefreshTrigger::Action) {
            self.clear_pending(project_id, false);
            return;
        }
        let next = self.next_auto_pool(project_id);
        if let Some(issue_id) = next {
            match self.start_bound_run_with_agent(&issue_id, &pending.agent_id) {
                Ok(()) => self.clear_pending(project_id, true),
                Err(_) => self.clear_pending(project_id, false),
            }
        } else {
            self.clear_pending(project_id, false);
        }
    }

    fn next_auto_pool(&self, project_id: &str) -> Option<String> {
        self.loaded_issues
            .get(project_id)?
            .iter()
            .find(|issue| advance::in_auto_pool(issue))
            .map(IssueRecord::id)
    }

    pub(crate) fn harvest_live_signals(&mut self) {
        let ids: Vec<String> = self.live.keys().cloned().collect();
        for id in ids {
            self.harvest_run_signals(&id);
        }
    }

    pub(crate) fn harvest_run_signals(&mut self, run_id: &str) {
        let session_signals = self
            .live
            .get(run_id)
            .map(|session| session.completion_signals())
            .unwrap_or_default();
        let (hook_dir, agent_id) = self
            .runs
            .iter()
            .find(|run| run.id == run_id)
            .map(|run| (run.hook_dir.clone(), run.agent_id.clone()))
            .unwrap_or((None, String::new()));
        let file_signals = hook_dir
            .as_ref()
            .and_then(|dir| {
                self.agents
                    .iter()
                    .find(|agent| agent.id() == agent_id)
                    .map(|agent| agent.read_completion_signals(dir))
            })
            .unwrap_or_default();
        if let Some(run) = self.runs.iter_mut().find(|run| run.id == run_id) {
            run.session_end |= session_signals.session_end || file_signals.session_end;
            run.stop_failure |= session_signals.stop_failure || file_signals.stop_failure;
        }
    }

    pub(crate) fn consider_auto_advance(&mut self, run_id: &str) {
        let Some(run) = self.runs.iter().find(|run| run.id == run_id).cloned() else {
            return;
        };
        let Some(issue_id) = run.issue_id.clone() else {
            return;
        };
        if matches!(
            run.ended_reason,
            Some(RunEndedReason::Stopped | RunEndedReason::Crash)
        ) {
            return;
        }
        if !self.auto_advance_allowed(&run.project_id) || !run.hooks_attached {
            return;
        }
        let issue_closed = self.issue_by_id(&issue_id).is_some_and(|issue| !issue.open);
        let process_ok = run.ended_reason == Some(RunEndedReason::Exited);
        let normal = advance::normal_completion(
            issue_closed,
            run.hooks_attached,
            run.session_end,
            run.stop_failure,
            process_ok,
        );
        if run.self_check {
            if normal {
                self.begin_pending(&run, &issue_id);
            }
            return;
        }
        if normal {
            self.begin_pending(&run, &issue_id);
            return;
        }
        if issue_closed {
            self.open_view_changes_run_id = Some(run.id.clone());
            return;
        }
        self.launch_self_check_run(&run, &issue_id);
    }

    fn launch_self_check_run(&mut self, previous: &RunSummary, issue_id: &str) {
        if previous.self_check {
            return;
        }
        if let Some(run) = self.runs.iter_mut().find(|run| run.id == previous.id) {
            run.self_check_attempted = true;
        }
        let agent = match self
            .agents
            .iter()
            .find(|agent| agent.id() == previous.agent_id)
            .cloned()
        {
            Some(agent) => agent,
            None => return,
        };
        let values = self
            .launch_defaults
            .get(&previous.project_id)
            .and_then(|agents| agents.get(&previous.agent_id))
            .cloned()
            .unwrap_or_else(|| agent.seed_config());
        let opening = advance::self_check_text(self.appearance.language);
        if self
            .start_unbound_run(
                &previous.project_id,
                RunLaunchConfig {
                    agent_id: previous.agent_id.clone(),
                    values,
                    opening_text: opening,
                },
                Some(issue_id.to_string()),
                false,
                Some(kernel::run::PreviousRun {
                    id: previous.id.clone(),
                    native_session_id: previous.native_session_id.clone(),
                    working_directory: previous.working_directory.clone(),
                    isolated: previous.isolated,
                }),
            )
            .is_err()
        {
            return;
        }
        if let Some(run) = self
            .runs
            .iter_mut()
            .rev()
            .find(|run| run.issue_id.as_deref() == Some(issue_id))
        {
            run.self_check = true;
            run.self_check_attempted = true;
        }
    }

    pub(crate) fn maybe_inject_self_check(&mut self, run_id: &str) {
        let Some(run) = self.runs.iter().find(|run| run.id == run_id).cloned() else {
            return;
        };
        if !run.is_active() || !run.stop_failure || run.self_check_attempted {
            return;
        }
        if !self.auto_advance_allowed(&run.project_id) || !run.hooks_attached {
            return;
        }
        if !self.refresh_project(&run.project_id, RefreshTrigger::Action) {
            return;
        }
        if run
            .issue_id
            .as_ref()
            .and_then(|issue_id| self.issue_by_id(issue_id))
            .is_some_and(|issue| !issue.open)
        {
            return;
        }
        if let Some(run) = self.runs.iter_mut().find(|run| run.id == run_id) {
            run.self_check = true;
            run.self_check_attempted = true;
        }
        let text = run::submitted_input(&advance::self_check_text(self.appearance.language));
        if self.write_pty(run_id, &text).is_ok() {
            return;
        }
        let previous = self
            .runs
            .iter()
            .find(|run| run.id == run_id)
            .cloned()
            .unwrap_or(run);
        self.mark_run_ended(run_id, RunEndedReason::Abnormal);
        if let Some(issue_id) = previous.issue_id.clone() {
            self.launch_self_check_run(&previous, &issue_id);
        }
    }
}
