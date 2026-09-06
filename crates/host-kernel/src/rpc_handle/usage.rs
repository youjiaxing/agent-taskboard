use super::*;

impl HostKernel {
    pub(crate) fn handle_usage_rpc(
        &mut self,
        op: &str,
        request: serde_json::Value,
    ) -> Result<CommandOutcome, KernelError> {
        match op {
            "openUsage" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::OpenUsage)
            }
            "closeUsage" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::CloseUsage)
            }
            "setUsageRange" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                let range = serde_json::from_value(
                    request
                        .get("range")
                        .cloned()
                        .ok_or_else(|| KernelError::Protocol("missing range".into()))?,
                )?;
                self.dispatch(Command::SetUsageRange {
                    range,
                    custom_from_ms: request.get("fromMs").and_then(|value| value.as_u64()),
                    custom_to_ms: request.get("toMs").and_then(|value| value.as_u64()),
                })
            }
            "setUsageFilter" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::SetUsageFilter {
                    project_id: request
                        .get("projectId")
                        .and_then(|value| value.as_str())
                        .filter(|value| !value.is_empty())
                        .map(ToOwned::to_owned),
                    agent_id: request
                        .get("agentId")
                        .and_then(|value| value.as_str())
                        .filter(|value| !value.is_empty())
                        .map(ToOwned::to_owned),
                    model: request
                        .get("model")
                        .and_then(|value| value.as_str())
                        .filter(|value| !value.is_empty())
                        .map(ToOwned::to_owned),
                })
            }
            "openUsageForRun" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::OpenUsageForRun {
                    run_id: required_string(&request, "runId")?,
                })
            }
            "openRunFromUsage" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::OpenRunFromUsage {
                    run_id: required_string(&request, "runId")?,
                })
            }
            other => Err(KernelError::Protocol(format!("unknown op {other}"))),
        }
    }
}
