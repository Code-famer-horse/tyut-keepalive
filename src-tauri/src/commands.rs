use std::{fs, sync::Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::{core::config, state::paths, SharedStatus};

#[tauri::command]
pub(crate) fn get_status(status: State<'_, SharedStatus>) -> Result<serde_json::Value, String> {
    let guard = status.lock().map_err(|_| "状态锁被占用".to_string())?;
    serde_json::to_value(guard.clone()).map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn reconnect(app: AppHandle) -> Result<(), String> {
    let (_, control) = paths(&app).map_err(|error| error.to_string())?;
    fs::write(control.join("reconnect"), b"1").map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn reset_credentials(app: AppHandle) -> Result<(), String> {
    let (config_path, control) = paths(&app).map_err(|error| error.to_string())?;
    if config_path.exists() {
        fs::remove_file(&config_path).map_err(|error| error.to_string())?;
    }
    fs::write(control.join("reset"), b"1").map_err(|error| error.to_string())?;
    fs::write(control.join("reconnect"), b"1").map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn setup_credentials(
    app: AppHandle,
    account: String,
    password: String,
) -> Result<(), String> {
    if account.trim().is_empty() || password.trim().is_empty() {
        return Err("账号和密码不能为空".into());
    }
    let (config_path, _) = paths(&app).map_err(|error| error.to_string())?;
    let credentials = config::Credentials {
        account: config::xor_encode(account.trim()),
        password: config::xor_encode(password.trim()),
        saved_at: Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()),
    };
    config::save_credentials(&config_path, &credentials).map_err(|error| error.to_string())?;
    let _ = app.emit("credentials-saved", ());
    Ok(())
}

#[tauri::command]
pub(crate) fn set_autostart(app: AppHandle, enabled: bool) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    let autostart = app.autolaunch();
    if enabled {
        autostart.enable().map_err(|error| error.to_string())?;
    } else {
        autostart.disable().map_err(|error| error.to_string())?;
    }
    autostart.is_enabled().map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn is_autostart(app: AppHandle) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch()
        .is_enabled()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn quit(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub(crate) fn show_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[tauri::command]
pub(crate) fn hide_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.minimize();
    }
}

pub(crate) fn _unused_mutex_type() -> Mutex<()> {
    Mutex::new(())
}
