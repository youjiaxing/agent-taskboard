use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::{
    additional_args_field, append_additional_args, append_flag, hooks, initial_instruction_field,
    local_bin, probe_binary, select_field, text_field, AgentField, AgentPort, CompletionHookPlan,
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
