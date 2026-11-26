use anyhow::{bail, Context, Result};
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
};

use crate::log;

const XOR_KEY: u32 = 0x77;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Credentials {
    pub(crate) account: String,
    pub(crate) password: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) saved_at: Option<String>,
}

/// 与 Python 版逐字符 XOR 一致；这是协议混淆，不是加密。
pub(crate) fn xor_encode(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{:02x}", (ch as u32) ^ XOR_KEY);
    }
    out
}

pub(crate) fn read_credentials(path: &Path) -> Result<Option<Credentials>> {
    if !path.exists() {
        return Ok(None);
    }
    let data = fs::read_to_string(path).with_context(|| format!("读取 {:?} 失败", path))?;
    let credentials: Credentials = serde_json::from_str(&data).context("config.enc 格式有误")?;
    if credentials.account.is_empty() || credentials.password.is_empty() {
        bail!("config.enc 中的账号或密码为空");
    }
    Ok(Some(credentials))
}

pub(crate) fn save_credentials(path: &Path, credentials: &Credentials) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut options = OpenOptions::new();
    options.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        options.mode(0o600);
        if path.exists() {
            fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        }
    }
    let mut file = options.open(path)?;
    serde_json::to_writer_pretty(&mut file, credentials)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}

pub(crate) fn prompt_credentials(path: &Path) -> Result<Credentials> {
    println!("首次运行，请输入 TYUT 校园网账号和密码");
    println!("注意：config.enc 仅使用兼容原项目的 XOR 混淆，不是真正的加密。");
    print!("账号: ");
    io::stdout().flush()?;
    let mut account = String::new();
    io::stdin().read_line(&mut account)?;
    let password = rpassword::prompt_password("密码: ")?;
    let account = account.trim();
    let password = password.trim();
    if account.is_empty() || password.is_empty() {
        bail!("账号和密码不能为空");
    }
    let credentials = Credentials {
        account: xor_encode(account),
        password: xor_encode(password),
        saved_at: Some(Local::now().format("%Y-%m-%d %H:%M:%S").to_string()),
    };
    save_credentials(path, &credentials)?;
    log(format!("凭证已写入 {}", path.display()));
    Ok(credentials)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xor_matches_python_protocol() {
        assert_eq!(xor_encode("dr1003"), "130546474744");
        assert_eq!(xor_encode("1"), "46");
        assert_eq!(xor_encode("中"), format!("{:02x}", ('中' as u32) ^ 0x77));
    }
}
