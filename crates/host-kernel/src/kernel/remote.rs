use super::super::*;

pub(crate) struct RemoteCallBatch {
    client_id: String,
    calls: Vec<PreparedRemoteCall>,
}

pub(crate) struct PreparedRemoteCall {
    remote: pairing::RemoteHost,
    request: serde_json::Value,
    client_id: String,
    generation: u64,
}

impl PreparedRemoteCall {
    pub(crate) fn execute(self) -> (Self, Result<serde_json::Value, KernelError>) {
        let result = pairing::post_rpc(
            &self.remote.address,
            Some(&self.remote.token),
            &self.request,
        );
        (self, result)
    }
}

impl HostKernel {
    pub(crate) fn begin_deferred_remote_calls(&mut self, request: &serde_json::Value) {
        debug_assert!(self.deferred_remote_calls.is_none());
        self.deferred_remote_calls = Some(RemoteCallBatch {
            client_id: request
                .get("clientInstanceId")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string(),
            calls: Vec::new(),
        });
    }

    pub(crate) fn take_deferred_remote_calls(&mut self) -> Vec<PreparedRemoteCall> {
        self.deferred_remote_calls
            .take()
            .map(|batch| batch.calls)
            .unwrap_or_default()
    }

    pub(crate) fn defer_remote_call(
        &mut self,
        remote: &pairing::RemoteHost,
        request: &serde_json::Value,
    ) -> bool {
        let Some(batch) = self.deferred_remote_calls.as_mut() else {
            return false;
        };
        let mut request = request.clone();
        if !batch.client_id.is_empty() {
            request["clientInstanceId"] = serde_json::Value::String(batch.client_id.clone());
        }
        self.next_remote_call_generation = self.next_remote_call_generation.saturating_add(1);
        let generation = self.next_remote_call_generation;
        self.remote_call_generations
            .insert((batch.client_id.clone(), remote.id.clone()), generation);
        batch.calls.push(PreparedRemoteCall {
            remote: remote.clone(),
            request,
            client_id: batch.client_id.clone(),
            generation,
        });
        true
    }

    pub(crate) fn finish_remote_call(
        &mut self,
        prepared: PreparedRemoteCall,
        response: &serde_json::Value,
    ) -> Result<bool, KernelError> {
        let key = (prepared.client_id.clone(), prepared.remote.id.clone());
        if self.remote_call_generations.get(&key) != Some(&prepared.generation) {
            return Ok(false);
        }
        self.remote_call_generations.remove(&key);
        if prepared.client_id.is_empty() {
            if self.focused_host_id != prepared.remote.id {
                return Ok(false);
            }
            self.apply_remote_view(&prepared.remote.id, response)?;
            return Ok(true);
        }
        let Some(current) = self.client_navigation.get(&prepared.client_id).cloned() else {
            return Ok(false);
        };
        if current.focused_host_id != prepared.remote.id {
            return Ok(false);
        }
        let previous = self.capture_client_navigation();
        self.apply_client_navigation(current);
        let result = self.apply_remote_view(&prepared.remote.id, response);
        if result.is_ok() {
            self.client_navigation
                .insert(prepared.client_id, self.capture_client_navigation());
        }
        self.apply_client_navigation(previous);
        result.map(|_| true)
    }
}

impl HostKernel {
    pub(crate) fn capture_client_navigation(&self) -> ClientNavigationState {
        ClientNavigationState {
            focused_host_id: self.focused_host_id.clone(),
            remote_view: self.remote_view.clone(),
            focused_project_id: self.focused_project_id.clone(),
            selected_issue_id: self.selected_issue_id.clone(),
            parent_filter: self.parent_filter.clone(),
            issue_search: self.issue_search.clone(),
            center_view: self.center_view,
            workspace_view: self.workspace_view,
            graph_center_issue_id: self.graph_center_issue_id.clone(),
            complete_dependency_graph: self.complete_dependency_graph,
            focused_run_id: self.focused_run_id.clone(),
            launch_form: self.launch_form.clone(),
            usage_open: self.usage_open,
            usage_query: self.usage_query.clone(),
        }
    }

    pub(crate) fn normalize_client_navigation(
        &self,
        mut state: ClientNavigationState,
    ) -> ClientNavigationState {
        let host_known = state.focused_host_id == LOCAL_HOST_ID
            || self
                .remote_hosts
                .iter()
                .any(|host| host.id == state.focused_host_id);
        if !host_known {
            state.focused_host_id = if self.host_mode == HostMode::HostAndClient {
                LOCAL_HOST_ID.to_string()
            } else {
                self.remote_hosts
                    .first()
                    .map(|host| host.id.clone())
                    .unwrap_or_default()
            };
            state.remote_view = None;
        }

        if state.focused_host_id == LOCAL_HOST_ID {
            if state
                .focused_project_id
                .as_ref()
                .is_none_or(|project_id| !self.projects.iter().any(|p| p.id == *project_id))
            {
                state.focused_project_id = self.projects.first().map(|project| project.id.clone());
                state.selected_issue_id = None;
                state.parent_filter = None;
                state.graph_center_issue_id = None;
                state.complete_dependency_graph = false;
            }
            if state
                .focused_run_id
                .as_ref()
                .is_some_and(|run_id| !self.runs.iter().any(|run| run.id == *run_id))
            {
                state.focused_run_id = None;
                if state.workspace_view == WorkspaceView::Run {
                    state.workspace_view = WorkspaceView::Project;
                }
            }
        }
        state
    }

    pub(crate) fn apply_client_navigation(&mut self, state: ClientNavigationState) {
        self.focused_host_id = state.focused_host_id;
        self.remote_view = state.remote_view;
        self.focused_project_id = state.focused_project_id;
        self.selected_issue_id = state.selected_issue_id;
        self.parent_filter = state.parent_filter;
        self.issue_search = state.issue_search;
        self.center_view = state.center_view;
        self.workspace_view = state.workspace_view;
        self.graph_center_issue_id = state.graph_center_issue_id;
        self.complete_dependency_graph = state.complete_dependency_graph;
        self.focused_run_id = state.focused_run_id;
        self.launch_form = state.launch_form;
        self.usage_open = state.usage_open;
        self.usage_query = state.usage_query;
    }

    pub fn pairing_token_valid(&self, token: &str) -> bool {
        let token = token.trim();
        !token.is_empty()
            && self
                .paired_clients
                .iter()
                .any(|client| client.token == token)
    }

    pub(crate) fn persist_client_settings(
        &self,
        appearance: &AppearanceSelection,
    ) -> Result<(), KernelError> {
        let file = ClientSettingsFile {
            language: appearance.language,
            appearance_preference: appearance.appearance_preference,
            focused_host_id: self.focused_host_id.clone(),
            remote_hosts: self
                .remote_hosts
                .iter()
                .map(pairing::RemoteHost::to_saved)
                .collect(),
            recent_completed_limit: self.recent_limit,
            center_view: self.center_view,
            show_command_preview: self.show_command_preview,
            notify_desktop: self.notify_desktop,
            notify_sound: self.notify_sound,
        };
        write_json(&self.data.desktop_client_settings_path, &file)?;
        let secrets = ClientSecretsFile {
            tokens: self
                .remote_hosts
                .iter()
                .map(|host| (host.id.clone(), host.token.clone()))
                .collect(),
        };
        write_json_inner(&self.data.desktop_client_secrets_path, &secrets, true)
    }

    pub(crate) fn refresh_remote_view(&mut self, host_id: &str) -> Result<(), KernelError> {
        self.refresh_remote_view_for_client(host_id, None)
    }

    pub(crate) fn refresh_remote_view_for_client(
        &mut self,
        host_id: &str,
        client_instance_id: Option<&str>,
    ) -> Result<(), KernelError> {
        if host_id == LOCAL_HOST_ID {
            self.remote_view = None;
            return Ok(());
        }
        if self
            .remote_view
            .as_ref()
            .is_some_and(|view| view.host_id != host_id)
        {
            self.remote_view = None;
        }
        let remote = self
            .remote_hosts
            .iter()
            .find(|host| host.id == host_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown host".into()))?;
        let mut request = serde_json::json!({ "op": "snapshot" });
        if let Some(client_instance_id) = client_instance_id {
            request["clientInstanceId"] = serde_json::Value::String(client_instance_id.into());
        }
        if self.defer_remote_call(&remote, &request) {
            return Ok(());
        }
        let response =
            pairing::post_rpc(&remote.address, Some(&remote.token), &request).map_err(|err| {
                match err {
                    KernelError::Io(_) => KernelError::Protocol("address is not reachable".into()),
                    other => other,
                }
            })?;
        self.apply_remote_view(host_id, &response)
    }

    pub(crate) fn apply_remote_view(
        &mut self,
        host_id: &str,
        response: &serde_json::Value,
    ) -> Result<(), KernelError> {
        let snapshot = response
            .get("snapshot")
            .cloned()
            .ok_or_else(|| KernelError::Protocol("remote Host returned no snapshot".into()))?;
        let projects = snapshot
            .get("projects")
            .cloned()
            .map(serde_json::from_value)
            .transpose()?
            .unwrap_or_default();
        let empty_actions = snapshot
            .get("emptyActions")
            .cloned()
            .map(serde_json::from_value)
            .transpose()?
            .unwrap_or_default();
        let focused_project_id = snapshot
            .get("focusedProjectId")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        let board = match snapshot.get("board") {
            Some(value) if !value.is_null() => serde_json::from_value(value.clone())?,
            _ => None,
        };
        let runs = snapshot
            .get("runs")
            .cloned()
            .map(serde_json::from_value)
            .transpose()?
            .unwrap_or_default();
        let focused_run_id = snapshot
            .get("focusedRunId")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        let workspace_view = snapshot
            .get("workspaceView")
            .cloned()
            .map(serde_json::from_value)
            .transpose()?
            .unwrap_or(WorkspaceView::Project);
        let quit_offer = match snapshot.get("quitOffer") {
            Some(value) if !value.is_null() => serde_json::from_value(value.clone())?,
            _ => None,
        };
        self.remote_view = Some(RemoteView {
            host_id: host_id.to_string(),
            projects,
            focused_project_id,
            empty_actions,
            board,
            runs,
            focused_run_id,
            workspace_view,
            quit_offer,
            launch_form: match snapshot.get("launchForm") {
                Some(value) if !value.is_null() => serde_json::from_value(value.clone())?,
                _ => None,
            },
            usage_open: snapshot
                .get("usageOpen")
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
            usage: match snapshot.get("usage") {
                Some(value) if !value.is_null() => serde_json::from_value(value.clone())?,
                _ => usage::build_usage_page(&usage::UsageQuery::default(), 0, 0, &[], &[]),
            },
            refresh_interval_ms: snapshot
                .get("refreshIntervalMs")
                .and_then(|value| value.as_u64())
                .map(refresh::clamp_refresh_interval_ms)
                .unwrap_or(refresh::DEFAULT_REFRESH_INTERVAL_MS),
        });
        Ok(())
    }

    pub(crate) fn pair_remote_host(
        &mut self,
        address: &str,
        code: &str,
    ) -> Result<(), KernelError> {
        let address = pairing::parse_http_url(address).map_err(KernelError::Protocol)?;
        if self.is_own_loopback(&address) {
            return Err(KernelError::Protocol(
                "cannot pair this window to its own Host".into(),
            ));
        }
        let body = serde_json::json!({
            "op": "redeemPairing",
            "code": code,
            "clientName": self.host_display_name,
        });
        let response = pairing::post_rpc(&address, None, &body).map_err(|err| match err {
            KernelError::Io(_) => KernelError::Protocol("address is not reachable".into()),
            other => other,
        })?;
        let pairing = response
            .get("pairing")
            .cloned()
            .ok_or_else(|| KernelError::Denied("invalid pairing code".into()))?;
        let issued: IssuedPairing = serde_json::from_value(pairing)?;
        if issued.token.is_empty() || issued.host_id.is_empty() {
            return Err(KernelError::Denied("invalid pairing code".into()));
        }
        if issued.host_id == LOCAL_HOST_ID {
            return Err(KernelError::Protocol("remote Host id is invalid".into()));
        }
        self.remote_hosts.retain(|host| host.id != issued.host_id);
        self.remote_hosts.push(pairing::RemoteHost {
            id: issued.host_id,
            display_name: issued.display_name,
            address,
            token: issued.token,
        });
        self.persist_client_settings(&self.appearance.clone())
    }

    pub(crate) fn is_own_loopback(&self, address: &str) -> bool {
        if self.loopback_kind != LoopbackKind::Serving {
            return false;
        }
        let rest = address
            .split_once("://")
            .map(|(_, rest)| rest)
            .unwrap_or(address);
        let hostport = rest.split('/').next().unwrap_or(rest);
        let (host, port) = match hostport.rsplit_once(':') {
            Some((host, port)) if port.bytes().all(|byte| byte.is_ascii_digit()) => {
                (host, port.parse::<u16>().ok())
            }
            _ => (hostport, Some(LOCAL_RPC_PORT)),
        };
        let host = host.trim_start_matches('[').trim_end_matches(']');
        matches!(host, "127.0.0.1" | "localhost" | "::1") && port == Some(self.loopback_port)
    }

    pub(crate) fn focus_host(&mut self, host_id: &str) -> Result<(), KernelError> {
        if host_id != LOCAL_HOST_ID && !self.remote_hosts.iter().any(|host| host.id == host_id) {
            return Err(KernelError::Protocol("unknown host".into()));
        }
        self.focused_host_id = host_id.to_string();
        self.refresh_remote_view(host_id)?;
        self.persist_client_settings(&self.appearance.clone())
    }

    pub(crate) fn redeem_pairing(
        &mut self,
        code: &str,
        client_name: &str,
    ) -> Result<IssuedPairing, KernelError> {
        let Some(offer) = &self.pairing_offer else {
            return Err(KernelError::Denied("invalid pairing code".into()));
        };
        if !pairing::codes_match(&offer.code, code) {
            return Err(KernelError::Denied("invalid pairing code".into()));
        }
        let client_name = client_name.trim();
        if client_name.is_empty() {
            return Err(KernelError::Protocol("missing clientName".into()));
        }
        let client = pairing::IssuedClient {
            id: pairing::random_id(),
            name: client_name.to_string(),
            token: pairing::generate_token(),
        };
        let issued = IssuedPairing {
            token: client.token.clone(),
            host_id: self.host_id.clone(),
            display_name: self.host_display_name.clone(),
        };
        self.pairing_offer = None;
        self.paired_clients.push(client);
        self.persist_host_secrets()?;
        Ok(issued)
    }

    pub(crate) fn persist_host_secrets(&self) -> Result<(), KernelError> {
        let github_pats = read_github_pats(&self.data.host_secrets_path)?;
        let file = HostSecretsFile {
            clients: self.paired_clients.clone(),
            github_pats,
        };
        write_json_inner(&self.data.host_secrets_path, &file, true)
    }

    pub(crate) fn persist_host_settings(&self) -> Result<(), KernelError> {
        self.persist_host_settings_state(&self.projects, self.focused_project_id.as_deref())
    }

    pub(crate) fn persist_host_settings_state(
        &self,
        projects: &[ProjectRecord],
        focused_project_id: Option<&str>,
    ) -> Result<(), KernelError> {
        let file = HostSettingsFile {
            id: self.host_id.clone(),
            focused_project_id: focused_project_id.map(ToOwned::to_owned),
            projects: projects.iter().map(ProjectRecord::stored).collect(),
            refresh_interval_ms: self.refresh_interval_ms,
            agent_launch_defaults: self.launch_defaults.clone(),
            last_successful_agent: self.last_successful_agent.clone(),
            auto_advance: self.host_auto_advance,
        };
        write_json(&self.data.host_settings_path, &file)
    }
}
