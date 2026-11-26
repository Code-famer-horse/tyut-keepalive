# Tauri 2 Desktop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把现有 Rust 保活核心改造为 Tauri 2 桌面应用，实现开机自启、后台留存、界面显示正确三个目标。

**Architecture:** 根目录使用零框架静态前端 `dist/`，Rust 侧放在 `src-tauri/`，核心登录/保活逻辑放在 `src-tauri/src/core/` 并与 UI 解耦。后台任务由 Rust 独立线程持有，状态用 `AppHandle::emit` 推送给前端；托盘隐藏与退出用 Tauri 原生 API，Linux 无托盘时降级为最小化 + 二次启动唤回。

**Tech Stack:** Tauri 2、Rust 1.90+、原生 HTML/CSS/JS、tauri-plugin-autostart、tauri-plugin-single-instance、tauri-action、GitHub Actions。

---

## 文件结构

- `package.json`：npm 脚本与 Tauri CLI。
- `dist/index.html`、`dist/styles.css`、`dist/main.js`：静态前端，无框架/无构建步骤。
- `src-tauri/Cargo.toml`：Rust 依赖。
- `src-tauri/build.rs`、`src-tauri/src/main.rs`：Tauri 入口。
- `src-tauri/src/lib.rs`：插件注册、窗口事件、托盘装配、常量与日志。
- `src-tauri/src/core/`：从现有 `src/` 迁移来的纯 Rust 核心。
- `src-tauri/src/state.rs`：共享状态、后台 worker、状态事件。
- `src-tauri/src/commands.rs`：`invoke` 命令。
- `src-tauri/src/tray.rs`：托盘菜单与可用性标记。
- `src-tauri/tauri.conf.json`、`src-tauri/capabilities/default.json`：Tauri 配置与权限。
- `.github/workflows/release.yml`：三平台发布工作流。

---

### Task 1: 搭建 Tauri 2 最小工程

**Files:**
- Create: `package.json`
- Create: `dist/index.html`
- Create: `dist/styles.css`
- Create: `dist/main.js`
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/build.rs`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/lib.rs`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/capabilities/default.json`
- Test: `npx tauri build --debug`

- [ ] **Step 1: 写入 `package.json`**

```json
{
  "name": "tyut-keepalive",
  "private": true,
  "version": "0.3.0",
  "scripts": {
    "tauri": "tauri"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2"
  },
  "dependencies": {
    "@tauri-apps/api": "^2"
  }
}
```

- [ ] **Step 2: 写入 `src-tauri/Cargo.toml`**

```toml
[package]
name = "tyut-keepalive"
version = "0.3.0"
edition = "2021"
description = "TYUT campus network auto-login (Tauri desktop)"

[lib]
name = "tyut_keepalive_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-autostart = "2"
tauri-plugin-single-instance = "2"
anyhow = "1"
base64 = "0.22"
chrono = { version = "0.4", default-features = false, features = ["clock"] }
netdev = { version = "0.47", default-features = false }
rand = "0.9"
regex = "1"
reqwest = { version = "0.13", default-features = false, features = ["blocking", "cookies", "query", "rustls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 3: 写入 Tauri 入口文件**

`src-tauri/build.rs`:

```rust
fn main() {
    tauri_build::build()
}
```

`src-tauri/src/main.rs`:

```rust
fn main() {
    tyut_keepalive_lib::run()
}
```

`src-tauri/src/lib.rs`（Task 1 最小版本，后续任务扩展）:

```rust
pub(crate) const PORTAL_URL: &str = "http://219.226.127.250";
pub(crate) const LOGIN_HOST: &str = "https://drcom.tyut.edu.cn:804";
pub(crate) const LOGIN_DOMAIN: &str = "drcom.tyut.edu.cn";
pub(crate) const TARGET: &str = "223.5.5.5";
pub(crate) const REFERER_URL: &str = "https://drcom.tyut.edu.cn:804/eportal/portal/index";
pub(crate) const USER_AGENT_VALUE: &str = "Mozilla/5.0 Chrome/146.0.0.0 Edg/146.0.0.0";
pub(crate) const LOGIN_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36 Edg/146.0.0.0";

pub(crate) fn log(message: impl AsRef<str>) {
    println!("[{}] {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), message.as_ref());
}

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("Tauri 应用启动失败");
}
```

- [ ] **Step 4: 写入配置与权限**

`src-tauri/tauri.conf.json`:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "TYUT Keepalive",
  "version": "0.3.0",
  "identifier": "com.tyut.keepalive",
  "build": {
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "label": "main",
        "title": "TYUT 校园网保活",
        "width": 480,
        "height": 380,
        "minWidth": 360,
        "minHeight": 280,
        "resizable": true,
        "center": true
      }
    ],
    "security": { "csp": null }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.ico",
      "icons/icon.icns"
    ]
  }
}
```

`src-tauri/capabilities/default.json`:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "主窗口权限",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "autostart:allow-enable",
    "autostart:allow-disable",
    "autostart:allow-is-enabled"
  ]
}
```

- [ ] **Step 5: 写入最小前端占位文件**

`dist/index.html`：

```html
<!doctype html>
<html lang="zh-CN">
  <head><meta charset="UTF-8" /><title>TYUT Keepalive</title></head>
  <body><h1>TYUT Keepalive</h1></body>
</html>
```

`dist/styles.css`：

```css
body { font-family: system-ui, "Microsoft YaHei", sans-serif; }
```

`dist/main.js`：

```js
console.log("Tauri shell ready");
```

- [ ] **Step 6: 生成图标并清理源文件**

```bash
printf '%s' 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PzWvWQAAAABJRU5ErkJggg==' | base64 -d > app-icon.png
npx tauri icon app-icon.png
rm app-icon.png
```

Expected: `src-tauri/icons/` 生成 32x32.png、128x128.png、icon.ico、icon.icns 等文件，根目录无 `app-icon.png`。

- [ ] **Step 7: 写入 `src-tauri/.gitignore`**

```gitignore
/target/
```

- [ ] **Step 8: 安装依赖并构建**

```bash
npm install
npx tauri build --debug
```

Expected: 输出包含 `Finished` 与 `Built application`；`target/debug/tyut-keepalive.exe` 存在（Windows）。若 macOS/Linux runner 执行则对应平台产物存在。

- [ ] **Step 9: 提交**

```bash
git add package.json package-lock.json dist src-tauri
git commit -m "feat(tauri): 搭建 Tauri 2 桌面工程骨架"
```

只暂存上述路径；工作区已有与本任务无关的未提交改动，禁止使用 `git add -A`。

---

### Task 2: 复用 Rust 核心（DNS、Portal、配置）

**Files:**
- Create: `src-tauri/src/core/mod.rs`
- Create: `src-tauri/src/core/dns.rs`
- Create: `src-tauri/src/core/portal.rs`
- Create: `src-tauri/src/core/config.rs`
- Test: `cargo test --manifest-path src-tauri/Cargo.toml`

- [ ] **Step 1: 写入 `core/mod.rs`**

```rust
pub mod config;
pub mod dns;
pub mod portal;

pub const XOR_KEY: u32 = 0x77;
```

- [ ] **Step 2: 复制核心文件并增加根级重导出**

```bash
mkdir -p src-tauri/src/core
cp src/dns.rs src-tauri/src/core/dns.rs
cp src/portal.rs src-tauri/src/core/portal.rs
cp src/config.rs src-tauri/src/core/config.rs
```

在 `src-tauri/src/lib.rs` 常量定义前增加：

```rust
mod core;
use crate::core::{config, dns};
```

`dns.rs` 与 `portal.rs` 中的 `crate::log`、`crate::config`、`crate::dns` 与常量引用由根级的模块声明、`use` 重导出与常量定义满足，不改动文件。

`config.rs` 中测试保留；`prompt_credentials` 在 Tauri 中不再使用，保留函数但不调用。

- [ ] **Step 3: 运行核心测试**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: `config::tests::xor_matches_python_protocol`、`config::tests::xor_round_trip_and_mask`、`dns::tests::*`、`portal::tests::login_status_is_conservative_for_timeout` 全部通过。

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/core
git commit -m "feat(tauri): 迁移 DNS/Portal/凭证核心到 src-tauri"
```

---

### Task 3: 后台保活任务与状态事件

**Files:**
- Create: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `cargo test --manifest-path src-tauri/Cargo.toml`

- [ ] **Step 1: 写入共享状态**

`src-tauri/src/state.rs` 完整内容：

```rust
use anyhow::Result;
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    core::{config, portal},
    log,
};

const WATCH_INTERVAL: Duration = Duration::from_secs(600);
const RETRY_INITIAL: Duration = Duration::from_secs(10);
const RETRY_MAX: Duration = Duration::from_secs(3600);

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Status {
    pub(crate) state: String,
    pub(crate) account: String,
    pub(crate) last_check: String,
    pub(crate) last_login: String,
    pub(crate) message: String,
}

pub(crate) type SharedStatus = Arc<Mutex<Status>>;

fn now() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn flag(path: &PathBuf, name: &str) -> bool {
    let file = path.join(name);
    if file.exists() {
        let _ = std::fs::remove_file(file);
        return true;
    }
    false
}

fn tick(duration: Duration, control: &PathBuf) -> bool {
    let deadline = Instant::now() + duration;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return false;
        }
        thread::sleep(left.min(Duration::from_secs(1)));
        if flag(control, "reconnect") {
            return true;
        }
    }
}

fn publish(app: &AppHandle, shared: &SharedStatus, state: &str, account: &str, last_login: &str, message: &str) {
    let status = Status {
        state: state.into(),
        account: account.into(),
        last_check: now(),
        last_login: last_login.into(),
        message: message.into(),
    };
    if let Ok(mut guard) = shared.lock() {
        *guard = status.clone();
    }
    let _ = app.emit("status", status);
}

pub(crate) fn paths(app: &AppHandle) -> Result<(PathBuf, PathBuf)> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    let control = dir.join("control");
    std::fs::create_dir_all(&control)?;
    Ok((dir.join("config.enc"), control))
}

pub(crate) fn start_worker(app: AppHandle, shared: SharedStatus) -> Result<()> {
    let (config_path, control) = paths(&app)?;
    thread::spawn(move || {
        let probe = match portal::client(None) {
            Ok(client) => client,
            Err(error) => {
                log(format!("初始化 HTTP 客户端失败：{error:#}"));
                return;
            }
        };
        let mut credentials: Option<config::Credentials> = None;
        let mut account = "未配置".to_string();
        let mut last_login = "尚未登录".to_string();
        let mut retry = RETRY_INITIAL;

        loop {
            if flag(&control, "reset") {
                credentials = None;
                account = "未配置".to_string();
                log("凭证已清除");
            }
            if credentials.is_none() {
                match config::read_credentials(&config_path) {
                    Ok(Some(loaded)) => {
                        account = config::masked_account(&loaded.account);
                        credentials = Some(loaded);
                    }
                    _ => {
                        publish(&app, &shared, "need-setup", &account, &last_login, "请打开界面输入账号密码");
                        thread::sleep(Duration::from_secs(2));
                        continue;
                    }
                }
            }
            let current = credentials.as_ref().expect("凭证已加载");
            let force = flag(&control, "reconnect");
            let online = !force && portal::check_network(&probe);
            if online {
                retry = RETRY_INITIAL;
                publish(&app, &shared, "online", &account, &last_login, "网络正常，10 分钟后复查");
                let _ = tick(WATCH_INTERVAL, &control);
                continue;
            }
            if !force {
                thread::sleep(Duration::from_secs(1));
                if portal::check_network(&probe) {
                    continue;
                }
            }
            publish(&app, &shared, "logging-in", &account, &last_login, "正在尝试登录");
            let _ = tick(retry, &control);
            match portal::attempt_login(current, None) {
                Ok(true) => {
                    last_login = now();
                    retry = RETRY_INITIAL;
                    log("登录成功");
                    publish(&app, &shared, "online", &account, &last_login, "登录成功");
                }
                Ok(false) => {
                    retry = (retry * 2).min(RETRY_MAX);
                    publish(&app, &shared, "offline", &account, &last_login, "登录失败，等待重试");
                }
                Err(error) => {
                    retry = (retry * 2).min(RETRY_MAX);
                    log(format!("登录异常：{error:#}"));
                    publish(&app, &shared, "offline", &account, &last_login, "登录异常，等待重试");
                }
            }
        }
    });
    Ok(())
}
```

- [ ] **Step 2: 在 `lib.rs` 增加模块与状态测试**

在 `src-tauri/src/lib.rs` 的常量前增加（若 `mod core;` 已存在则只补其余三行）：

```rust
mod commands;
mod core;
mod state;
mod tray;
```

在 `pub fn run()` 前增加：

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn status_default_is_empty() {
        let status = crate::state::Status::default();
        assert!(status.state.is_empty());
        assert!(status.account.is_empty());
    }
}
```

Expected: 该测试确认后台状态结构可用；Task 4 将替换为序列化测试。

- [ ] **Step 3: 运行测试使新模块可编译**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: 编译可能失败，提示 `commands`/`tray` 模块不存在；这是预期的失败。

- [ ] **Step 4: 创建临时占位模块**

`src-tauri/src/commands.rs`:

```rust
```

`src-tauri/src/tray.rs`:

```rust
```

- [ ] **Step 5: 测试通过**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: 全部通过，包含 `status_default_is_empty`。

- [ ] **Step 6: 提交**

```bash
git add src-tauri/src/state.rs src-tauri/src/lib.rs src-tauri/src/commands.rs src-tauri/src/tray.rs
git commit -m "feat(tauri): Rust 后台保活任务与状态事件"
```

---

### Task 4: 前端命令与自启控制

**Files:**
- Create (replace): `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `cargo test --manifest-path src-tauri/Cargo.toml`

- [ ] **Step 1: 写失败测试**

在 `src-tauri/src/lib.rs` 的 `tests` 模块删除占位测试，替换为：

```rust
#[cfg(test)]
mod tests {
    use crate::state::Status;

    #[test]
    fn status_serializes_camel_case() {
        let status = Status {
            state: "online".into(),
            account: "d****03".into(),
            last_check: "2026-11-26 15:00:00".into(),
            last_login: "尚未登录".into(),
            message: "网络正常".into(),
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"lastCheck\""));
        assert!(json.contains("\"lastLogin\""));
    }
}
```

- [ ] **Step 2: 运行测试验证序列化**

```bash
cargo test --manifest-path src-tauri/Cargo.toml status_serializes_camel_case -- --exact
```

Expected: PASS（状态结构已在 Task 3 定义）。

- [ ] **Step 3: 写入命令实现**

`src-tauri/src/commands.rs` 完整内容：

```rust
use std::{fs, sync::Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::{core::config, state::paths, SharedStatus};

#[tauri::command]
pub(crate) fn get_status(status: State<'_, SharedStatus>) -> Result<serde_json::Value, String> {
    let guard = status.lock().map_err(|_| "状态锁被占用".to_string())?;
    serde_json::to_value(guard.clone()).map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn reconnect(app: AppHandle) -> Result<(), String> {
    let (_, control) = paths(&app).map_err(|error| error.to_string())?;
    fs::write(control.join("reconnect"), b"1").map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn reset_credentials(app: AppHandle) -> Result<(), String> {
    let (config_path, control) = paths(&app).map_err(|error| error.to_string())?;
    if config_path.exists() {
        fs::remove_file(&config_path).map_err(|error| error.to_string())?;
    }
    fs::write(control.join("reset"), b"1").map_err(|error| error.to_string())?;
    fs::write(control.join("reconnect"), b"1").map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn setup_credentials(app: AppHandle, account: String, password: String) -> Result<(), String> {
    if account.trim().is_empty() || password.trim().is_empty() {
        return Err("账号和密码不能为空".into());
    }
    let (config_path, _) = paths(&app).map_err(|error| error.to_string())?;
    let credentials = config::Credentials {
        account: config::xor_encode(account.trim()),
        password: config::xor_encode(password.trim()),
        saved_at: Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()),
    };
    config::save_credentials(&config_path, &credentials).map_err(|error| error.to_string())?;
    let _ = app.emit("credentials-saved", ());
    Ok(())
}

#[tauri::command]
pub(crate) fn set_autostart(app: AppHandle, enabled: bool) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    let autostart = app.autostart();
    if enabled {
        autostart.enable().map_err(|error| error.to_string())?;
    } else {
        autostart.disable().map_err(|error| error.to_string())?;
    }
    autostart.is_enabled().map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn is_autostart(app: AppHandle) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    app.autostart().is_enabled().map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn quit(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub(crate) fn show_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[tauri::command]
pub(crate) fn hide_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.minimize();
    }
}

pub(crate) fn _unused_mutex_type() -> Mutex<()> {
    Mutex::new(())
}
```

- [ ] **Step 4: 注册命令**

在 `src-tauri/src/lib.rs` 的 `run()` 中，把最小版本替换为：

```rust
pub fn run() {
    let shared: SharedStatus = std::sync::Arc::new(std::sync::Mutex::new(state::Status::default()));
    let app = tauri::Builder::default()
        .manage(shared.clone())
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::reconnect,
            commands::reset_credentials,
            commands::setup_credentials,
            commands::set_autostart,
            commands::is_autostart,
            commands::quit,
            commands::show_window,
            commands::hide_window,
        ])
        .setup(move |app| {
            state::start_worker(app.handle().clone(), shared)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Tauri 应用启动失败");
    let _ = app;
}
```

在 `lib.rs` 顶部增加：

```rust
pub(crate) type SharedStatus = state::SharedStatus;
```

- [ ] **Step 5: 测试与提交**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(tauri): 前端命令与自启控制"
```

Expected: 全部测试通过。

---

### Task 5: 官方插件：自启与单实例

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/capabilities/default.json`（Task 1 已含权限，仅需核对）
- Test: `cargo check --manifest-path src-tauri/Cargo.toml`

- [ ] **Step 1: 注册插件**

把 `lib.rs` 的 `run()` 中 `tauri::Builder::default()` 后追加：

```rust
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
```

最终顺序为：`Builder::default() -> plugin(autostart) -> plugin(single_instance) -> manage(...) -> invoke_handler(...) -> setup(...) -> run(...)`。

- [ ] **Step 2: 校验权限文件包含**

`src-tauri/capabilities/default.json` 的 `permissions` 必须包含：

```json
"autostart:allow-enable",
"autostart:allow-disable",
"autostart:allow-is-enabled"
```

- [ ] **Step 3: 编译校验**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: 无错误。

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/lib.rs src-tauri/capabilities/default.json
git commit -m "feat(tauri): 接入官方自启与单实例插件"
```

---

### Task 6: 托盘、关窗隐藏与 Linux 降级

**Files:**
- Create (replace): `src-tauri/src/tray.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `cargo check --manifest-path src-tauri/Cargo.toml`

- [ ] **Step 1: 写入托盘实现**

`src-tauri/src/tray.rs` 完整内容：

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Manager,
};

static TRAY_AVAILABLE: AtomicBool = AtomicBool::new(false);

pub(crate) fn is_tray_available() -> bool {
    TRAY_AVAILABLE.load(Ordering::Relaxed)
}

pub(crate) fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "显示界面", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "彻底退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    let tray = TrayIconBuilder::with_id("main-tray")
        .icon(app.default_window_icon().expect("默认窗口图标缺失").clone())
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app);

    match tray {
        Ok(_) => {
            TRAY_AVAILABLE.store(true, Ordering::Relaxed);
            Ok(())
        }
        Err(error) => {
            TRAY_AVAILABLE.store(false, Ordering::Relaxed);
            log(format!("系统托盘不可用，已启用降级：{error}"));
            Ok(())
        }
    }
}

pub(crate) fn install_close_handler(app: &mut App) {
    app.on_window_event(|window, event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            if window.label() == "main" {
                if is_tray_available() {
                    let _ = window.hide();
                } else {
                    let _ = window.minimize();
                }
            }
        }
    });
}

fn log(message: impl AsRef<str>) {
    println!("[{}] {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), message.as_ref());
}
```

- [ ] **Step 2: 在 `setup` 中创建托盘**

在 Task 5 的 `setup` 闭包中，`start_worker` 后追加：

```rust
            tray::create_tray(app.handle())?;
```

- [ ] **Step 3: 在 `run()` 里安装关闭处理**

把 `.run(tauri::generate_context!())` 改为：

```rust
        .build(tauri::generate_context!())
        .expect("Tauri 应用构建失败");

    tray::install_close_handler(&app);
    app.run(|_, _| {});
```

- [ ] **Step 4: 让隐藏按钮随托盘能力降级**

在 `src-tauri/src/commands.rs` 中把 `hide_window` 替换为：

```rust
#[tauri::command]
pub(crate) fn hide_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if crate::tray::is_tray_available() {
            let _ = window.hide();
        } else {
            let _ = window.minimize();
        }
    }
}
```

- [ ] **Step 5: 编译校验**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: 无错误。

- [ ] **Step 6: 提交**

```bash
git add src-tauri/src/tray.rs src-tauri/src/lib.rs src-tauri/src/commands.rs
git commit -m "feat(tauri): 关窗隐藏至托盘与 Linux 降级"
```

---

### Task 7: 静态前端与响应式布局

**Files:**
- Modify: `dist/index.html`
- Modify: `dist/styles.css`
- Modify: `dist/main.js`
- Test: `npx tauri build --debug`

- [ ] **Step 1: 写入 `dist/index.html`**

```html
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>TYUT 校园网保活</title>
    <link rel="stylesheet" href="./styles.css" />
  </head>
  <body>
    <main class="app">
      <h1>TYUT 校园网保活</h1>
      <section class="card" aria-live="polite">
        <p><span class="key">状态</span><span id="state">加载中</span></p>
        <p><span class="key">账号</span><span id="account">-</span></p>
        <p><span class="key">最近检查</span><span id="lastCheck">-</span></p>
        <p><span class="key">最近登录</span><span id="lastLogin">-</span></p>
        <p class="message" id="message">-</p>
      </section>
      <section class="actions">
        <button id="setup">设置凭证</button>
        <button id="reconnect">立即重连</button>
        <button id="reset">重置凭据</button>
        <button id="autostart">开机自启：检查中</button>
        <button id="hide">隐藏到后台</button>
        <button id="quit" class="danger">彻底退出</button>
      </section>
      <form id="credentials" class="credentials" hidden>
        <label>账号<input id="accountInput" autocomplete="username" /></label>
        <label>密码<input id="passwordInput" type="password" autocomplete="current-password" /></label>
        <button type="submit">保存</button>
      </form>
      <p id="notice" role="status"></p>
    </main>
    <script type="module" src="./main.js"></script>
  </body>
</html>
```

- [ ] **Step 2: 写入 `dist/styles.css`**

```css
:root {
  color-scheme: light;
  font-family: system-ui, "Microsoft YaHei", "PingFang SC", sans-serif;
  font-size: 15px;
}
* { box-sizing: border-box; }
body { margin: 0; background: #f5f7fa; color: #1f2937; }
.app { max-width: 760px; margin: 0 auto; padding: 16px; }
h1 { font-size: 1.35rem; margin: 4px 0 12px; }
.card { background: #fff; border: 1px solid #d8dee9; border-radius: 8px; padding: 12px; }
.card p { display: flex; gap: 8px; margin: 6px 0; flex-wrap: wrap; }
.key { min-width: 84px; color: #64748b; }
.message { color: #2563eb; }
.actions { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 12px; }
button { padding: 8px 12px; border: 1px solid #94a3b8; border-radius: 6px; background: #fff; cursor: pointer; }
button:hover { background: #eef2ff; }
.danger { color: #b91c1c; border-color: #fca5a5; }
.credentials { display: grid; gap: 8px; margin-top: 12px; padding: 12px; background: #fff; border-radius: 8px; }
.credentials label { display: grid; gap: 4px; }
input { padding: 8px; border: 1px solid #94a3b8; border-radius: 6px; }
#notice { min-height: 1.4em; color: #059669; }
@media (max-width: 420px) {
  .app { padding: 10px; }
  .key { min-width: 72px; }
  button { flex: 1 1 46%; }
}
```

- [ ] **Step 3: 写入 `dist/main.js`**

```js
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const $ = (id) => document.getElementById(id);
const notice = (text) => { $("notice").textContent = text; };

function render(status) {
  $("state").textContent = status.state || "-";
  $("account").textContent = status.account || "-";
  $("lastCheck").textContent = status.lastCheck || "-";
  $("lastLogin").textContent = status.lastLogin || "-";
  $("message").textContent = status.message || "-";
}

async function refresh() {
  try {
    render(await invoke("get_status"));
    const enabled = await invoke("is_autostart");
    $("autostart").textContent = enabled ? "开机自启：已开启" : "开机自启：已关闭";
  } catch (error) {
    notice(String(error));
  }
}

listen("status", (event) => render(event.payload));
refresh();
setInterval(refresh, 1000);

$("reconnect").onclick = async () => {
  try { await invoke("reconnect"); notice("已发送重连请求"); }
  catch (error) { notice(String(error)); }
};
$("reset").onclick = async () => {
  try { await invoke("reset_credentials"); notice("凭据已删除，请重新设置"); }
  catch (error) { notice(String(error)); }
};
$("setup").onclick = () => { $("credentials").hidden = !$("credentials").hidden; };
$("credentials").onsubmit = async (event) => {
  event.preventDefault();
  try {
    await invoke("setup_credentials", {
      account: $("accountInput").value,
      password: $("passwordInput").value,
    });
    $("credentials").hidden = true;
    $("accountInput").value = "";
    $("passwordInput").value = "";
    notice("凭证已保存");
  } catch (error) { notice(String(error)); }
};
$("autostart").onclick = async () => {
  try {
    const enabled = await invoke("is_autostart");
    await invoke("set_autostart", { enabled: !enabled });
    notice("开机自启设置已更新");
    refresh();
  } catch (error) { notice(String(error)); }
};
$("hide").onclick = async () => {
  try { await invoke("hide_window"); notice("已隐藏到后台；托盘可唤回，无托盘时重新启动本程序即可显示"); }
  catch (error) { notice(String(error)); }
};
$("quit").onclick = async () => { await invoke("quit"); };
```

- [ ] **Step 4: 构建校验**

```bash
npx tauri build --debug
```

Expected: 构建成功，前端文件被打包。

- [ ] **Step 5: 提交**

```bash
git add dist
git commit -m "feat(tauri): 状态界面与响应式布局"
```

---

### Task 8: GitHub Actions 三平台发布

**Files:**
- Modify: `.github/workflows/release.yml`
- Test: 推送 tag `v0.3.0` 后 CI

- [ ] **Step 1: 替换工作流文件**

`.github/workflows/release.yml` 完整内容：

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

permissions:
  contents: write

jobs:
  release:
    strategy:
      fail-fast: false
      matrix:
        include:
          - platform: windows-latest
            args: --bundles msi
          - platform: macos-latest
            args: --bundles dmg
          - platform: ubuntu-latest
            args: --bundles deb,appimage
    runs-on: ${{ matrix.platform }}
    steps:
      - uses: actions/checkout@v4

      - name: Install Linux dependencies
        if: matrix.platform == 'ubuntu-latest'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
            libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev

      - uses: actions/setup-node@v4
        with:
          node-version: 24

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - uses: Swatinem/rust-cache@v2

      - name: Install frontend dependencies
        run: npm install

      - name: Build and publish
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: 'TYUT Keepalive ${{ github.ref_name }}'
          releaseBody: 'Tauri 2 三平台桌面版：开机自启、后台留存、托盘控制。'
          releaseDraft: false
          prerelease: false
          args: ${{ matrix.args }}
```

- [ ] **Step 2: 本地语法核对**

```bash
git diff --check -- .github/workflows/release.yml
```

Expected: 无输出。

- [ ] **Step 3: 提交（不触发发布）**

```bash
git add .github/workflows/release.yml
git commit -m "ci: 使用 tauri-action 发布三平台安装包"
```

- [ ] **Step 4: 触发验证（由用户确认后执行）**

```bash
git tag v0.3.0
git push origin v0.3.0
```

Expected: GitHub Actions 创建三个构建作业并上传 `.msi`、`.dmg`、`.deb`、`.AppImage` 到 Release。

---

### Task 9: 功能验证与交付边界

**Files:** 无代码文件
**Test:** Windows 本机手工验证 + CI 资产核验

- [ ] **Step 1: Windows 功能验证（手工，逐项打勾）**

1. 双击安装程序安装后启动，输入账号密码并保存，确认界面显示「在线」且中文无乱码。
2. 拖大/拖小窗口，确认布局跟随缩放且按钮不重叠。
3. 关闭主窗口，确认任务管理器中进程仍在、托盘图标存在。
4. 点击托盘「显示界面」，确认窗口恢复。
5. 重启登录，确认应用自动启动。
6. 再次启动安装程序，确认只出现单个进程且原窗口被聚焦。
7. 托盘「彻底退出」后确认进程消失；再次启动仍可正常显示界面。

- [ ] **Step 2: CI 资产核验**

```bash
gh run list --limit 1
gh run view <run-id> --json status,conclusion,jobs
gh api repos/NoRegret-L/tyut-keepalive/releases/tags/v0.3.0 --jq '{assets:[.assets[]|{name,size}]}'
```

Expected: `conclusion` 为 `success`，资产包含 Windows `.msi`、macOS `.dmg`、Linux `.deb` 与 `.AppImage`。

- [ ] **Step 3: 明确验证边界**

Windows 为本地功能验证；macOS/Linux 仅有 CI 构建与资产结构检查，不声称已在本机功能验证。交付说明必须写明这一限制。

---

### Task 10: 清理旧 eframe 工程残留

**Files:**
- Delete: `Cargo.toml`、`Cargo.lock`、`src/`、`target/`（仅旧工程根级文件，不动 `src-tauri/`）
- Modify: `.gitignore`

- [ ] **Step 1: 删除旧二进制工程**

```bash
git rm -r Cargo.toml Cargo.lock src target
```

Expected: 根目录只剩 `src-tauri/` 一个 Cargo 工程。

- [ ] **Step 2: 更新 `.gitignore`**

删除旧条目 `/target/*`、`!/target/release/`、`/target/release/*`、`!/target/release/tyut-keepalive`；保留 `logs/`、`config.enc` 等现有规则。

- [ ] **Step 3: 验证并提交**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
node --version
git add .gitignore
git commit -m "chore: 移除旧 eframe 工程残留"
```

Expected: 测试通过，根目录无旧 `Cargo.toml`。禁止 `git add -A`。
