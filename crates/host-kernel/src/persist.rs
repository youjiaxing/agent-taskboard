use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::advance;
use crate::board::{self, CenterView};
use crate::owner;
use crate::pairing;
use crate::protocol::AppearanceSelection;
use crate::refresh;
use crate::{
    AppearancePreference, KernelError, Language, SystemAppearance, TrackerKind, LOCAL_HOST_ID,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataLayout {
    pub host_dir: PathBuf,
    pub desktop_client_dir: PathBuf,
    pub host_settings_path: PathBuf,
    pub host_secrets_path: PathBuf,
    pub desktop_client_settings_path: PathBuf,
    pub desktop_client_secrets_path: PathBuf,
    pub log_dir: PathBuf,
}

impl DataLayout {
    pub(crate) fn prepare(
        app_local_data_dir: &Path,
        app_log_dir: &Path,
    ) -> Result<Self, KernelError> {
        let host_dir = app_local_data_dir.join("host");
        let desktop_client_dir = app_local_data_dir.join("desktop-client");
        fs::create_dir_all(&host_dir)?;
        fs::create_dir_all(&desktop_client_dir)?;
        fs::create_dir_all(app_log_dir)?;
        Ok(Self {
            host_dir: host_dir.clone(),
            desktop_client_dir: desktop_client_dir.clone(),
            host_settings_path: host_dir.join("settings.json"),
            host_secrets_path: host_dir.join("secrets.json"),
            desktop_client_settings_path: desktop_client_dir.join("settings.json"),
            desktop_client_secrets_path: desktop_client_dir.join("secrets.json"),
            log_dir: app_log_dir.to_path_buf(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HostSettingsFile {
    #[serde(default)]
    pub(crate) id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) focused_project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) projects: Vec<StoredProject>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) project_tombstones: Vec<StoredProjectTombstone>,
    #[serde(default = "default_refresh_interval_ms")]
    pub(crate) refresh_interval_ms: u64,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) agent_launch_defaults: BTreeMap<String, BTreeMap<String, BTreeMap<String, String>>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) last_successful_agent: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) auto_advance: bool,
}

impl Default for HostSettingsFile {
    fn default() -> Self {
        Self {
            id: String::new(),
            focused_project_id: None,
            projects: Vec::new(),
            project_tombstones: Vec::new(),
            refresh_interval_ms: refresh::DEFAULT_REFRESH_INTERVAL_MS,
            agent_launch_defaults: BTreeMap::new(),
            last_successful_agent: BTreeMap::new(),
            auto_advance: false,
        }
    }
}

pub(crate) fn default_refresh_interval_ms() -> u64 {
    refresh::DEFAULT_REFRESH_INTERVAL_MS
}

pub(crate) fn default_tracker_kind() -> TrackerKind {
    TrackerKind::Github
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoredProject {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) local_path: PathBuf,
    /// Tracker 类型；旧数据缺失时默认 GitHub。
    #[serde(default = "default_tracker_kind")]
    pub(crate) tracker: TrackerKind,
    pub(crate) github_host: String,
    pub(crate) repository: String,
    #[serde(default)]
    pub(crate) auto_advance: bool,
    #[serde(default)]
    pub(crate) restore_auto_advance: bool,
    #[serde(default = "advance::default_restore_delay_ms")]
    pub(crate) restore_delay_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoredProjectTombstone {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) local_path: PathBuf,
    pub(crate) tracker: TrackerKind,
    pub(crate) github_host: String,
    pub(crate) repository: String,
    pub(crate) revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HostSecretsFile {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) clients: Vec<pairing::IssuedClient>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) github_pats: BTreeMap<String, String>,
}

pub(crate) fn load_or_init_host_settings(path: &Path) -> Result<HostSettingsFile, KernelError> {
    let mut file = if path.exists() {
        serde_json::from_str(&fs::read_to_string(path)?)?
    } else {
        HostSettingsFile::default()
    };
    if file.id.trim().is_empty() {
        file.id = pairing::random_id();
        write_json(path, &file)?;
    }
    Ok(file)
}

pub(crate) fn load_paired_clients(path: &Path) -> Result<Vec<pairing::IssuedClient>, KernelError> {
    if !path.exists() {
        write_json_inner(path, &HostSecretsFile::default(), true)?;
        return Ok(Vec::new());
    }
    owner::restrict_to_owner(path)?;
    let raw = fs::read_to_string(path)?;
    let file = serde_json::from_str::<HostSecretsFile>(&raw).unwrap_or_default();
    Ok(file.clients)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ClientSettingsFile {
    pub(crate) language: Language,
    #[serde(default)]
    pub(crate) appearance_preference: AppearancePreference,
    #[serde(default = "local_host_id")]
    pub(crate) focused_host_id: String,
    #[serde(default)]
    pub(crate) remote_hosts: Vec<pairing::SavedRemoteHost>,
    #[serde(default = "default_recent_limit")]
    pub(crate) recent_completed_limit: u32,
    #[serde(default)]
    pub(crate) center_view: CenterView,
    #[serde(default = "default_true")]
    pub(crate) show_command_preview: bool,
    #[serde(default = "default_true")]
    pub(crate) notify_desktop: bool,
    #[serde(default = "default_true")]
    pub(crate) notify_sound: bool,
}

pub(crate) fn default_true() -> bool {
    true
}

pub(crate) fn default_recent_limit() -> u32 {
    board::DEFAULT_RECENT_LIMIT
}

pub(crate) fn local_host_id() -> String {
    LOCAL_HOST_ID.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct ClientSecretsFile {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) tokens: BTreeMap<String, String>,
}

pub(crate) fn load_or_init_appearance(
    path: &Path,
    system_locale: &str,
    _system_appearance: SystemAppearance,
) -> Result<
    (
        AppearanceSelection,
        String,
        Vec<pairing::SavedRemoteHost>,
        u32,
        CenterView,
        bool,
        bool,
        bool,
    ),
    KernelError,
> {
    if path.exists() {
        let raw = fs::read_to_string(path)?;
        // 旧文件里的 theme/lastLightTheme 等字段由 serde 直接忽略，偏好回落到默认 system。
        if let Ok(file) = serde_json::from_str::<ClientSettingsFile>(&raw) {
            return Ok((
                AppearanceSelection {
                    language: file.language,
                    appearance_preference: file.appearance_preference,
                },
                file.focused_host_id,
                file.remote_hosts,
                board::clamp_recent_limit(file.recent_completed_limit),
                file.center_view,
                file.show_command_preview,
                file.notify_desktop,
                file.notify_sound,
            ));
        }
    }
    let language = match_language(system_locale);
    let appearance = AppearanceSelection {
        language,
        appearance_preference: AppearancePreference::System,
    };
    let file = ClientSettingsFile {
        language,
        appearance_preference: AppearancePreference::System,
        focused_host_id: LOCAL_HOST_ID.to_string(),
        remote_hosts: Vec::new(),
        recent_completed_limit: board::DEFAULT_RECENT_LIMIT,
        center_view: CenterView::Board,
        show_command_preview: true,
        notify_desktop: true,
        notify_sound: true,
    };
    write_json(path, &file)?;
    Ok((
        appearance,
        LOCAL_HOST_ID.to_string(),
        Vec::new(),
        board::DEFAULT_RECENT_LIMIT,
        CenterView::Board,
        true,
        true,
        true,
    ))
}

pub(crate) fn load_client_tokens(path: &Path) -> Result<BTreeMap<String, String>, KernelError> {
    if !path.exists() {
        write_json_inner(path, &ClientSecretsFile::default(), true)?;
        return Ok(BTreeMap::new());
    }
    owner::restrict_to_owner(path)?;
    let raw = fs::read_to_string(path)?;
    let file = serde_json::from_str::<ClientSecretsFile>(&raw).unwrap_or_default();
    Ok(file.tokens)
}

pub(crate) fn occupied_reason(language: Language, port: u16) -> String {
    match language {
        Language::ZhCn => {
            format!("本机网页入口没起来：端口 {port} 已被占用。桌面窗口可以继续用。")
        }
        Language::En => format!(
            "The local web entry could not start: port {port} is already in use. The desktop window still works."
        ),
    }
}

pub(crate) fn host_not_running_reason(language: Language) -> String {
    match language {
        Language::ZhCn => "本机没有在跑 Host，所以没有这份回环页。".into(),
        Language::En => {
            "The local Host is not running, so this loopback page is not available.".into()
        }
    }
}

pub(crate) fn match_language(locale: &str) -> Language {
    let normalized = locale.to_ascii_lowercase().replace('_', "-");
    if normalized == "zh" || normalized.starts_with("zh-") {
        Language::ZhCn
    } else {
        Language::En
    }
}

pub(crate) fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), KernelError> {
    write_json_inner(path, value, false)
}

pub(crate) fn write_json_inner<T: Serialize>(
    path: &Path,
    value: &T,
    owner_only: bool,
) -> Result<(), KernelError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_vec_pretty(value)?;
    let tmp = path.with_extension("json.tmp");
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(&body)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
    }
    if owner_only {
        owner::restrict_to_owner(&tmp)?;
    }
    owner::replace_file(&tmp, path)?;
    if owner_only {
        owner::restrict_to_owner(path)?;
    }
    Ok(())
}

pub(crate) fn read_github_pats(path: &Path) -> Result<BTreeMap<String, String>, KernelError> {
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    owner::restrict_to_owner(path)?;
    let raw = fs::read_to_string(path)?;
    let file = serde_json::from_str::<HostSecretsFile>(&raw).unwrap_or_default();
    Ok(file.github_pats)
}

pub(crate) fn read_github_pat(path: &Path, host: &str) -> Option<String> {
    read_github_pats(path)
        .ok()
        .and_then(|pats| pats.get(host).cloned())
        .and_then(|token| {
            let token = token.trim().to_string();
            (!token.is_empty()).then_some(token)
        })
}
