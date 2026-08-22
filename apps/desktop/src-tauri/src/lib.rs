use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use host_kernel::{
    BootRequest, Command, HostKernel, HostSnapshot, LoopbackAssets, LoopbackServer, ProcessIntent,
    SystemAppearance, LOCAL_RPC_PORT,
};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Manager, WindowEvent};

struct AppState {
    kernel: Arc<Mutex<HostKernel>>,
    protocol_url: String,
    _loopback: LoopbackServer,
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let kernel = boot_kernel(app.handle())?;
            let kernel = Arc::new(Mutex::new(kernel));
            let app_handle = app.handle().clone();
            let loopback = LoopbackServer::attach_with(
                Arc::clone(&kernel),
                LOCAL_RPC_PORT,
                loopback_assets(app.handle()),
                move |outcome| {
                    let _ = refresh_shell(&app_handle, &outcome.snapshot);
                    if outcome.process == ProcessIntent::Exit {
                        app_handle.exit(0);
                    }
                },
            )?;
            let protocol_url = loopback.protocol_url().to_string();
            let snapshot = kernel.lock().map_err(|err| err.to_string())?.snapshot();
            app.manage(AppState {
                kernel,
                protocol_url: protocol_url.clone(),
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
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                if let Some(state) = window.try_state::<AppState>() {
                    if let Ok(mut kernel) = state.kernel.lock() {
                        let _ = kernel.dispatch(Command::HideWindow);
                    }
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building Agent Taskboard")
        .run(|app, event| match event {
            tauri::RunEvent::ExitRequested { api, code, .. } => {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
            tauri::RunEvent::Reopen { .. } => {
                show_main(app);
            }
            _ => {}
        });
}

fn loopback_assets(app: &AppHandle) -> LoopbackAssets {
    if cfg!(debug_assertions) {
        return LoopbackAssets::DevProxy {
            origin: "http://127.0.0.1:1420".into(),
        };
    }
    let mut candidates = Vec::new();
    if let Ok(dir) = app.path().resource_dir() {
        candidates.push(dir.clone());
        candidates.push(dir.join("dist"));
    }
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../dist"));
    for dir in candidates {
        if dir.join("index.html").exists() {
            return LoopbackAssets::Directory(dir);
        }
    }
    LoopbackAssets::Builtin
}

fn inject_protocol_url(window: &tauri::WebviewWindow, url: &str) {
    let encoded = serde_json::to_string(url).unwrap();
    let _ = window.eval(format!("window.__HOST_PROTOCOL__ = {encoded};"));
}

fn boot_kernel(app: &AppHandle) -> Result<HostKernel, Box<dyn std::error::Error>> {
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
    Ok(HostKernel::boot(BootRequest {
        app_local_data_dir,
        app_log_dir,
        system_locale,
        system_appearance,
        host_display_name: host_display_name(),
    })?)
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
    rebuild_tray_menu(app, snapshot).map_err(|err| err.to_string())?;
    rebuild_app_menu(app, snapshot).map_err(|err| err.to_string())
}

fn resident_items(
    app: &AppHandle,
    snapshot: &HostSnapshot,
) -> tauri::Result<(MenuItem<tauri::Wry>, MenuItem<tauri::Wry>)> {
    let show = MenuItem::with_id(app, "show", &snapshot.copy.show_window, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", &snapshot.copy.quit_host, true, None::<&str>)?;
    Ok((show, quit))
}

fn rebuild_tray_menu(app: &AppHandle, snapshot: &HostSnapshot) -> tauri::Result<()> {
    let (show, quit) = resident_items(app, snapshot)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_menu(Some(menu))?;
        tray.set_tooltip(Some(&snapshot.copy.app_name))?;
    }
    Ok(())
}

fn rebuild_app_menu(app: &AppHandle, snapshot: &HostSnapshot) -> tauri::Result<()> {
    let (show, quit) = resident_items(app, snapshot)?;
    let app_menu = Submenu::with_items(
        app,
        &snapshot.copy.app_name,
        true,
        &[&show, &PredefinedMenuItem::separator(app)?, &quit],
    )?;
    let edit = Submenu::with_items(
        app,
        &snapshot.copy.edit_menu,
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;
    app.set_menu(Menu::with_items(app, &[&app_menu, &edit])?)?;
    Ok(())
}

fn handle_shell_menu(app: &AppHandle, id: &str) {
    match id {
        "show" => show_main(app),
        "quit" => quit_host(app),
        _ => {}
    }
}

fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut kernel) = state.kernel.lock() {
            let _ = kernel.dispatch(Command::ShowWindow);
        }
    }
}

fn quit_host(app: &AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut kernel) = state.kernel.lock() {
            let outcome = kernel.dispatch(Command::QuitHost);
            if !matches!(
                outcome.as_ref().map(|value| value.process),
                Ok(ProcessIntent::Exit)
            ) {
                return;
            }
        }
    }
    app.exit(0);
}
