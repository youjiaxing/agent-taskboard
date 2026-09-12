use super::*;

impl HostKernel {
    pub(crate) fn handle_host_rpc(
        &mut self,
        op: &str,
        request: serde_json::Value,
    ) -> Result<CommandOutcome, KernelError> {
        match op {
            "snapshot" => {
                self.observe_live_runs();
                Ok(self.outcome())
            }
            "updateInstallGate" => {
                let gate = self.update_install_gate();
                let mut outcome = self.outcome();
                outcome.update_install_gate = Some(gate);
                Ok(outcome)
            }
            "beginUpdateInstall" => {
                let gate = self.update_install_gate();
                if gate.allowed {
                    self.update_installing = true;
                }
                let mut outcome = self.outcome();
                outcome.update_install_gate = Some(gate);
                Ok(outcome)
            }
            "cancelUpdateInstall" => {
                self.update_installing = false;
                Ok(self.outcome())
            }
            "hideWindow" => self.dispatch(Command::HideWindow),
            "showWindow" => self.dispatch(Command::ShowWindow),
            "quitHost" => self.dispatch(Command::QuitHost),
            "setLanguage" => {
                let language = serde_json::from_value(
                    request
                        .get("language")
                        .cloned()
                        .ok_or_else(|| KernelError::Protocol("missing language".into()))?,
                )?;
                self.dispatch(Command::SetLanguage(language))
            }
            "setTheme" => {
                let theme = serde_json::from_value(
                    request
                        .get("theme")
                        .cloned()
                        .ok_or_else(|| KernelError::Protocol("missing theme".into()))?,
                )?;
                self.dispatch(Command::SetTheme(theme))
            }
            "beginPairingOffer" => {
                let address = request
                    .get("address")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| KernelError::Protocol("missing address".into()))?
                    .to_string();
                self.dispatch(Command::BeginPairingOffer { address })
            }
            "redeemPairing" => {
                let code = request
                    .get("code")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| KernelError::Protocol("missing code".into()))?
                    .to_string();
                let client_name = request
                    .get("clientName")
                    .and_then(|value| value.as_str())
                    .unwrap_or("Client")
                    .to_string();
                self.dispatch(Command::RedeemPairing { code, client_name })
            }
            "revokeClient" => {
                let client_id = request
                    .get("clientId")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| KernelError::Protocol("missing clientId".into()))?
                    .to_string();
                self.dispatch(Command::RevokeClient { client_id })
            }
            "pairRemoteHost" => {
                let address = request
                    .get("address")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| KernelError::Protocol("missing address".into()))?
                    .to_string();
                let code = request
                    .get("code")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| KernelError::Protocol("missing code".into()))?
                    .to_string();
                self.dispatch(Command::PairRemoteHost { address, code })
            }
            "focusHost" => {
                let host_id = request
                    .get("hostId")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| KernelError::Protocol("missing hostId".into()))?
                    .to_string();
                if let Some(client_instance_id) = request
                    .get("clientInstanceId")
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.is_empty())
                {
                    self.focused_host_id = host_id.clone();
                    self.refresh_remote_view_for_client(&host_id, Some(client_instance_id))?;
                    self.persist_client_settings(&self.appearance.clone())?;
                    Ok(self.outcome())
                } else {
                    self.dispatch(Command::FocusHost { host_id })
                }
            }
            "setClientView" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::SetClientView {
                    client_id: required_string(&request, "clientId")?,
                    project_id: optional_string(&request, "projectId"),
                    visible: request
                        .get("visible")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false),
                })
            }
            "setShowCommandPreview" => self.dispatch(Command::SetShowCommandPreview {
                show: request
                    .get("show")
                    .and_then(|value| value.as_bool())
                    .ok_or_else(|| KernelError::Protocol("missing show".into()))?,
            }),
            "setNotificationPrefs" => self.dispatch(Command::SetNotificationPrefs {
                desktop: request
                    .get("desktop")
                    .and_then(|value| value.as_bool())
                    .ok_or_else(|| KernelError::Protocol("missing desktop".into()))?,
                sound: request
                    .get("sound")
                    .and_then(|value| value.as_bool())
                    .ok_or_else(|| KernelError::Protocol("missing sound".into()))?,
            }),
            "setHostAutoAdvance" => {
                if let Some(outcome) = self.forward_if_remote(&request)? {
                    return Ok(outcome);
                }
                self.dispatch(Command::SetHostAutoAdvance {
                    enabled: request
                        .get("enabled")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false),
                })
            }
            other => Err(KernelError::Protocol(format!("unknown op {other}"))),
        }
    }
}
