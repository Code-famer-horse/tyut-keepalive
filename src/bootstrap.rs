use anyhow::Result;

use crate::{config, install, log, paths::InstallPaths};

pub(crate) fn run(paths: &InstallPaths) -> Result<()> {
    install::migrate_config(paths)?;

    if install::is_installed(paths) {
        if install::is_service_running(paths) {
            println!("后台保活已在运行，可通过托盘图标打开界面。");
            return Ok(());
        }
        install::launch_service(paths)?;
        log("后台保活已启动");
        return Ok(());
    }

    if config::read_credentials(&paths.config)?.is_none() {
        config::prompt_credentials(&paths.config)?;
    }
    install::copy_executable(paths)?;
    install::register_autostart(paths)?;
    install::launch_service(paths)?;
    println!("安装完成：后台保活已启动，托盘图标可打开界面或退出。");
    Ok(())
}
