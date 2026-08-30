use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::mpsc;
use std::time::Instant;

use serde_json::Value;

use super::{
    additional_args_field, append_additional_args, append_flag, discovery, hooks,
    initial_instruction_field, local_bin, probe_binary, select_field, text_field,
    AgentConfigDiscovery, AgentField, AgentPort, CompletionHookPlan, ProbeResult,
};
use crate::{Language, LaunchEnvironment};

pub const CODEX_ID: &str = "codex";
pub const CODEX_NAME: &str = "Codex";
pub const CODEX_BIN: &str = "codex";

#[derive(Debug, Clone)]
pub struct CodexAdapter;

impl AgentPort for CodexAdapter {
    fn id(&self) -> &str {
        CODEX_ID
    }

    fn name(&self) -> &str {
        CODEX_NAME
    }

    fn bin(&self) -> &str {
        CODEX_BIN
    }

    fn known_install_locations(&self) -> Vec<PathBuf> {
        local_bin().into_iter().collect()
    }

    fn probe(&self, env: &LaunchEnvironment) -> ProbeResult {
        probe_binary(self.bin(), env, &self.known_install_locations())
    }

    fn assemble_argv(&self, executable: &Path) -> Vec<String> {
        vec![executable.to_string_lossy().into_owned()]
    }

    fn config_fields(&self) -> Vec<AgentField> {
        vec![
            text_field("model", "model", true, false),
            select_field("effort", "effort", &["low", "medium", "high"], true, false),
            select_field(
                "approval",
                "approval",
                &["untrusted", "on-request", "never"],
                true,
                false,
            ),
            select_field(
                "sandbox",
                "sandbox",
                &["read-only", "workspace-write", "danger-full-access"],
                true,
                false,
            ),
            initial_instruction_field(),
            text_field("profile", "profile", false, true),
            additional_args_field(),
        ]
    }

    fn seed_config(&self) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("model".into(), "gpt-5.1".into()),
            ("effort".into(), "medium".into()),
            ("approval".into(), "on-request".into()),
            ("sandbox".into(), "workspace-write".into()),
            ("initial-instruction".into(), String::new()),
            ("profile".into(), String::new()),
            ("additional-args".into(), String::new()),
        ])
    }

    fn discover_config(
        &self,
        executable: &Path,
        env: &LaunchEnvironment,
    ) -> Result<AgentConfigDiscovery, String> {
        let response = codex_model_list(executable, env)?;
        let help = discovery::run_cli(executable, &["--help"], env)?;
        let models = response
            .pointer("/result/data")
            .and_then(Value::as_array)
            .ok_or_else(|| "Codex CLI model/list returned no model data".to_string())?;
        let mut fields = self.config_fields();
        let mut seed = self.seed_config();
        let mut model_options = Vec::new();
        let mut efforts_by_model = BTreeMap::new();
        let mut default_model = None;
        let mut default_effort = None;
        for model in models {
            let Some(id) = model
                .get("model")
                .or_else(|| model.get("id"))
                .and_then(Value::as_str)
            else {
                continue;
            };
            model_options.push(id.to_string());
            let efforts = model
                .get("supportedReasoningEfforts")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|option| option.get("reasoningEffort").and_then(Value::as_str))
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>();
            if !efforts.is_empty() {
                efforts_by_model.insert(id.to_string(), efforts);
            }
            if model
                .get("isDefault")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                default_model = Some(id.to_string());
                default_effort = model
                    .get("defaultReasoningEffort")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned);
            }
        }
        if model_options.is_empty() {
            return Err("Codex CLI model/list returned an empty model list".into());
        }
        discovery::set_options(&mut fields, "model", model_options);
        discovery::set_option_filter(&mut fields, "effort", "model", efforts_by_model);
        discovery::set_options_if_found(
            &mut fields,
            "approval",
            discovery::option_values(&help, "--ask-for-approval"),
        );
        discovery::set_options_if_found(
            &mut fields,
            "sandbox",
            discovery::option_values(&help, "--sandbox"),
        );
        if let Some(model) = default_model {
            seed.insert("model".into(), model);
        }
        if let Some(effort) = default_effort {
            seed.insert("effort".into(), effort);
        }
        Ok(AgentConfigDiscovery { fields, seed })
    }

    fn assemble_argv_for(
        &self,
        executable: &Path,
        values: &BTreeMap<String, String>,
    ) -> Vec<String> {
        let mut argv = vec![executable.to_string_lossy().into_owned()];
        append_flag(&mut argv, "--model", values.get("model"));
        if let Some(effort) = values
            .get("effort")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        {
            argv.push("-c".into());
            argv.push(format!("model_reasoning_effort=\"{effort}\""));
        }
        append_flag(&mut argv, "--ask-for-approval", values.get("approval"));
        append_flag(&mut argv, "--sandbox", values.get("sandbox"));
        append_flag(&mut argv, "--profile", values.get("profile"));
        append_additional_args(&mut argv, values);
        argv
    }

    fn isolation_unavailable_reason(&self, language: Language) -> String {
        match language {
            Language::ZhCn => "Codex CLI 没有原生 --worktree，隔离不可用。".into(),
            Language::En => {
                "Codex CLI has no native --worktree, so isolation is unavailable.".into()
            }
        }
    }

    fn completion_hooks_supported(&self) -> bool {
        true
    }

    fn attach_completion_hooks(
        &self,
        sink_dir: &Path,
        _project_dir: &Path,
    ) -> Result<CompletionHookPlan, String> {
        let recorder = hooks::write_recorder(sink_dir)?;
        let command = hooks::recorder_command(&recorder, "SessionEnd");
        Ok(CompletionHookPlan {
            extra_argv: vec![
                "-c".into(),
                "features.hooks=true".into(),
                "-c".into(),
                format!(
                    "[[hooks.SessionEnd]]\n[[hooks.SessionEnd.hooks]]\ntype = \"command\"\ncommand = {command:?}\ntimeout = 3"
                ),
            ],
            extra_env: hooks::sink_env(sink_dir),
        })
    }
}

fn codex_model_list(executable: &Path, env: &LaunchEnvironment) -> Result<Value, String> {
    let mut command = discovery::configured_command(executable, env);
    command
        .args(["app-server", "--stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|err| format!("could not run Codex app-server: {err}"))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or("Codex app-server stdin unavailable")?;
    let stdout = child
        .stdout
        .take()
        .ok_or("Codex app-server stdout unavailable")?;
    let stderr = child.stderr.take();
    let (sender, receiver) = mpsc::channel();
    let stdout_reader = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            match line {
                Ok(line) => {
                    if sender.send(line).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    let stderr_reader = std::thread::spawn(move || discovery::read_all(stderr));
    writeln!(
        stdin,
        "{}",
        serde_json::json!({
            "method": "initialize",
            "id": 0,
            "params": {
                "clientInfo": {
                    "name": "agent-taskboard",
                    "title": "Agent Taskboard",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": {}
            }
        })
    )
    .map_err(|err| format!("could not initialize Codex app-server: {err}"))?;
    stdin.flush().ok();
    let started = Instant::now();
    let mut requested_models = false;
    loop {
        let remaining = discovery::PROBE_TIMEOUT.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let stderr = stderr_reader.join().unwrap_or_default();
            return Err(format!(
                "Codex CLI option discovery timed out{}",
                discovery::stderr_suffix(&stderr)
            ));
        }
        let line = receiver
            .recv_timeout(remaining)
            .map_err(|_| "Codex CLI option discovery timed out".to_string())?;
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if message.get("id").and_then(Value::as_i64) == Some(0) && !requested_models {
            writeln!(
                stdin,
                "{}",
                serde_json::json!({
                    "method": "model/list",
                    "id": 1,
                    "params": { "limit": 100 }
                })
            )
            .map_err(|err| format!("could not request Codex models: {err}"))?;
            stdin.flush().ok();
            requested_models = true;
            continue;
        }
        if message.get("id").and_then(Value::as_i64) == Some(1) {
            let _ = child.kill();
            let _ = child.wait();
            drop(receiver);
            let _ = stdout_reader.join();
            let stderr = stderr_reader.join().unwrap_or_default();
            if let Some(error) = message.get("error") {
                return Err(format!(
                    "Codex CLI model/list failed: {error}{}",
                    discovery::stderr_suffix(&stderr)
                ));
            }
            return Ok(message);
        }
    }
}
