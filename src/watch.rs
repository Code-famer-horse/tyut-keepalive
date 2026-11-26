use anyhow::{bail, Result};
use std::{thread, time::Duration};

use crate::{
    args::Args,
    config, log,
    portal::{attempt_login, check_network, client},
};

const WATCH_INTERVAL: Duration = Duration::from_secs(600);
const INITIAL_RETRY: Duration = Duration::from_secs(10);
const MAX_RETRY: Duration = Duration::from_secs(3600);

pub(crate) fn run(args: Args) -> Result<()> {
    if args.reset {
        if args.config.exists() {
            std::fs::remove_file(&args.config)?;
            log(format!("已删除凭证：{}", args.config.display()));
        } else {
            log("凭证文件不存在");
        }
        return Ok(());
    }
    log("TYUT 校园网自动登录 (Rust)");
    let credentials = match config::read_credentials(&args.config)? {
        Some(credentials) => {
            log(format!("已读取凭证：{}", args.config.display()));
            credentials
        }
        None => config::prompt_credentials(&args.config)?,
    };
    let probe_client = client(None)?;
    let mut retry = INITIAL_RETRY;
    loop {
        if check_network(&probe_client) {
            retry = INITIAL_RETRY;
            if args.once {
                log("网络已连通，无需登录");
                return Ok(());
            }
            log("网络正常，10 分钟后复查");
            thread::sleep(WATCH_INTERVAL);
            continue;
        }
        log("网络不可用，再次确认...");
        thread::sleep(Duration::from_secs(1));
        if check_network(&probe_client) {
            continue;
        }
        thread::sleep(Duration::from_secs(2));
        if check_network(&probe_client) {
            continue;
        }
        log(format!("确认断网，{} 秒后尝试重连", retry.as_secs()));
        thread::sleep(retry);
        match attempt_login(&credentials, args.mac_override.as_deref()) {
            Ok(true) => {
                log("登录流程已完成");
                retry = INITIAL_RETRY;
                if args.once {
                    return Ok(());
                }
            }
            Ok(false) => {
                retry = (retry * 2).min(MAX_RETRY);
                log(format!("重连失败，下次重试等待 {} 秒", retry.as_secs()));
                if args.once {
                    bail!("重连失败");
                }
            }
            Err(error) => {
                log(format!("本次重连失败：{error:#}"));
                retry = (retry * 2).min(MAX_RETRY);
                if args.once {
                    return Err(error);
                }
                thread::sleep(Duration::from_secs(30));
            }
        }
    }
}
