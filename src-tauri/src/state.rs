use anyhow::Result;
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    core::{config, portal},
    log,
};

const WATCH_INTERVAL: Duration = Duration::from_secs(600);
const RETRY_INITIAL: Duration = Duration::from_secs(10);
const RETRY_MAX: Duration = Duration::from_secs(3600);

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Status {
    pub(crate) state: String,
    pub(crate) account: String,
    pub(crate) last_check: String,
    pub(crate) last_login: String,
    pub(crate) message: String,
}

pub(crate) type SharedStatus = Arc<Mutex<Status>>;

fn now() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn flag(path: &PathBuf, name: &str) -> bool {
    let file = path.join(name);
    if file.exists() {
        let _ = std::fs::remove_file(file);
        return true;
    }
    false
}

fn tick(duration: Duration, control: &PathBuf) -> bool {
    let deadline = Instant::now() + duration;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return false;
        }
        thread::sleep(left.min(Duration::from_secs(1)));
        if flag(control, "reconnect") {
            return true;
        }
    }
}

fn publish(
    app: &AppHandle,
    shared: &SharedStatus,
    state: &str,
    account: &str,
    last_login: &str,
    message: &str,
) {
    let status = Status {
        state: state.into(),
        account: account.into(),
        last_check: now(),
        last_login: last_login.into(),
        message: message.into(),
    };
    if let Ok(mut guard) = shared.lock() {
        *guard = status.clone();
    }
    let _ = app.emit("status", status);
}

pub(crate) fn paths(app: &AppHandle) -> Result<(PathBuf, PathBuf)> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    let control = dir.join("control");
    std::fs::create_dir_all(&control)?;
    Ok((dir.join("config.enc"), control))
}

pub(crate) fn start_worker(app: AppHandle, shared: SharedStatus) -> Result<()> {
    let (config_path, control) = paths(&app)?;
    thread::spawn(move || {
        let probe = match portal::client(None) {
            Ok(client) => client,
            Err(error) => {
                log(format!("初始化 HTTP 客户端失败：{error:#}"));
                return;
            }
        };
        let mut credentials: Option<config::Credentials> = None;
        let mut account = "未配置".to_string();
        let mut last_login = "尚未登录".to_string();
        let mut retry = RETRY_INITIAL;

        loop {
            if flag(&control, "reset") {
                credentials = None;
                account = "未配置".to_string();
                log("凭证已清除");
            }
            if credentials.is_none() {
                match config::read_credentials(&config_path) {
                    Ok(Some(loaded)) => {
                        account = config::masked_account(&loaded.account);
                        credentials = Some(loaded);
                    }
                    _ => {
                        publish(
                            &app,
                            &shared,
                            "need-setup",
                            &account,
                            &last_login,
                            "请打开界面输入账号密码",
                        );
                        thread::sleep(Duration::from_secs(2));
                        continue;
                    }
                }
            }
            let current = credentials.as_ref().expect("凭证已加载");
            let force = flag(&control, "reconnect");
            let online = !force && portal::check_network(&probe);
            if online {
                retry = RETRY_INITIAL;
                publish(
                    &app,
                    &shared,
                    "online",
                    &account,
                    &last_login,
                    "网络正常，10 分钟后复查",
                );
                let _ = tick(WATCH_INTERVAL, &control);
                continue;
            }
            if !force {
                thread::sleep(Duration::from_secs(1));
                if portal::check_network(&probe) {
                    continue;
                }
            }
            publish(
                &app,
                &shared,
                "logging-in",
                &account,
                &last_login,
                "正在尝试登录",
            );
            let _ = tick(retry, &control);
            match portal::attempt_login(current, None) {
                Ok(true) => {
                    last_login = now();
                    retry = RETRY_INITIAL;
                    log("登录成功");
                    publish(&app, &shared, "online", &account, &last_login, "登录成功");
                }
                Ok(false) => {
                    retry = (retry * 2).min(RETRY_MAX);
                    publish(
                        &app,
                        &shared,
                        "offline",
                        &account,
                        &last_login,
                        "登录失败，等待重试",
                    );
                }
                Err(error) => {
                    retry = (retry * 2).min(RETRY_MAX);
                    log(format!("登录异常：{error:#}"));
                    publish(
                        &app,
                        &shared,
                        "offline",
                        &account,
                        &last_login,
                        "登录异常，等待重试",
                    );
                }
            }
        }
    });
    Ok(())
}
