use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::{
    additional_args_field, append_additional_args, append_flag, append_isolation_flag,
    boolean_field, discovery, home_dir, hooks, initial_instruction_field, probe_binary,
    select_field, text_field, AgentConfigDiscovery, AgentField, AgentPort, CompletionHookPlan,
    ProbeResult,
};
use crate::LaunchEnvironment;

pub const GROK_BUILD_ID: &str = "grok-build";
pub const GROK_BUILD_NAME: &str = "Grok Build";
pub const GROK_BIN: &str = "grok";

#[derive(Debug, Clone)]
pub struct GrokAdapter;

impl GrokAdapter {
    pub fn known_location() -> Option<PathBuf> {
        home_dir().map(|home| home.join(".grok").join("bin"))
    }
}

impl AgentPort for GrokAdapter {
    fn id(&self) -> &str {
        GROK_BUILD_ID
    }

    fn name(&self) -> &str {
        GROK_BUILD_NAME
    }

    fn bin(&self) -> &str {
        GROK_BIN
    }

    fn known_install_locations(&self) -> Vec<PathBuf> {
        Self::known_location().into_iter().collect()
    }

    fn probe(&self, env: &LaunchEnvironment) -> ProbeResult {
        probe_binary(self.bin(), env, &self.known_install_locations())
    }

    fn assemble_argv(&self, executable: &Path) -> Vec<String> {
        vec![executable.to_string_lossy().into_owned()]
    }

    fn config_fields(&self) -> Vec<AgentField> {
        grok_fields()
    }

    fn seed_config(&self) -> BTreeMap<String, String> {
        grok_seed()
    }

    fn discover_config(
        &self,
        executable: &Path,
        env: &LaunchEnvironment,
    ) -> Result<AgentConfigDiscovery, String> {
        let models_output = discovery::run_cli(executable, &["models"], env)?;
        let help = discovery::run_cli(executable, &["--help"], env)?;
        let models = bullet_models(&models_output);
        if models.is_empty() {
            return Err("Grok CLI returned an empty model list".into());
        }
        let mut fields = self.config_fields();
        let mut seed = self.seed_config();
        discovery::set_options(&mut fields, "model", models);
        discovery::set_options_if_found(
            &mut fields,
            "effort",
            discovery::option_values(&help, "--reasoning-effort"),
        );
        discovery::set_options_if_found(
            &mut fields,
            "permission-mode",
            discovery::option_values(&help, "--permission-mode"),
        );
        if let Some(model) = discovery::prefixed_value(&models_output, "Default model:") {
            seed.insert("model".into(), model);
        }
        Ok(AgentConfigDiscovery { fields, seed })
    }

    fn assemble_argv_for(
        &self,
        executable: &Path,
        values: &BTreeMap<String, String>,
    ) -> Vec<String> {
        grok_argv(executable, values, true)
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
        let overlay = hooks::grok_home_overlay(sink_dir)?;
        hooks::write_json_hooks(
            &overlay.join("hooks").join("agent-taskboard.json"),
            &recorder,
        )?;
        let mut extra_env = hooks::sink_env(sink_dir);
        extra_env.insert("GROK_HOME".into(), overlay.to_string_lossy().into_owned());
        Ok(CompletionHookPlan {
            extra_argv: Vec::new(),
            extra_env,
        })
    }
}

pub(super) fn grok_fields() -> Vec<AgentField> {
    vec![
        text_field("model", "model", true, false),
        select_field("effort", "effort", &["low", "medium", "high"], true, false),
        select_field("permission-mode", "权限模式", &[], true, false),
        boolean_field("always-approve", "alwaysApprove", false),
        select_field("sandbox", "sandbox", &[], true, false),
        initial_instruction_field(),
        additional_args_field(),
    ]
}

pub(super) fn grok_seed() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("model".into(), "grok-4.6".into()),
        ("effort".into(), "high".into()),
        ("permission-mode".into(), "default".into()),
        ("always-approve".into(), "false".into()),
        ("sandbox".into(), "off".into()),
        ("initial-instruction".into(), String::new()),
        ("additional-args".into(), String::new()),
    ])
}

pub(super) fn grok_argv(
    executable: &Path,
    values: &BTreeMap<String, String>,
    native_isolation: bool,
) -> Vec<String> {
    let mut argv = vec![executable.to_string_lossy().into_owned()];
    append_flag(&mut argv, "--model", values.get("model"));
    append_flag(&mut argv, "--effort", values.get("effort"));
    append_flag(
        &mut argv,
        "--permission-mode",
        values.get("permission-mode"),
    );
    append_flag(&mut argv, "--sandbox", values.get("sandbox"));
    if values
        .get("always-approve")
        .is_some_and(|value| value == "true")
    {
        argv.push("--always-approve".into());
    }
    append_isolation_flag(&mut argv, values, native_isolation);
    append_additional_args(&mut argv, values);
    argv
}

fn bullet_models(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let value = line
                .strip_prefix("* ")
                .or_else(|| line.strip_prefix("- "))?;
            value
                .split_whitespace()
                .next()
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
        .collect()
}
