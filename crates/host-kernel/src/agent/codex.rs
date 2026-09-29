use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::mpsc;
use std::time::Instant;

use serde_json::Value;

use super::{
    additional_args_field, append_additional_args, append_flag, discovery, hooks,
    initial_instruction_field, local_bin, probe_binary, select_field, text_field,
    AgentConfigDiscovery, AgentField, AgentPort, CompletionHookPlan, NativeSyncLaunchContext,
    ProbeResult,
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

    fn skill_invocation(&self, skill: &str) -> String {
        format!("${skill}")
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

    fn localize_field(&self, field: &mut AgentField, language: Language) {
        match (language, field.id.as_str()) {
            (Language::ZhCn, "approval") => {
                field.label = "approval（确认策略）".into();
                field.description =
                    "决定越过沙箱边界时是否暂停询问；sandbox 决定技术边界，approval 决定是否先问。"
                        .into();
                field.option_labels = BTreeMap::from([
                    ("untrusted".into(), "不受信任".into()),
                    ("on-request".into(), "需要时询问".into()),
                    ("never".into(), "不询问".into()),
                ]);
            }
            (Language::En, "approval") => {
                field.label = "approval (confirmation policy)".into();
                field.description = "Controls whether Codex pauses to ask before crossing a sandbox boundary; sandbox sets the boundary, approval controls whether to ask first.".into();
                field.option_labels = BTreeMap::from([
                    ("untrusted".into(), "untrusted".into()),
                    ("on-request".into(), "ask when needed".into()),
                    ("never".into(), "never ask".into()),
                ]);
            }
            (Language::ZhCn, "profile") => {
                field.label = "profile（配置档）".into();
                field.description =
                    "填写本机 Codex CLI 能识别的 profile 名称。Taskboard 只会将它作为 --profile 原样传递，不读取或校验 profile；留空则不传。"
                        .into();
            }
            (Language::En, "profile") => {
                field.label = "profile (configuration profile)".into();
                field.description = "Enter a profile name recognized by the local Codex CLI. Taskboard only passes it as --profile and does not read or validate the profile; leave it blank to omit it.".into();
            }
            (Language::ZhCn, "sandbox") => {
                field.label = "sandbox（命令沙箱）".into();
                field.description =
                    "控制 Codex 执行命令时的文件系统和网络边界。它不是 git worktree，也不隔离多个 Run。"
                        .into();
                field.option_labels = BTreeMap::from([
                    ("read-only".into(), "只读".into()),
                    ("workspace-write".into(), "可写工作区".into()),
                    ("danger-full-access".into(), "完全访问，风险最高".into()),
                ]);
            }
            (Language::En, "sandbox") => {
                field.label = "sandbox (command sandbox)".into();
                field.description = "Controls the file-system and network boundary for commands run by Codex. It is not a git worktree and does not isolate multiple Runs.".into();
                field.option_labels = BTreeMap::from([
                    ("read-only".into(), "read-only".into()),
                    ("workspace-write".into(), "workspace writable".into()),
                    (
                        "danger-full-access".into(),
                        "full access, highest risk".into(),
                    ),
                ]);
            }
            _ => {}
        }
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
        let (config, response) = codex_discovery(executable, env)?;
        let help = discovery::run_cli(executable, &["--help"], env)?;
        let models = response
            .pointer("/result/data")
            .and_then(Value::as_array)
            .ok_or_else(|| "Codex CLI model/list returned no model data".to_string())?;
        let mut fields = self.config_fields();
        let mut seed = self.seed_config();
        apply_config_seed(&mut seed, config.as_ref());
        let mut model_options = Vec::new();
        let mut efforts_by_model = BTreeMap::new();
        let mut defaults_by_model = BTreeMap::new();
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
            if let Some(effort) = model
                .get("defaultReasoningEffort")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
            {
                defaults_by_model.insert(id.to_string(), effort.to_string());
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
        discovery::set_option_filter_with_defaults(
            &mut fields,
            "effort",
            "model",
            efforts_by_model,
            defaults_by_model,
        );
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

    fn append_opening_prompt(&self, argv: &mut Vec<String>, prompt: &str) -> bool {
        argv.push("--".into());
        argv.push(prompt.to_string());
        true
    }

    fn isolation_unavailable_reason(&self, language: Language) -> String {
        match language {
            Language::ZhCn => "Codex CLI 没有原生 --worktree，隔离不可用。".into(),
            Language::En => {
                "Codex CLI has no native --worktree, so isolation is unavailable.".into()
            }
        }
    }

    fn native_sync_context(
        &self,
        executable: &Path,
        env: &LaunchEnvironment,
    ) -> Option<NativeSyncLaunchContext> {
        let (program, argv_prefix) = native_invocation(executable, env);
        let fallback_cwd = super::home_dir()
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        let environment = [
            "CODEX_HOME",
            "HOME",
            "USERPROFILE",
            "XDG_CONFIG_HOME",
            "XDG_DATA_HOME",
            "APPDATA",
            "LOCALAPPDATA",
        ]
        .into_iter()
        .filter_map(|key| env.vars.get(key).cloned().map(|value| (key.into(), value)))
        .collect();
        Some(NativeSyncLaunchContext {
            program,
            argv_prefix,
            cwd: env.cwd.clone(),
            fallback_cwd,
            environment,
        })
    }

    fn native_sync_supported(&self) -> bool {
        true
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
        let permission_request = hooks::recorder_command(&recorder, "PermissionRequest");
        let user_prompt_submit = hooks::recorder_command(&recorder, "UserPromptSubmit");
        let session_end = hooks::recorder_command(&recorder, "SessionEnd");
        let stop_failure = hooks::recorder_command(&recorder, "StopFailure");
        Ok(CompletionHookPlan {
            extra_argv: vec![
                // This per-Run hook is generated by Host from a fixed recorder template.
                // Codex otherwise skips every fresh path until a human trusts its hash.
                "--dangerously-bypass-hook-trust".into(),
                "-c".into(),
                "features.hooks=true".into(),
                "-c".into(),
                format!(
                    "[[hooks.PermissionRequest]]\n[[hooks.PermissionRequest.hooks]]\ntype = \"command\"\ncommand = {permission_request:?}\ntimeout = 3\n\n[[hooks.UserPromptSubmit]]\n[[hooks.UserPromptSubmit.hooks]]\ntype = \"command\"\ncommand = {user_prompt_submit:?}\ntimeout = 3\n\n[[hooks.SessionEnd]]\n[[hooks.SessionEnd.hooks]]\ntype = \"command\"\ncommand = {session_end:?}\ntimeout = 3\n\n[[hooks.StopFailure]]\n[[hooks.StopFailure.hooks]]\ntype = \"command\"\ncommand = {stop_failure:?}\ntimeout = 3"
                ),
            ],
            extra_env: hooks::sink_env(sink_dir),
        })
    }
}

fn native_invocation(executable: &Path, env: &LaunchEnvironment) -> (PathBuf, Vec<String>) {
    let resolved = fs::canonicalize(executable).unwrap_or_else(|_| executable.to_path_buf());
    let Ok(body) = fs::read_to_string(&resolved) else {
        return (resolved, Vec::new());
    };
    let Some(shebang) = body.lines().next().and_then(|line| line.strip_prefix("#!")) else {
        return (resolved, Vec::new());
    };
    let mut parts = shebang.split_whitespace();
    let interpreter = parts.next().unwrap_or_default();
    let interpreter_arg = parts.next();
    if interpreter == "/usr/bin/env" {
        let Some(name) = interpreter_arg else {
            return (resolved, Vec::new());
        };
        if let Some(found) = env
            .path_dirs()
            .into_iter()
            .find_map(|dir| super::executable_in_for_sync(&dir, name))
        {
            return (found, vec![resolved.to_string_lossy().into_owned()]);
        }
    }
    let interpreter = PathBuf::from(interpreter);
    if interpreter.is_file() {
        let mut prefix = Vec::new();
        if let Some(arg) = interpreter_arg {
            prefix.push(arg.to_string());
        }
        prefix.push(resolved.to_string_lossy().into_owned());
        return (interpreter, prefix);
    }
    (resolved, Vec::new())
}

fn apply_config_seed(seed: &mut BTreeMap<String, String>, config: Option<&Value>) {
    let Some(config) = config else {
        return;
    };
    for (config_key, field_id) in [("approval_policy", "approval"), ("sandbox_mode", "sandbox")] {
        if let Some(value) = config
            .get(config_key)
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
        {
            seed.insert(field_id.into(), value.to_string());
        }
    }
}

fn stop_codex_app_server(child: &mut std::process::Child) {
    #[cfg(unix)]
    {
        let process_group = format!("-{}", child.id());
        let _ = std::process::Command::new("/bin/kill")
            .args(["-s", "KILL", "--", &process_group])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(windows)]
    {
        let pid = child.id().to_string();
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid, "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn codex_discovery(
    executable: &Path,
    env: &LaunchEnvironment,
) -> Result<(Option<Value>, Value), String> {
    let mut command = discovery::configured_command(executable, env);
    command
        .args(["app-server", "--stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .spawn()
        .map_err(|err| format!("could not run Codex app-server: {err}"))?;
    let mut stdin = match child.stdin.take() {
        Some(stdin) => stdin,
        None => {
            stop_codex_app_server(&mut child);
            return Err("Codex app-server stdin unavailable".into());
        }
    };
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            stop_codex_app_server(&mut child);
            return Err("Codex app-server stdout unavailable".into());
        }
    };
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
    if let Err(err) = writeln!(
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
    ) {
        stop_codex_app_server(&mut child);
        return Err(format!("could not initialize Codex app-server: {err}"));
    }
    if let Err(err) = stdin.flush() {
        stop_codex_app_server(&mut child);
        return Err(format!("could not initialize Codex app-server: {err}"));
    }
    let started = Instant::now();
    let mut requested_probes = false;
    let mut config_request_done = false;
    let mut config = None;
    let mut model_response = None;
    loop {
        let remaining = discovery::PROBE_TIMEOUT.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            if model_response.is_some() {
                if !config_request_done {
                    stop_codex_app_server(&mut child);
                    return Err("Codex CLI config/read returned no response".into());
                }
                break;
            }
            stop_codex_app_server(&mut child);
            if config_request_done {
                return Err("Codex CLI model/list returned no response".into());
            }
            return Err("Codex CLI option discovery timed out".into());
        }
        let line = match receiver.recv_timeout(remaining) {
            Ok(line) => line,
            Err(_) if model_response.is_some() => {
                if !config_request_done {
                    stop_codex_app_server(&mut child);
                    return Err("Codex CLI config/read returned no response".into());
                }
                break;
            }
            Err(_) => {
                stop_codex_app_server(&mut child);
                if config_request_done {
                    return Err("Codex CLI model/list returned no response".into());
                }
                return Err("Codex CLI option discovery timed out".into());
            }
        };
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if message.get("id").and_then(Value::as_i64) == Some(0) && !requested_probes {
            if let Err(err) = writeln!(
                stdin,
                "{}",
                serde_json::json!({
                    "method": "initialized",
                    "params": {}
                })
            ) {
                stop_codex_app_server(&mut child);
                return Err(format!("could not acknowledge Codex app-server: {err}"));
            }
            if let Err(err) = writeln!(
                stdin,
                "{}",
                serde_json::json!({
                    "method": "config/read",
                    "id": 1,
                    "params": {
                        "cwd": env.cwd.display().to_string(),
                        "includeLayers": false
                    }
                })
            ) {
                stop_codex_app_server(&mut child);
                return Err(format!("could not request Codex config: {err}"));
            }
            if let Err(err) = writeln!(
                stdin,
                "{}",
                serde_json::json!({
                    "method": "model/list",
                    "id": 2,
                    "params": { "limit": 100 }
                })
            ) {
                stop_codex_app_server(&mut child);
                return Err(format!("could not request Codex models: {err}"));
            }
            if let Err(err) = stdin.flush() {
                stop_codex_app_server(&mut child);
                return Err(format!("could not request Codex config and models: {err}"));
            }
            requested_probes = true;
            continue;
        }
        if message.get("id").and_then(Value::as_i64) == Some(1) {
            config_request_done = true;
            if message.get("error").is_none() {
                let Some(found) = message
                    .pointer("/result/config")
                    .filter(|value| value.is_object())
                    .cloned()
                else {
                    stop_codex_app_server(&mut child);
                    return Err("Codex CLI config/read returned no config data".into());
                };
                config = Some(found);
            }
            if model_response.is_some() {
                break;
            }
            continue;
        }
        if message.get("id").and_then(Value::as_i64) == Some(2) {
            model_response = Some(message);
            if config_request_done {
                break;
            }
        }
    }
    stop_codex_app_server(&mut child);
    drop(receiver);
    let _ = stdout_reader.join();
    let stderr = stderr_reader.join().unwrap_or_default();
    let model_response =
        model_response.ok_or_else(|| "Codex CLI model/list returned no response".to_string())?;
    if let Some(error) = model_response.get("error") {
        return Err(format!(
            "Codex CLI model/list failed: {error}{}",
            discovery::stderr_suffix(&stderr)
        ));
    }
    Ok((config, model_response))
}
