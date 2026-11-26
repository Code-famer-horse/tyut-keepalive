# TYUT Keepalive Rust 迁移实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 tyut-keepalive 从 Python 实现迁移为 Rust 实现，归档 Python 文件，保留 psmux/tmux 会话启动方式。

**Architecture:** 以 zip 中的 Rust 核心为基线，按职责拆分为 args/config/dns/portal/watch 五个模块加 main.rs；启动脚本原位改为调用 Rust 可执行文件；协议参数、XOR 编码、重试常数均保持不变。

**Tech Stack:** Rust 1.100、Cargo、reqwest 0.13 (rustls)、serde、netdev、rpassword、psmux、tmux、PowerShell、Bash。

用户已明确不执行 `git commit`，因此本计划不含提交步骤。

---

### Task 1: 归档 Python 文件并更新忽略规则

**Files:**
- Move: `AutoNet.py`、`pyproject.toml`、`uv.lock`、`.python-version`、`README.md` → `archive/python/`
- Move: `tests/test_autostart.py`、`tests/test_linux_support.py` → `archive/python/tests/`
- Modify: `.gitignore`

- [ ] **Step 1: 建立归档目录并移动文件**

Run:
```sh
mkdir -p archive/python/tests
mv AutoNet.py pyproject.toml uv.lock .python-version README.md archive/python/
mv tests/test_autostart.py tests/test_linux_support.py archive/python/tests/
rmdir tests
```
Expected: 根目录不再有 Python 源码、测试、uv 配置。

- [ ] **Step 2: 写入合并后的忽略规则**

Create `.gitignore` with exactly:

```gitignore
.venv/
__pycache__/
*.py[cod]
.env
.env.*
config.enc
debug/
logs/
*.log
*.pid
*.bak
*.pre-*
校园网自动登陆.zip

/target/
```

- [ ] **Step 3: 核对归档结果**

Run: `find archive/python -type f | sort`
Expected: 7 个文件，与 Step 1 移动列表一致。

---

### Task 2: Cargo 清单与程序入口

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`

- [ ] **Step 1: 写入 Cargo.toml**

```toml
[package]
name = "tyut-keepalive-rs"
version = "0.1.0"
edition = "2021"
description = "TYUT campus network auto-login, implemented in Rust"

[dependencies]
anyhow = "1"
base64 = "0.22"
chrono = { version = "0.4", default-features = false, features = ["clock"] }
netdev = { version = "0.47", default-features = false }
rand = "0.9"
regex = "1"
reqwest = { version = "0.13", default-features = false, features = ["blocking", "cookies", "rustls"] }
rpassword = "7"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 2: 写入 src/main.rs**

```rust
mod args;
mod config;
mod dns;
mod portal;
mod watch;

use anyhow::Result;
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
```

---

### Task 3: 命令行参数模块

**Files:**
- Create: `src/args.rs`

- [ ] **Step 1: 写入完整实现**

```rust
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
用法: tyut-keepalive-rs [--once] [--reset] [--config PATH] [--mac 12位十六进制]

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
```

---

### Task 4: 凭证模块与 XOR 兼容测试

**Files:**
- Create: `src/config.rs`

- [ ] **Step 1: 写入完整实现与单元测试**

```rust
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
        assert_eq!(xor_encode("dr1003"), "130545464744");
        assert_eq!(xor_encode("1"), "46");
        assert_eq!(xor_encode("中"), format!("{:02x}", ('中' as u32) ^ 0x77));
    }
}
```

---

### Task 5: 校园 DNS 模块与解析测试

**Files:**
- Create: `src/dns.rs`

- [ ] **Step 1: 写入完整实现与单元测试**

```rust
use anyhow::{bail, Context, Result};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    time::Duration,
};

use crate::{log, LOGIN_DOMAIN};

const CAMPUS_DNS: [&str; 2] = ["202.207.240.225", "202.207.247.227"];

fn get_u16(data: &[u8], offset: usize) -> Result<u16> {
    let bytes = data.get(offset..offset + 2).context("DNS 数据被截断")?;
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn skip_dns_name(data: &[u8], mut offset: usize) -> Result<usize> {
    loop {
        let len = *data.get(offset).context("DNS 名称被截断")?;
        if len == 0 {
            return Ok(offset + 1);
        }
        if len & 0xc0 == 0xc0 {
            data.get(offset + 1).context("DNS 指针被截断")?;
            return Ok(offset + 2);
        }
        if len & 0xc0 != 0 {
            bail!("DNS 标签长度非法");
        }
        offset += 1 + len as usize;
        if offset > data.len() {
            bail!("DNS 标签超出数据范围");
        }
    }
}

fn parse_dns_response(data: &[u8], expected_id: u16) -> Result<Vec<Ipv4Addr>> {
    if data.len() < 12 || get_u16(data, 0)? != expected_id {
        bail!("DNS 响应 ID 不匹配或长度不足");
    }
    let flags = get_u16(data, 2)?;
    if flags & 0x8000 == 0 || flags & 0x000f != 0 || flags & 0x0200 != 0 {
        bail!("DNS 响应标志错误或被截断");
    }
    let mut offset = 12;
    for _ in 0..get_u16(data, 4)? {
        offset = skip_dns_name(data, offset)?;
        data.get(offset..offset + 4).context("DNS 问题不完整")?;
        offset += 4;
    }
    let mut addresses = Vec::new();
    for _ in 0..get_u16(data, 6)? {
        offset = skip_dns_name(data, offset)?;
        data.get(offset..offset + 10).context("DNS 记录头不完整")?;
        let record_type = get_u16(data, offset)?;
        let record_class = get_u16(data, offset + 2)?;
        let data_length = get_u16(data, offset + 8)? as usize;
        offset += 10;
        let raw = data
            .get(offset..offset + data_length)
            .context("DNS 记录值不完整")?;
        if record_type == 1 && record_class == 1 && data_length == 4 {
            let addr = Ipv4Addr::new(raw[0], raw[1], raw[2], raw[3]);
            if !addresses.contains(&addr) {
                addresses.push(addr);
            }
        }
        offset += data_length;
    }
    if addresses.is_empty() {
        bail!("校园 DNS 没有返回 IPv4 A 记录");
    }
    Ok(addresses)
}

fn query_campus_dns(server: &str, host: &str) -> Result<Vec<Ipv4Addr>> {
    let id: u16 = rand::random();
    let mut packet = Vec::new();
    packet.extend_from_slice(&id.to_be_bytes());
    packet.extend_from_slice(&0x0100_u16.to_be_bytes());
    packet.extend_from_slice(&1_u16.to_be_bytes());
    packet.extend_from_slice(&[0_u8; 6]);
    for part in host.split('.') {
        if part.is_empty() || part.len() > 63 || !part.is_ascii() {
            bail!("DNS 查询域名无效");
        }
        packet.push(part.len() as u8);
        packet.extend_from_slice(part.as_bytes());
    }
    packet.extend_from_slice(&[0, 0, 1, 0, 1]);
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(Duration::from_secs(2)))?;
    socket.set_write_timeout(Some(Duration::from_secs(2)))?;
    socket.connect(format!("{server}:53"))?;
    socket.send(&packet)?;
    let mut buf = [0_u8; 4096];
    let len = socket.recv(&mut buf)?;
    parse_dns_response(&buf[..len], id)
}

pub(crate) fn campus_host_ips() -> Result<Vec<Ipv4Addr>> {
    let mut errors = Vec::new();
    for server in CAMPUS_DNS {
        match query_campus_dns(server, LOGIN_DOMAIN) {
            Ok(addresses) => {
                log(format!("校园 DNS {}: {} -> {:?}", server, LOGIN_DOMAIN, addresses));
                return Ok(addresses);
            }
            Err(error) => errors.push(format!("{server}: {error:#}")),
        }
    }
    bail!("校园 DNS 解析失败：{}", errors.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_dns_a_record() {
        let mut dns = vec![0x12, 0x34, 0x81, 0x80, 0, 1, 0, 1, 0, 0, 0, 0];
        dns.extend_from_slice(&[3, b'f', b'o', b'o', 0, 0, 1, 0, 1]);
        dns.extend_from_slice(&[0xc0, 0x0c, 0, 1, 0, 1, 0, 0, 0, 30, 0, 4, 1, 2, 3, 4]);
        assert_eq!(
            parse_dns_response(&dns, 0x1234).unwrap(),
            vec![Ipv4Addr::new(1, 2, 3, 4)]
        );
    }

    #[test]
    fn reject_wrong_dns_id() {
        let dns = [0u8; 12];
        assert!(parse_dns_response(&dns, 0x1234).is_err());
    }
}
```

---

### Task 6: Portal 协议模块与登录判定测试

**Files:**
- Create: `src/portal.rs`

- [ ] **Step 1: 写入完整实现与单元测试**

```rust
use anyhow::{bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use regex::Regex;
use reqwest::{
    blocking::Client,
    header::{HeaderMap, HeaderValue, ACCEPT, LOCATION, REFERER, USER_AGENT},
    redirect::Policy,
    Url,
};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::{
    config::{xor_encode, Credentials},
    dns::campus_host_ips,
    log, LOGIN_DOMAIN, LOGIN_HOST, LOGIN_USER_AGENT, PORTAL_URL, REFERER_URL, TARGET,
    USER_AGENT_VALUE,
};

fn local_ip() -> Result<Ipv4Addr> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect(format!("{TARGET}:80"))?;
    match socket.local_addr()?.ip() {
        IpAddr::V4(ip) => Ok(ip),
        _ => bail!("无法获取 IPv4 本地地址"),
    }
}

fn local_mac(ip: Ipv4Addr) -> Result<String> {
    for interface in netdev::get_interfaces() {
        if interface.ipv4_addrs().contains(&ip) {
            if let Some(mac) = interface.mac_addr {
                return Ok(mac.to_string().replace(':', "").to_ascii_lowercase());
            }
        }
    }
    bail!("未找到 {ip} 对应网卡的 MAC；可通过 --mac 手动指定")
}

pub(crate) fn client(campus_ips: Option<Vec<Ipv4Addr>>) -> Result<Client> {
    let mut builder = Client::builder()
        .no_proxy()
        .cookie_store(true)
        .redirect(Policy::none())
        .timeout(Duration::from_secs(15));
    if let Some(ips) = campus_ips {
        let addrs: Vec<_> = ips
            .into_iter()
            .map(|ip| SocketAddr::new(IpAddr::V4(ip), 804))
            .collect();
        builder = builder.resolve_to_addrs(LOGIN_DOMAIN, &addrs);
    }
    Ok(builder.build()?)
}

pub(crate) fn check_network(client: &Client) -> bool {
    if let Ok(response) = client
        .get(format!("http://{TARGET}"))
        .timeout(Duration::from_secs(3))
        .send()
    {
        if response.status().as_u16() == 200 {
            return true;
        }
    }
    if let Ok(response) = client
        .get(PORTAL_URL)
        .timeout(Duration::from_secs(5))
        .send()
    {
        if matches!(response.status().as_u16(), 301 | 302) {
            return true;
        }
        if response.status().is_success() {
            if let Ok(body) = response.text() {
                return !body.contains("v46ip") && !body.contains("wlan_user_ip");
            }
        }
    }
    false
}

fn portal_ip(client: &Client) -> Result<Option<Ipv4Addr>> {
    log(format!("访问 Portal：{PORTAL_URL}"));
    let mut url = Url::parse(PORTAL_URL)?;
    let mut response = client.get(url.clone()).timeout(Duration::from_secs(10)).send()?;
    for _ in 0..10 {
        if !response.status().is_redirection() {
            break;
        }
        let location = match response.headers().get(LOCATION) {
            Some(header) => header.to_str()?.to_owned(),
            None => break,
        };
        url = url.join(&location)?;
        log(format!("Portal 重定向到：{url}"));
        response = client.get(url.clone()).timeout(Duration::from_secs(10)).send()?;
    }
    if response.status().is_redirection() && response.headers().contains_key(LOCATION) {
        bail!("Portal 重定向次数过多");
    }
    if !response.status().is_success() {
        bail!("Portal HTTP 状态异常：{}", response.status());
    }
    let html = response.text()?;
    for pattern in [
        r#"v46ip\s*[:=]\s*["']([^"']+)"#,
        r#"wlan_user_ip["']?\s*[:=]\s*["']([^"']+)"#,
    ] {
        let re = Regex::new(pattern)?;
        if let Some(ip) = re
            .captures(&html)
            .and_then(|capture| capture.get(1))
            .and_then(|matched| matched.as_str().parse::<Ipv4Addr>().ok())
        {
            log(format!("Portal 提取到 IP：{ip}"));
            return Ok(Some(ip));
        }
    }
    Ok(None)
}

fn protocol_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
    headers.insert(REFERER, HeaderValue::from_static(REFERER_URL));
    headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_VALUE));
    headers
}

fn timestamp_param() -> Result<String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    Ok(((now % 10_000) + 500).to_string())
}

fn load_config(client: &Client, ip: Ipv4Addr, mac: &str) -> Result<()> {
    let ip_encoded = xor_encode(&STANDARD.encode(ip.to_string()));
    let mac_encoded = xor_encode(mac);
    let params = vec![
        ("callback", xor_encode("dr1003")),
        ("program_index", String::new()),
        ("wlan_vlan_id", xor_encode("46")),
        ("wlan_user_ip", ip_encoded),
        ("wlan_user_ipv6", String::new()),
        ("wlan_user_ssid", String::new()),
        ("wlan_user_areaid", String::new()),
        ("wlan_ac_ip", String::new()),
        ("wlan_ap_mac", mac_encoded.clone()),
        ("gw_id", mac_encoded),
        ("jsVersion", xor_encode("4.X")),
        ("encrypt", "1".to_owned()),
        ("v", timestamp_param()?),
        ("lang", "zh".to_owned()),
    ];
    log("请求 loadConfig ...");
    let response = client
        .get(format!("{LOGIN_HOST}/eportal/portal/page/loadConfig"))
        .headers(protocol_headers())
        .query(&params)
        .timeout(Duration::from_secs(15))
        .send()?;
    let status = response.status();
    if !status.is_success() {
        bail!("loadConfig 返回 HTTP {status}");
    }
    let _body = response.text()?;
    log("loadConfig 请求完成");
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum LoginOutcome {
    Success,
    BadPassword,
    Indeterminate,
}

pub(crate) fn interpret_login(text: &str) -> LoginOutcome {
    let compact: String = text.chars().filter(|ch| !ch.is_ascii_whitespace()).collect();
    if (text.contains("认证成功") && compact.contains("\"result\":1")) || text.contains("已在线") {
        LoginOutcome::Success
    } else if text.contains("密码错误") {
        LoginOutcome::BadPassword
    } else {
        LoginOutcome::Indeterminate
    }
}

fn login(client: &Client, ip: Ipv4Addr, credentials: &Credentials) -> Result<LoginOutcome> {
    let params = vec![
        ("callback", xor_encode("dr1004")),
        ("login_method", xor_encode("1")),
        ("user_account", credentials.account.clone()),
        ("user_password", credentials.password.clone()),
        ("wlan_user_ip", xor_encode(&ip.to_string())),
        ("wlan_user_ipv6", String::new()),
        ("wlan_user_mac", xor_encode("000000000000")),
        ("wlan_ac_ip", String::new()),
        ("wlan_ac_name", String::new()),
        ("mac_type", xor_encode("0")),
        ("authex_enable", String::new()),
        ("jsVersion", xor_encode("4.3")),
        ("web", xor_encode("0")),
        ("terminal_type", xor_encode("1")),
        ("lang", xor_encode("zh-cn")),
        ("user_agent", xor_encode(LOGIN_USER_AGENT)),
        ("enable_r3", xor_encode("0")),
        ("v", timestamp_param()?),
        ("encrypt", "1".to_owned()),
    ];
    log("请求 login ...");
    let response = client
        .get(format!("{LOGIN_HOST}/eportal/portal/login"))
        .headers(protocol_headers())
        .query(&params)
        .timeout(Duration::from_secs(15))
        .send()?;
    let status = response.status();
    let body = response.text()?;
    if !status.is_success() {
        bail!("登录接口 HTTP 状态异常：{status}");
    }
    let outcome = interpret_login(&body);
    match outcome {
        LoginOutcome::Success => log("登录接口确认成功或已在线"),
        LoginOutcome::BadPassword => log("登录失败：密码错误"),
        LoginOutcome::Indeterminate => log("登录响应无法确认结果；将检测网络连通性"),
    }
    Ok(outcome)
}

pub(crate) fn attempt_login(credentials: &Credentials, mac_override: Option<&str>) -> Result<bool> {
    let ip = local_ip()?;
    let mac = match mac_override {
        Some(mac) => mac.to_owned(),
        None => local_mac(ip)?,
    };
    log(format!("本机 IP：{ip}，MAC：{mac}"));
    let resolved_ips = match campus_host_ips() {
        Ok(ips) => Some(ips),
        Err(error) => {
            log(format!("校园 DNS 不可用，改用系统 DNS：{error:#}"));
            None
        }
    };
    let client = client(resolved_ips)?;
    let user_ip = portal_ip(&client)?.unwrap_or(ip);
    log(format!("最终使用 IP：{user_ip}"));
    load_config(&client, user_ip, &mac)?;
    thread::sleep(Duration::from_millis(500));
    let outcome = login(&client, user_ip, credentials)?;
    match outcome {
        LoginOutcome::Success => Ok(true),
        LoginOutcome::BadPassword => Ok(false),
        LoginOutcome::Indeterminate => Ok(check_network(&client)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_status_is_conservative_for_timeout() {
        assert_eq!(
            interpret_login(r#"dr1004({"result":1,"msg":"认证成功"})"#),
            LoginOutcome::Success
        );
        assert_eq!(interpret_login("已在线"), LoginOutcome::Success);
        assert_eq!(interpret_login("密码错误"), LoginOutcome::BadPassword);
        assert_eq!(interpret_login("认证超时"), LoginOutcome::Indeterminate);
    }
}
```

---

### Task 7: 监控循环模块

**Files:**
- Create: `src/watch.rs`

- [ ] **Step 1: 写入完整实现**

```rust
use anyhow::{bail, Result};
use std::{thread, time::Duration};

use crate::{
    args::Args,
    config, log,
    portal::{attempt_login, check_network, client},
};

const WATCH_INTERVAL: Duration = Duration::from_secs(600);
const INITIAL_RETRY: Duration = Duration::from_secs(10);
const MAX_RETRY: Duration = Duration::from_secs(3600);

pub(crate) fn run(args: Args) -> Result<()> {
    if args.reset {
        if args.config.exists() {
            std::fs::remove_file(&args.config)?;
            log(format!("已删除凭证：{}", args.config.display()));
        } else {
            log("凭证文件不存在");
        }
        return Ok(());
    }
    log("TYUT 校园网自动登录 (Rust)");
    let credentials = match config::read_credentials(&args.config)? {
        Some(credentials) => {
            log(format!("已读取凭证：{}", args.config.display()));
            credentials
        }
        None => config::prompt_credentials(&args.config)?,
    };
    let probe_client = client(None)?;
    let mut retry = INITIAL_RETRY;
    loop {
        if check_network(&probe_client) {
            retry = INITIAL_RETRY;
            if args.once {
                log("网络已连通，无需登录");
                return Ok(());
            }
            log("网络正常，10 分钟后复查");
            thread::sleep(WATCH_INTERVAL);
            continue;
        }
        log("网络不可用，再次确认...");
        thread::sleep(Duration::from_secs(1));
        if check_network(&probe_client) {
            continue;
        }
        thread::sleep(Duration::from_secs(2));
        if check_network(&probe_client) {
            continue;
        }
        log(format!("确认断网，{} 秒后尝试重连", retry.as_secs()));
        thread::sleep(retry);
        match attempt_login(&credentials, args.mac_override.as_deref()) {
            Ok(true) => {
                log("登录流程已完成");
                retry = INITIAL_RETRY;
                if args.once {
                    return Ok(());
                }
            }
            Ok(false) => {
                retry = (retry * 2).min(MAX_RETRY);
                log(format!("重连失败，下次重试等待 {} 秒", retry.as_secs()));
                if args.once {
                    bail!("重连失败");
                }
            }
            Err(error) => {
                log(format!("本次重连失败：{error:#}"));
                retry = (retry * 2).min(MAX_RETRY);
                if args.once {
                    return Err(error);
                }
                thread::sleep(Duration::from_secs(30));
            }
        }
    }
}
```

---

### Task 8: 启动与自启脚本

**Files:**
- Create/Modify: `Start-AutoNetPsmux.ps1`、`Register-AutoNetAutostart.ps1`、`scripts/register-windows-task.ps1`、`start-linux.sh`、`install-linux-autostart.sh`、`启动校园网登录.bat`

- [ ] **Step 1: 写入 Start-AutoNetPsmux.ps1**

```powershell
# Requires psmux.exe in PATH. Reuses upstream session `net` and window `AutoNet`.
$ErrorActionPreference = 'Stop'

$PsmuxCommand = Get-Command psmux.exe -CommandType Application -ErrorAction SilentlyContinue
if (-not $PsmuxCommand) { throw '找不到 psmux.exe，请先安装 psmux 并将其加入 PATH。' }
$PsmuxPath = $PsmuxCommand.Source

$WorkingDirectory = $PSScriptRoot
$Executable = Join-Path $WorkingDirectory 'target\release\tyut-keepalive-rs.exe'
$ConfigPath = Join-Path $WorkingDirectory 'config.enc'
foreach ($Path in @($Executable, $ConfigPath)) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "找不到文件：$Path。请先编译 Rust 并在前台保存凭证。"
    }
}

$SessionName = 'net'
$WindowName = 'AutoNet'
$SessionTarget = "${SessionName}:${WindowName}"

function Invoke-Psmux {
    param([string[]]$PsmuxArguments)
    & $PsmuxPath @PsmuxArguments
    if ($LASTEXITCODE -ne 0) {
        throw ("psmux 执行失败 (exit $LASTEXITCODE)：" + ($PsmuxArguments -join ' '))
    }
}

& $PsmuxPath has-session -t "=$SessionName" *> $null
$SessionExists = ($LASTEXITCODE -eq 0)
$BufferWindow = $null

if ($SessionExists) {
    $Windows = @(& $PsmuxPath list-windows -t $SessionName -F '#{window_name}')
    if ($LASTEXITCODE -ne 0) { throw "无法列出 psmux $SessionName 的窗口。" }

    if ($Windows -contains $WindowName) {
        $BufferWindow = "__net_restart_$PID"
        Invoke-Psmux -PsmuxArguments @('new-window', '-d', '-t', $SessionName, '-n', $BufferWindow)
        Invoke-Psmux -PsmuxArguments @('kill-window', '-t', $SessionTarget)
    }
    Invoke-Psmux -PsmuxArguments @(
        'new-window', '-d', '-t', $SessionName, '-n', $WindowName,
        '-c', $WorkingDirectory, '--', $Executable, '--config', $ConfigPath
    )
} else {
    Invoke-Psmux -PsmuxArguments @(
        'new-session', '-d', '-s', $SessionName, '-n', $WindowName,
        '-c', $WorkingDirectory, '--', $Executable, '--config', $ConfigPath
    )
}

Invoke-Psmux -PsmuxArguments @('set-window-option', '-t', $SessionTarget, 'remain-on-exit', 'on')
if ($BufferWindow) {
    Invoke-Psmux -PsmuxArguments @('kill-window', '-t', "${SessionName}:${BufferWindow}")
}
Invoke-Psmux -PsmuxArguments @('select-window', '-t', $SessionTarget)
Write-Output 'Rust AutoNet 已在 psmux net:AutoNet 窗口中启动/重启。使用 psmux attach -t net 查看。'
```

- [ ] **Step 2: 写入 Register-AutoNetAutostart.ps1**

```powershell
# Registers a logon task that starts/reuses the existing `net` psmux session.
# Will not overwrite a task from the original Python implementation.
$ErrorActionPreference = 'Stop'
$TaskName = 'AutoNet校园网自动登录'
$UserId = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name
$StartScript = Join-Path $PSScriptRoot 'Start-AutoNetPsmux.ps1'
$Executable = Join-Path $PSScriptRoot 'target\release\tyut-keepalive-rs.exe'
$ConfigPath = Join-Path $PSScriptRoot 'config.enc'

if (Get-ScheduledTask -TaskName $TaskName -TaskPath '\' -ErrorAction SilentlyContinue) {
    throw "任务 '$TaskName' 已存在，未覆盖；请先在任务计划程序中检查旧版 Python 任务。"
}
if (-not (Get-Command psmux.exe -CommandType Application -ErrorAction SilentlyContinue)) {
    throw '找不到 psmux.exe，请将其加入 PATH。'
}
foreach ($Path in @($StartScript, $Executable, $ConfigPath)) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "找不到必需文件：$Path。请先编译并在前台保存凭证。"
    }
}

$PowerShell = Join-Path $PSHOME 'powershell.exe'
if (-not (Test-Path -LiteralPath $PowerShell)) {
    $PowerShell = (Get-Command powershell.exe -CommandType Application -ErrorAction Stop).Source
}
$Action = New-ScheduledTaskAction -Execute $PowerShell -Argument "-NoProfile -ExecutionPolicy Bypass -File `"$StartScript`"" -WorkingDirectory $PSScriptRoot
$Trigger = New-ScheduledTaskTrigger -AtLogOn -User $UserId
$Principal = New-ScheduledTaskPrincipal -UserId $UserId -LogonType Interactive
$Settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -MultipleInstances IgnoreNew
Register-ScheduledTask -TaskName $TaskName -Action $Action -Trigger $Trigger -Principal $Principal -Settings $Settings | Out-Null
Write-Output "已配置登录后通过 psmux 启动 Rust AutoNet：$TaskName"
```

- [ ] **Step 3: 写入 scripts/register-windows-task.ps1**

```powershell
# Backwards-compatible entry point: the logon task now launches psmux, not the Rust EXE directly.
$ErrorActionPreference = 'Stop'
& (Join-Path $PSScriptRoot '..\Register-AutoNetAutostart.ps1')
if (-not $?) { throw '注册 psmux 自启动任务失败。' }
```

- [ ] **Step 4: 写入 start-linux.sh**

```bash
#!/usr/bin/env bash
# Keep the original upstream `autonet` tmux session semantics.
set -euo pipefail

cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
root=$(pwd -P)

command -v tmux >/dev/null 2>&1 || { echo '找不到 tmux，请先安装。' >&2; exit 1; }

exe="$root/target/release/tyut-keepalive-rs"
config="$root/config.enc"
[[ -x "$exe" ]] || { echo "找不到 Rust 可执行文件：$exe；请运行 cargo build --release。" >&2; exit 1; }
[[ -f "$config" ]] || { echo "找不到凭证：$config；请先在终端执行 '$exe' --once --config '$config'。" >&2; exit 1; }

if tmux has-session -t '=autonet' 2>/dev/null; then
    echo 'tmux autonet 会话已经存在，不重复启动。'
    exit 0
fi

tmux new-session -d -s autonet -c "$root" -- "$exe" --config "$config"
echo '已创建 tmux 会话 autonet。使用 tmux attach -t autonet 查看。'
```

- [ ] **Step 5: 写入 install-linux-autostart.sh**

```bash
#!/usr/bin/env bash
# Mirrors upstream ~/.profile login-shell autostart, but invokes the Rust launcher.
set -euo pipefail

cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
root=$(pwd -P)
[[ "$root" != *$'\n'* ]] || { echo '项目路径不能包含换行符。' >&2; exit 1; }

command -v tmux >/dev/null 2>&1 || { echo '找不到 tmux，请先安装。' >&2; exit 1; }
[[ -x "$root/target/release/tyut-keepalive-rs" ]] || { echo '请先运行 cargo build --release。' >&2; exit 1; }
[[ -f "$root/config.enc" ]] || { echo '请先执行 Rust 程序的 --once --config ./config.enc 保存凭证。' >&2; exit 1; }

profile="$HOME/.profile"
marker='# AutoNet login startup'
if [[ -f "$profile" ]] && grep -Fqx -- "$marker" "$profile"; then
    echo '发现原版或当前版 AutoNet 开机脚本配置，未修改 ~/.profile；请检查它是否指向此 Rust 项目。'
    exit 0
fi

escaped=${root//\'/\'\\\'\'}
printf '\n%s\n%s\n' "$marker" "bash '$escaped/start-linux.sh'" >> "$profile"
echo '已写入 ~/.profile；下次使用读取该文件的登录 Shell 时运行。'
```

- [ ] **Step 6: 写入 启动校园网登录.bat**

```bat
@echo off
cd /d "%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0Start-AutoNetPsmux.ps1"
if errorlevel 1 (
    echo Failed to start or restart the psmux net session.
    pause
)
```

---

### Task 9: 项目说明文档

**Files:**
- Create: `README.md`

- [ ] **Step 1: 写入 Rust 版 README**

````markdown
# TYUT Keepalive（Rust，保留 tmux / psmux）

Rust 版校园网自动登录核心，参考 [NoRegret-L/tyut-keepalive](https://github.com/NoRegret-L/tyut-keepalive) 的 ePortal 协议实现。**保留原版管理方式：Windows 使用 psmux，Linux 使用 tmux。** Rust 只替换 Python 常驻进程，不替换终端会话管理器。

> 原 Python 版已归档到 `archive/python/`，不参与构建与启动。

## 1. 编译（Windows / Linux）

```bash
cargo build --release
cargo test
```

编译后：Windows 为 `target\release\tyut-keepalive-rs.exe`，Linux 为 `target/release/tyut-keepalive-rs`。

## 2. 保存凭证（只需首次进行）

本方案固定使用项目根目录 `config.enc`，以便与原 Python 项目保持一致。也可以将原版 `config.enc` 放到本目录复用；不要把它上传到 Git。

Windows PowerShell（从项目根目录运行）：

```powershell
.\target\release\tyut-keepalive-rs.exe --once --config .\config.enc
```

Linux：

```bash
./target/release/tyut-keepalive-rs --once --config ./config.enc
```

首次输入账号和密码后保存。`--once` 会执行网络检测和最多一次重连尝试，建议在校园网环境运行。

其他参数：`--reset` 删除凭证、`--mac HEX` 手动指定 MAC、`--help` 查看帮助。

## 3. Windows：psmux 会话

先安装 [psmux](https://github.com/psmux/psmux)，确认 `psmux.exe` 在 `PATH`。

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\Start-AutoNetPsmux.ps1
# 或双击“启动校园网登录.bat”
```

会话 `net`、窗口 `AutoNet`；已有会话时只替换 `AutoNet` 窗口。`psmux attach -t net` 查看，`psmux kill-session -t net` 停止。

登录自启：

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\Register-AutoNetAutostart.ps1
```

复用原计划任务名 `AutoNet校园网自动登录`；已有同名任务时拒绝覆盖。

## 4. Linux：tmux 会话

```bash
bash start-linux.sh
tmux attach -t autonet
```

已存在 `autonet` 会话时脚本直接退出。登录 Shell 自启：`bash install-linux-autostart.sh`（检测 `# AutoNet login startup` 标记，不覆盖现有配置）。

## 5. 兼容性与风险

| 功能 | Rust + tmux / psmux |
|---|---|
| `Portal → loadConfig → login`、Cookie、校园 DNS | 保留移植逻辑 |
| `config.enc` XOR 混淆格式 | 兼容原版；非安全加密 |
| 断线检测、指数退避 | 沿用先前实现 |
| 认证超时 | 不直接视为成功，改用网络实测确认 |
| 调试 HTML 落盘 | 已移除，避免会话信息落盘 |
| Windows 内置 `--daemon` | 取消，由 psmux 会话取代 |

部署时先退出旧 Python 常驻进程，避免双进程同时重连；不要同时启用旧的 systemd 服务或直启 EXE 的计划任务。
````

---

### Task 10: 验证

**Files:**
- 无新增源码；日志写入 `logs/`

- [ ] **Step 1: 准备日志目录**

Run: `mkdir -p logs`

- [ ] **Step 2: 运行单元测试**

Run:
```sh
{ date; echo 'purpose: cargo test'; cargo test 2>&1; } > logs/cargo-test.log
```
Expected: 4 个测试全部通过（`xor_matches_python_protocol`、`parse_dns_a_record`、`reject_wrong_dns_id`、`login_status_is_conservative_for_timeout`）。若输出超过 50 行，只保留前 10 行与末尾 40 行；若失败，提取含 `error` 的上下文 30 行。

- [ ] **Step 3: 运行发布构建**

Run:
```sh
{ date; echo 'purpose: cargo build --release'; cargo build --release 2>&1; } > logs/cargo-build.log
```
Expected: 构建成功，生成 `target/release/tyut-keepalive-rs`（Windows 为 `.exe`）。若 crates.io 不可达或沙箱限制，停止自动尝试并输出需人工执行的完整命令。

- [ ] **Step 4: 检查脚本语法**

Run:
```sh
{ date; echo 'purpose: bash -n'; bash -n start-linux.sh install-linux-autostart.sh 2>&1; } > logs/bash-syntax.log
```
Expected: 无输出、退出码 0。

- [ ] **Step 5: 检查 PowerShell 脚本语法（环境具备时）**

Run:
```sh
powershell.exe -NoProfile -Command "$errors=$null; [System.Management.Automation.Language.Parser]::ParseFile('Start-AutoNetPsmux.ps1',[ref]$null,[ref]$errors)|Out-Null; if($errors){$errors; exit 1}"
```
Expected: 退出码 0。若本机无 PowerShell，记录为“环境未具备”，不视为失败。

- [ ] **Step 6: 核对最终文件树**

Run: `find . -maxdepth 3 -type f -not -path './.git/*' | sort`
Expected: 根目录仅含 Rust 项目文件、脚本、README、`.gitignore`；Python 文件仅在 `archive/python/`。
