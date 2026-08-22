use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::{
    additional_args_field, append_additional_args, append_flag, append_switch, boolean_field,
    initial_instruction_field, local_bin, probe_binary, select_field, text_field, AgentField,
    AgentPort, ProbeResult,
};
use crate::LaunchEnvironment;

pub const ANTIGRAVITY_ID: &str = "antigravity-cli";
pub const ANTIGRAVITY_NAME: &str = "Antigravity CLI";
pub const ANTIGRAVITY_BIN: &str = "agy";

#[derive(Debug, Clone)]
pub struct AntigravityAdapter;

impl AgentPort for AntigravityAdapter {
    fn id(&self) -> &str {
        ANTIGRAVITY_ID
    }

    fn name(&self) -> &str {
        ANTIGRAVITY_NAME
    }

    fn bin(&self) -> &str {
        ANTIGRAVITY_BIN
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
            text_field("model", "model", false, false),
            select_field("effort", "effort", &["low", "medium", "high"], true, false),
            select_field(
                "execution-mode",
                "执行模式",
                &["accept-edits", "plan"],
                true,
                false,
            ),
            boolean_field("skip-permissions", "跳过权限确认", false),
            boolean_field("sandbox", "sandbox", false),
            initial_instruction_field(),
            text_field("agent", "子 Agent", false, true),
            text_field("add-dir", "额外目录", false, true),
            additional_args_field(),
        ]
    }

    fn seed_config(&self) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("model".into(), String::new()),
            ("effort".into(), "medium".into()),
            ("execution-mode".into(), "accept-edits".into()),
            ("skip-permissions".into(), "false".into()),
            ("sandbox".into(), "false".into()),
            ("initial-instruction".into(), String::new()),
            ("agent".into(), String::new()),
            ("add-dir".into(), String::new()),
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
        append_flag(&mut argv, "--mode", values.get("execution-mode"));
        append_switch(
            &mut argv,
            "--dangerously-skip-permissions",
            values,
            "skip-permissions",
        );
        append_switch(&mut argv, "--sandbox", values, "sandbox");
        append_flag(&mut argv, "--agent", values.get("agent"));
        if let Some(dirs) = values.get("add-dir") {
            for dir in dirs.split_whitespace().filter(|part| !part.is_empty()) {
                argv.push("--add-dir".into());
                argv.push(dir.to_string());
            }
        }
        append_additional_args(&mut argv, values);
        argv
    }
}
