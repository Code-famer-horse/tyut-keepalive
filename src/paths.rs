use anyhow::{Context, Result};
use std::{env, fs, path::PathBuf};

#[derive(Clone)]
pub(crate) struct InstallPaths {
    pub(crate) root: PathBuf,
    pub(crate) exe: PathBuf,
    pub(crate) config: PathBuf,
    pub(crate) log: PathBuf,
    pub(crate) status: PathBuf,
    pub(crate) control: PathBuf,
}

impl InstallPaths {
    pub(crate) fn control_file(&self, name: &str) -> PathBuf {
        self.control.join(name)
    }

    pub(crate) fn ensure(&self) -> Result<()> {
        fs::create_dir_all(&self.control)?;
        Ok(())
    }
}

fn home_dir() -> Result<PathBuf> {
    let key = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    env::var_os(key)
        .map(PathBuf::from)
        .with_context(|| format!("无法获取 {key} 目录"))
}

pub(crate) fn install_paths() -> Result<InstallPaths> {
    let root = if cfg!(windows) {
        env::var_os("LOCALAPPDATA")
            .or_else(|| env::var_os("APPDATA"))
            .map(PathBuf::from)
            .unwrap_or_else(|| home_dir().unwrap_or_default().join("AppData").join("Local"))
            .join("tyut-keepalive")
    } else {
        env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home_dir().unwrap_or_default().join(".local").join("share"))
            .join("tyut-keepalive")
    };

    let exe_name = if cfg!(windows) {
        "tyut-keepalive.exe"
    } else {
        "tyut-keepalive"
    };

    Ok(InstallPaths {
        exe: root.join(exe_name),
        config: root.join("config.enc"),
        log: root.join("tyut-keepalive.log"),
        status: root.join("status.txt"),
        control: root.join("control"),
        root,
    })
}
