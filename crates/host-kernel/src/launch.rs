use std::collections::BTreeMap;
use std::path::Path;

use crate::agent::prepare_launch_env;
use crate::agent::{
    intent_prefix, AgentField, AgentFieldKind, AgentPort, AgentSummary, IntentOption,
    PrefillSource, ProbeResult, RunIntent, RunLaunchConfig, RunLaunchForm,
};
use crate::{Language, LaunchEnvPort};

pub const INITIAL_INSTRUCTION: &str = "initial-instruction";
pub const ISOLATION_FIELD: &str = "isolation";

pub fn is_ephemeral_field(id: &str) -> bool {
    id == INITIAL_INSTRUCTION || id == ISOLATION_FIELD
}

pub fn remembered_values(values: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    values
        .iter()
        .filter(|(id, _)| !is_ephemeral_field(id))
        .map(|(id, value)| (id.clone(), value.clone()))
        .collect()
}

pub fn merge_prefill(
    seed: &BTreeMap<String, String>,
    current_project: Option<&BTreeMap<String, String>>,
    other_project: Option<&BTreeMap<String, String>>,
) -> (BTreeMap<String, String>, PrefillSource) {
    let mut values = seed.clone();
    if let Some(current) = current_project.filter(|map| !map.is_empty()) {
        overlay(&mut values, current);
        (values, PrefillSource::CurrentProject)
    } else if let Some(other) = other_project.filter(|map| !map.is_empty()) {
        overlay(&mut values, other);
        (values, PrefillSource::OtherProject)
    } else {
        (values, PrefillSource::CliSeed)
    }
}

fn overlay(values: &mut BTreeMap<String, String>, remembered: &BTreeMap<String, String>) {
    for (id, value) in remembered {
        if !is_ephemeral_field(id) {
            values.insert(id.clone(), value.clone());
        }
    }
}

pub fn other_project_memory<'a>(
    defaults: &'a BTreeMap<String, BTreeMap<String, BTreeMap<String, String>>>,
    project_id: &str,
    agent_id: &str,
) -> Option<&'a BTreeMap<String, String>> {
    defaults.iter().find_map(|(id, agents)| {
        if id == project_id {
            None
        } else {
            agents.get(agent_id).filter(|map| !map.is_empty())
        }
    })
}

pub fn intent_options(language: Language) -> Vec<IntentOption> {
    [
        RunIntent::Modify,
        RunIntent::Continue,
        RunIntent::Answer,
        RunIntent::Review,
    ]
    .into_iter()
    .map(|intent| IntentOption {
        id: intent_id(intent).into(),
        label: intent_label(intent, language).into(),
        prefix: intent_prefix(Some(intent), language),
    })
    .collect()
}

pub fn intent_id(intent: RunIntent) -> &'static str {
    match intent {
        RunIntent::Modify => "modify",
        RunIntent::Continue => "continue",
        RunIntent::Answer => "answer",
        RunIntent::Review => "review",
    }
}

fn intent_label(intent: RunIntent, language: Language) -> &'static str {
    match (language, intent) {
        (Language::ZhCn, RunIntent::Modify) => "修改",
        (Language::ZhCn, RunIntent::Continue) => "继续",
        (Language::ZhCn, RunIntent::Answer) => "只回答",
        (Language::ZhCn, RunIntent::Review) => "复查",
        (Language::En, RunIntent::Modify) => "Modify",
        (Language::En, RunIntent::Continue) => "Continue",
        (Language::En, RunIntent::Answer) => "Answer only",
        (Language::En, RunIntent::Review) => "Review",
    }
}

pub fn localize_fields(fields: Vec<AgentField>, language: Language) -> Vec<AgentField> {
    fields
        .into_iter()
        .map(|mut field| {
            field.label = field_label(&field.id, language).unwrap_or(field.label);
            field
        })
        .collect()
}

fn field_label(id: &str, language: Language) -> Option<String> {
    let label = match (language, id) {
        (Language::ZhCn, "permission-mode") => "权限模式",
        (Language::En, "permission-mode") => "Permission mode",
        (Language::ZhCn, "always-approve") => "alwaysApprove",
        (Language::En, "always-approve") => "alwaysApprove",
        (Language::ZhCn, INITIAL_INSTRUCTION) => "初始指令",
        (Language::En, INITIAL_INSTRUCTION) => "Initial instruction",
        (Language::ZhCn, "additional-args") => "附加参数",
        (Language::En, "additional-args") => "Extra arguments",
        (Language::ZhCn, "approval") => "approval",
        (Language::En, "approval") => "approval",
        (Language::ZhCn, "profile") => "profile",
        (Language::En, "profile") => "profile",
        (Language::ZhCn, "execution-mode") => "执行模式",
        (Language::En, "execution-mode") => "Execution mode",
        (Language::ZhCn, "skip-permissions") => "跳过权限确认",
        (Language::En, "skip-permissions") => "Skip permission prompts",
        (Language::ZhCn, "agent") => "子 Agent",
        (Language::En, "agent") => "Sub-agent",
        (Language::ZhCn, "add-dir") => "额外目录",
        (Language::En, "add-dir") => "Additional directories",
        (_, "model") => "model",
        (_, "effort") => "effort",
        (_, "sandbox") => "sandbox",
        _ => return None,
    };
    Some(label.into())
}

pub fn unknown_enum_warnings(
    fields: &[AgentField],
    values: &BTreeMap<String, String>,
    language: Language,
) -> Vec<String> {
    let mut warnings = Vec::new();
    for field in fields {
        if field.kind != AgentFieldKind::Select || field.options.is_empty() {
            continue;
        }
        let Some(value) = values.get(&field.id).map(|value| value.trim()) else {
            continue;
        };
        if value.is_empty() || field.options.iter().any(|option| option == value) {
            continue;
        }
        warnings.push(match language {
            Language::ZhCn => {
                format!("{value} 不是已知的 {}，仍可启动。", field.label)
            }
            Language::En => {
                format!(
                    "{value} is not a known {}; launch is still allowed.",
                    field.label
                )
            }
        });
    }
    warnings
}

pub fn missing_required(
    fields: &[AgentField],
    values: &BTreeMap<String, String>,
    language: Language,
) -> Option<String> {
    for field in fields {
        if !field.required || is_ephemeral_field(&field.id) {
            continue;
        }
        let empty = values
            .get(&field.id)
            .map(|value| value.trim().is_empty())
            .unwrap_or(true);
        if empty {
            return Some(match language {
                Language::ZhCn => format!("请填写 {}。", field.label),
                Language::En => format!("{} is required.", field.label),
            });
        }
    }
    None
}

pub fn opening_required(opening: &str, language: Language) -> Option<String> {
    if opening.trim().is_empty() {
        Some(match language {
            Language::ZhCn => "请填写要 Agent 做什么。".into(),
            Language::En => "Tell the Agent what to do.".into(),
        })
    } else {
        None
    }
}

pub fn command_preview(argv: &[String]) -> String {
    argv.join(" ")
}

pub fn isolation_reason(language: Language) -> String {
    match language {
        Language::ZhCn => "本票先关掉隔离，留给隔离票。".into(),
        Language::En => "Isolation is disabled on this ticket.".into(),
    }
}

pub fn summarize_agents(
    agents: &[std::sync::Arc<dyn AgentPort>],
    launch_env: &dyn LaunchEnvPort,
    cwd: &Path,
    language: Language,
) -> Vec<AgentSummary> {
    let captured = launch_env.capture(cwd).ok();
    agents
        .iter()
        .map(|agent| {
            let installed = captured
                .as_ref()
                .map(|env| {
                    let env =
                        prepare_launch_env(env.clone(), &[], &agent.known_install_locations());
                    matches!(agent.probe(&env), ProbeResult::Found { .. })
                })
                .unwrap_or(false);
            AgentSummary {
                id: agent.id().to_string(),
                name: agent.name().to_string(),
                installed,
                fields: localize_fields(agent.config_fields(), language),
            }
        })
        .collect()
}

pub fn preview_argv(agent: &dyn AgentPort, values: &BTreeMap<String, String>) -> Vec<String> {
    agent.assemble_argv_for(Path::new(agent.bin()), values)
}

pub fn default_agent_id(
    agents: &[AgentSummary],
    last_successful: Option<&str>,
    requested: Option<&str>,
) -> String {
    if let Some(id) = requested.filter(|id| agents.iter().any(|agent| agent.id == *id)) {
        return id.to_string();
    }
    if let Some(id) = last_successful.filter(|id| agents.iter().any(|agent| agent.id == *id)) {
        return id.to_string();
    }
    agents
        .iter()
        .find(|agent| agent.installed)
        .or_else(|| agents.first())
        .map(|agent| agent.id.clone())
        .unwrap_or_default()
}

pub fn apply_submitted_form(form: &mut RunLaunchForm, config: &RunLaunchConfig) {
    form.selected_agent_id = config.agent_id.clone();
    form.values = config.values.clone();
    form.opening_text = config.opening_text.clone();
    form.error = None;
}
