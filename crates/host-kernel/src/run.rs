use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Deserializer, Serialize};

use crate::agent::{
    format_not_found, prepare_launch_env, AgentPort, CompletionHookPlan, ProbeResult,
    RunLaunchConfig,
};
use crate::pairing;
use crate::session::{AgentSession, SessionFactory, SpawnRequest};
use crate::{Language, LaunchEnvPort};

pub const DEFAULT_PTY_COLS: u16 = 80;
pub const DEFAULT_PTY_ROWS: u16 = 24;
const RESUME_CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(2);

pub(crate) fn submitted_input(text: &str) -> Vec<u8> {
    // Keep Enter outside the paste so interactive TUIs do not absorb it into
    // their paste-burst buffer. Raw Embedded Terminal keystrokes bypass this.
    let mut bytes = b"\x1b[200~".to_vec();
    bytes.extend_from_slice(text.trim_end_matches(['\r', '\n']).as_bytes());
    bytes.extend_from_slice(b"\x1b[201~\r");
    bytes
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_summary: Option<String>,
    pub unbound: bool,
    pub status: RunStatus,
    #[serde(default)]
    pub waiting_for_user: bool,
    #[serde(
        default,
        deserialize_with = "deserialize_organization_timestamp",
        skip_serializing_if = "Option::is_none"
    )]
    pub pinned_at_ms: Option<u64>,
    #[serde(
        default,
        deserialize_with = "deserialize_organization_timestamp",
        skip_serializing_if = "Option::is_none"
    )]
    pub archived_at_ms: Option<u64>,
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
    pub isolation_pending: Option<Vec<std::path::PathBuf>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isolation_note: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub git_baselines: Vec<crate::changes::GitBaseline>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hooks_attached: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub session_end: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stop_failure: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub self_check: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub self_check_attempted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hook_dir: Option<std::path::PathBuf>,
    #[serde(default)]
    pub started_at_ms: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub telemetry: Vec<crate::usage::RunTelemetryLane>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub recent_output: String,
}

fn deserialize_organization_timestamp<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(value.and_then(|value| value.as_u64()))
}

impl RunSummary {
    pub fn is_active(&self) -> bool {
        matches!(self.status, RunStatus::Starting | RunStatus::Running)
    }

    pub fn is_archived(&self) -> bool {
        self.archived_at_ms.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuitOffer {
    pub active_run_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInstallGate {
    pub allowed: bool,
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
    hooks: Option<&CompletionHookPlan>,
) -> StartResult {
    let id = pairing::random_id();
    let mut record = RunSummary {
        id,
        project_id: project_id.to_string(),
        agent_id: agent.id().to_string(),
        agent_name: agent.name().to_string(),
        issue_id: issue_id.filter(|id| !id.is_empty()).map(ToOwned::to_owned),
        task_summary: None,
        unbound: issue_id.map(|id| id.is_empty()).unwrap_or(true),
        status: RunStatus::Starting,
        waiting_for_user: false,
        pinned_at_ms: None,
        archived_at_ms: None,
        recent_action: agent.recent_action().filter(|text| !text.is_empty()),
        failure: None,
        previous_run_id: previous_run_id
            .filter(|id| !id.is_empty())
            .map(ToOwned::to_owned),
        native_session_id: None,
        ended_reason: None,
        working_directory: cwd.to_string_lossy().into_owned(),
        isolated: false,
        isolation_pending: None,
        isolation_note: None,
        git_baselines: Vec::new(),
        hooks_attached: hooks.is_some(),
        session_end: false,
        stop_failure: false,
        self_check: false,
        self_check_attempted: false,
        hook_dir: None,
        started_at_ms: 0,
        telemetry: Vec::new(),
        recent_output: String::new(),
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
            let mut argv = if let Some(session_id) = resume_session_id.filter(|id| !id.is_empty()) {
                agent.assemble_argv_for_resume(&executable, &config.values, session_id)
            } else {
                agent.assemble_argv_for(&executable, &config.values)
            };
            let mut env = env;
            if let Some(hooks) = hooks {
                argv.extend(hooks.extra_argv.iter().cloned());
                for (key, value) in &hooks.extra_env {
                    env.vars.insert(key.clone(), value.clone());
                }
            }
            if resume_session_id.is_some() {
                if let Some(sink) = env.vars.get("AGENT_TASKBOARD_HOOK_SINK") {
                    for signal in [
                        "native-session-id",
                        "session-end",
                        "stop-failure",
                        "waiting-for-user",
                    ] {
                        match std::fs::remove_file(Path::new(sink).join(signal)) {
                            Ok(()) => {}
                            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                            Err(err) => {
                                record.status = RunStatus::Ended;
                                record.ended_reason = Some(RunEndedReason::Abnormal);
                                record.failure =
                                    Some(format!("could not reset resume hook state: {err}"));
                                return StartResult {
                                    record,
                                    session: None,
                                };
                            }
                        }
                    }
                }
            }
            let opening_text = config.opening_text.trim();
            let opening_submitted_at_launch =
                !opening_text.is_empty() && agent.append_opening_prompt(&mut argv, opening_text);
            let request = SpawnRequest {
                argv,
                cwd: cwd.to_path_buf(),
                env: env.vars,
                cols: DEFAULT_PTY_COLS,
                rows: DEFAULT_PTY_ROWS,
            };
            match sessions.spawn(request) {
                Ok(session) => {
                    if resume_session_id.is_some() && agent.resume_confirmation_supported() {
                        if let Err(err) = verify_resume_startup(
                            session.as_ref(),
                            resume_session_id.expect("checked above"),
                        ) {
                            session.stop();
                            record.status = RunStatus::Ended;
                            record.ended_reason = Some(RunEndedReason::Abnormal);
                            record.failure = Some(err);
                            return StartResult {
                                record,
                                session: None,
                            };
                        }
                    }
                    if !opening_text.is_empty() && !opening_submitted_at_launch {
                        let opening = submitted_input(opening_text);
                        if let Err(err) = session.write(&opening) {
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

fn verify_resume_startup(
    session: &dyn AgentSession,
    expected_session_id: &str,
) -> Result<(), String> {
    let deadline = Instant::now() + RESUME_CONFIRMATION_TIMEOUT;
    loop {
        if let Some(session_id) = session.completion_signals().native_session_id {
            if session_id == expected_session_id {
                return Ok(());
            }
            return Err(format!(
                "Agent resumed a different native session ({session_id}) than requested ({expected_session_id})."
            ));
        }
        if let Some(exit_code) = session.exit_code() {
            let output = session.recent_output();
            return Err(if output.is_empty() {
                format!(
                    "Agent exited during native session resume startup (exit code {exit_code})."
                )
            } else {
                format!(
                    "Agent exited during native session resume startup (exit code {exit_code}): {output}"
                )
            });
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "Agent did not confirm native session resume ({expected_session_id}) within 2 seconds."
            ));
        }
        std::thread::sleep(Duration::from_millis(15));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn rejects_a_resume_process_that_exits_during_startup() {
        let session = crate::session::PtySessionFactory
            .spawn(SpawnRequest {
                argv: vec![
                    "/bin/sh".into(),
                    "-c".into(),
                    "printf 'session not found'; exit 23".into(),
                ],
                cwd: std::env::current_dir().unwrap(),
                env: Default::default(),
                cols: DEFAULT_PTY_COLS,
                rows: DEFAULT_PTY_ROWS,
            })
            .unwrap();

        let error = verify_resume_startup(session.as_ref(), "expected-session").unwrap_err();
        assert!(error.contains("exit code 23"), "{error}");
        assert!(error.contains("session not found"), "{error}");
    }

    #[test]
    fn confirms_that_resume_started_the_requested_native_session() {
        let session = crate::session::MemorySession::new();
        session.set_native_session_id(Some("expected-session".into()));
        assert!(verify_resume_startup(&session, "expected-session").is_ok());

        session.set_native_session_id(Some("new-session".into()));
        let error = verify_resume_startup(&session, "expected-session").unwrap_err();
        assert!(error.contains("different native session"), "{error}");
    }
}
