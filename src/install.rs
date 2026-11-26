use anyhow::{bail, Context, Result};
#[cfg(not(windows))]
use std::path::Path;
use std::{env, fs, process::Command};

use crate::{log, paths::InstallPaths};

const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_NAME: &str = "tyut-keepalive";
#[cfg(not(windows))]
const SERVICE_NAME: &str = "tyut-keepalive.service";

pub(crate) fn is_installed(paths: &InstallPaths) -> bool {
    paths.exe.exists() && autostart_enabled(paths)
}

fn pid_path(paths: &InstallPaths) -> std::path::PathBuf {
    paths.root.join("service.pid")
}

pub(crate) fn mark_service_started(paths: &InstallPaths) {
    let _ = fs::write(pid_path(paths), std::process::id().to_string());
}

pub(crate) fn mark_service_stopped(paths: &InstallPaths) {
    let _ = fs::remove_file(pid_path(paths));
}

pub(crate) fn is_service_running(paths: &InstallPaths) -> bool {
    let Ok(text) = fs::read_to_string(pid_path(paths)) else {
        return false;
    };
    let Ok(pid) = text.trim().parse::<u32>() else {
        return false;
    };
    #[cfg(windows)]
    {
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .map(|out| {
                String::from_utf8_lossy(&out.stdout).contains(&format!(" {pid} "))
                    || String::from_utf8_lossy(&out.stdout).contains(&format!("{pid}"))
            })
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        std::path::Path::new(&format!("/proc/{pid}")).exists()
    }
}

pub(crate) fn migrate_config(paths: &InstallPaths) -> Result<()> {
    let Ok(current) = env::current_exe() else {
        return Ok(());
    };
    let Some(source) = current.parent().map(|dir| dir.join("config.enc")) else {
        return Ok(());
    };
    if source != paths.config && source.exists() && !paths.config.exists() {
        fs::copy(&source, &paths.config)?;
        log(format!("已迁移旧凭证：{}", paths.config.display()));
    }
    Ok(())
}

pub(crate) fn copy_executable(paths: &InstallPaths) -> Result<()> {
    let current = env::current_exe().context("无法获取当前可执行文件路径")?;
    fs::create_dir_all(&paths.root)?;
    if current == paths.exe {
        return Ok(());
    }
    fs::copy(&current, &paths.exe)
        .with_context(|| format!("复制可执行文件到 {} 失败", paths.exe.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&paths.exe, fs::Permissions::from_mode(0o755))?;
    }
    log(format!("已安装到 {}", paths.exe.display()));
    Ok(())
}

pub(crate) fn register_autostart(paths: &InstallPaths) -> Result<()> {
    #[cfg(windows)]
    {
        // [设计意图/根因] -> 计划任务 OnLogon 触发器要求管理员令牌
        // [理论先验/数学假设] -> HKCU Run 是用户可写项，登录时由 Explorer 执行，无需提权
        // [机制闭环解释] -> 安装阶段写入该键即完成登录自启，禁用时删除同名值即可逆转
        let value = format!("\"{}\" --service", paths.exe.display());
        let status = Command::new("reg")
            .args([
                "add", RUN_KEY, "/v", RUN_NAME, "/t", "REG_SZ", "/d", &value, "/f",
            ])
            .status()
            .context("无法执行 reg add")?;
        if !status.success() {
            bail!("写入 HKCU Run 自启项失败");
        }
        log("已注册登录后自启（HKCU Run）");
    }
    #[cfg(not(windows))]
    {
        let unit_dir = systemd_dir()?;
        fs::create_dir_all(&unit_dir)?;
        let unit = format!(
            "[Unit]\nDescription=TYUT Keepalive\nAfter=network-online.target\nWants=network-online.target\nStartLimitIntervalSec=0\n\n[Service]\nType=simple\nExecStart=\"{}\" --worker --config \"{}\"\nRestart=always\nRestartSec=5\n\n[Install]\nWantedBy=default.target\n",
            paths.exe.display(),
            paths.config.display()
        );
        let unit_path = unit_dir.join(SERVICE_NAME);
        fs::write(&unit_path, unit)?;
        let reload = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status();
        let enable = Command::new("systemctl")
            .args(["--user", "enable", "--now", SERVICE_NAME])
            .status();
        match (reload, enable) {
            (Ok(r), Ok(e)) if r.success() && e.success() => log("已注册 systemd --user 自启并启动"),
            _ => log("systemd --user 不可用；已写入服务单元，请手动启用或以后台模式运行"),
        }
    }
    Ok(())
}

pub(crate) fn unregister_autostart(paths: &InstallPaths) -> Result<()> {
    #[cfg(windows)]
    {
        if autostart_enabled(paths) {
            let status = Command::new("reg")
                .args(["delete", RUN_KEY, "/v", RUN_NAME, "/f"])
                .status()
                .context("无法执行 reg delete")?;
            if !status.success() {
                bail!("删除 HKCU Run 自启项失败");
            }
        }
        log("已禁用登录后自启");
    }
    #[cfg(not(windows))]
    {
        let status = Command::new("systemctl")
            .args(["--user", "disable", "--now", SERVICE_NAME])
            .status();
        if let Ok(s) = status {
            let _ = s;
        }
        let _ = fs::remove_file(systemd_dir()?.join(SERVICE_NAME));
        let _ = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status();
        log("已禁用 systemd --user 自启");
    }
    Ok(())
}

pub(crate) fn autostart_enabled(paths: &InstallPaths) -> bool {
    #[cfg(windows)]
    {
        let _ = paths;
        Command::new("reg")
            .args(["query", RUN_KEY, "/v", RUN_NAME])
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        let _ = paths;
        systemd_dir()
            .map(|dir| dir.join(SERVICE_NAME).exists())
            .unwrap_or(false)
    }
}

pub(crate) fn launch_service(paths: &InstallPaths) -> Result<()> {
    let mut command = Command::new(&paths.exe);
    command.arg("--service");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    command
        .spawn()
        .with_context(|| format!("启动 {} 失败", paths.exe.display()))?;
    Ok(())
}

#[cfg(not(windows))]
fn systemd_dir() -> Result<std::path::PathBuf> {
    if let Ok(base) = env::var("XDG_CONFIG_HOME") {
        return Ok(Path::new(&base).join("systemd").join("user"));
    }
    let home = env::var("HOME").context("无法获取 HOME 目录")?;
    Ok(Path::new(&home)
        .join(".config")
        .join("systemd")
        .join("user"))
}

#[cfg(windows)]
pub(crate) fn hide_console() {
    unsafe extern "system" {
        fn FreeConsole() -> i32;
    }
    unsafe {
        let _ = FreeConsole();
    }
}
