use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::agent::{format_not_found, prepare_launch_env};
use crate::agent::{
    intent_prefix, AgentField, AgentFieldKind, AgentPort, AgentSummary, IntentOption,
    PrefillSource, ProbeResult, RunIntent, RunLaunchConfig, RunLaunchForm,
};
use crate::{Language, LaunchEnvPort};

pub const INITIAL_INSTRUCTION: &str = "initial-instruction";
pub const ISOLATION_FIELD: &str = "isolation";
const AGENT_OPTIONS_CACHE_TTL: Duration = Duration::from_secs(600);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct DynamicAgentOptions {
    models: Vec<String>,
    effort_by_model: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
struct CachedAgentOptions {
    loaded_at: Instant,
    options: DynamicAgentOptions,
}

static AGENT_OPTIONS_CACHE: OnceLock<Mutex<BTreeMap<String, CachedAgentOptions>>> = OnceLock::new();

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

pub fn isolation_requested(values: &BTreeMap<String, String>) -> bool {
    values
        .get(ISOLATION_FIELD)
        .is_some_and(|value| value == "true")
}

pub fn is_git_repo(dir: &Path) -> bool {
    dir.join(".git").exists()
}

pub fn isolation_availability(
    agent: &dyn AgentPort,
    project_dir: &Path,
    language: Language,
) -> (bool, String) {
    if !agent.native_isolation() {
        (false, agent.isolation_unavailable_reason(language))
    } else if !is_git_repo(project_dir) {
        (
            false,
            match language {
                Language::ZhCn => "这个 Project 不是 git 仓库，不能隔离。".into(),
                Language::En => {
                    "This Project is not a git repository, so isolation is unavailable.".into()
                }
            },
        )
    } else {
        (true, String::new())
    }
}

pub fn side_effect_warnings(
    project_dir: &Path,
    has_active_run: bool,
    language: Language,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if has_active_run {
        warnings.push(match language {
            Language::ZhCn => {
                "这个 Project 已有活跃 Run。端口和本地锁文件可能冲突，但不禁止启动。".into()
            }
            Language::En => {
                "This Project already has an active Run. Ports and local lock files may conflict, but launch is still allowed.".into()
            }
        });
    }
    if project_dir.join(".git").join("index.lock").exists() {
        warnings.push(match language {
            Language::ZhCn => "检测到本地锁文件，仍可启动。".into(),
            Language::En => "A local lock file was found. Launch is still allowed.".into(),
        });
    }
    warnings
}

pub fn isolation_missing_tree_note(language: Language) -> String {
    match language {
        Language::ZhCn => "上次的隔离执行目录已经不在，已回到 Project 主目录。".into(),
        Language::En => {
            "The previous isolated work directory is gone. This Run uses the Project directory."
                .into()
        }
    }
}

pub fn git_worktrees(project_dir: &Path) -> Vec<PathBuf> {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(project_dir)
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.strip_prefix("worktree ").map(PathBuf::from))
        .collect()
}

pub fn new_git_worktree(project_dir: &Path, before: &[PathBuf]) -> Option<PathBuf> {
    git_worktrees(project_dir).into_iter().find(|path| {
        !same_path(path, project_dir) && !before.iter().any(|seen| same_path(seen, path))
    })
}

fn same_path(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn cached_agent_options(agent_id: &str) -> DynamicAgentOptions {
    let cache = AGENT_OPTIONS_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
    if let Some(entry) = cache
        .lock()
        .expect("agent options cache")
        .get(agent_id)
        .filter(|entry| entry.loaded_at.elapsed() < AGENT_OPTIONS_CACHE_TTL)
        .cloned()
    {
        return entry.options;
    }

    let options = agent_options_from_disk(agent_id);
    cache.lock().expect("agent options cache").insert(
        agent_id.to_string(),
        CachedAgentOptions {
            loaded_at: Instant::now(),
            options: options.clone(),
        },
    );
    options
}

fn agent_options_from_disk(agent_id: &str) -> DynamicAgentOptions {
    let Some(home) = crate::agent::home_dir() else {
        return DynamicAgentOptions::default();
    };
    let paths = match agent_id {
        crate::agent::GROK_BUILD_ID => vec![home.join(".grok/models_cache.json")],
        crate::agent::CODEX_ID => vec![home.join(".codex/models_cache.json")],
        crate::agent::CLAUDE_CODE_ID => vec![
            home.join(".claude/models_cache.json"),
            home.join(".claude/settings.json"),
        ],
        crate::agent::ANTIGRAVITY_ID => vec![
            home.join(".agy/models_cache.json"),
            home.join(".config/agy/models_cache.json"),
        ],
        _ => Vec::new(),
    };
    paths
        .into_iter()
        .find_map(|path| {
            let text = fs::read_to_string(path).ok()?;
            let value = serde_json::from_str::<Value>(&text).ok()?;
            let options = parse_agent_options(agent_id, &value);
            (options != DynamicAgentOptions::default()).then_some(options)
        })
        .unwrap_or_default()
}

fn parse_agent_options(agent_id: &str, value: &Value) -> DynamicAgentOptions {
    if agent_id == crate::agent::CLAUDE_CODE_ID && value.get("env").is_some() {
        return parse_claude_settings(value);
    }

    let mut parsed = DynamicAgentOptions::default();
    match value.get("models") {
        Some(Value::Array(models)) => {
            for model in models {
                let Some(object) = model.as_object() else {
                    continue;
                };
                if object
                    .get("visibility")
                    .and_then(Value::as_str)
                    .is_some_and(|visibility| visibility.eq_ignore_ascii_case("hidden"))
                {
                    continue;
                }
                let model_id = ["slug", "id", "model", "name"]
                    .iter()
                    .find_map(|key| object.get(*key).and_then(Value::as_str))
                    .map(str::trim)
                    .filter(|id| !id.is_empty())
                    .map(ToOwned::to_owned);
                let Some(model_id) = model_id else {
                    continue;
                };
                parsed.models.push(model_id.clone());
                let efforts = object
                    .get("supported_reasoning_levels")
                    .or_else(|| object.get("reasoning_efforts"))
                    .and_then(Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|item| {
                                item.get("effort")
                                    .or_else(|| item.get("id"))
                                    .or_else(|| item.get("value"))
                                    .and_then(Value::as_str)
                                    .map(str::trim)
                                    .filter(|effort| !effort.is_empty())
                                    .map(ToOwned::to_owned)
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if !efforts.is_empty() {
                    parsed.effort_by_model.insert(model_id, efforts);
                }
            }
        }
        Some(Value::Object(models)) => {
            for (key, model) in models {
                let info = model.get("info").unwrap_or(model);
                if info.get("hidden").and_then(Value::as_bool) == Some(true) {
                    continue;
                }
                let model_id = ["slug", "id", "model"]
                    .iter()
                    .find_map(|field| info.get(*field).and_then(Value::as_str))
                    .map(str::trim)
                    .filter(|id| !id.is_empty())
                    .unwrap_or(key)
                    .to_string();
                parsed.models.push(model_id.clone());
                let efforts = info
                    .get("reasoning_efforts")
                    .and_then(Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|item| {
                                item.get("id")
                                    .or_else(|| item.get("value"))
                                    .and_then(Value::as_str)
                                    .map(str::trim)
                                    .filter(|effort| !effort.is_empty())
                                    .map(ToOwned::to_owned)
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if !efforts.is_empty() {
                    parsed.effort_by_model.insert(model_id, efforts);
                }
            }
        }
        _ => {}
    }
    normalize_agent_options(&mut parsed);
    parsed
}

fn parse_claude_settings(value: &Value) -> DynamicAgentOptions {
    let mut parsed = DynamicAgentOptions::default();
    if let Some(env) = value.get("env").and_then(Value::as_object) {
        for (key, value) in env {
            if key.ends_with("_MODEL_NAME") {
                if let Some(model) = value
                    .as_str()
                    .map(str::trim)
                    .filter(|model| !model.is_empty())
                {
                    parsed.models.push(model.to_string());
                }
            }
        }
    }
    if let Some(model) = value
        .get("model")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|model| !model.is_empty())
    {
        parsed.models.push(model.to_string());
    }
    if let Some(effort) = value
        .get("effortLevel")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|effort| !effort.is_empty())
    {
        for model in parsed.models.clone() {
            parsed
                .effort_by_model
                .insert(model, vec![effort.to_string()]);
        }
    }
    normalize_agent_options(&mut parsed);
    parsed
}

fn normalize_agent_options(options: &mut DynamicAgentOptions) {
    options.models.sort();
    options.models.dedup();
    for efforts in options.effort_by_model.values_mut() {
        efforts.sort_by_key(|effort| match effort.as_str() {
            "low" => 0,
            "medium" => 1,
            "high" => 2,
            "xhigh" => 3,
            "max" => 4,
            "ultra" => 5,
            _ => 10,
        });
        efforts.dedup();
    }
}

pub(crate) fn enrich_agent_fields(agent_id: &str, mut fields: Vec<AgentField>) -> Vec<AgentField> {
    let options = cached_agent_options(agent_id);
    apply_agent_options(&mut fields, &options);
    fields
}

fn apply_agent_options(fields: &mut [AgentField], options: &DynamicAgentOptions) {
    if options.models.is_empty() && options.effort_by_model.is_empty() {
        return;
    }
    let mut effort_options = Vec::new();
    for field in fields.iter_mut() {
        if field.id == "model" && !options.models.is_empty() {
            field.options = options.models.clone();
            field.kind = AgentFieldKind::Select;
        }
        if field.id == "effort" && !options.effort_by_model.is_empty() {
            field.option_groups = options.effort_by_model.clone();
            for efforts in field.option_groups.values() {
                for effort in efforts {
                    if !effort_options.contains(effort) {
                        effort_options.push(effort.clone());
                    }
                }
            }
            field.options = effort_options.clone();
            field.kind = AgentFieldKind::Select;
        }
    }
}

pub fn summarize_agents(
    agents: &[std::sync::Arc<dyn AgentPort>],
    launch_env: &dyn LaunchEnvPort,
    cwd: &Path,
    language: Language,
) -> Vec<AgentSummary> {
    let captured = launch_env.capture(cwd);
    agents
        .iter()
        .map(|agent| {
            let (installed, unavailable_reason) = match &captured {
                Ok(env) => {
                    let env =
                        prepare_launch_env(env.clone(), &[], &agent.known_install_locations());
                    match agent.probe(&env) {
                        ProbeResult::Found { .. } => (true, None),
                        ProbeResult::Missing {
                            command,
                            searched_path,
                            known_locations,
                        } => (
                            false,
                            Some(format_not_found(
                                language,
                                &command,
                                &searched_path,
                                &known_locations,
                            )),
                        ),
                    }
                }
                Err(error) => (
                    false,
                    Some(match language {
                        Language::ZhCn => format!("无法读取启动环境：{error}"),
                        Language::En => format!("Could not read the launch environment: {error}"),
                    }),
                ),
            };
            AgentSummary {
                id: agent.id().to_string(),
                name: agent.name().to_string(),
                installed,
                unavailable_reason,
                fields: localize_fields(
                    enrich_agent_fields(agent.id(), agent.config_fields()),
                    language,
                ),
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
    if let Some(id) = last_successful.filter(|id| {
        agents
            .iter()
            .any(|agent| agent.id == *id && agent.installed)
    }) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_model_and_model_specific_effort_options_from_cli_cache() {
        let catalog = serde_json::json!({
            "models": [
                {
                    "slug": "gpt-5.6-sol",
                    "supported_reasoning_levels": [
                        {"effort": "low"},
                        {"effort": "high"},
                        {"effort": "ultra"}
                    ]
                },
                {
                    "slug": "gpt-5.5",
                    "supported_reasoning_levels": [
                        {"effort": "medium"}
                    ]
                }
            ]
        });
        let options = parse_agent_options(crate::agent::CODEX_ID, &catalog);
        assert_eq!(options.models, vec!["gpt-5.5", "gpt-5.6-sol"]);
        assert_eq!(
            options.effort_by_model.get("gpt-5.6-sol"),
            Some(&vec!["low".into(), "high".into(), "ultra".into()])
        );
    }

    #[test]
    fn enriches_model_and_effort_fields_while_preserving_manual_fallback() {
        let mut fields = vec![
            AgentField {
                id: "model".into(),
                label: "model".into(),
                kind: AgentFieldKind::Text,
                options: Vec::new(),
                option_groups: BTreeMap::new(),
                required: true,
                folded: false,
            },
            AgentField {
                id: "effort".into(),
                label: "effort".into(),
                kind: AgentFieldKind::Select,
                options: vec!["low".into(), "medium".into(), "high".into()],
                option_groups: BTreeMap::new(),
                required: true,
                folded: false,
            },
        ];
        let catalog = serde_json::json!({
            "models": {
                "one": {"info": {"model": "model-a", "reasoning_efforts": [{"id": "low"}, {"id": "high"}]}},
                "two": {"info": {"model": "model-b", "reasoning_efforts": [{"id": "medium"}]}}
            }
        });
        let options = parse_agent_options(crate::agent::GROK_BUILD_ID, &catalog);
        apply_agent_options(&mut fields, &options);
        assert_eq!(fields[0].options, vec!["model-a", "model-b"]);
        assert_eq!(fields[0].kind, AgentFieldKind::Select);
        assert_eq!(fields[1].option_groups["model-a"], vec!["low", "high"]);
        assert_eq!(fields[1].option_groups["model-b"], vec!["medium"]);
    }
}
