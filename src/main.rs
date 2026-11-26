mod args;
mod bootstrap;
mod config;
mod dns;
mod install;
mod logging;
mod paths;
mod portal;
mod service;
mod state;
mod watch;

pub(crate) use logging::log;

pub(crate) const PORTAL_URL: &str = "http://219.226.127.250";
pub(crate) const LOGIN_HOST: &str = "https://drcom.tyut.edu.cn:804";
pub(crate) const LOGIN_DOMAIN: &str = "drcom.tyut.edu.cn";
pub(crate) const TARGET: &str = "223.5.5.5";
pub(crate) const REFERER_URL: &str = "https://drcom.tyut.edu.cn:804/eportal/portal/index";
pub(crate) const USER_AGENT_VALUE: &str = "Mozilla/5.0 Chrome/146.0.0.0 Edg/146.0.0.0";
pub(crate) const LOGIN_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36 Edg/146.0.0.0";

use anyhow::Result;
use args::Mode;

fn main() {
    let result = run();
    if let Err(error) = result {
        eprintln!("错误：{error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let Some(parsed) = args::parse()? else {
        return Ok(());
    };
    let paths = paths::install_paths()?;
    paths.ensure()?;
    logging::init(paths.log.clone());

    match parsed.mode {
        Mode::Bootstrap => bootstrap::run(&paths),
        Mode::Service => service::run(&paths),
        Mode::Worker => watch::run_worker(&paths),
        Mode::Setup => {
            config::prompt_credentials(&paths.config)?;
            Ok(())
        }
        Mode::Status => {
            println!("{}", state::read(&paths.status));
            println!("--- 最近日志 ---");
            println!("{}", logging::tail(30));
            Ok(())
        }
        Mode::Once => watch::run_once(&parsed),
        Mode::Reset => {
            if paths.config.exists() {
                std::fs::remove_file(&paths.config)?;
                log(format!("已删除凭证：{}", paths.config.display()));
            } else {
                log("凭证文件不存在");
            }
            Ok(())
        }
        Mode::EnableAutostart => install::register_autostart(&paths),
        Mode::DisableAutostart => install::unregister_autostart(&paths),
    }
}
