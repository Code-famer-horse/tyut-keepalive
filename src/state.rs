use anyhow::Result;
use std::{fs, path::Path};

#[derive(Clone, Debug, Default)]
pub(crate) struct Status {
    pub(crate) state: String,
    pub(crate) account: String,
    pub(crate) last_check: String,
    pub(crate) last_login: String,
    pub(crate) message: String,
}

impl Status {
    pub(crate) fn to_text(&self) -> String {
        format!(
            "状态：{}\n账号：{}\n最近检查：{}\n最近登录：{}\n说明：{}\n",
            self.state, self.account, self.last_check, self.last_login, self.message
        )
    }
}

pub(crate) fn write(path: &Path, status: &Status) -> Result<()> {
    let temp = path.with_extension("tmp");
    fs::write(&temp, status.to_text())?;
    fs::rename(temp, path)?;
    Ok(())
}

pub(crate) fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|_| "状态：尚未启动\n".to_string())
}
