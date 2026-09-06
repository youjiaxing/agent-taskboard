use super::*;

impl HostKernel {
    pub(crate) fn handle_board_rpc(
        &mut self,
        op: &str,
        request: serde_json::Value,
    ) -> Result<CommandOutcome, KernelError> {
        match op {
            "registerProject" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::RegisterProject {
                    name: required_string(&request, "name")?,
                    local_path: required_string(&request, "localPath")?,
                    github_host: optional_string(&request, "githubHost"),
                    repository: required_string(&request, "repository")?,
                })
            }
            "editProject" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::EditProject {
                    project_id: required_string(&request, "projectId")?,
                    name: required_string(&request, "name")?,
                    local_path: required_string(&request, "localPath")?,
                    github_host: optional_string(&request, "githubHost"),
                    repository: required_string(&request, "repository")?,
                })
            }
            "removeProject" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::RemoveProject {
                    project_id: required_string(&request, "projectId")?,
                })
            }
            "focusProject" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::FocusProject {
                    project_id: required_string(&request, "projectId")?,
                })
            }
            "inferProject" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::InferProject {
                    local_path: required_string(&request, "localPath")?,
                })
            }
            "focusIssue" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::FocusIssue {
                    issue_id: required_string(&request, "issueId")?,
                })
            }
            "loadIssueDocument" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::LoadIssueDocument {
                    issue_id: required_string(&request, "issueId")?,
                })
            }
            "filterParent" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::FilterParent {
                    issue_id: required_string(&request, "issueId")?,
                })
            }
            "clearParentFilter" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::ClearParentFilter)
            }
            "setCenterView" => {
                let view = serde_json::from_value(
                    request
                        .get("view")
                        .cloned()
                        .ok_or_else(|| KernelError::Protocol("missing view".into()))?,
                )?;
                if view == CenterView::Graph
                    && self.center_view != CenterView::Graph
                    && self.focused_host_id != LOCAL_HOST_ID
                {
                    self.forward_if_remote(&serde_json::json!({
                        "op": "showDependencyGraphOverview",
                    }))?;
                }
                self.dispatch(Command::SetCenterView { view })
            }
            "centerDependencyGraph" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::CenterDependencyGraph {
                    issue_id: required_string(&request, "issueId")?,
                })
            }
            "showDependencyGraphOverview" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::ShowDependencyGraphOverview)
            }
            "setDependencyGraphComplete" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::SetDependencyGraphComplete {
                    complete: request
                        .get("complete")
                        .and_then(|value| value.as_bool())
                        .ok_or_else(|| KernelError::Protocol("missing complete".into()))?,
                })
            }
            "setShowClosedGraphContext" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::SetDependencyGraphComplete {
                    complete: request
                        .get("show")
                        .and_then(|value| value.as_bool())
                        .ok_or_else(|| KernelError::Protocol("missing show".into()))?,
                })
            }
            "setRecentCompletedLimit" => self.dispatch(Command::SetRecentCompletedLimit {
                limit: request
                    .get("limit")
                    .and_then(|value| value.as_u64())
                    .ok_or_else(|| KernelError::Protocol("missing limit".into()))?
                    as u32,
            }),
            "searchIssues" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                let triage_role = request
                    .get("triageRole")
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.is_empty())
                    .map(|value| serde_json::from_value(serde_json::Value::String(value.into())))
                    .transpose()
                    .map_err(|_| KernelError::Protocol("invalid triageRole".into()))?;
                let state = request
                    .get("state")
                    .and_then(|value| value.as_str())
                    .map(|value| serde_json::from_value(serde_json::Value::String(value.into())))
                    .transpose()
                    .map_err(|_| KernelError::Protocol("invalid state".into()))?
                    .unwrap_or_default();
                self.dispatch(Command::SearchIssues {
                    project_id: required_string(&request, "projectId")?,
                    search: IssueSearch {
                        title: optional_string(&request, "title"),
                        triage_role,
                        state,
                    },
                })
            }
            "refresh" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::Refresh {
                    project_id: request
                        .get("projectId")
                        .and_then(|value| value.as_str())
                        .filter(|value| !value.is_empty())
                        .map(ToOwned::to_owned),
                })
            }
            "tick" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                let now_ms = request
                    .get("nowMs")
                    .and_then(|value| value.as_u64())
                    .unwrap_or_else(refresh::wall_ms);
                self.now_ms = now_ms;
                if let Some(client_id) = request
                    .get("clientId")
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.is_empty())
                {
                    self.set_client_view(
                        client_id,
                        &optional_string(&request, "projectId"),
                        request
                            .get("visible")
                            .and_then(|value| value.as_bool())
                            .unwrap_or(false),
                    );
                }
                self.dispatch(Command::Tick {
                    now_ms: Some(now_ms),
                })
            }
            "claimIssue" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::ClaimIssue {
                    issue_id: required_string(&request, "issueId")?,
                })
            }
            "releaseIssue" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::ReleaseIssue {
                    issue_id: required_string(&request, "issueId")?,
                })
            }
            "createIssue" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::CreateIssue {
                    project_id: required_string(&request, "projectId")?,
                    title: required_string(&request, "title")?,
                    body: optional_string(&request, "body"),
                })
            }
            "updateIssue" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::UpdateIssue {
                    issue_id: required_string(&request, "issueId")?,
                    title: required_string(&request, "title")?,
                    body: optional_string(&request, "body"),
                })
            }
            "setIssueOpen" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::SetIssueOpen {
                    issue_id: required_string(&request, "issueId")?,
                    open: request
                        .get("open")
                        .and_then(|value| value.as_bool())
                        .ok_or_else(|| KernelError::Protocol("missing open".into()))?,
                })
            }
            "addIssueComment" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::AddIssueComment {
                    issue_id: required_string(&request, "issueId")?,
                    body: required_string(&request, "body")?,
                })
            }
            "setIssueParent" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                let parent = request
                    .get("parent")
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned);
                if let Some(id) = parent.as_deref() {
                    parse_issue_ref(id)?;
                }
                self.dispatch(Command::SetIssueParent {
                    issue_id: required_string(&request, "issueId")?,
                    parent,
                })
            }
            "setIssueBlockedBy" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                let blocked_by = request
                    .get("blockedBy")
                    .and_then(|value| value.as_array())
                    .ok_or_else(|| KernelError::Protocol("missing blockedBy".into()))?
                    .iter()
                    .map(|item| {
                        item.as_str()
                            .map(ToOwned::to_owned)
                            .ok_or_else(|| KernelError::Protocol("invalid blockedBy".into()))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                for id in &blocked_by {
                    parse_issue_ref(id)?;
                }
                self.dispatch(Command::SetIssueBlockedBy {
                    issue_id: required_string(&request, "issueId")?,
                    blocked_by,
                })
            }
            other => Err(KernelError::Protocol(format!("unknown op {other}"))),
        }
    }
}
