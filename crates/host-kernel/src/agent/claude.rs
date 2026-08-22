use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::{
    additional_args_field, append_additional_args, append_flag, initial_instruction_field,
    local_bin, probe_binary, select_field, text_field, AgentField, AgentPort, ProbeResult,
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
        append_additional_args(&mut argv, values);
        argv
    }

    fn native_isolation(&self) -> bool {
        true
    }
}
