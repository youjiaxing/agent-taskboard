use super::super::*;

impl HostKernel {
    pub fn dispatch(&mut self, command: Command) -> Result<CommandOutcome, KernelError> {
        self.observe_live_runs();
        match command {
            Command::HideWindow => self.window_visible = false,
            Command::ShowWindow => {
                if self.process_alive() {
                    self.window_visible = true;
                    self.now_ms = refresh::wall_ms();
                    if self.running {
                        if let Some(project_id) = self.focused_project_id.clone() {
                            self.refresh_project(&project_id, RefreshTrigger::Immediate);
                        }
                    } else if !self.focused_host_id.is_empty() {
                        let focused = self.focused_host_id.clone();
                        let _ = self.refresh_remote_view(&focused);
                    }
                }
            }
            Command::QuitHost => {
                if self.active_run_count() > 0 {
                    self.quit_offer = Some(QuitOffer {
                        active_run_count: self.active_run_count(),
                    });
                } else {
                    self.running = false;
                    self.window_visible = false;
                    self.exiting = true;
                    self.loopback_kind = LoopbackKind::HostNotRunning;
                }
            }
            Command::SetLanguage(language) => {
                let appearance = self.appearance.with_language(language);
                self.persist_client_settings(&appearance)?;
                self.appearance = appearance;
            }
            Command::SetTheme(theme) => {
                let appearance = self.appearance.with_theme(theme);
                self.persist_client_settings(&appearance)?;
                self.appearance = appearance;
            }
            Command::BeginPairingOffer { address } => {
                let address = pairing::parse_http_url(&address).map_err(KernelError::Protocol)?;
                self.pairing_offer = Some(pairing::ActiveOffer::new(address));
            }
            Command::RedeemPairing { code, client_name } => {
                let pairing = self.redeem_pairing(&code, &client_name)?;
                return Ok(self.outcome_with(Some(pairing), None));
            }
            Command::RevokeClient { client_id } => {
                self.paired_clients.retain(|client| client.id != client_id);
                self.persist_host_secrets()?;
            }
            Command::PairRemoteHost { address, code } => {
                self.pair_remote_host(&address, &code)?;
            }
            Command::FocusHost { host_id } => {
                self.focus_host(&host_id)?;
            }
            Command::RegisterProject {
                name,
                local_path,
                github_host,
                repository,
            } => {
                self.register_project(&name, &local_path, &github_host, &repository)?;
            }
            Command::EditProject {
                project_id,
                name,
                local_path,
                github_host,
                repository,
            } => {
                self.edit_project(&project_id, &name, &local_path, &github_host, &repository)?;
            }
            Command::RemoveProject { project_id } => {
                self.remove_project(&project_id)?;
            }
            Command::FocusProject { project_id } => {
                self.usage_open = false;
                self.usage_query.highlighted_run_id = None;
                self.workspace_view = WorkspaceView::Project;
                self.focused_run_id = None;
                self.focus_project(&project_id)?;
            }
            Command::InferProject { local_path } => {
                let inference = self.infer_project(&local_path)?;
                return Ok(self.outcome_with(None, inference));
            }
            Command::FocusIssue { issue_id } => {
                self.focus_issue(&issue_id);
            }
            Command::LoadIssueDocument { issue_id } => {
                self.load_issue_document(&issue_id)?;
            }
            Command::FilterParent { issue_id } => {
                self.parent_filter = Some(issue_id);
            }
            Command::ClearParentFilter => {
                self.parent_filter = None;
            }
            Command::SetCenterView { view } => {
                if view == CenterView::Graph
                    && self.center_view != CenterView::Graph
                    && self.focused_host_id == LOCAL_HOST_ID
                {
                    self.graph_center_issue_id = None;
                    self.complete_dependency_graph = false;
                }
                self.center_view = view;
                self.persist_client_settings(&self.appearance.clone())?;
            }
            Command::CenterDependencyGraph { issue_id } => {
                self.focus_issue(&issue_id);
                self.graph_center_issue_id = Some(issue_id);
            }
            Command::ShowDependencyGraphOverview => {
                self.graph_center_issue_id = None;
                self.complete_dependency_graph = false;
            }
            Command::SetDependencyGraphComplete { complete } => {
                self.complete_dependency_graph = complete;
            }
            Command::SetRecentCompletedLimit { limit } => {
                self.recent_limit = board::clamp_recent_limit(limit);
                self.persist_client_settings(&self.appearance.clone())?;
            }
            Command::RefreshLaunchEnvironment => {
                let status = self.refresh_launch_environment()?;
                let mut outcome = self.outcome();
                outcome.launch_environment = Some(status);
                return Ok(outcome);
            }
            Command::SearchIssues { project_id, search } => {
                if !self.projects.iter().any(|project| project.id == project_id) {
                    return Err(KernelError::Protocol(format!(
                        "unknown project: {project_id}"
                    )));
                }
                self.focused_project_id = Some(project_id.clone());
                self.issue_search.insert(project_id, search);
            }
            Command::Refresh { project_id } => {
                let project_id = project_id
                    .or_else(|| self.focused_project_id.clone())
                    .ok_or_else(|| KernelError::Protocol("missing projectId".into()))?;
                self.refresh_project(&project_id, RefreshTrigger::Immediate);
            }
            Command::Tick { now_ms } => {
                self.now_ms = now_ms.unwrap_or_else(refresh::wall_ms);
                self.expire_stale_client_views();
                self.maybe_refresh_local_markdown();
                self.maybe_auto_refresh();
                self.finish_due_pending();
            }
            Command::SetClientView {
                client_id,
                project_id,
                visible,
            } => {
                self.set_client_view(&client_id, &project_id, visible);
            }
            Command::NoteRunEnded { project_id } => {
                self.refresh_project(&project_id, RefreshTrigger::RunEnded);
            }
            Command::ClaimIssue { issue_id } => {
                self.claim_issue(&issue_id)?;
            }
            Command::ReleaseIssue { issue_id } => {
                self.release_issue(&issue_id)?;
            }
            Command::CreateIssue {
                project_id,
                title,
                body,
            } => {
                self.create_issue(&project_id, &title, &body)?;
            }
            Command::UpdateIssue {
                issue_id,
                title,
                body,
                base_title,
                base_body,
                overwrite_conflict,
            } => {
                self.update_issue(
                    &issue_id,
                    &title,
                    &body,
                    base_title,
                    base_body,
                    overwrite_conflict,
                )?;
            }
            Command::SetIssueOpen { issue_id, open } => {
                self.set_issue_open(&issue_id, open)?;
            }
            Command::AddIssueComment { issue_id, body } => {
                self.add_issue_comment(&issue_id, &body)?;
            }
            Command::SetIssueParent {
                issue_id,
                parent,
                base_parent,
                overwrite_conflict,
            } => {
                self.set_issue_parent(
                    &issue_id,
                    parent.as_deref(),
                    base_parent,
                    overwrite_conflict,
                )?;
            }
            Command::SetIssueBlockedBy {
                issue_id,
                blocked_by,
                base_blocked_by,
                overwrite_conflict,
            } => {
                self.set_issue_blocked_by(
                    &issue_id,
                    &blocked_by,
                    base_blocked_by,
                    overwrite_conflict,
                )?;
            }
            Command::AutoAdvance { project_id } => {
                self.require_live_tracker(&project_id)?;
                self.finish_pending_if_due(&project_id);
            }
            Command::CheckIssueClosed { issue_id } => {
                self.require_live_tracker_for_issue(&issue_id)?;
            }
            Command::StartBoundRun { issue_id } => {
                self.start_bound_run(&issue_id)?;
            }
            Command::ContinueRun { issue_id } => {
                self.continue_run(&issue_id)?;
            }
            Command::PrepareRunLaunch {
                project_id,
                issue_id,
                agent_id,
                pick_agent,
                defer_discovery,
                language,
            } => {
                self.prepare_run_launch(
                    &project_id,
                    issue_id,
                    agent_id,
                    pick_agent,
                    defer_discovery,
                    language,
                )?;
            }
            Command::UpdateRunLaunch {
                project_id,
                config,
                language,
            } => {
                self.update_run_launch(&project_id, config, language)?;
            }
            Command::CancelRunLaunch => {
                self.launch_form = None;
            }
            Command::StartUnboundRun { project_id } => {
                let agent = self.default_agent_for_project(&project_id)?;
                self.start_unbound_run(
                    &project_id,
                    RunLaunchConfig {
                        agent_id: agent.id().to_string(),
                        values: agent.seed_config(),
                        opening_text: String::new(),
                    },
                    None,
                    false,
                    None,
                )?;
            }
            Command::StartUnboundRunWithConfig {
                project_id,
                config,
                issue_id,
            } => {
                self.start_unbound_run(&project_id, config, issue_id, true, None)?;
            }
            Command::SetShowCommandPreview { show } => {
                self.show_command_preview = show;
                self.persist_client_settings(&self.appearance.clone())?;
            }
            Command::SetNotificationPrefs { desktop, sound } => {
                self.notify_desktop = desktop;
                self.notify_sound = sound;
                self.persist_client_settings(&self.appearance.clone())?;
            }
            Command::StopRun { run_id } => {
                self.stop_run(&run_id)?;
            }
            Command::FocusRun { run_id } => {
                self.usage_open = false;
                self.usage_query.highlighted_run_id = None;
                self.focus_run(&run_id)?;
                self.workspace_view = WorkspaceView::Run;
            }
            Command::OpenHostOverview => {
                self.usage_open = false;
                self.usage_query.highlighted_run_id = None;
                self.focused_run_id = None;
                self.workspace_view = WorkspaceView::HostOverview;
            }
            Command::ReturnToBoard => {
                self.usage_open = false;
                self.usage_query.highlighted_run_id = None;
                self.focused_run_id = self
                    .selected_issue_id
                    .clone()
                    .and_then(|issue_id| self.active_run_id_for_issue(&issue_id));
                self.workspace_view = WorkspaceView::Project;
            }
            Command::InjectRunInput { run_id, text } => {
                let mut data = text.into_bytes();
                if !data.ends_with(&[b'\n']) {
                    data.push(b'\n');
                }
                self.write_pty(&run_id, &data)?;
            }
            Command::CancelQuit => {
                self.quit_offer = None;
            }
            Command::ConfirmQuitStopAll => {
                self.stop_all_runs();
                self.quit_offer = None;
                self.running = false;
                self.window_visible = false;
                self.exiting = true;
                self.loopback_kind = LoopbackKind::HostNotRunning;
            }
            Command::SetRefreshInterval { interval_ms } => {
                self.refresh_interval_ms = refresh::clamp_refresh_interval_ms(interval_ms);
                self.persist_host_settings()?;
            }
            Command::WriteChangeNote {
                run_id,
                repo,
                path,
                line,
                text,
            } => {
                self.write_change_note(&run_id, repo, path, line, text)?;
            }
            Command::DeleteChangeNote { note_id } => {
                self.delete_change_note(&note_id)?;
            }
            Command::SetHostAutoAdvance { enabled } => {
                self.set_host_auto_advance(enabled)?;
            }
            Command::SetProjectAutoAdvance {
                project_id,
                enabled,
            } => {
                self.set_project_auto_advance(&project_id, enabled)?;
            }
            Command::SetProjectRestoreAutoAdvance {
                project_id,
                enabled,
            } => {
                self.set_project_restore_auto_advance(&project_id, enabled)?;
            }
            Command::SetProjectRestoreDelay {
                project_id,
                delay_ms,
            } => {
                self.set_project_restore_delay(&project_id, delay_ms)?;
            }
            Command::VetoPendingConfirmation { project_id } => {
                self.veto_pending(&project_id);
            }
            Command::OpenUsage => {
                self.usage_open = true;
            }
            Command::CloseUsage => {
                self.usage_open = false;
                self.usage_query.highlighted_run_id = None;
            }
            Command::SetUsageRange {
                range,
                custom_from_ms,
                custom_to_ms,
            } => {
                self.usage_query.range = range;
                self.usage_query.custom_from_ms = custom_from_ms;
                self.usage_query.custom_to_ms = custom_to_ms;
            }
            Command::SetUsageFilter {
                project_id,
                agent_id,
                model,
            } => {
                self.usage_query.filter.project_id = project_id;
                self.usage_query.filter.agent_id = agent_id;
                self.usage_query.filter.model = model;
            }
            Command::OpenUsageForRun { run_id } => {
                self.open_usage_for_run(&run_id)?;
            }
            Command::OpenRunFromUsage { run_id } => {
                self.open_run_from_usage(&run_id)?;
            }
        }
        Ok(self.outcome())
    }

    pub(crate) fn dispatch_background_tick(
        &mut self,
        now_ms: Option<u64>,
    ) -> Result<ProcessIntent, KernelError> {
        let mut outcome = self.dispatch(Command::Tick { now_ms })?;
        self.pending_events = std::mem::take(&mut outcome.events);
        Ok(outcome.process)
    }
}
