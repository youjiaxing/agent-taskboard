use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::{
    additional_args_field, append_additional_args, append_flag, append_isolation_flag, discovery,
    hooks, initial_instruction_field, local_bin, probe_binary, select_field, text_field,
    AgentConfigDiscovery, AgentField, AgentPort, CompletionHookPlan, ProbeResult,
};
use crate::LaunchEnvironment;

pub const CLAUDE_CODE_ID: &str = "claude-code";
pub const CLAUDE_CODE_NAME: &str = "Claude Code";
pub const CLAUDE_BIN: &str = "claude";

#[derive(Debug, Clone)]
pub struct ClaudeAdapter;

impl AgentPort for ClaudeAdapter {
    fn id(&self) -> &str {
        CLAUDE_CODE_ID
    }

    fn name(&self) -> &str {
        CLAUDE_CODE_NAME
    }

    fn bin(&self) -> &str {
        CLAUDE_BIN
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
            select_field(
                "effort",
                "effort",
                &["low", "medium", "high", "xhigh", "max"],
                true,
                false,
            ),
            select_field(
                "permission-mode",
                "权限模式",
                &[
                    "acceptEdits",
                    "auto",
                    "bypassPermissions",
                    "manual",
                    "dontAsk",
                    "plan",
                ],
                true,
                false,
            ),
            initial_instruction_field(),
            additional_args_field(),
        ]
    }

    fn seed_config(&self) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("model".into(), "sonnet".into()),
            ("effort".into(), "medium".into()),
            ("permission-mode".into(), "acceptEdits".into()),
            ("initial-instruction".into(), String::new()),
            ("additional-args".into(), String::new()),
        ])
    }

    fn discover_config(
        &self,
        executable: &Path,
        env: &LaunchEnvironment,
    ) -> Result<AgentConfigDiscovery, String> {
        let help = discovery::run_cli(executable, &["--help"], env)?;
        let mut fields = self.config_fields();
        discovery::set_options_if_found(
            &mut fields,
            "model",
            discovery::quoted_values_near(&help, "--model"),
        );
        discovery::set_options_if_found(
            &mut fields,
            "effort",
            discovery::option_values(&help, "--effort"),
        );
        discovery::set_options_if_found(
            &mut fields,
            "permission-mode",
            discovery::option_values(&help, "--permission-mode"),
        );
        Ok(AgentConfigDiscovery {
            fields,
            seed: self.seed_config(),
        })
    }

    fn assemble_argv_for(
        &self,
        executable: &Path,
        values: &BTreeMap<String, String>,
    ) -> Vec<String> {
        let mut argv = vec![executable.to_string_lossy().into_owned()];
        append_flag(&mut argv, "--model", values.get("model"));
        append_flag(&mut argv, "--effort", values.get("effort"));
        append_flag(
            &mut argv,
            "--permission-mode",
            values.get("permission-mode"),
        );
        append_isolation_flag(&mut argv, values, true);
        append_additional_args(&mut argv, values);
        argv
    }

    fn native_isolation(&self) -> bool {
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
        let settings = sink_dir.join("claude-settings.json");
        hooks::write_json_hooks(&settings, &recorder)?;
        Ok(CompletionHookPlan {
            extra_argv: vec!["--settings".into(), settings.to_string_lossy().into_owned()],
            extra_env: hooks::sink_env(sink_dir),
        })
    }
}
