# TYUT Keepalive

太原理工大学校园网自动认证与保活桌面工具，基于 Rust 和 Tauri 2。当前仓库为 [NoRegret-L/tyut-keepalive](https://github.com/NoRegret-L/tyut-keepalive) 的 fork。

> v0.4.0 正在 develop/v0.4.0 开发；本分支不是已发布版本。Windows/Linux 开发 CI 和 macOS 兼容性 CI 分开运行，正式发布前必须通过三平台验证。应用版本将在正式发版前统一更新。

## 安装与使用

从本 fork 的 [Releases](https://github.com/Code-famer-horse/tyut-keepalive/releases) 下载实际已发布的对应平台安装包。旧版发布资产的名称可能不同，不要把开发分支当成已发布安装包。

| 平台 | 计划提供的安装包 | 注意事项 |
| --- | --- | --- |
| Windows x64 | MSI | 按安装器提示安装，需 WebView2 |
| macOS Apple Silicon | DMG | 拖入应用程序目录；实际兼容性和系统签名仍需验证 |
| Linux x64 | DEB、AppImage | DEB 使用系统包管理器升级；AppImage 可使用签名更新 |

1. 打开应用，在连接设置中输入校园网账号和密码，保存凭据。
2. 概览页查看实时状态、最近检测和认证时间、观测中断及在线时长。
3. 设置页可切换浅色、深色及系统主题，开启自启和系统通知（通知默认关闭）。
4. 关闭窗口会隐藏至托盘；无托盘时降级为最小化。彻底退出才结束后台服务，自启需单独关闭。
5. 暂停保活不会注销当前网络，且仅对本次运行有效；唤醒和网络变化不会自动解除暂停。密码被拒绝后自动重试停止。

## 日志、凭据与安全

- 凭据保存在系统密钥库：Windows Credential Manager、macOS Keychain、Linux Secret Service。旧版应用数据目录中的 config.enc 在安全迁移并校验成功后删除；不会在迁移失败时自动回退到不安全存储。
- 日志位于 Tauri 应用数据目录的 telemetry 子目录；两个约 1 MiB 的 JSONL 文件轮转，界面显示最近 200 条。日志不保存账号、密码、IP、MAC 或认证请求 URL。
- 统计是程序的网络观察结果，不等于校方服务器的登录会话统计。原始校园网探测规则可能误判。Linux 电源监听依赖 systemd-logind。
- 后端统一处理网络变化、休眠、暂停和恢复，防止并发认证及过期任务结果覆盖状态。
- 仅本地主窗口获准调用必要 IPC；不授权 WebView 通用通知、更新、文件系统、Shell 或远程事件发送权限。CSP 禁止远程脚本。RustSec 审计及配置检查进入 CI。
- 校园 Portal 探测仍使用校方既有 HTTP 协议；认证请求使用 HTTPS。密钥库、真实校园网、托盘、通知及休眠恢复需要实机验证。

## 签名更新

关于页在配置有效公钥后启动检查一次，并提供手动检查；安装必须由用户确认。官方 updater 使用 HTTPS、Minisign 签名和签名版本绑定，禁止降级。未配置公钥时更新停用，构建仍可用于开发。DEB 等系统包管理器安装方式不保证应用内更新。

发布维护者在可信本机运行：

```sh
npm ci
npm run tauri -- signer generate -w ~/.tauri/tyut-keepalive.key
```

仅将 .pub 文件内容填入 src-tauri/tauri.conf.json 的 plugins.updater.pubkey。私钥和密码应保存于 GitHub Actions Secrets（TAURI_SIGNING_PRIVATE_KEY、TAURI_SIGNING_PRIVATE_KEY_PASSWORD），不得提交或发送给他人。丢失私钥可能使既有安装无法安全升级。更新签名不是 Windows Authenticode 或 macOS Developer ID 签名的替代品。

正式发布前须统一 package.json、package-lock.json、Cargo.toml、Cargo.lock、tauri.conf.json 中的应用版本，并由维护者确认发布标签。发布工作流先完成三平台严格锁定测试、RustSec 审计和公钥/标签检查，再签名构建上传草稿；更新清单包含三平台后才公开 Release。当前未配置公钥、未创建标签、未进行真实签名升级测试。

## 源码开发

安装 Rust stable、Node.js 24、Python 3.11+ 及 [Tauri 平台依赖](https://v2.tauri.app/start/prerequisites/)。

Ubuntu/Debian 参考依赖：

```sh
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev libdbus-1-dev
```

```sh
npm ci
npm run tauri -- dev
npm run check:html
node --test scripts/test_ui.mjs
python scripts/check_security.py
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo check --manifest-path src-tauri/Cargo.toml --release --locked
cargo install cargo-audit --version 0.23.0 --locked
cargo audit --file src-tauri/Cargo.lock
```

开发检查采用锁定依赖；新增依赖的提交阶段可临时由开发 CI 生成 Cargo.lock 产物，维护者校验提交后须恢复严格锁定。正式发布始终严格锁定。

开发设计记录见 [docs](docs/)。
