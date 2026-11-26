use anyhow::{bail, Result};
use chrono::Local;
use std::{
    fs, thread,
    time::{Duration, Instant},
};

use crate::{
    args::Args,
    config, log,
    paths::InstallPaths,
    portal::{attempt_login, check_network, client},
    state::{self, Status},
};

const WATCH_INTERVAL: Duration = Duration::from_secs(600);
const INITIAL_RETRY: Duration = Duration::from_secs(10);
const MAX_RETRY: Duration = Duration::from_secs(3600);

fn now() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn take_flag(path: &std::path::Path) -> bool {
    if path.exists() {
        let _ = fs::remove_file(path);
        return true;
    }
    false
}

fn sleep_tick(duration: Duration, control: &std::path::Path) -> bool {
    let deadline = Instant::now() + duration;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return false;
        }
        thread::sleep(remaining.min(Duration::from_secs(1)));
        if take_flag(&control.join("reconnect")) {
            return true;
        }
    }
}

fn publish(path: &std::path::Path, state: &str, account: &str, last_login: &str, message: &str) {
    let status = Status {
        state: state.to_string(),
        account: account.to_string(),
        last_check: now(),
        last_login: last_login.to_string(),
        message: message.to_string(),
    };
    if let Err(error) = state::write(path, &status) {
        log(format!("写入状态失败：{error:#}"));
    }
}

pub(crate) fn run_once(args: &Args) -> Result<()> {
    let credentials = match config::read_credentials(&args.config)? {
        Some(credentials) => credentials,
        None => config::prompt_credentials(&args.config)?,
    };
    let probe = client(None)?;
    if check_network(&probe) {
        log("网络已连通，无需登录");
        return Ok(());
    }
    log("网络不可用，再次确认...");
    thread::sleep(Duration::from_secs(1));
    if check_network(&probe) {
        log("网络已连通，无需登录");
        return Ok(());
    }
    log("确认断网，尝试重连");
    if attempt_login(&credentials, args.mac_override.as_deref())? {
        log("登录流程已完成");
        Ok(())
    } else {
        bail!("重连失败")
    }
}

pub(crate) fn run_worker(paths: &InstallPaths) -> Result<()> {
    // [设计意图/根因] -> 后台进程无控制台，凭证可能在重置后被删除
    // [理论先验/数学假设] -> 配置文件存在即有可用凭证；缺失时等待重新生成而非提示
    // [机制闭环解释] -> 周期性重读配置，配合重置控制文件清空内存凭证，保证删除后立即失效
    let probe = client(None)?;
    let mut credentials: Option<config::Credentials> = None;
    let mut account = "未配置".to_string();
    let mut last_login = "尚未登录".to_string();
    let mut retry = INITIAL_RETRY;

    loop {
        if take_flag(&paths.control_file("reset")) {
            credentials = None;
            account = "未配置".to_string();
            log("凭证已在内存与磁盘清除，等待重新配置");
        }
        if credentials.is_none() {
            match config::read_credentials(&paths.config)? {
                Some(loaded) => {
                    account = config::masked_account(&loaded.account);
                    credentials = Some(loaded);
                    log("已读取凭证");
                }
                None => {
                    publish(
                        &paths.status,
                        "未配置凭证",
                        &account,
                        &last_login,
                        "请运行程序重新输入账号密码",
                    );
                    thread::sleep(Duration::from_secs(2));
                    continue;
                }
            }
        }
        let current = credentials.as_ref().expect("凭证已加载");
        let reconnect = take_flag(&paths.control_file("reconnect"));
        let online = if reconnect {
            false
        } else {
            check_network(&probe)
        };

        if online {
            retry = INITIAL_RETRY;
            publish(
                &paths.status,
                "在线",
                &account,
                &last_login,
                "网络正常，10 分钟后复查",
            );
            if sleep_tick(WATCH_INTERVAL, &paths.control) {
                log("收到立即重连请求");
            }
            continue;
        }

        if !reconnect {
            publish(
                &paths.status,
                "复查中",
                &account,
                &last_login,
                "网络不可用，二次确认",
            );
            thread::sleep(Duration::from_secs(1));
            if check_network(&probe) {
                continue;
            }
            thread::sleep(Duration::from_secs(2));
            if check_network(&probe) {
                continue;
            }
        }

        publish(
            &paths.status,
            "重连中",
            &account,
            &last_login,
            "正在尝试 Portal 登录",
        );
        log(format!("确认断网，{} 秒后尝试重连", retry.as_secs()));
        if sleep_tick(retry, &paths.control) {
            log("收到立即重连请求，取消等待");
        }
        match attempt_login(current, None) {
            Ok(true) => {
                last_login = now();
                log("登录流程已完成");
                retry = INITIAL_RETRY;
                publish(&paths.status, "在线", &account, &last_login, "登录成功");
            }
            Ok(false) => {
                retry = (retry * 2).min(MAX_RETRY);
                log(format!("重连失败，下次重试等待 {} 秒", retry.as_secs()));
                publish(
                    &paths.status,
                    "离线",
                    &account,
                    &last_login,
                    "重连失败，等待重试",
                );
            }
            Err(error) => {
                log(format!("本次重连失败：{error:#}"));
                retry = (retry * 2).min(MAX_RETRY);
                publish(
                    &paths.status,
                    "离线",
                    &account,
                    &last_login,
                    "重连异常，等待重试",
                );
            }
        }
    }
}
