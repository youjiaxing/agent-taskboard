//! Host 内核：桌面窗口、浏览器和以后的远程 Client 都走这一条接缝。

mod local_rpc;
mod owner;
mod pairing;

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub use local_rpc::{
    bind_local_rpc, local_client_origin_allowed, spawn_local_rpc, LoopbackAssets, LoopbackServer,
    LOCAL_RPC_PORT,
};
pub use pairing::{IssuedPairing, PairedClient, PairingOffer};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    HideWindow,
    ShowWindow,
    QuitHost,
    SetLanguage(Language),
    SetTheme(Theme),
    BeginPairingOffer { address: String },
    RedeemPairing { code: String, client_name: String },
    RevokeClient { client_id: String },
    PairRemoteHost { address: String, code: String },
    FocusHost { host_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProcessIntent {
    KeepRunning,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EmptyAction {
    RegisterFirstProject,
    PairAnotherHost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoopbackKind {
    Serving,
    Occupied,
    HostNotRunning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum LoopbackPage {
    Serving { url: String },
    Occupied { url: String, reason: String },
    HostNotRunning { url: String, reason: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandOutcome {
    pub snapshot: HostSnapshot,
    pub process: ProcessIntent,
    pub pairing: Option<IssuedPairing>,
}

impl CommandOutcome {
    pub fn to_json(&self) -> serde_json::Value {
        let mut value = serde_json::json!({
            "snapshot": self.snapshot,
            "process": self.process,
        });
        if let Some(pairing) = &self.pairing {
            value["pairing"] = serde_json::to_value(pairing).expect("pairing json");
        }
        value
    }
}

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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostSummary {
    pub id: String,
    pub display_name: String,
    pub local: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppearanceSelection {
    language: Language,
    theme: Theme,
    last_light_theme: Theme,
}

impl AppearanceSelection {
    fn with_language(self, language: Language) -> Self {
        Self { language, ..self }
    }

    fn with_theme(self, theme: Theme) -> Self {
        Self {
            theme,
            last_light_theme: if matches!(theme, Theme::PlainNight) {
                self.last_light_theme
            } else {
                daytime_theme(theme)
            },
            ..self
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceState {
    pub language: Language,
    pub theme: Theme,
    pub last_light_theme: Theme,
    pub languages: Vec<Language>,
    pub themes: Vec<Theme>,
}

impl AppearanceState {
    fn from_selection(selection: AppearanceSelection) -> Self {
        Self {
            language: selection.language,
            theme: selection.theme,
            last_light_theme: selection.last_light_theme,
            languages: vec![Language::ZhCn, Language::En],
            themes: vec![Theme::WarmPaper, Theme::PlainPaper, Theme::PlainNight],
        }
    }
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
    pub pairing_required: String,
    pub pairing_title: String,
    pub pairing_this_host: String,
    pub pairing_to_another: String,
    pub pairing_address: String,
    pub pairing_show: String,
    pub pairing_copy: String,
    pub pairing_same_payload: String,
    pub pairing_paste: String,
    pub pairing_connect: String,
    pub paired_clients: String,
    pub revoke_client: String,
    pub no_paired_clients: String,
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
    pub loopback_page: LoopbackPage,
    pub pairing_offer: Option<PairingOffer>,
    pub paired_clients: Vec<PairedClient>,
}

#[derive(Debug)]
pub struct HostKernel {
    running: bool,
    window_visible: bool,
    host_display_name: String,
    data: DataLayout,
    appearance: AppearanceSelection,
    projects: Vec<ProjectSummary>,
    loopback_kind: LoopbackKind,
    loopback_port: u16,
    pairing_offer: Option<pairing::ActiveOffer>,
    host_id: String,
    paired_clients: Vec<pairing::IssuedClient>,
    focused_host_id: String,
    remote_hosts: Vec<pairing::RemoteHost>,
    remote_view: Option<RemoteView>,
}

#[derive(Debug, Clone)]
struct RemoteView {
    host_id: String,
    projects: Vec<ProjectSummary>,
    empty_actions: Vec<EmptyAction>,
}

impl HostKernel {
    pub fn boot(request: BootRequest) -> Result<Self, KernelError> {
        let data = DataLayout::prepare(&request.app_local_data_dir, &request.app_log_dir)?;
        let host_id = load_or_init_host_id(&data.host_settings_path)?;
        let paired_clients = load_paired_clients(&data.host_secrets_path)?;

        let (appearance, focused_host_id, saved_remotes) = load_or_init_appearance(
            &data.desktop_client_settings_path,
            &request.system_locale,
            request.system_appearance,
        )?;
        let tokens = load_client_tokens(&data.desktop_client_secrets_path)?;
        let remote_hosts = saved_remotes
            .into_iter()
            .filter_map(|saved| {
                tokens.get(&saved.id).map(|token| pairing::RemoteHost {
                    id: saved.id,
                    display_name: saved.display_name,
                    address: saved.address,
                    token: token.clone(),
                })
            })
            .collect::<Vec<_>>();
        let focused_host_id = if focused_host_id == LOCAL_HOST_ID
            || remote_hosts.iter().any(|host| host.id == focused_host_id)
        {
            focused_host_id
        } else {
            LOCAL_HOST_ID.to_string()
        };

        Ok(Self {
            running: true,
            window_visible: true,
            host_display_name: request.host_display_name,
            data,
            appearance,
            projects: Vec::new(),
            loopback_kind: LoopbackKind::HostNotRunning,
            loopback_port: LOCAL_RPC_PORT,
            pairing_offer: None,
            host_id,
            paired_clients,
            focused_host_id,
            remote_hosts,
            remote_view: None,
        })
    }

    pub fn snapshot(&self) -> HostSnapshot {
        let (projects, empty_actions) = self.board_for_focus();
        HostSnapshot {
            running: self.running,
            window_visible: self.window_visible,
            focused_host_id: self.focused_host_id.clone(),
            hosts: self.connected_hosts(),
            projects,
            appearance: AppearanceState::from_selection(self.appearance),
            data: self.data.clone(),
            copy: ShellCopy::for_language(self.appearance.language),
            empty_actions,
            loopback_page: self.loopback_page(),
            pairing_offer: self
                .pairing_offer
                .as_ref()
                .map(pairing::ActiveOffer::to_offer),
            paired_clients: self
                .paired_clients
                .iter()
                .map(pairing::IssuedClient::summary)
                .collect(),
        }
    }

    fn outcome(&self) -> CommandOutcome {
        CommandOutcome {
            snapshot: self.snapshot(),
            process: if self.running {
                ProcessIntent::KeepRunning
            } else {
                ProcessIntent::Exit
            },
            pairing: None,
        }
    }

    pub(crate) fn note_loopback_page(&mut self, kind: LoopbackKind, port: u16) {
        self.loopback_kind = kind;
        self.loopback_port = if port == 0 { LOCAL_RPC_PORT } else { port };
    }

    fn loopback_page(&self) -> LoopbackPage {
        let url = format!("http://127.0.0.1:{}/", self.loopback_port);
        match self.loopback_kind {
            LoopbackKind::Serving => LoopbackPage::Serving { url },
            LoopbackKind::Occupied => LoopbackPage::Occupied {
                url,
                reason: occupied_reason(self.appearance.language, self.loopback_port),
            },
            LoopbackKind::HostNotRunning => LoopbackPage::HostNotRunning {
                url,
                reason: host_not_running_reason(self.appearance.language),
            },
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
                self.loopback_kind = LoopbackKind::HostNotRunning;
            }
            Command::SetLanguage(language) => {
                let appearance = self.appearance.with_language(language);
                self.persist_client_settings(&appearance)?;
                self.appearance = appearance;
            }
            Command::SetTheme(theme) => {
                let appearance = self.appearance.with_theme(theme);
                self.persist_client_settings(&appearance)?;
                self.appearance = appearance;
            }
            Command::BeginPairingOffer { address } => {
                let address = pairing::parse_http_url(&address).map_err(KernelError::Protocol)?;
                self.pairing_offer = Some(pairing::ActiveOffer::new(address));
            }
            Command::RedeemPairing { code, client_name } => {
                let pairing = self.redeem_pairing(&code, &client_name)?;
                return Ok(CommandOutcome {
                    snapshot: self.snapshot(),
                    process: ProcessIntent::KeepRunning,
                    pairing: Some(pairing),
                });
            }
            Command::RevokeClient { client_id } => {
                self.paired_clients.retain(|client| client.id != client_id);
                self.persist_host_secrets()?;
            }
            Command::PairRemoteHost { address, code } => {
                self.pair_remote_host(&address, &code)?;
            }
            Command::FocusHost { host_id } => {
                self.focus_host(&host_id)?;
            }
        }
        Ok(self.outcome())
    }

    pub fn handle(&mut self, request: serde_json::Value) -> Result<CommandOutcome, KernelError> {
        let op = request
            .get("op")
            .and_then(|value| value.as_str())
            .unwrap_or("snapshot");
        match op {
            "snapshot" => Ok(self.outcome()),
            "hideWindow" => self.dispatch(Command::HideWindow),
            "showWindow" => self.dispatch(Command::ShowWindow),
            "quitHost" => self.dispatch(Command::QuitHost),
            "setLanguage" => {
                let language = serde_json::from_value(
                    request
                        .get("language")
                        .cloned()
                        .ok_or_else(|| KernelError::Protocol("missing language".into()))?,
                )?;
                self.dispatch(Command::SetLanguage(language))
            }
            "setTheme" => {
                let theme = serde_json::from_value(
                    request
                        .get("theme")
                        .cloned()
                        .ok_or_else(|| KernelError::Protocol("missing theme".into()))?,
                )?;
                self.dispatch(Command::SetTheme(theme))
            }
            "beginPairingOffer" => {
                let address = request
                    .get("address")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| KernelError::Protocol("missing address".into()))?
                    .to_string();
                self.dispatch(Command::BeginPairingOffer { address })
            }
            "redeemPairing" => {
                let code = request
                    .get("code")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| KernelError::Protocol("missing code".into()))?
                    .to_string();
                let client_name = request
                    .get("clientName")
                    .and_then(|value| value.as_str())
                    .unwrap_or("Client")
                    .to_string();
                self.dispatch(Command::RedeemPairing { code, client_name })
            }
            "revokeClient" => {
                let client_id = request
                    .get("clientId")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| KernelError::Protocol("missing clientId".into()))?
                    .to_string();
                self.dispatch(Command::RevokeClient { client_id })
            }
            "pairRemoteHost" => {
                let address = request
                    .get("address")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| KernelError::Protocol("missing address".into()))?
                    .to_string();
                let code = request
                    .get("code")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| KernelError::Protocol("missing code".into()))?
                    .to_string();
                self.dispatch(Command::PairRemoteHost { address, code })
            }
            "focusHost" => {
                let host_id = request
                    .get("hostId")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| KernelError::Protocol("missing hostId".into()))?
                    .to_string();
                self.dispatch(Command::FocusHost { host_id })
            }
            other => Err(KernelError::Protocol(format!("unknown op {other}"))),
        }
    }

    pub fn pairing_token_valid(&self, token: &str) -> bool {
        let token = token.trim();
        !token.is_empty()
            && self
                .paired_clients
                .iter()
                .any(|client| client.token == token)
    }

    fn persist_client_settings(&self, appearance: &AppearanceSelection) -> Result<(), KernelError> {
        let file = ClientSettingsFile {
            language: appearance.language,
            theme: appearance.theme,
            last_light_theme: appearance.last_light_theme,
            focused_host_id: self.focused_host_id.clone(),
            remote_hosts: self
                .remote_hosts
                .iter()
                .map(pairing::RemoteHost::to_saved)
                .collect(),
        };
        write_json(&self.data.desktop_client_settings_path, &file)?;
        let secrets = ClientSecretsFile {
            tokens: self
                .remote_hosts
                .iter()
                .map(|host| (host.id.clone(), host.token.clone()))
                .collect(),
        };
        write_json_inner(&self.data.desktop_client_secrets_path, &secrets, true)
    }

    fn connected_hosts(&self) -> Vec<HostSummary> {
        let mut hosts = vec![HostSummary {
            id: LOCAL_HOST_ID.to_string(),
            display_name: self.host_display_name.clone(),
            local: true,
        }];
        hosts.extend(self.remote_hosts.iter().map(|host| HostSummary {
            id: host.id.clone(),
            display_name: host.display_name.clone(),
            local: false,
        }));
        hosts
    }

    fn board_for_focus(&self) -> (Vec<ProjectSummary>, Vec<EmptyAction>) {
        if self.focused_host_id != LOCAL_HOST_ID {
            if let Some(view) = &self.remote_view {
                if view.host_id == self.focused_host_id {
                    return (view.projects.clone(), view.empty_actions.clone());
                }
            }
        }
        let empty_actions = if self.projects.is_empty() {
            vec![
                EmptyAction::RegisterFirstProject,
                EmptyAction::PairAnotherHost,
            ]
        } else {
            Vec::new()
        };
        (self.projects.clone(), empty_actions)
    }

    fn refresh_remote_view(&mut self, host_id: &str) -> Result<(), KernelError> {
        if host_id == LOCAL_HOST_ID {
            self.remote_view = None;
            return Ok(());
        }
        let remote = self
            .remote_hosts
            .iter()
            .find(|host| host.id == host_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown host".into()))?;
        let response = pairing::post_rpc(
            &remote.address,
            Some(&remote.token),
            &serde_json::json!({ "op": "snapshot" }),
        )
        .map_err(|err| match err {
            KernelError::Io(_) | KernelError::Denied(_) => {
                KernelError::Protocol("address is not reachable".into())
            }
            other => other,
        })?;
        let snapshot = response
            .get("snapshot")
            .cloned()
            .ok_or_else(|| KernelError::Protocol("remote Host returned no snapshot".into()))?;
        let projects = snapshot
            .get("projects")
            .cloned()
            .map(serde_json::from_value)
            .transpose()?
            .unwrap_or_default();
        let empty_actions = snapshot
            .get("emptyActions")
            .cloned()
            .map(serde_json::from_value)
            .transpose()?
            .unwrap_or_default();
        self.remote_view = Some(RemoteView {
            host_id: host_id.to_string(),
            projects,
            empty_actions,
        });
        Ok(())
    }

    fn pair_remote_host(&mut self, address: &str, code: &str) -> Result<(), KernelError> {
        let address = pairing::parse_http_url(address).map_err(KernelError::Protocol)?;
        if self.is_own_loopback(&address) {
            return Err(KernelError::Protocol(
                "cannot pair this window to its own Host".into(),
            ));
        }
        let body = serde_json::json!({
            "op": "redeemPairing",
            "code": code,
            "clientName": self.host_display_name,
        });
        let response = pairing::post_rpc(&address, None, &body).map_err(|err| match err {
            KernelError::Io(_) => KernelError::Protocol("address is not reachable".into()),
            other => other,
        })?;
        let pairing = response
            .get("pairing")
            .cloned()
            .ok_or_else(|| KernelError::Denied("invalid pairing code".into()))?;
        let issued: IssuedPairing = serde_json::from_value(pairing)?;
        if issued.token.is_empty() || issued.host_id.is_empty() {
            return Err(KernelError::Denied("invalid pairing code".into()));
        }
        if issued.host_id == LOCAL_HOST_ID {
            return Err(KernelError::Protocol("remote Host id is invalid".into()));
        }
        self.remote_hosts.retain(|host| host.id != issued.host_id);
        self.remote_hosts.push(pairing::RemoteHost {
            id: issued.host_id,
            display_name: issued.display_name,
            address,
            token: issued.token,
        });
        self.persist_client_settings(&self.appearance.clone())
    }

    fn is_own_loopback(&self, address: &str) -> bool {
        if self.loopback_kind != LoopbackKind::Serving {
            return false;
        }
        let rest = address
            .split_once("://")
            .map(|(_, rest)| rest)
            .unwrap_or(address);
        let hostport = rest.split('/').next().unwrap_or(rest);
        let (host, port) = match hostport.rsplit_once(':') {
            Some((host, port)) if port.bytes().all(|byte| byte.is_ascii_digit()) => {
                (host, port.parse::<u16>().ok())
            }
            _ => (hostport, Some(LOCAL_RPC_PORT)),
        };
        let host = host.trim_start_matches('[').trim_end_matches(']');
        matches!(host, "127.0.0.1" | "localhost" | "::1") && port == Some(self.loopback_port)
    }

    fn focus_host(&mut self, host_id: &str) -> Result<(), KernelError> {
        if host_id != LOCAL_HOST_ID && !self.remote_hosts.iter().any(|host| host.id == host_id) {
            return Err(KernelError::Protocol("unknown host".into()));
        }
        self.focused_host_id = host_id.to_string();
        self.refresh_remote_view(host_id)?;
        self.persist_client_settings(&self.appearance.clone())
    }

    fn redeem_pairing(
        &mut self,
        code: &str,
        client_name: &str,
    ) -> Result<IssuedPairing, KernelError> {
        let Some(offer) = &self.pairing_offer else {
            return Err(KernelError::Denied("invalid pairing code".into()));
        };
        if !pairing::codes_match(&offer.code, code) {
            return Err(KernelError::Denied("invalid pairing code".into()));
        }
        let client_name = client_name.trim();
        if client_name.is_empty() {
            return Err(KernelError::Protocol("missing clientName".into()));
        }
        let client = pairing::IssuedClient {
            id: pairing::random_id(),
            name: client_name.to_string(),
            token: pairing::generate_token(),
        };
        let issued = IssuedPairing {
            token: client.token.clone(),
            host_id: self.host_id.clone(),
            display_name: self.host_display_name.clone(),
        };
        self.pairing_offer = None;
        self.paired_clients.push(client);
        self.persist_host_secrets()?;
        Ok(issued)
    }

    fn persist_host_secrets(&self) -> Result<(), KernelError> {
        let file = HostSecretsFile {
            clients: self.paired_clients.clone(),
        };
        write_json_inner(&self.data.host_secrets_path, &file, true)
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
            desktop_client_secrets_path: desktop_client_dir.join("secrets.json"),
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
                pairing_required: "经 Tailscale、局域网或其它站点访问需要长期令牌。本机回环页 http://127.0.0.1:10529/ 免配对。".into(),
                pairing_title: "配对".into(),
                pairing_this_host: "让别人连这台".into(),
                pairing_to_another: "连到另一台 Host".into(),
                pairing_address: "可到达地址".into(),
                pairing_show: "出示配对码".into(),
                pairing_copy: "复制".into(),
                pairing_same_payload: "二维码和复制文本是同一份信息。连通走你自己的 Tailscale、局域网或 VPN，没有产品中继。".into(),
                pairing_paste: "粘贴配对信息".into(),
                pairing_connect: "连接".into(),
                paired_clients: "已配对的 Client".into(),
                revoke_client: "撤销".into(),
                no_paired_clients: "还没有已配对的 Client。".into(),
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
                pairing_required: "Access via Tailscale, LAN, or another site needs a long-term token. The loopback page at http://127.0.0.1:10529/ does not require pairing.".into(),
                pairing_title: "Pairing".into(),
                pairing_this_host: "Let others join this Host".into(),
                pairing_to_another: "Connect to another Host".into(),
                pairing_address: "Reachable address".into(),
                pairing_show: "Show pairing code".into(),
                pairing_copy: "Copy".into(),
                pairing_same_payload: "The QR code and the copyable text are the same payload. Reach this Host on your own Tailscale, LAN, or VPN — there is no product relay.".into(),
                pairing_paste: "Paste pairing payload".into(),
                pairing_connect: "Connect".into(),
                paired_clients: "Paired clients".into(),
                revoke_client: "Revoke".into(),
                no_paired_clients: "No paired clients yet.".into(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct HostSettingsFile {
    #[serde(default)]
    id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct HostSecretsFile {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    clients: Vec<pairing::IssuedClient>,
}

fn load_or_init_host_id(path: &Path) -> Result<String, KernelError> {
    let mut file = if path.exists() {
        serde_json::from_str(&fs::read_to_string(path)?)?
    } else {
        HostSettingsFile::default()
    };
    if file.id.trim().is_empty() {
        file.id = pairing::random_id();
        write_json(path, &file)?;
    }
    Ok(file.id)
}

fn load_paired_clients(path: &Path) -> Result<Vec<pairing::IssuedClient>, KernelError> {
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
struct ClientSettingsFile {
    language: Language,
    theme: Theme,
    last_light_theme: Theme,
    #[serde(default = "local_host_id")]
    focused_host_id: String,
    #[serde(default)]
    remote_hosts: Vec<pairing::SavedRemoteHost>,
}

fn local_host_id() -> String {
    LOCAL_HOST_ID.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ClientSecretsFile {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    tokens: BTreeMap<String, String>,
}

fn load_or_init_appearance(
    path: &Path,
    system_locale: &str,
    system_appearance: SystemAppearance,
) -> Result<(AppearanceSelection, String, Vec<pairing::SavedRemoteHost>), KernelError> {
    if path.exists() {
        let raw = fs::read_to_string(path)?;
        if let Ok(mut file) = serde_json::from_str::<ClientSettingsFile>(&raw) {
            file.last_light_theme = daytime_theme(file.last_light_theme);
            return Ok((
                AppearanceSelection {
                    language: file.language,
                    theme: file.theme,
                    last_light_theme: file.last_light_theme,
                },
                file.focused_host_id,
                file.remote_hosts,
            ));
        }
        if let Ok(mut file) = serde_json::from_str::<AppearanceSelection>(&raw) {
            file.last_light_theme = daytime_theme(file.last_light_theme);
            return Ok((file, LOCAL_HOST_ID.to_string(), Vec::new()));
        }
    }
    let language = match_language(system_locale);
    let theme = match system_appearance {
        SystemAppearance::Light => Theme::WarmPaper,
        SystemAppearance::Dark => Theme::PlainNight,
    };
    let appearance = AppearanceSelection {
        language,
        theme,
        last_light_theme: Theme::WarmPaper,
    };
    let file = ClientSettingsFile {
        language,
        theme,
        last_light_theme: Theme::WarmPaper,
        focused_host_id: LOCAL_HOST_ID.to_string(),
        remote_hosts: Vec::new(),
    };
    write_json(path, &file)?;
    Ok((appearance, LOCAL_HOST_ID.to_string(), Vec::new()))
}

fn load_client_tokens(path: &Path) -> Result<BTreeMap<String, String>, KernelError> {
    if !path.exists() {
        write_json_inner(path, &ClientSecretsFile::default(), true)?;
        return Ok(BTreeMap::new());
    }
    owner::restrict_to_owner(path)?;
    let raw = fs::read_to_string(path)?;
    let file = serde_json::from_str::<ClientSecretsFile>(&raw).unwrap_or_default();
    Ok(file.tokens)
}

fn occupied_reason(language: Language, port: u16) -> String {
    match language {
        Language::ZhCn => {
            format!("本机网页入口没起来：端口 {port} 已被占用。桌面窗口可以继续用。")
        }
        Language::En => format!(
            "The local web entry could not start: port {port} is already in use. The desktop window still works."
        ),
    }
}

fn host_not_running_reason(language: Language) -> String {
    match language {
        Language::ZhCn => "本机没有在跑 Host，所以没有这份回环页。".into(),
        Language::En => {
            "The local Host is not running, so this loopback page is not available.".into()
        }
    }
}

fn match_language(locale: &str) -> Language {
    let normalized = locale.to_ascii_lowercase().replace('_', "-");
    if normalized == "zh" || normalized.starts_with("zh-") {
        Language::ZhCn
    } else {
        Language::En
    }
}

fn daytime_theme(theme: Theme) -> Theme {
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
        owner::restrict_to_owner(&tmp)?;
    }
    owner::replace_file(&tmp, path)?;
    if owner_only {
        owner::restrict_to_owner(path)?;
    }
    Ok(())
}

#[derive(Debug)]
pub enum KernelError {
    Io(io::Error),
    Json(serde_json::Error),
    Protocol(String),
    Denied(String),
}

impl std::fmt::Display for KernelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KernelError::Io(err) => write!(f, "{err}"),
            KernelError::Json(err) => write!(f, "{err}"),
            KernelError::Protocol(err) => write!(f, "{err}"),
            KernelError::Denied(err) => write!(f, "{err}"),
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
