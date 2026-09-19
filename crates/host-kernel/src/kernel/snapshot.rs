use super::super::*;

impl HostKernel {
    pub fn snapshot(&self) -> HostSnapshot {
        let (projects, focused_project_id, empty_actions) = self.board_for_focus();
        let board = self.current_board(&focused_project_id);
        let (runs, focused_run_id, workspace_view, quit_offer) = self.runs_for_focus();
        HostSnapshot {
            running: self.running,
            window_visible: self.window_visible,
            host_mode: self.host_mode,
            focused_host_id: self.focused_host_id.clone(),
            focused_project_id: focused_project_id.clone(),
            hosts: self.connected_hosts(),
            projects,
            appearance: AppearanceState::from_selection(self.appearance),
            data: self.data.clone(),
            copy: ShellCopy::for_language(self.appearance.language),
            copy_catalog: BTreeMap::from([
                (Language::ZhCn, ShellCopy::for_language(Language::ZhCn)),
                (Language::En, ShellCopy::for_language(Language::En)),
            ]),
            empty_actions,
            loopback_page: self.loopback_page(),
            pairing_offer: self
                .pairing_offer
                .as_ref()
                .map(pairing::ActiveOffer::to_offer),
            paired_clients: self
                .paired_clients
                .iter()
                .map(pairing::IssuedClient::summary)
                .collect(),
            board,
            recent_completed_limit: self.recent_limit,
            refresh_interval_ms: self.refresh_interval_for_focus(),
            center_view: self.center_view,
            workspace_view,
            runs,
            focused_run_id,
            quit_offer,
            launch_form: self.launch_form_for_focus(),
            show_command_preview: self.show_command_preview,
            notify_desktop: self.notify_desktop,
            notify_sound: self.notify_sound,
            auto_advance: self.host_auto_advance,
            pending_confirmation: self
                .pending_advance
                .get(&focused_project_id)
                .map(|pending| pending.to_snapshot(self.now_ms)),
            usage_open: self.usage_open_for_focus(),
            usage: self.usage_for_focus(),
        }
    }

    pub(crate) fn outcome(&mut self) -> CommandOutcome {
        self.outcome_with(None, None)
    }

    pub(crate) fn outcome_with(
        &mut self,
        pairing: Option<IssuedPairing>,
        inference: Option<ProjectInference>,
    ) -> CommandOutcome {
        let view_changes = self.open_view_changes_run_id.take().and_then(|run_id| {
            self.view_changes(Some(&run_id), None, ChangeScope::ThisRound)
                .ok()
        });
        CommandOutcome {
            snapshot: Box::new(self.snapshot()),
            process: if self.process_alive() {
                ProcessIntent::KeepRunning
            } else {
                ProcessIntent::Exit
            },
            pairing,
            inference,
            update_install_gate: None,
            events: std::mem::take(&mut self.pending_events),
            view_changes,
            launch_environment: None,
        }
    }

    pub(crate) fn note_loopback_page(&mut self, kind: LoopbackKind, port: u16) {
        self.loopback_kind = kind;
        self.loopback_port = if port == 0 { LOCAL_RPC_PORT } else { port };
    }

    pub(crate) fn loopback_page(&self) -> LoopbackPage {
        let url = format!("http://127.0.0.1:{}/", self.loopback_port);
        match self.loopback_kind {
            LoopbackKind::Serving => LoopbackPage::Serving { url },
            LoopbackKind::Occupied => LoopbackPage::Occupied {
                url,
                reason: occupied_reason(self.appearance.language, self.loopback_port),
            },
            LoopbackKind::HostNotRunning => LoopbackPage::HostNotRunning {
                url,
                reason: host_not_running_reason(self.appearance.language),
            },
        }
    }
    pub(crate) fn connected_hosts(&self) -> Vec<HostSummary> {
        let mut hosts = if self.host_mode == HostMode::HostAndClient {
            vec![HostSummary {
                id: LOCAL_HOST_ID.to_string(),
                display_name: self.host_display_name.clone(),
                local: true,
            }]
        } else {
            Vec::new()
        };
        hosts.extend(self.remote_hosts.iter().map(|host| HostSummary {
            id: host.id.clone(),
            display_name: host.display_name.clone(),
            local: false,
        }));
        hosts
    }

    pub(crate) fn board_for_focus(&self) -> (Vec<ProjectSummary>, String, Vec<EmptyAction>) {
        if self.focused_host_id != LOCAL_HOST_ID {
            if let Some(view) = &self.remote_view {
                if view.host_id == self.focused_host_id {
                    return (
                        view.projects.clone(),
                        view.focused_project_id.clone(),
                        view.empty_actions.clone(),
                    );
                }
            }
        }
        let empty_actions = if self.projects.is_empty() {
            vec![
                EmptyAction::RegisterFirstProject,
                EmptyAction::PairAnotherHost,
            ]
        } else {
            Vec::new()
        };
        let projects = self
            .projects
            .iter()
            .map(|project| {
                project.summary(
                    self.project_active_run_count(&project.id),
                    self.project_has_execution_stopped(&project.id),
                    self.project_issue_counts(&project.id),
                )
            })
            .collect();
        let focused_project_id = self.focused_project_id.clone().unwrap_or_default();
        (projects, focused_project_id, empty_actions)
    }

    pub(crate) fn project_issue_counts(&self, project_id: &str) -> ProjectIssueCounts {
        board::project_issue_counts(
            self.loaded_issues.get(project_id).map(Vec::as_slice),
            self.refresh
                .get(project_id)
                .is_some_and(|state| state.complete),
        )
    }

    pub(crate) fn refresh_interval_for_focus(&self) -> u64 {
        if self.focused_host_id != LOCAL_HOST_ID {
            if let Some(view) = &self.remote_view {
                if view.host_id == self.focused_host_id {
                    return view.refresh_interval_ms;
                }
            }
        }
        self.refresh_interval_ms
    }

    pub(crate) fn launch_form_for_focus(&self) -> Option<RunLaunchForm> {
        if self.focused_host_id != LOCAL_HOST_ID {
            if let Some(view) = &self.remote_view {
                if view.host_id == self.focused_host_id {
                    return view.launch_form.clone();
                }
            }
        }
        self.launch_form.clone()
    }

    pub(crate) fn current_board(&self, focused_project_id: &str) -> Option<BoardSnapshot> {
        if focused_project_id.is_empty() {
            return None;
        }
        if self.focused_host_id != LOCAL_HOST_ID {
            return self.remote_view.as_ref().and_then(|view| {
                if view.host_id == self.focused_host_id {
                    view.board.clone()
                } else {
                    None
                }
            });
        }
        let loaded = self
            .loaded_issues
            .get(focused_project_id)
            .map(Vec::as_slice);
        let mut board = board::project_board(
            focused_project_id,
            loaded,
            self.refresh
                .get(focused_project_id)
                .is_some_and(|state| state.complete),
            self.parent_filter.as_deref(),
            self.selected_issue_id.as_deref(),
            self.recent_limit,
            self.refresh_status_for(focused_project_id),
            self.graph_center_issue_id.as_deref(),
            self.complete_dependency_graph,
            self.issue_search
                .get(focused_project_id)
                .cloned()
                .unwrap_or_default(),
        );
        if let Some(columns) = board.columns.as_mut() {
            for card in &mut columns.in_progress {
                card.activity = self.issue_activity(&card.id);
                card.run_id = self.run_for_issue(&card.id).map(|run| run.id.clone());
            }
            for card in &mut columns.recently_completed {
                card.run_id = self.run_for_issue(&card.id).map(|run| run.id.clone());
            }
        }
        if let Some(selected) = board.selected.as_mut() {
            selected.document = self
                .issue_documents
                .get(focused_project_id)
                .and_then(|documents| documents.get(&selected.id))
                .cloned()
                .unwrap_or_default();
            selected.active_run_id = self.active_run_id_for_issue(&selected.id);
            selected.execution_stopped = self.execution_stopped(&selected.id);
            selected.waiting_for_user = self.issue_waiting(&selected.id);
        }
        Some(board)
    }
}
