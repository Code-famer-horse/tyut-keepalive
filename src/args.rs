use anyhow::{bail, Context, Result};
use std::{env, path::PathBuf};

pub(crate) struct Args {
    pub(crate) config: PathBuf,
    pub(crate) once: bool,
    pub(crate) reset: bool,
    pub(crate) mac_override: Option<String>,
}

fn help() {
    println!(
        "\
TYUT 校园网自动登录 (Rust)
用法: tyut-keepalive [--once] [--reset] [--config PATH] [--mac 12位十六进制]

  --once          只检查并尝试重连一次
  --reset         删除本地保存的凭证
  --config PATH   指定 config.enc，兼容原 Python 版
  --mac HEX       手动指定网卡 MAC（多网卡/VPN 环境可用）
  --help          显示帮助

默认 config.enc 位于可执行文件的同一目录。"
    );
}

pub(crate) fn parse() -> Result<Option<Args>> {
    let default_config = env::current_exe()?
        .parent()
        .context("无法找到可执行文件所在目录")?
        .join("config.enc");
    let mut args = Args {
        config: default_config,
        once: false,
        reset: false,
        mac_override: None,
    };
    let mut items = env::args().skip(1);
    while let Some(arg) = items.next() {
        match arg.as_str() {
            "--once" => args.once = true,
            "--reset" => args.reset = true,
            "--config" => {
                args.config = PathBuf::from(items.next().context("--config 缺少路径")?);
            }
            "--mac" => {
                let mac = items.next().context("--mac 缺少地址")?;
                let normalized: String = mac.chars().filter(|ch| *ch != ':' && *ch != '-').collect();
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
    Ok(Some(args))
}
