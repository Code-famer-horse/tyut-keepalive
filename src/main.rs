mod args;
mod config;
mod dns;
mod portal;
mod watch;

use chrono::Local;

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
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        message.as_ref()
    );
}

fn main() {
    let result = args::parse().and_then(|parsed| match parsed {
        Some(parsed) => watch::run(parsed),
        None => Ok(()),
    });
    if let Err(error) = result {
        eprintln!("错误：{error:#}");
        std::process::exit(1);
    }
}
