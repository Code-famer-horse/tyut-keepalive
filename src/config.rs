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

// [设计意图/根因] -> 保持与原 ePortal 协议及历史 Python 版逐字节兼容
// [理论先验/数学假设] -> 对每个字符码点异或固定密钥 0x77，十六进制定长两位
// [机制闭环解释] -> 编码值可直接作为登录参数发送；这是协议混淆而非加密，不得作为隐私保护
pub(crate) fn xor_encode(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{:02x}", (ch as u32) ^ XOR_KEY);
    }
    out
}

pub(crate) fn xor_decode(encoded: &str) -> Option<String> {
    let mut out = String::new();
    let bytes: Vec<char> = encoded.chars().collect();
    if bytes.len() % 2 != 0 {
        return None;
    }
    for pair in bytes.chunks(2) {
        let hex: String = pair.iter().collect();
        let byte = u32::from_str_radix(&hex, 16).ok()?;
        out.push(char::from_u32(byte ^ XOR_KEY)?);
    }
    Some(out)
}

pub(crate) fn masked_account(encoded: &str) -> String {
    let Some(account) = xor_decode(encoded) else {
        return "未知".to_string();
    };
    let chars: Vec<char> = account.chars().collect();
    if chars.len() <= 4 {
        return "****".to_string();
    }
    let suffix: String = chars[chars.len() - 2..].iter().collect();
    format!("{}****{}", chars[0], suffix)
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

    #[test]
    fn xor_round_trip_and_mask() {
        let encoded = xor_encode("dr1003");
        assert_eq!(xor_decode(&encoded).as_deref(), Some("dr1003"));
        assert_eq!(masked_account(&encoded), "d****03");
    }
}
