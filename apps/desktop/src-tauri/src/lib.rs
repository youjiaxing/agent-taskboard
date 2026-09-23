mod edit_menu;
mod signal;
mod startup;

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use edit_menu::{
    edit_menu_enabled, EditMenuContext, EditMenuEnabled, EDIT_COPY, EDIT_CUT, EDIT_PASTE,
    EDIT_REDO, EDIT_SELECT_ALL, EDIT_UNDO,
};
use host_kernel::{
    BootRequest, Command, HostKernel, HostMode, HostSnapshot, LoopbackAssets, LoopbackServer,
    ProcessIntent, SystemAppearance, LOCAL_RPC_PORT,
};
use tauri::menu::{
    AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu, HELP_SUBMENU_ID, WINDOW_SUBMENU_ID,
};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_log::RotationStrategy;
use tauri_plugin_opener::OpenerExt;

const LOG_FILE_SIZE_BYTES: u128 = 5 * 1024 * 1024;
const LOG_FILE_COUNT: usize = 5;
const USAGE_GUIDE_URL: &str = "https://github.com/youjiaxing/agent-taskboard";
const RUN_WINDOW_WIDTH: f64 = 1180.0;
const RUN_WINDOW_HEIGHT: f64 = 760.0;
const RUN_WINDOW_MIN_WIDTH: f64 = 900.0;
const RUN_WINDOW_MIN_HEIGHT: f64 = 640.0;
const ACCEPTANCE_MANIFEST_SHA256: Option<&str> =
    option_env!("AGENT_TASKBOARD_ACCEPTANCE_MANIFEST_SHA256");

/// Stable webview event carrying the resolved system appearance (`light` / `dark`).
const SYSTEM_APPEARANCE_CHANGED: &str = "system-appearance-changed";

struct AppState {
    kernel: Arc<Mutex<HostKernel>>,
    protocol_url: String,
    startup_settings_path: PathBuf,
    menu_signature: Mutex<Option<ShellMenuSignature>>,
    edit_context: Mutex<EditMenuContext>,
    edit_items: Mutex<Option<EditMenuItems>>,
    _loopback: LoopbackServer,
}

struct EditMenuItems {
    undo: MenuItem<tauri::Wry>,
    redo: MenuItem<tauri::Wry>,
    cut: MenuItem<tauri::Wry>,
    copy: MenuItem<tauri::Wry>,
    paste: MenuItem<tauri::Wry>,
    select_all: MenuItem<tauri::Wry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ShellMenuSignature {
    app_name: String,
    show_window: String,
    quit_host: String,
    settings: String,
    edit_menu: String,
    edit_undo: String,
    edit_redo: String,
    edit_cut: String,
    edit_copy: String,
    edit_paste: String,
    edit_select_all: String,
    window_menu: String,
    help_menu: String,
    keyboard_help: String,
    usage_guide: String,
}

impl ShellMenuSignature {
    fn from_snapshot(snapshot: &HostSnapshot) -> Self {
        let copy = &snapshot.copy;
        Self {
            app_name: copy.app_name.clone(),
            show_window: copy.show_window.clone(),
            quit_host: copy.quit_host.clone(),
            settings: copy.settings.clone(),
            edit_menu: copy.edit_menu.clone(),
            edit_undo: copy.edit_undo.clone(),
            edit_redo: copy.edit_redo.clone(),
            edit_cut: copy.edit_cut.clone(),
            edit_copy: copy.edit_copy.clone(),
            edit_paste: copy.edit_paste.clone(),
            edit_select_all: copy.edit_select_all.clone(),
            window_menu: copy.window_menu.clone(),
            help_menu: copy.help_menu.clone(),
            keyboard_help: copy.keyboard_help.clone(),
            usage_guide: copy.usage_guide.clone(),
        }
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .max_file_size(LOG_FILE_SIZE_BYTES)
                .rotation_strategy(RotationStrategy::KeepSome(LOG_FILE_COUNT))
                .build(),
        )
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name("com.youjiaxing.agent-taskboard")
                .args(["--autostart"])
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            set_host_mode,
            set_edit_menu_context,
            open_run_window
        ])
        .setup(|app| {
            signal::spawn_ctrl_c_handler(app.handle().clone());
            let startup_settings_path = startup_settings_path(app.handle())?;
            let host_mode = startup::requested_host_mode(&startup_settings_path, std::env::args());
            let kernel = boot_kernel(app.handle(), host_mode)?;
            let kernel = Arc::new(Mutex::new(kernel));
            let app_handle = app.handle().clone();
            let on_outcome = move |outcome: host_kernel::CommandOutcome| {
                let app_handle = app_handle.clone();
                let ui = app_handle.clone();
                let _ = app_handle.run_on_main_thread(move || {
                    let _ = refresh_shell(&ui, &outcome.snapshot);
                    if let Some(window) = ui.get_webview_window("main") {
                        if outcome.snapshot.window_visible {
                            if !window.is_visible().unwrap_or(true) {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        } else if window.is_visible().unwrap_or(false) {
                            // Keep every HideWindow command (including the
                            // local RPC path used by headless Windows shells)
                            // consistent with the native close request.
                            let _ = window.hide();
                        }
                    }
                    if outcome.process == ProcessIntent::Exit {
                        ui.exit(0);
                    }
                });
            };
            let loopback = if host_mode == HostMode::ClientOnly {
                LoopbackServer::attach_client_transport(Arc::clone(&kernel), on_outcome)?
            } else {
                LoopbackServer::attach_with(
                    Arc::clone(&kernel),
                    LOCAL_RPC_PORT,
                    loopback_assets(app.handle())?,
                    on_outcome,
                )?
            };
            let protocol_url = loopback.protocol_url().to_string();
            let snapshot = kernel.lock().map_err(|err| err.to_string())?.snapshot();
            app.manage(AppState {
                kernel,
                protocol_url: protocol_url.clone(),
                startup_settings_path,
                menu_signature: Mutex::new(None),
                edit_context: Mutex::new(EditMenuContext::default()),
                edit_items: Mutex::new(None),
                _loopback: loopback,
            });
            build_tray(app.handle())?;
            refresh_shell(app.handle(), &snapshot)?;
            app.on_menu_event(|app, event| handle_shell_menu(app, event.id().as_ref()));
            if let Some(window) = app.get_webview_window("main") {
                inject_protocol_url(&window, &protocol_url);
            }
            Ok(())
        })
        .on_page_load(|webview, payload| {
            if payload.event() != PageLoadEvent::Finished {
                return;
            }
            if let Some(state) = webview.try_state::<AppState>() {
                let encoded = serde_json::to_string(&state.protocol_url).unwrap();
                let _ = webview.eval(format!("window.__HOST_PROTOCOL__ = {encoded};"));
            }
        })
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => match close_behavior(window.label()) {
                CloseBehavior::HideMain => {
                    api.prevent_close();
                    let _ = window.hide();
                    if let Some(state) = window.try_state::<AppState>() {
                        if let Ok(mut kernel) = state.kernel.lock() {
                            let _ = kernel.dispatch(Command::HideWindow);
                        }
                    }
                    if let Some(webview) = window.app_handle().get_webview_window(window.label()) {
                        let _ = webview.eval(
                            "window.dispatchEvent(new Event('agent-taskboard:host-window-hidden'));",
                        );
                    }
                }
                // A Run window closes for real. It must stay out of the Host:
                // no HideWindow command, no hiding `main`, no stopping the Run.
                CloseBehavior::DestroyRunWindow | CloseBehavior::Native => {}
            },
            WindowEvent::ThemeChanged(theme) => {
                let appearance = match theme {
                    tauri::Theme::Dark => "dark",
                    _ => "light",
                };
                let _ = window.emit(SYSTEM_APPEARANCE_CHANGED, appearance);
            }
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("error while building Agent Taskboard")
        .run(|app, event| match event {
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => {
                show_main(app);
            }
            _ => {}
        });
}

fn loopback_assets(app: &AppHandle) -> Result<LoopbackAssets, String> {
    if cfg!(debug_assertions) {
        return Ok(LoopbackAssets::DevProxy {
            origin: "http://127.0.0.1:1420".into(),
        });
    }
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|error| error.to_string())?;
    packaged_loopback_assets(packaged_client_candidates(resource_dir))
}

fn packaged_client_candidates(resource_dir: PathBuf) -> [PathBuf; 3] {
    [
        resource_dir.join("_up_").join("dist"),
        resource_dir.join("dist"),
        resource_dir,
    ]
}

fn packaged_loopback_assets(
    candidates: impl IntoIterator<Item = PathBuf>,
) -> Result<LoopbackAssets, String> {
    let candidates = candidates.into_iter().collect::<Vec<_>>();
    if let Some(dir) = candidates
        .iter()
        .find(|dir| dir.join("index.html").is_file())
    {
        return Ok(LoopbackAssets::Directory(dir.clone()));
    }
    let searched = candidates
        .iter()
        .map(|dir| dir.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!(
        "packaged Client assets are missing; searched: {searched}"
    ))
}

fn inject_protocol_url(window: &tauri::WebviewWindow, url: &str) {
    let encoded = serde_json::to_string(url).unwrap();
    let _ = window.eval(format!("window.__HOST_PROTOCOL__ = {encoded};"));
}

fn startup_settings_path(app: &AppHandle) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = app
        .path()
        .app_local_data_dir()?
        .join("desktop-client/startup.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(path)
}

#[tauri::command]
fn set_host_mode(mode: &str, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let client_only = match mode {
        "client-only" => true,
        "host-and-client" => false,
        _ => return Err("unknown Host mode".into()),
    };
    if client_only
        && !state
            .kernel
            .lock()
            .map_err(|err| err.to_string())?
            .begin_client_only_switch()
            .allowed
    {
        return Err("active Runs must finish or stop before enabling Client only".into());
    }
    startup::write_host_mode(
        &state.startup_settings_path,
        if client_only {
            HostMode::ClientOnly
        } else {
            HostMode::HostAndClient
        },
    )
}

#[tauri::command]
fn set_edit_menu_context(context: EditMenuContext, app: AppHandle) -> Result<(), String> {
    if let Some(state) = app.try_state::<AppState>() {
        let mut cached = state.edit_context.lock().map_err(|err| err.to_string())?;
        if *cached == context {
            return Ok(());
        }
        *cached = context;
    }
    apply_edit_menu_state(&app)
}

const RUN_WINDOW_LABEL_PREFIX: &str = "run-";
const RUN_WINDOW_LABEL_SLUG_MAX: usize = 24;
const RUN_WINDOW_PATH: &str = "index.html";
const RUN_WINDOW_RUN_ID_PARAM: &str = "runId";
const RUN_WINDOW_HOST_ID_PARAM: &str = "hostId";
const RUN_WINDOW_PROJECT_ID_PARAM: &str = "projectId";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CloseBehavior {
    HideMain,
    DestroyRunWindow,
    Native,
}

fn close_behavior(label: &str) -> CloseBehavior {
    if label == "main" {
        CloseBehavior::HideMain
    } else if is_run_window_label(label) {
        CloseBehavior::DestroyRunWindow
    } else {
        CloseBehavior::Native
    }
}

fn is_run_window_label(label: &str) -> bool {
    label.starts_with(RUN_WINDOW_LABEL_PREFIX)
}

// The digest distinguishes ids that collapse to the same safe readable slug.
fn run_window_label(run_id: &str) -> String {
    let slug: String = run_id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .take(RUN_WINDOW_LABEL_SLUG_MAX)
        .collect();
    let digest = stable_digest(run_id);
    if slug.is_empty() {
        format!("{RUN_WINDOW_LABEL_PREFIX}{digest}")
    } else {
        format!("{RUN_WINDOW_LABEL_PREFIX}{slug}-{digest}")
    }
}

fn stable_digest(value: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn normalize_run_id(run_id: &str) -> Option<&str> {
    let trimmed = run_id.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

fn run_window_path(run_id: &str, host_id: &str, project_id: &str) -> String {
    format!(
        "{RUN_WINDOW_PATH}?{RUN_WINDOW_RUN_ID_PARAM}={}&{RUN_WINDOW_HOST_ID_PARAM}={}&{RUN_WINDOW_PROJECT_ID_PARAM}={}",
        encode_query_value(run_id),
        encode_query_value(host_id),
        encode_query_value(project_id),
    )
}

fn encode_query_value(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

// Building a webview from a synchronous command deadlocks on Windows.
#[tauri::command]
async fn open_run_window(
    run_id: String,
    host_id: String,
    project_id: String,
    title: String,
    app: AppHandle,
) -> Result<(), String> {
    let run_id = normalize_run_id(&run_id).ok_or_else(|| "run id is required".to_string())?;
    let host_id = normalize_run_id(&host_id).ok_or_else(|| "Host id is required".to_string())?;
    let project_id =
        normalize_run_id(&project_id).ok_or_else(|| "Project id is required".to_string())?;
    let label = run_window_label(run_id);
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        return Ok(());
    }
    WebviewWindowBuilder::new(
        &app,
        label.as_str(),
        WebviewUrl::App(run_window_path(run_id, host_id, project_id).into()),
    )
    .title(title)
    .inner_size(RUN_WINDOW_WIDTH, RUN_WINDOW_HEIGHT)
    .min_inner_size(RUN_WINDOW_MIN_WIDTH, RUN_WINDOW_MIN_HEIGHT)
    .build()
    .map_err(|err| err.to_string())?;
    Ok(())
}

fn boot_kernel(
    app: &AppHandle,
    host_mode: HostMode,
) -> Result<HostKernel, Box<dyn std::error::Error>> {
    let app_local_data_dir = app.path().app_local_data_dir()?;
    let app_log_dir = app.path().app_log_dir()?;
    let system_locale = sys_locale::get_locale().unwrap_or_else(|| "en-US".to_string());
    let system_appearance = match app
        .get_webview_window("main")
        .and_then(|window| window.theme().ok())
    {
        Some(tauri::Theme::Dark) => SystemAppearance::Dark,
        _ => SystemAppearance::Light,
    };
    let request = BootRequest {
        app_local_data_dir,
        app_log_dir,
        system_locale,
        system_appearance,
        host_display_name: host_display_name(),
    };
    Ok(match host_mode {
        HostMode::HostAndClient => HostKernel::boot(request)?,
        HostMode::ClientOnly => HostKernel::boot_client_only(request)?,
    })
}

fn host_display_name() -> String {
    command_stdout("hostname", &[])
        .or_else(|| command_stdout("scutil", &["--get", "ComputerName"]))
        .unwrap_or_else(|| "Host".to_string())
}

fn command_stdout(program: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .ok()?;
    let name = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (!name.is_empty()).then_some(name)
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let icon = app
        .default_window_icon()
        .cloned()
        .expect("missing window icon");
    TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("Agent Taskboard")
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| handle_shell_menu(app, event.id.as_ref()))
        .build(app)?;
    Ok(())
}

fn refresh_shell(app: &AppHandle, snapshot: &HostSnapshot) -> Result<(), String> {
    let signature = ShellMenuSignature::from_snapshot(snapshot);
    if let Some(state) = app.try_state::<AppState>() {
        let cached = state.menu_signature.lock().map_err(|err| err.to_string())?;
        if cached.as_ref() == Some(&signature) {
            return Ok(());
        }
    }
    rebuild_tray_menu(app, snapshot).map_err(|err| err.to_string())?;
    rebuild_app_menu(app, snapshot).map_err(|err| err.to_string())?;
    if let Some(state) = app.try_state::<AppState>() {
        let mut cached = state.menu_signature.lock().map_err(|err| err.to_string())?;
        *cached = Some(signature);
    }
    Ok(())
}

fn resident_items(
    app: &AppHandle,
    snapshot: &HostSnapshot,
    quit_accelerator: Option<&str>,
) -> tauri::Result<(MenuItem<tauri::Wry>, MenuItem<tauri::Wry>)> {
    let show = MenuItem::with_id(app, "show", &snapshot.copy.show_window, true, None::<&str>)?;
    let quit = MenuItem::with_id(
        app,
        "quit",
        &snapshot.copy.quit_host,
        true,
        quit_accelerator,
    )?;
    Ok((show, quit))
}

fn rebuild_tray_menu(app: &AppHandle, snapshot: &HostSnapshot) -> tauri::Result<()> {
    let (show, quit) = resident_items(app, snapshot, None)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_menu(Some(menu))?;
        tray.set_tooltip(Some(&snapshot.copy.app_name))?;
    }
    Ok(())
}

fn about_name(app_name: &str, acceptance_digest: Option<&str>) -> String {
    match acceptance_digest {
        Some(digest) => format!(
            "{app_name} Acceptance {}",
            digest.chars().take(12).collect::<String>()
        ),
        None => app_name.to_string(),
    }
}

fn about_version(version: &str, acceptance_digest: Option<&str>) -> String {
    match acceptance_digest {
        Some(digest) => format!("{version} · acceptance manifest SHA-256 {digest}"),
        None => version.to_string(),
    }
}

fn rebuild_app_menu(app: &AppHandle, snapshot: &HostSnapshot) -> tauri::Result<()> {
    let copy = &snapshot.copy;
    let enabled = current_edit_enabled(app);
    let (show, quit) = resident_items(app, snapshot, Some("CmdOrCtrl+Q"))?;
    let settings = MenuItem::with_id(app, "settings", &copy.settings, true, Some("CmdOrCtrl+,"))?;
    let version = app.package_info().version.to_string();
    let about = PredefinedMenuItem::about(
        app,
        None,
        Some(AboutMetadata {
            name: Some(about_name(&copy.app_name, ACCEPTANCE_MANIFEST_SHA256)),
            version: Some(about_version(&version, ACCEPTANCE_MANIFEST_SHA256)),
            ..Default::default()
        }),
    )?;
    let app_menu = build_app_submenu(app, copy.app_name.as_str(), &about, &settings, &show, &quit)?;

    let redo_accelerator = if cfg!(target_os = "macos") {
        "Shift+CmdOrCtrl+Z"
    } else {
        "CmdOrCtrl+Y"
    };
    let undo = MenuItem::with_id(
        app,
        EDIT_UNDO,
        &copy.edit_undo,
        enabled.undo,
        Some("CmdOrCtrl+Z"),
    )?;
    let redo = MenuItem::with_id(
        app,
        EDIT_REDO,
        &copy.edit_redo,
        enabled.redo,
        Some(redo_accelerator),
    )?;
    let cut = MenuItem::with_id(
        app,
        EDIT_CUT,
        &copy.edit_cut,
        enabled.cut,
        Some("CmdOrCtrl+X"),
    )?;
    let copy_item = MenuItem::with_id(
        app,
        EDIT_COPY,
        &copy.edit_copy,
        enabled.copy,
        Some("CmdOrCtrl+C"),
    )?;
    let paste = MenuItem::with_id(
        app,
        EDIT_PASTE,
        &copy.edit_paste,
        enabled.paste,
        Some("CmdOrCtrl+V"),
    )?;
    let select_all = MenuItem::with_id(
        app,
        EDIT_SELECT_ALL,
        &copy.edit_select_all,
        enabled.select_all,
        Some("CmdOrCtrl+A"),
    )?;
    let edit = Submenu::with_items(
        app,
        &copy.edit_menu,
        true,
        &[
            &undo,
            &redo,
            &PredefinedMenuItem::separator(app)?,
            &cut,
            &copy_item,
            &paste,
            &select_all,
        ],
    )?;
    let window_menu = build_window_submenu(app, &copy.window_menu)?;
    let help_keyboard = MenuItem::with_id(
        app,
        "help-keyboard",
        &copy.keyboard_help,
        true,
        None::<&str>,
    )?;
    let help_usage = MenuItem::with_id(app, "help-usage", &copy.usage_guide, true, None::<&str>)?;
    let help = Submenu::with_id_and_items(
        app,
        HELP_SUBMENU_ID,
        &copy.help_menu,
        true,
        &[&help_keyboard, &help_usage],
    )?;
    app.set_menu(Menu::with_items(
        app,
        &[&app_menu, &edit, &window_menu, &help],
    )?)?;
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut items) = state.edit_items.lock() {
            *items = Some(EditMenuItems {
                undo,
                redo,
                cut,
                copy: copy_item,
                paste,
                select_all,
            });
        }
    }
    Ok(())
}

fn build_app_submenu(
    app: &AppHandle,
    title: &str,
    about: &PredefinedMenuItem<tauri::Wry>,
    settings: &MenuItem<tauri::Wry>,
    show: &MenuItem<tauri::Wry>,
    quit: &MenuItem<tauri::Wry>,
) -> tauri::Result<Submenu<tauri::Wry>> {
    #[cfg(target_os = "macos")]
    {
        Submenu::with_items(
            app,
            title,
            true,
            &[
                about,
                &PredefinedMenuItem::separator(app)?,
                settings,
                &PredefinedMenuItem::separator(app)?,
                &PredefinedMenuItem::services(app, None)?,
                &PredefinedMenuItem::separator(app)?,
                &PredefinedMenuItem::hide(app, None)?,
                &PredefinedMenuItem::hide_others(app, None)?,
                &PredefinedMenuItem::show_all(app, None)?,
                &PredefinedMenuItem::separator(app)?,
                show,
                &PredefinedMenuItem::separator(app)?,
                quit,
            ],
        )
    }
    #[cfg(not(target_os = "macos"))]
    {
        Submenu::with_items(
            app,
            title,
            true,
            &[
                about,
                &PredefinedMenuItem::separator(app)?,
                settings,
                &PredefinedMenuItem::separator(app)?,
                show,
                &PredefinedMenuItem::separator(app)?,
                quit,
            ],
        )
    }
}

fn build_window_submenu(app: &AppHandle, title: &str) -> tauri::Result<Submenu<tauri::Wry>> {
    let minimize = PredefinedMenuItem::minimize(app, None)?;
    let maximize = PredefinedMenuItem::maximize(app, None)?;
    let close_window = PredefinedMenuItem::close_window(app, None)?;
    #[cfg(target_os = "macos")]
    {
        Submenu::with_id_and_items(
            app,
            WINDOW_SUBMENU_ID,
            title,
            true,
            &[
                &minimize,
                &maximize,
                &PredefinedMenuItem::fullscreen(app, None)?,
                &PredefinedMenuItem::separator(app)?,
                &close_window,
                &PredefinedMenuItem::separator(app)?,
                &PredefinedMenuItem::bring_all_to_front(app, None)?,
            ],
        )
    }
    #[cfg(not(target_os = "macos"))]
    {
        Submenu::with_id_and_items(
            app,
            WINDOW_SUBMENU_ID,
            title,
            true,
            &[
                &minimize,
                &maximize,
                &PredefinedMenuItem::separator(app)?,
                &close_window,
            ],
        )
    }
}

fn current_edit_enabled(app: &AppHandle) -> EditMenuEnabled {
    app.try_state::<AppState>()
        .and_then(|state| state.edit_context.lock().ok().map(|context| *context))
        .map(edit_menu_enabled)
        .unwrap_or_else(|| edit_menu_enabled(EditMenuContext::default()))
}

fn apply_edit_menu_state(app: &AppHandle) -> Result<(), String> {
    let Some(state) = app.try_state::<AppState>() else {
        return Ok(());
    };
    let context = *state.edit_context.lock().map_err(|err| err.to_string())?;
    let enabled = edit_menu_enabled(context);
    let items = state.edit_items.lock().map_err(|err| err.to_string())?;
    let Some(items) = items.as_ref() else {
        return Ok(());
    };
    items
        .undo
        .set_enabled(enabled.undo)
        .map_err(|err| err.to_string())?;
    items
        .redo
        .set_enabled(enabled.redo)
        .map_err(|err| err.to_string())?;
    items
        .cut
        .set_enabled(enabled.cut)
        .map_err(|err| err.to_string())?;
    items
        .copy
        .set_enabled(enabled.copy)
        .map_err(|err| err.to_string())?;
    items
        .paste
        .set_enabled(enabled.paste)
        .map_err(|err| err.to_string())?;
    items
        .select_all
        .set_enabled(enabled.select_all)
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn handle_shell_menu(app: &AppHandle, id: &str) {
    match id {
        "show" => show_main(app),
        "settings" => {
            show_main(app);
            dispatch_webview_event(app, "agent-taskboard:open-settings");
        }
        "quit" => quit_host(app),
        EDIT_UNDO => dispatch_webview_event(app, "agent-taskboard:edit-undo"),
        EDIT_REDO => dispatch_webview_event(app, "agent-taskboard:edit-redo"),
        EDIT_CUT => dispatch_webview_event(app, "agent-taskboard:edit-cut"),
        EDIT_COPY => dispatch_webview_event(app, "agent-taskboard:edit-copy"),
        EDIT_PASTE => dispatch_webview_event(app, "agent-taskboard:edit-paste"),
        EDIT_SELECT_ALL => dispatch_webview_event(app, "agent-taskboard:edit-select-all"),
        "help-keyboard" => {
            show_main(app);
            dispatch_webview_event(app, "agent-taskboard:open-keyboard-help");
        }
        "help-usage" => {
            let _ = app.opener().open_url(USAGE_GUIDE_URL, None::<&str>);
        }
        _ => {}
    }
}

fn dispatch_webview_event(app: &AppHandle, event: &str) {
    if let Some(window) = app.get_webview_window("main") {
        let encoded = serde_json::to_string(event).unwrap();
        let _ = window.eval(format!("window.dispatchEvent(new Event({encoded}));"));
    }
}

fn show_main(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    {
        let _ = app.show();
    }
    let mut became_visible = false;
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut kernel) = state.kernel.lock() {
            became_visible = !kernel.snapshot().window_visible;
            let _ = kernel.dispatch(Command::ShowWindow);
        }
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        if became_visible {
            let _ = window.eval(
                "window.dispatchEvent(new Event('agent-taskboard:host-window-shown')); window.dispatchEvent(new Event('agent-taskboard:check-update'));",
            );
        }
    }
}

fn quit_host(app: &AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut kernel) = state.kernel.lock() {
            match kernel.dispatch(Command::QuitHost) {
                Ok(outcome) if outcome.process == ProcessIntent::Exit => {
                    drop(kernel);
                    app.exit(0);
                    return;
                }
                Ok(outcome) => {
                    drop(kernel);
                    show_main(app);
                    let _ = refresh_shell(app, &outcome.snapshot);
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.eval("window.dispatchEvent(new Event('focus'));");
                    }
                    return;
                }
                Err(_) => return,
            }
        }
    }
    app.exit(0);
}

#[cfg(test)]
mod run_window_tests {
    use super::*;

    const RUN_ID: &str = "5f2a9c1e4b7d8a30c6e1f4b2a9d3c0e7";

    #[test]
    fn label_is_stable_and_unique_per_run_id() {
        assert_eq!(run_window_label(RUN_ID), run_window_label(RUN_ID));
        assert_ne!(
            run_window_label(RUN_ID),
            run_window_label("5f2a9c1e4b7d8a30c6e1f4b2a9d3c0e8")
        );
        let label = run_window_label(RUN_ID);
        assert!(is_run_window_label(&label));
        assert_ne!(label, "main");
    }

    #[test]
    fn label_stays_unique_when_run_ids_only_differ_in_unsafe_characters() {
        assert_ne!(run_window_label("a/b"), run_window_label("a-b"));
        assert_ne!(run_window_label("a/b"), run_window_label("a b"));
        assert_ne!(run_window_label("a/b"), run_window_label("a:b"));
    }

    #[test]
    fn label_of_a_hostile_run_id_keeps_to_the_safe_character_set() {
        for run_id in [
            "../../etc/passwd",
            "a b/c:d",
            "run\u{0}x",
            "\u{65e5}\u{672c}\u{8a9e}",
            "",
        ] {
            let label = run_window_label(run_id);
            assert!(
                label
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '-'),
                "unsafe label {label} for {run_id:?}"
            );
            assert!(label.starts_with(RUN_WINDOW_LABEL_PREFIX), "{label}");
            assert_ne!(close_behavior(&label), CloseBehavior::HideMain);
        }
    }

    #[test]
    fn label_length_stays_bounded_for_long_run_ids() {
        let label = run_window_label(&"a".repeat(4096));
        let budget = RUN_WINDOW_LABEL_PREFIX.len() + RUN_WINDOW_LABEL_SLUG_MAX + 1 + 16;
        assert!(label.len() <= budget, "label too long: {}", label.len());
    }

    #[test]
    fn path_carries_the_run_id_as_one_encoded_query_parameter() {
        assert_eq!(
            run_window_path(RUN_ID, "local", "garden"),
            format!("{RUN_WINDOW_PATH}?{RUN_WINDOW_RUN_ID_PARAM}={RUN_ID}&{RUN_WINDOW_HOST_ID_PARAM}=local&{RUN_WINDOW_PROJECT_ID_PARAM}=garden")
        );
        assert_eq!(
            run_window_path("run id/1", "host&one", "project=one"),
            "index.html?runId=run%20id%2F1&hostId=host%26one&projectId=project%3Done"
        );
        assert_eq!(
            run_window_path("\u{65e5}", "\u{672c}", "\u{8a9e}"),
            "index.html?runId=%E6%97%A5&hostId=%E6%9C%AC&projectId=%E8%AA%9E"
        );
        assert_eq!(encode_query_value("~-._"), "~-._");
    }

    #[test]
    fn blank_run_ids_are_rejected() {
        assert_eq!(normalize_run_id(RUN_ID), Some(RUN_ID));
        assert_eq!(normalize_run_id(""), None);
        assert_eq!(normalize_run_id("   "), None);
    }

    #[test]
    fn only_main_keeps_the_resident_hide_on_close() {
        assert_eq!(close_behavior("main"), CloseBehavior::HideMain);
        assert_eq!(
            close_behavior(&run_window_label(RUN_ID)),
            CloseBehavior::DestroyRunWindow
        );
        assert_eq!(close_behavior("unrelated"), CloseBehavior::Native);
    }

    #[test]
    fn run_window_starts_and_stays_in_the_desktop_layout_range() {
        assert_eq!((RUN_WINDOW_WIDTH, RUN_WINDOW_HEIGHT), (1180.0, 760.0));
        assert_eq!(
            (RUN_WINDOW_MIN_WIDTH, RUN_WINDOW_MIN_HEIGHT),
            (900.0, 640.0)
        );
    }

    #[test]
    fn packaged_client_candidates_include_tauri_parent_resource_layout() {
        let root = PathBuf::from("/Resources");
        assert_eq!(
            packaged_client_candidates(root.clone()),
            [root.join("_up_/dist"), root.join("dist"), root,]
        );
    }

    #[test]
    fn packaged_loopback_uses_only_a_candidate_with_a_complete_client_entrypoint() {
        let empty = tempfile::tempdir().unwrap();
        let packaged = tempfile::tempdir().unwrap();
        fs::write(packaged.path().join("index.html"), "<main>Client</main>").unwrap();

        match packaged_loopback_assets([empty.path().to_path_buf(), packaged.path().to_path_buf()])
            .unwrap()
        {
            LoopbackAssets::Directory(path) => assert_eq!(path, packaged.path()),
            _ => panic!("packaged Client must use directory assets"),
        }
    }

    #[test]
    fn packaged_loopback_rejects_a_candidate_without_client_assets() {
        let empty = tempfile::tempdir().unwrap();
        let error = packaged_loopback_assets([empty.path().to_path_buf()]).unwrap_err();
        assert!(error.contains("packaged Client assets are missing"));
        assert!(error.contains(empty.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn acceptance_about_information_contains_the_complete_manifest_digest() {
        let digest = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        assert_eq!(
            about_name("Agent Taskboard", Some(digest)),
            "Agent Taskboard Acceptance 0123456789ab"
        );
        assert_eq!(
            about_version("0.1.2", Some(digest)),
            format!("0.1.2 · acceptance manifest SHA-256 {digest}")
        );
        assert_eq!(about_name("Agent Taskboard", None), "Agent Taskboard");
        assert_eq!(about_version("0.1.2", None), "0.1.2");
    }
}
