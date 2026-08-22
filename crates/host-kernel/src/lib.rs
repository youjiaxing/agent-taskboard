//! Host 内核：桌面窗口、浏览器和以后的远程 Client 都走这一条接缝。

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const LOCAL_HOST_ID: &str = "local";

#[derive(Debug, Clone)]
pub struct BootRequest {
    pub app_local_data_dir: PathBuf,
    pub app_log_dir: PathBuf,
    pub system_locale: String,
    pub system_appearance: SystemAppearance,
    pub host_display_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemAppearance {
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "en")]
    En,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    WarmPaper,
    PlainPaper,
    PlainNight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    HideWindow,
    ShowWindow,
    QuitHost,
    SetLanguage(Language),
    SetTheme(Theme),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProcessIntent {
    KeepRunning,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EmptyAction {
    RegisterFirstProject,
    PairAnotherHost,
}

#[derive(Debug, Clone)]
pub struct CommandOutcome {
    pub snapshot: HostSnapshot,
    pub process: ProcessIntent,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataLayout {
    pub host_dir: PathBuf,
    pub desktop_client_dir: PathBuf,
    pub host_settings_path: PathBuf,
    pub host_secrets_path: PathBuf,
    pub desktop_client_settings_path: PathBuf,
    pub log_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostSummary {
    pub id: String,
    pub display_name: String,
    pub local: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceState {
    pub language: Language,
    pub theme: Theme,
    pub last_light_theme: Theme,
    pub languages: Vec<Language>,
    pub themes: Vec<Theme>,
    pub follow_system: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellCopy {
    pub app_name: String,
    pub register_first_project: String,
    pub pair_another_host: String,
    pub no_project_title: String,
    pub no_project_body: String,
    pub quit_host: String,
    pub show_window: String,
    pub settings: String,
    pub language: String,
    pub theme: String,
    pub language_zh: String,
    pub language_en: String,
    pub theme_warm_paper: String,
    pub theme_plain_paper: String,
    pub theme_plain_night: String,
    pub hosts: String,
    pub projects: String,
    pub this_machine: String,
    pub shade_light: String,
    pub shade_dark: String,
    pub edit_menu: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostSnapshot {
    pub running: bool,
    pub window_visible: bool,
    pub focused_host_id: String,
    pub hosts: Vec<HostSummary>,
    pub projects: Vec<ProjectSummary>,
    pub appearance: AppearanceState,
    pub data: DataLayout,
    pub copy: ShellCopy,
    pub empty_actions: Vec<EmptyAction>,
}

#[derive(Debug)]
pub struct HostKernel {
    running: bool,
    window_visible: bool,
    host_display_name: String,
    data: DataLayout,
    language: Language,
    theme: Theme,
    last_light_theme: Theme,
    projects: Vec<ProjectSummary>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClientSettingsFile {
    language: Language,
    theme: Theme,
    last_light_theme: Theme,
}

impl HostKernel {
    pub fn boot(request: BootRequest) -> Result<Self, KernelError> {
        let data = DataLayout::prepare(&request.app_local_data_dir, &request.app_log_dir)?;
        if !data.host_settings_path.exists() {
            write_json(&data.host_settings_path, &serde_json::json!({}))?;
        }
        write_secrets(&data.host_secrets_path, &serde_json::json!({}))?;

        let (language, theme, last_light_theme) = load_or_init_appearance(
            &data.desktop_client_settings_path,
            &request.system_locale,
            request.system_appearance,
        )?;

        Ok(Self {
            running: true,
            window_visible: true,
            host_display_name: request.host_display_name,
            data,
            language,
            theme,
            last_light_theme,
            projects: Vec::new(),
        })
    }

    pub fn snapshot(&self) -> HostSnapshot {
        let empty_actions = if self.projects.is_empty() {
            vec![
                EmptyAction::RegisterFirstProject,
                EmptyAction::PairAnotherHost,
            ]
        } else {
            Vec::new()
        };
        HostSnapshot {
            running: self.running,
            window_visible: self.window_visible,
            focused_host_id: LOCAL_HOST_ID.to_string(),
            hosts: vec![HostSummary {
                id: LOCAL_HOST_ID.to_string(),
                display_name: self.host_display_name.clone(),
                local: true,
            }],
            projects: self.projects.clone(),
            appearance: AppearanceState {
                language: self.language,
                theme: self.theme,
                last_light_theme: self.last_light_theme,
                languages: vec![Language::ZhCn, Language::En],
                themes: vec![Theme::WarmPaper, Theme::PlainPaper, Theme::PlainNight],
                follow_system: false,
            },
            data: self.data.clone(),
            copy: ShellCopy::for_language(self.language),
            empty_actions,
        }
    }

    pub fn dispatch(&mut self, command: Command) -> Result<CommandOutcome, KernelError> {
        match command {
            Command::HideWindow => self.window_visible = false,
            Command::ShowWindow => {
                if self.running {
                    self.window_visible = true;
                }
            }
            Command::QuitHost => {
                self.running = false;
                self.window_visible = false;
            }
            Command::SetLanguage(language) => {
                self.persist_client_settings_values(language, self.theme, self.last_light_theme)?;
                self.language = language;
            }
            Command::SetTheme(theme) => {
                let last_light_theme = light_theme(if matches!(theme, Theme::PlainNight) {
                    self.last_light_theme
                } else {
                    theme
                });
                self.persist_client_settings_values(self.language, theme, last_light_theme)?;
                self.theme = theme;
                self.last_light_theme = last_light_theme;
            }
        }
        Ok(CommandOutcome {
            snapshot: self.snapshot(),
            process: if self.running {
                ProcessIntent::KeepRunning
            } else {
                ProcessIntent::Exit
            },
        })
    }

    pub fn handle(&mut self, request: serde_json::Value) -> Result<serde_json::Value, KernelError> {
        let op = request
            .get("op")
            .and_then(|value| value.as_str())
            .unwrap_or("snapshot");
        let outcome = match op {
            "snapshot" => CommandOutcome {
                snapshot: self.snapshot(),
                process: ProcessIntent::KeepRunning,
            },
            "hideWindow" => self.dispatch(Command::HideWindow)?,
            "showWindow" => self.dispatch(Command::ShowWindow)?,
            "quitHost" => self.dispatch(Command::QuitHost)?,
            "setLanguage" => {
                let language = serde_json::from_value(
                    request
                        .get("language")
                        .cloned()
                        .ok_or_else(|| KernelError::Protocol("missing language".into()))?,
                )?;
                self.dispatch(Command::SetLanguage(language))?
            }
            "setTheme" => {
                let theme = serde_json::from_value(
                    request
                        .get("theme")
                        .cloned()
                        .ok_or_else(|| KernelError::Protocol("missing theme".into()))?,
                )?;
                self.dispatch(Command::SetTheme(theme))?
            }
            other => {
                return Err(KernelError::Protocol(format!("unknown op {other}")));
            }
        };
        Ok(serde_json::json!({
            "snapshot": outcome.snapshot,
            "process": outcome.process,
        }))
    }

    fn persist_client_settings_values(
        &self,
        language: Language,
        theme: Theme,
        last_light_theme: Theme,
    ) -> Result<(), KernelError> {
        write_json(
            &self.data.desktop_client_settings_path,
            &ClientSettingsFile {
                language,
                theme,
                last_light_theme,
            },
        )
    }
}

impl DataLayout {
    fn prepare(app_local_data_dir: &Path, app_log_dir: &Path) -> Result<Self, KernelError> {
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
            log_dir: app_log_dir.to_path_buf(),
        })
    }
}

impl ShellCopy {
    fn for_language(language: Language) -> Self {
        match language {
            Language::ZhCn => Self {
                app_name: "Agent Taskboard".into(),
                register_first_project: "登记第一个 Project".into(),
                pair_another_host: "配对另一个 Host".into(),
                no_project_title: "这台 Host 上还没有 Project".into(),
                no_project_body: "先登记一个本地目录，并选好 Issue Tracker。".into(),
                quit_host: "退出 Host".into(),
                show_window: "打开窗口".into(),
                settings: "设置".into(),
                language: "界面语言".into(),
                theme: "主题".into(),
                language_zh: "简体中文".into(),
                language_en: "English".into(),
                theme_warm_paper: "暖纸".into(),
                theme_plain_paper: "素纸".into(),
                theme_plain_night: "素纸夜间".into(),
                hosts: "Host".into(),
                projects: "Project".into(),
                this_machine: "本机".into(),
                shade_light: "浅".into(),
                shade_dark: "深".into(),
                edit_menu: "编辑".into(),
            },
            Language::En => Self {
                app_name: "Agent Taskboard".into(),
                register_first_project: "Register the first Project".into(),
                pair_another_host: "Pair another Host".into(),
                no_project_title: "No Project on this Host yet".into(),
                no_project_body: "Register a local folder and pick an Issue Tracker.".into(),
                quit_host: "Quit Host".into(),
                show_window: "Open window".into(),
                settings: "Settings".into(),
                language: "Interface language".into(),
                theme: "Theme".into(),
                language_zh: "简体中文".into(),
                language_en: "English".into(),
                theme_warm_paper: "Warm paper".into(),
                theme_plain_paper: "Plain paper".into(),
                theme_plain_night: "Plain night".into(),
                hosts: "Host".into(),
                projects: "Project".into(),
                this_machine: "This machine".into(),
                shade_light: "Light".into(),
                shade_dark: "Dark".into(),
                edit_menu: "Edit".into(),
            },
        }
    }
}

fn load_or_init_appearance(
    path: &Path,
    system_locale: &str,
    system_appearance: SystemAppearance,
) -> Result<(Language, Theme, Theme), KernelError> {
    if path.exists() {
        let raw = fs::read_to_string(path)?;
        if let Ok(file) = serde_json::from_str::<ClientSettingsFile>(&raw) {
            return Ok((
                file.language,
                file.theme,
                light_theme(file.last_light_theme),
            ));
        }
    }
    let language = match_language(system_locale);
    let theme = match system_appearance {
        SystemAppearance::Light => Theme::WarmPaper,
        SystemAppearance::Dark => Theme::PlainNight,
    };
    let last_light_theme = Theme::WarmPaper;
    write_json(
        path,
        &ClientSettingsFile {
            language,
            theme,
            last_light_theme,
        },
    )?;
    Ok((language, theme, last_light_theme))
}

fn match_language(locale: &str) -> Language {
    let normalized = locale.to_ascii_lowercase().replace('_', "-");
    if normalized == "zh" || normalized.starts_with("zh-") {
        Language::ZhCn
    } else {
        Language::En
    }
}

fn light_theme(theme: Theme) -> Theme {
    match theme {
        Theme::PlainNight => Theme::WarmPaper,
        other => other,
    }
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), KernelError> {
    write_json_inner(path, value, false)
}

fn write_json_inner<T: Serialize>(
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
        restrict_to_owner(&tmp)?;
    }
    fs::rename(&tmp, path)?;
    if owner_only {
        restrict_to_owner(path)?;
    }
    Ok(())
}

fn write_secrets(path: &Path, value: &serde_json::Value) -> Result<(), KernelError> {
    if !path.exists() {
        write_json_inner(path, value, true)?;
    }
    restrict_to_owner(path)?;
    Ok(())
}

fn restrict_to_owner(path: &Path) -> Result<(), KernelError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(path, perms)?;
    }
    let _ = path;
    Ok(())
}

#[derive(Debug)]
pub enum KernelError {
    Io(io::Error),
    Json(serde_json::Error),
    Protocol(String),
}

impl std::fmt::Display for KernelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KernelError::Io(err) => write!(f, "{err}"),
            KernelError::Json(err) => write!(f, "{err}"),
            KernelError::Protocol(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for KernelError {}

impl From<io::Error> for KernelError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for KernelError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}
