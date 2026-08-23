use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::agent::{format_not_found, prepare_launch_env, AgentPort, ProbeResult, RunLaunchConfig};
use crate::pairing;
use crate::session::{AgentSession, SessionFactory, SpawnRequest};
use crate::{Language, LaunchEnvPort};

pub const DEFAULT_PTY_COLS: u16 = 80;
pub const DEFAULT_PTY_ROWS: u16 = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunStatus {
    Starting,
    Running,
    Ended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunEndedReason {
    Exited,
    Stopped,
    Abnormal,
    Crash,
}

impl RunEndedReason {
    pub fn execution_stopped(self) -> bool {
        matches!(self, Self::Stopped | Self::Abnormal | Self::Crash)
    }

    pub fn from_exit(code: i32, stopped: bool) -> Self {
        if stopped {
            Self::Stopped
        } else if code == 0 {
            Self::Exited
        } else {
            Self::Abnormal
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSummary {
    pub id: String,
    pub project_id: String,
    pub agent_id: String,
    pub agent_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue_id: Option<String>,
    pub unbound: bool,
    pub status: RunStatus,
    #[serde(default)]
    pub waiting_for_user: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recent_action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_reason: Option<RunEndedReason>,
    #[serde(default)]
    pub working_directory: String,
    #[serde(default)]
    pub isolated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isolation_note: Option<String>,
}

impl RunSummary {
    pub fn is_active(&self) -> bool {
        matches!(self.status, RunStatus::Starting | RunStatus::Running)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuitOffer {
    pub active_run_count: u32,
}

pub struct StartResult {
    pub record: RunSummary,
    pub session: Option<Arc<dyn AgentSession>>,
}

pub fn start_unbound(
    project_id: &str,
    cwd: &Path,
    agent: &dyn AgentPort,
    launch_env: &dyn LaunchEnvPort,
    sessions: &dyn SessionFactory,
    language: Language,
    host_path_prefix: &[std::path::PathBuf],
    config: &RunLaunchConfig,
    issue_id: Option<&str>,
    previous_run_id: Option<&str>,
    resume_session_id: Option<&str>,
) -> StartResult {
    let id = pairing::random_id();
    let mut record = RunSummary {
        id,
        project_id: project_id.to_string(),
        agent_id: agent.id().to_string(),
        agent_name: agent.name().to_string(),
        issue_id: issue_id.filter(|id| !id.is_empty()).map(ToOwned::to_owned),
        unbound: issue_id.map(|id| id.is_empty()).unwrap_or(true),
        status: RunStatus::Starting,
        waiting_for_user: false,
        recent_action: agent.recent_action().filter(|text| !text.is_empty()),
        failure: None,
        previous_run_id: previous_run_id
            .filter(|id| !id.is_empty())
            .map(ToOwned::to_owned),
        native_session_id: None,
        ended_reason: None,
        working_directory: cwd.to_string_lossy().into_owned(),
        isolated: false,
        isolation_note: None,
    };
    let captured = match launch_env.capture(cwd) {
        Ok(env) => env,
        Err(err) => {
            record.status = RunStatus::Ended;
            record.ended_reason = Some(RunEndedReason::Abnormal);
            record.failure = Some(err);
            return StartResult {
                record,
                session: None,
            };
        }
    };
    let env = prepare_launch_env(captured, host_path_prefix, &agent.known_install_locations());
    match agent.probe(&env) {
        ProbeResult::Missing {
            command,
            searched_path,
            known_locations,
        } => {
            record.status = RunStatus::Ended;
            record.ended_reason = Some(RunEndedReason::Abnormal);
            record.failure = Some(format_not_found(
                language,
                &command,
                &searched_path,
                &known_locations,
            ));
            StartResult {
                record,
                session: None,
            }
        }
        ProbeResult::Found { executable } => {
            let argv = if let Some(session_id) = resume_session_id.filter(|id| !id.is_empty()) {
                agent.assemble_argv_for_resume(&executable, &config.values, session_id)
            } else {
                agent.assemble_argv_for(&executable, &config.values)
            };
            let request = SpawnRequest {
                argv,
                cwd: cwd.to_path_buf(),
                env: env.vars,
                cols: DEFAULT_PTY_COLS,
                rows: DEFAULT_PTY_ROWS,
            };
            match sessions.spawn(request) {
                Ok(session) => {
                    if !config.opening_text.trim().is_empty() {
                        if let Err(err) =
                            session.write(format!("{}\n", config.opening_text.trim()).as_bytes())
                        {
                            session.stop();
                            record.status = RunStatus::Ended;
                            record.ended_reason = Some(RunEndedReason::Abnormal);
                            record.failure = Some(err.to_string());
                            return StartResult {
                                record,
                                session: None,
                            };
                        }
                    }
                    record.status = RunStatus::Running;
                    record.native_session_id = agent.native_session_id();
                    StartResult {
                        record,
                        session: Some(session),
                    }
                }
                Err(err) => {
                    record.status = RunStatus::Ended;
                    record.ended_reason = Some(RunEndedReason::Abnormal);
                    record.failure = Some(err);
                    StartResult {
                        record,
                        session: None,
                    }
                }
            }
        }
    }
}
