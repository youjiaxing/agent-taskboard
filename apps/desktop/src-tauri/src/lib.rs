use std::sync::Mutex;

use host_kernel::{
    BootRequest, Command, HostKernel, HostSnapshot, ProcessIntent, SystemAppearance,
};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, State, WindowEvent};

struct AppState {
    kernel: Mutex<HostKernel>,
}

#[tauri::command]
fn host_rpc(
    app: AppHandle,
    state: State<AppState>,
    request: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut kernel = state.kernel.lock().map_err(|err| err.to_string())?;
    let result = kernel.handle(request).map_err(|err| err.to_string())?;
    let snapshot = kernel.snapshot();
    let should_exit = matches!(
        result.get("process").and_then(|value| value.as_str()),
        Some("exit")
    );
    drop(kernel);
    refresh_shell(&app, &snapshot)?;
    if should_exit {
        app.exit(0);
    }
    Ok(result)
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let kernel = boot_kernel(app.handle())?;
            let snapshot = kernel.snapshot();
            app.manage(AppState {
                kernel: Mutex::new(kernel),
            });
            build_tray(app.handle())?;
            refresh_shell(app.handle(), &snapshot)?;
            app.on_menu_event(|app, event| handle_shell_menu(app, event.id().as_ref()));
            Ok(())
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
        .invoke_handler(tauri::generate_handler![host_rpc])
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
    hostname()
        .or_else(computer_name)
        .unwrap_or_else(|| "Host".to_string())
}

fn hostname() -> Option<String> {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|output| {
            let name = String::from_utf8(output.stdout).ok()?.trim().to_string();
            (!name.is_empty()).then_some(name)
        })
}

fn computer_name() -> Option<String> {
    std::process::Command::new("scutil")
        .args(["--get", "ComputerName"])
        .output()
        .ok()
        .and_then(|output| {
            let name = String::from_utf8(output.stdout).ok()?.trim().to_string();
            (!name.is_empty()).then_some(name)
        })
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

fn rebuild_tray_menu(app: &AppHandle, snapshot: &HostSnapshot) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", &snapshot.copy.show_window, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", &snapshot.copy.quit_host, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_menu(Some(menu))?;
        tray.set_tooltip(Some(&snapshot.copy.app_name))?;
    }
    Ok(())
}

fn rebuild_app_menu(app: &AppHandle, snapshot: &HostSnapshot) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", &snapshot.copy.show_window, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", &snapshot.copy.quit_host, true, None::<&str>)?;
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
