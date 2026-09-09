#[cfg(unix)]
use std::sync::mpsc;

#[cfg(unix)]
pub fn spawn_ctrl_c_handler(app: tauri::AppHandle) {
    std::thread::Builder::new()
        .name("ctrl-c-handler".into())
        .spawn(move || {
            let (sender, receiver) = mpsc::channel();
            if let Err(error) = ctrlc::set_handler(move || {
                let _ = sender.send(());
            }) {
                eprintln!("failed to register Ctrl+C handler: {error}");
                return;
            }
            if receiver.recv().is_ok() {
                app.exit(130);
            }
        })
        .expect("failed to start Ctrl+C handler");
}

#[cfg(not(unix))]
pub fn spawn_ctrl_c_handler(_app: tauri::AppHandle) {}
