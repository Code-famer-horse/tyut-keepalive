use anyhow::{bail, Context, Result};
use std::{env, path::PathBuf};

use crate::paths;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Bootstrap,
    Service,
    Worker,
    Setup,
    Status,
    Once,
    Reset,
    EnableAutostart,
    DisableAutostart,
}

pub(crate) struct Args {
    pub(crate) config: PathBuf,
    pub(crate) once: bool,
    pub(crate) reset: bool,
    pub(crate) mac_override: Option<String>,
    pub(crate) mode: Mode,
}

fn help() {
    println!(
        "\
TYUT 校园网自动登录 (Rust)
用法: tyut-keepalive [选项]

  （无参数）      首次安装或启动后台保活 + 托盘界面
  --service       后台模式：管理进程 + 托盘（Windows）
  --worker        保活工作进程（由管理进程或 systemd 启动）
  --setup         仅重新输入账号密码
  --status        查看状态与最近日志
  --reset         删除本地保存的凭证
  --enable-autostart / --disable-autostart  启用/禁用登录自启
  --once          只检查并尝试重连一次
  --config PATH   指定 config.enc（兼容原 Python 版）
  --mac HEX       手动指定 12 位十六进制 MAC
  --help          显示帮助

默认配置与日志位于系统用户目录下的 tyut-keepalive 文件夹。"
    );
}

pub(crate) fn parse() -> Result<Option<Args>> {
    let mut args = Args {
        config: paths::install_paths()?.config,
        once: false,
        reset: false,
        mac_override: None,
        mode: Mode::Bootstrap,
    };
    let mut mode: Option<Mode> = None;

    let mut items = env::args().skip(1);
    while let Some(arg) = items.next() {
        match arg.as_str() {
            "--once" => {
                args.once = true;
                mode = Some(Mode::Once);
            }
            "--reset" => {
                args.reset = true;
                mode = Some(Mode::Reset);
            }
            "--service" => mode = Some(Mode::Service),
            "--worker" => mode = Some(Mode::Worker),
            "--setup" => mode = Some(Mode::Setup),
            "--status" => mode = Some(Mode::Status),
            "--enable-autostart" => mode = Some(Mode::EnableAutostart),
            "--disable-autostart" => mode = Some(Mode::DisableAutostart),
            "--config" => {
                args.config = PathBuf::from(items.next().context("--config 缺少路径")?);
            }
            "--mac" => {
                let mac = items.next().context("--mac 缺少地址")?;
                let normalized: String =
                    mac.chars().filter(|ch| *ch != ':' && *ch != '-').collect();
                let normalized = normalized.to_ascii_lowercase();
                if normalized.len() != 12 || !normalized.chars().all(|ch| ch.is_ascii_hexdigit()) {
                    bail!("--mac 应为12位十六进制 MAC 地址");
                }
                args.mac_override = Some(normalized);
            }
            "--help" | "-h" => {
                help();
                return Ok(None);
            }
            other => bail!("不支持的参数：{other}（使用 --help）"),
        }
    }
    if let Some(selected) = mode {
        args.mode = selected;
    }
    Ok(Some(args))
}
