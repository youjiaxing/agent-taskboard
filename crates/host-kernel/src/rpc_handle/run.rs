use super::*;

impl HostKernel {
    pub(crate) fn handle_run_rpc(
        &mut self,
        op: &str,
        request: serde_json::Value,
    ) -> Result<CommandOutcome, KernelError> {
        match op {
            "refreshLaunchEnvironment" => {
                if self.focused_host_id != LOCAL_HOST_ID {
                    return Err(KernelError::Denied(
                        "switch to this machine to reread its launch environment".into(),
                    ));
                }
                self.dispatch(Command::RefreshLaunchEnvironment)
            }
            "noteRunEnded" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::NoteRunEnded {
                    project_id: required_string(&request, "projectId")?,
                })
            }
            "autoAdvance" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::AutoAdvance {
                    project_id: required_string(&request, "projectId")?,
                })
            }
            "checkIssueClosed" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::CheckIssueClosed {
                    issue_id: required_string(&request, "issueId")?,
                })
            }
            "startBoundRun" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::StartBoundRun {
                    issue_id: required_string(&request, "issueId")?,
                })
            }
            "continueRun" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::ContinueRun {
                    issue_id: required_string(&request, "issueId")?,
                })
            }
            "prepareRunLaunch" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::PrepareRunLaunch {
                    project_id: required_string(&request, "projectId")?,
                    issue_id: request
                        .get("issueId")
                        .and_then(|value| value.as_str())
                        .filter(|value| !value.is_empty())
                        .map(ToOwned::to_owned),
                    agent_id: request
                        .get("agentId")
                        .and_then(|value| value.as_str())
                        .filter(|value| !value.is_empty())
                        .map(ToOwned::to_owned),
                    pick_agent: request
                        .get("pickAgent")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false),
                    defer_discovery: request
                        .get("deferDiscovery")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false),
                    language: request_language(&request).unwrap_or(self.appearance.language),
                })
            }
            "cancelRunLaunch" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::CancelRunLaunch)
            }
            "updateRunLaunch" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::UpdateRunLaunch {
                    project_id: required_string(&request, "projectId")?,
                    config: parse_launch_config(&request)?,
                    language: request_language(&request).unwrap_or(self.appearance.language),
                })
            }
            "startUnboundRun" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                let project_id = required_string(&request, "projectId")?;
                if request.get("agentId").is_some()
                    || request.get("values").is_some()
                    || request.get("openingText").is_some()
                {
                    self.dispatch(Command::StartUnboundRunWithConfig {
                        project_id,
                        config: parse_launch_config(&request)?,
                        issue_id: request
                            .get("issueId")
                            .and_then(|value| value.as_str())
                            .filter(|value| !value.is_empty())
                            .map(ToOwned::to_owned),
                    })
                } else {
                    self.dispatch(Command::StartUnboundRun { project_id })
                }
            }
            "stopRun" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::StopRun {
                    run_id: required_string(&request, "runId")?,
                })
            }
            "focusRun" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::FocusRun {
                    run_id: required_string(&request, "runId")?,
                })
            }
            "setRunPinned" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::SetRunPinned {
                    run_id: required_string(&request, "runId")?,
                    pinned: request
                        .get("pinned")
                        .and_then(|value| value.as_bool())
                        .ok_or_else(|| KernelError::Protocol("missing pinned".into()))?,
                })
            }
            "archiveRun" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::ArchiveRun {
                    run_id: required_string(&request, "runId")?,
                })
            }
            "prepareRestoreRun" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::PrepareRestoreRun {
                    run_id: required_string(&request, "runId")?,
                })
            }
            "restoreRun" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::RestoreRun {
                    run_id: required_string(&request, "runId")?,
                    confirm_project_recreate: request
                        .get("confirmProjectRecreate")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false),
                    expected_tombstone_revision: request
                        .get("expectedTombstoneRevision")
                        .and_then(|value| value.as_str())
                        .filter(|value| !value.is_empty())
                        .map(ToOwned::to_owned),
                })
            }
            "retryRunPersistenceLoad" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::RetryRunPersistenceLoad)
            }
            "listArchivedRuns" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                let project_id = request
                    .get("projectId")
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.is_empty());
                let mut outcome = self.outcome();
                outcome.archived_runs = Some(self.list_archived_runs(project_id));
                Ok(outcome)
            }
            "openHostOverview" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::OpenHostOverview)
            }
            "returnToBoard" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::ReturnToBoard)
            }
            "injectRunInput" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::InjectRunInput {
                    run_id: required_string(&request, "runId")?,
                    text: request
                        .get("text")
                        .and_then(|value| value.as_str())
                        .unwrap_or("")
                        .to_string(),
                })
            }
            "cancelQuit" => self.dispatch(Command::CancelQuit),
            "confirmQuitStopAll" => self.dispatch(Command::ConfirmQuitStopAll),
            "setRefreshInterval" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::SetRefreshInterval {
                    interval_ms: request
                        .get("intervalMs")
                        .and_then(|value| value.as_u64())
                        .ok_or_else(|| KernelError::Protocol("missing intervalMs".into()))?,
                })
            }
            "viewChanges" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                let scope =
                    ChangeScope::parse(request.get("scope").and_then(|value| value.as_str()))
                        .map_err(KernelError::Protocol)?;
                let view = self.view_changes(
                    request
                        .get("runId")
                        .and_then(|value| value.as_str())
                        .filter(|value| !value.is_empty()),
                    request
                        .get("issueId")
                        .and_then(|value| value.as_str())
                        .filter(|value| !value.is_empty()),
                    scope,
                )?;
                let mut outcome = self.outcome();
                outcome.view_changes = Some(view);
                Ok(outcome)
            }
            "writeChangeNote" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::WriteChangeNote {
                    run_id: required_string(&request, "runId")?,
                    repo: optional_string(&request, "repo"),
                    path: required_string(&request, "path")?,
                    line: request
                        .get("line")
                        .and_then(|value| value.as_u64())
                        .ok_or_else(|| KernelError::Protocol("missing line".into()))?
                        as u32,
                    text: required_string(&request, "text")?,
                })
            }
            "deleteChangeNote" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::DeleteChangeNote {
                    note_id: required_string(&request, "noteId")?,
                })
            }
            "setProjectAutoAdvance" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::SetProjectAutoAdvance {
                    project_id: required_string(&request, "projectId")?,
                    enabled: request
                        .get("enabled")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false),
                })
            }
            "setProjectRestoreAutoAdvance" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::SetProjectRestoreAutoAdvance {
                    project_id: required_string(&request, "projectId")?,
                    enabled: request
                        .get("enabled")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false),
                })
            }
            "setProjectRestoreDelay" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::SetProjectRestoreDelay {
                    project_id: required_string(&request, "projectId")?,
                    delay_ms: request
                        .get("delayMs")
                        .and_then(|value| value.as_u64())
                        .unwrap_or(advance::DEFAULT_RESTORE_DELAY_MS),
                })
            }
            "vetoPendingConfirmation" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::VetoPendingConfirmation {
                    project_id: required_string(&request, "projectId")?,
                })
            }
            other => Err(KernelError::Protocol(format!("unknown op {other}"))),
        }
    }
}
