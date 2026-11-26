mod commands;
mod core;
mod state;
mod tray;
use crate::core::{config, dns};
use tauri::Manager;

pub(crate) type SharedStatus = state::SharedStatus;

pub(crate) const PORTAL_URL: &str = "http://219.226.127.250";
pub(crate) const LOGIN_HOST: &str = "https://drcom.tyut.edu.cn:804";
pub(crate) const LOGIN_DOMAIN: &str = "drcom.tyut.edu.cn";
pub(crate) const TARGET: &str = "223.5.5.5";
pub(crate) const REFERER_URL: &str = "https://drcom.tyut.edu.cn:804/eportal/portal/index";
pub(crate) const USER_AGENT_VALUE: &str = "Mozilla/5.0 Chrome/146.0.0.0 Edg/146.0.0.0";
pub(crate) const LOGIN_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36 Edg/146.0.0.0";

pub(crate) fn log(message: impl AsRef<str>) {
    println!(
        "[{}] {}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        message.as_ref()
    );
}

#[cfg(test)]
mod tests {
    use crate::state::Status;

    #[test]
    fn status_serializes_camel_case() {
        let status = Status {
            state: "online".into(),
            account: "d****03".into(),
            last_check: "2026-11-26 15:00:00".into(),
            last_login: "尚未登录".into(),
            message: "网络正常".into(),
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"lastCheck\""));
        assert!(json.contains("\"lastLogin\""));
    }

    #[test]
    fn reconnect_flag_is_consumed_once() {
        let dir = std::env::temp_dir().join(format!("tyut-keepalive-flag-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("reconnect"), b"").unwrap();
        assert!(crate::state::flag(&dir, "reconnect"));
        assert!(!crate::state::flag(&dir, "reconnect"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

pub fn run() {
    let shared: SharedStatus = std::sync::Arc::new(std::sync::Mutex::new(state::Status::default()));
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .manage(shared.clone())
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::reconnect,
            commands::reset_credentials,
            commands::setup_credentials,
            commands::set_autostart,
            commands::is_autostart,
            commands::quit,
            commands::show_window,
            commands::hide_window,
        ])
        .setup(move |app| {
            state::start_worker(app.handle().clone(), shared)?;
            tray::create_tray(app.handle())?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Tauri 应用构建失败");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::WindowEvent {
            label,
            event: tauri::WindowEvent::CloseRequested { api, .. },
            ..
        } = event
        {
            api.prevent_close();
            if label == "main" {
                if let Some(window) = app_handle.get_webview_window(&label) {
                    if tray::is_tray_available() {
                        let _ = window.hide();
                    } else {
                        let _ = window.minimize();
                    }
                }
            }
        }
    });
}
