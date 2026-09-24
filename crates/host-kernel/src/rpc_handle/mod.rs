use crate::*;

mod board;
mod host;
mod run;
mod usage;

impl HostKernel {
    pub fn handle(&mut self, request: serde_json::Value) -> Result<CommandOutcome, KernelError> {
        let focuses_project =
            request.get("op").and_then(|value| value.as_str()) == Some("focusProject");
        let client_instance_id = request
            .get("clientInstanceId")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let Some(client_instance_id) = client_instance_id else {
            return self.handle_active_request(request);
        };

        let previous = self.capture_client_navigation();
        let wanted = self
            .client_navigation
            .get(&client_instance_id)
            .cloned()
            .unwrap_or_else(|| self.client_navigation_seed.clone());
        let wanted = self.normalize_client_navigation(wanted);
        self.apply_client_navigation(wanted);

        let result = self.handle_active_request(request);
        let current = self.normalize_client_navigation(self.capture_client_navigation());
        if focuses_project && result.is_ok() {
            if let Some(project_id) = current.focused_project_id.clone() {
                self.client_views.insert(
                    client_instance_id.clone(),
                    ClientView {
                        project_id,
                        visible: true,
                        last_seen_ms: self.now_ms,
                    },
                );
            }
        }
        self.client_navigation.insert(client_instance_id, current);
        let previous = self.normalize_client_navigation(previous);
        self.apply_client_navigation(previous);
        result
    }

    fn handle_active_request(
        &mut self,
        request: serde_json::Value,
    ) -> Result<CommandOutcome, KernelError> {
        let op = request
            .get("op")
            .and_then(|value| value.as_str())
            .unwrap_or("snapshot")
            .to_string();
        match op.as_str() {
            "beginPairingOffer"
            | "beginUpdateInstall"
            | "cancelUpdateInstall"
            | "focusHost"
            | "hideWindow"
            | "pairRemoteHost"
            | "quitHost"
            | "redeemPairing"
            | "revokeClient"
            | "setAppearancePreference"
            | "setClientView"
            | "setHostAutoAdvance"
            | "setLanguage"
            | "setNotificationPrefs"
            | "setShowCommandPreview"
            | "showWindow"
            | "snapshot"
            | "updateInstallGate" => self.handle_host_rpc(&op, request),
            "addIssueComment"
            | "centerDependencyGraph"
            | "claimIssue"
            | "clearParentFilter"
            | "createIssue"
            | "editProject"
            | "filterParent"
            | "focusIssue"
            | "focusProject"
            | "inferProject"
            | "loadIssueDocument"
            | "refresh"
            | "registerProject"
            | "releaseIssue"
            | "removeProject"
            | "searchIssues"
            | "setCenterView"
            | "setDependencyGraphComplete"
            | "setIssueBlockedBy"
            | "setIssueOpen"
            | "setIssueParent"
            | "setRecentCompletedLimit"
            | "setShowClosedGraphContext"
            | "showDependencyGraphOverview"
            | "tick"
            | "updateIssue" => self.handle_board_rpc(&op, request),
            "autoAdvance"
            | "cancelQuit"
            | "cancelRunLaunch"
            | "checkIssueClosed"
            | "confirmQuitStopAll"
            | "continueRun"
            | "deleteChangeNote"
            | "focusRun"
            | "setRunPinned"
            | "archiveRun"
            | "restoreRun"
            | "listArchivedRuns"
            | "injectRunInput"
            | "noteRunEnded"
            | "openHostOverview"
            | "prepareRunLaunch"
            | "refreshLaunchEnvironment"
            | "returnToBoard"
            | "setProjectAutoAdvance"
            | "setProjectRestoreAutoAdvance"
            | "setProjectRestoreDelay"
            | "setRefreshInterval"
            | "startBoundRun"
            | "startUnboundRun"
            | "stopRun"
            | "updateRunLaunch"
            | "vetoPendingConfirmation"
            | "viewChanges"
            | "writeChangeNote" => self.handle_run_rpc(&op, request),
            "closeUsage" | "openRunFromUsage" | "openUsage" | "openUsageForRun"
            | "setUsageFilter" | "setUsageRange" => self.handle_usage_rpc(&op, request),
            other => Err(KernelError::Protocol(format!("unknown op {other}"))),
        }
    }
}

pub(crate) fn parse_launch_config(
    request: &serde_json::Value,
) -> Result<RunLaunchConfig, KernelError> {
    let agent_id = required_string(request, "agentId")?;
    let values = request
        .get("values")
        .and_then(|value| value.as_object())
        .map(|object| {
            object
                .iter()
                .filter_map(|(key, value)| {
                    value
                        .as_str()
                        .map(|text| (key.clone(), text.to_string()))
                        .or_else(|| value.as_bool().map(|flag| (key.clone(), flag.to_string())))
                        .or_else(|| {
                            value
                                .as_i64()
                                .map(|number| (key.clone(), number.to_string()))
                        })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(RunLaunchConfig {
        agent_id,
        values,
        opening_text: optional_string(request, "openingText"),
    })
}

pub(crate) fn required_string(
    request: &serde_json::Value,
    key: &str,
) -> Result<String, KernelError> {
    request
        .get(key)
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| KernelError::Protocol(format!("missing {key}")))
}

pub(crate) fn request_language(request: &serde_json::Value) -> Option<Language> {
    request
        .get("language")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
}

pub(crate) fn optional_string(request: &serde_json::Value, key: &str) -> String {
    request
        .get(key)
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string()
}

pub(crate) fn tracker_kind_for_host(github_host: &str) -> TrackerKind {
    if github_host == "local" {
        TrackerKind::LocalMarkdown
    } else {
        TrackerKind::Github
    }
}

pub(crate) fn parse_issue_ref(id: &str) -> Result<IssueRef, KernelError> {
    let (repository, number) = parse_issue_id(id)
        .ok_or_else(|| KernelError::Protocol(format!("invalid issue id: {id}")))?;
    Ok(IssueRef::new(repository, number, ""))
}
