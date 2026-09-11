use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{
    additional_args_field, append_additional_args, append_flag, append_isolation_flag,
    boolean_field, discovery, home_dir, hooks, initial_instruction_field, probe_binary,
    select_field, AgentConfigDiscovery, AgentField, AgentPort, CompletionHookPlan, ProbeResult,
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
        discovery::set_options(&mut fields, "model", models.clone());
        let mut effort_by_model = model_efforts_from_home(env);
        effort_by_model.retain(|model, _| models.iter().any(|option| option == model));
        let options_by_value = effort_by_model
            .iter()
            .map(|(model, info)| (model.clone(), info.efforts.clone()))
            .collect();
        let defaults_by_value = effort_by_model
            .iter()
            .filter_map(|(model, info)| {
                info.default_effort
                    .clone()
                    .map(|effort| (model.clone(), effort))
            })
            .collect();
        discovery::set_option_filter_with_defaults(
            &mut fields,
            "effort",
            "model",
            options_by_value,
            defaults_by_value,
        );
        let mut effort_options = discovery::option_values(&help, "--reasoning-effort");
        for info in effort_by_model.values() {
            for effort in &info.efforts {
                if !effort_options.iter().any(|option| option == effort) {
                    effort_options.push(effort.clone());
                }
            }
        }
        discovery::set_options_if_found(&mut fields, "effort", effort_options);
        discovery::set_options_if_found(
            &mut fields,
            "permission-mode",
            discovery::option_values(&help, "--permission-mode"),
        );
        if let Some(model) = discovery::prefixed_value(&models_output, "Default model:") {
            seed.insert("model".into(), model.clone());
            if let Some(effort) = effort_by_model
                .get(&model)
                .and_then(|info| info.default_effort.clone())
            {
                seed.insert("effort".into(), effort);
            }
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
        select_field("model", "model", &[], true, false),
        select_field("effort", "effort", &["low", "medium", "high"], true, false),
        select_field("permission-mode", "权限模式", &[], true, false),
        boolean_field("always-approve", "alwaysApprove", false),
        select_field(
            "sandbox",
            "sandbox",
            &["off", "workspace", "devbox", "read-only", "strict"],
            true,
            false,
        ),
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ModelEffortInfo {
    efforts: Vec<String>,
    default_effort: Option<String>,
}

fn grok_home(env: &LaunchEnvironment) -> Option<PathBuf> {
    env.vars
        .get("HOME")
        .cloned()
        .or_else(|| env.vars.get("USERPROFILE").cloned())
        .map(PathBuf::from)
        .or_else(home_dir)
}

fn model_efforts_from_home(env: &LaunchEnvironment) -> BTreeMap<String, ModelEffortInfo> {
    let Some(home) = grok_home(env) else {
        return BTreeMap::new();
    };
    let grok_dir = home.join(".grok");
    let mut by_model = fs::read_to_string(grok_dir.join("models_cache.json"))
        .map(|raw| model_efforts_from_cache_raw(&raw))
        .unwrap_or_default();
    if let Ok(raw) = fs::read_to_string(grok_dir.join("config.toml")) {
        for (id, info) in model_efforts_from_config_raw(&raw) {
            if !info.efforts.is_empty() {
                by_model.insert(id, info);
            }
        }
    }
    by_model
}

fn effort_value(effort: &Value) -> Option<String> {
    effort
        .get("value")
        .or_else(|| effort.get("id"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
}

fn default_effort_from_list(
    efforts: &[String],
    marked: Option<String>,
    declared: Option<String>,
    fallback: Option<String>,
) -> Option<String> {
    marked
        .filter(|value| efforts.iter().any(|effort| effort == value))
        .or_else(|| declared.filter(|value| efforts.iter().any(|effort| effort == value)))
        .or_else(|| fallback.filter(|value| efforts.iter().any(|effort| effort == value)))
}

fn model_efforts_from_cache_raw(raw: &str) -> BTreeMap<String, ModelEffortInfo> {
    let Ok(root) = serde_json::from_str::<Value>(raw) else {
        return BTreeMap::new();
    };
    root.get("models")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|models| models.iter())
        .filter_map(|(id, model)| {
            let info = model.get("info")?;
            let array = info.get("reasoning_efforts")?.as_array()?;
            let mut efforts = Vec::new();
            let mut marked = None;
            for effort in array {
                let Some(value) = effort_value(effort) else {
                    continue;
                };
                if marked.is_none() && effort.get("default").and_then(Value::as_bool) == Some(true)
                {
                    marked = Some(value.clone());
                }
                efforts.push(value);
            }
            if efforts.is_empty() {
                return None;
            }
            let declared = info
                .get("reasoning_effort")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(ToOwned::to_owned);
            Some((
                id.clone(),
                ModelEffortInfo {
                    default_effort: default_effort_from_list(&efforts, marked, declared, None),
                    efforts,
                },
            ))
        })
        .collect()
}

fn toml_effort_value(effort: &toml::Value) -> Option<String> {
    effort
        .get("value")
        .or_else(|| effort.get("id"))
        .and_then(toml::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
}

fn model_efforts_from_config_raw(raw: &str) -> BTreeMap<String, ModelEffortInfo> {
    let Ok(root) = raw.parse::<toml::Value>() else {
        return BTreeMap::new();
    };
    let fallback = root
        .get("models")
        .and_then(|models| models.get("default_reasoning_effort"))
        .and_then(toml::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned);
    let Some(models) = root.get("model").and_then(toml::Value::as_table) else {
        return BTreeMap::new();
    };
    models
        .iter()
        .filter_map(|(id, table)| {
            let table = table.as_table()?;
            let array = table.get("reasoning_efforts")?.as_array()?;
            let mut efforts = Vec::new();
            let mut marked = None;
            for effort in array {
                let Some(value) = toml_effort_value(effort) else {
                    continue;
                };
                if marked.is_none()
                    && effort.get("default").and_then(toml::Value::as_bool) == Some(true)
                {
                    marked = Some(value.clone());
                }
                efforts.push(value);
            }
            if efforts.is_empty() {
                return None;
            }
            let declared = table
                .get("reasoning_effort")
                .and_then(toml::Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(ToOwned::to_owned);
            Some((
                id.clone(),
                ModelEffortInfo {
                    default_effort: default_effort_from_list(
                        &efforts,
                        marked,
                        declared,
                        fallback.clone(),
                    ),
                    efforts,
                },
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{model_efforts_from_cache_raw, model_efforts_from_config_raw};

    #[test]
    fn grok_model_cache_exposes_efforts_per_model() {
        let options = model_efforts_from_cache_raw(
            r#"{
                "models": {
                    "grok-fast": {"info": {"reasoning_efforts": [{"value":"low"},{"value":"medium"}]}},
                    "grok-deep": {"info": {"reasoning_efforts": [{"id":"high"},{"id":"xhigh"}]}}
                }
            }"#,
        );
        assert_eq!(options["grok-fast"].efforts, vec!["low", "medium"]);
        assert_eq!(options["grok-deep"].efforts, vec!["high", "xhigh"]);
    }

    #[test]
    fn grok_model_cache_uses_marked_or_declared_default_effort() {
        let options = model_efforts_from_cache_raw(
            r#"{
                "models": {
                    "grok-fast": {
                        "info": {
                            "reasoning_effort": "medium",
                            "reasoning_efforts": [
                                {"value":"low"},
                                {"value":"medium","default":true}
                            ]
                        }
                    },
                    "grok-deep": {
                        "info": {
                            "reasoning_effort": "xhigh",
                            "reasoning_efforts": [{"id":"high"},{"id":"xhigh"}]
                        }
                    }
                }
            }"#,
        );
        assert_eq!(
            options["grok-fast"].default_effort.as_deref(),
            Some("medium")
        );
        assert_eq!(
            options["grok-deep"].default_effort.as_deref(),
            Some("xhigh")
        );
    }

    #[test]
    fn grok_model_config_exposes_custom_model_efforts_and_defaults() {
        let options = model_efforts_from_config_raw(
            r#"
[models]
default_reasoning_effort = "medium"

[model."grok-fast"]
context_window = 300000

[model.custom-max]
supports_reasoning_effort = true
reasoning_effort = "max"

[[model.custom-max.reasoning_efforts]]
id = "max"
value = "max"
default = true

[[model.custom-max.reasoning_efforts]]
id = "high"
value = "high"

[model.custom-high]
supports_reasoning_effort = true
reasoning_effort = "high"

[[model.custom-high.reasoning_efforts]]
value = "xhigh"
[[model.custom-high.reasoning_efforts]]
value = "high"
"#,
        );
        assert!(!options.contains_key("grok-fast"));
        assert_eq!(options["custom-max"].efforts, vec!["max", "high"]);
        assert_eq!(options["custom-max"].default_effort.as_deref(), Some("max"));
        assert_eq!(options["custom-high"].efforts, vec!["xhigh", "high"]);
        assert_eq!(
            options["custom-high"].default_effort.as_deref(),
            Some("high")
        );
    }
}
