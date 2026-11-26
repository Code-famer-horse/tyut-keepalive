# TYUT Keepalive Rust 迁移设计

## 目标

将仓库从 Python 实现迁移为 Rust 实现：以 `tyut-keepalive-rs-tmux-psmux.zip` 中的 Rust 核心为基线，融合现有仓库的启动脚本、会话管理与文档；Python 版本归档保留，不参与构建。

## 已确认决策

1. Python 特有文件（`AutoNet.py`、`pyproject.toml`、`uv.lock`、`.python-version`、`tests/`、原 `README.md`）移入 `archive/python/`；根目录启动脚本原位改为调用 Rust 可执行文件。
2. zip 中的单文件 `src/main.rs` 按职责拆分为 `args`、`config`、`dns`、`portal`、`watch` 模块加 `main.rs`；行为与 4 个单元测试保持不变。
3. 沿用 zip 的 `Cargo.toml` 依赖版本；构建失败时只做最小调整。脚本语法检查在本次验证时手工执行，不固化为 Rust 测试。
4. 接受 Rust 版的行为变更：`认证超时` 不直接视为登录成功而是实测网络确认；取消 Windows `--daemon`，由 psmux 会话取代。
5. 本设计文档写入仓库，不执行 `git commit`。

## 目录结构

```
Cargo.toml
src/
  main.rs       程序入口与顶层错误处理
  args.rs       命令行参数解析与帮助
  config.rs     config.enc 的 XOR 兼容读写与首次输入
  dns.rs        校园 DNS UDP 查询与响应解析
  portal.rs     Portal 探测、loadConfig、login 与网络判定
  watch.rs      在线监控、断网确认与指数退避循环
Start-AutoNetPsmux.ps1
Register-AutoNetAutostart.ps1
scripts/register-windows-task.ps1
start-linux.sh
install-linux-autostart.sh
启动校园网登录.bat
README.md
.gitignore
archive/python/
  AutoNet.py
  pyproject.toml
  uv.lock
  .python-version
  tests/
  README.md
docs/superpowers/specs/2026-11-24-rust-migration-design.md
```

## 行为约定

- 协议参数、XOR 编码、校园 DNS 与 Portal 流程以 zip 中的 Rust 实现为准；不重新设计协议。
- `config.enc` 保持与 Python 版相同的 XOR hex JSON 格式；启动脚本统一传入 `--config` 指向项目根目录，避免多份凭证。
- Windows 继续使用 psmux 的 `net:AutoNet` 窗口，Linux 继续使用 tmux 的 `autonet` 会话；已存在会话时不重复启动。
- 不再写 `debug/portal_full.html`，不在日志中输出完整登录响应，避免会话信息落盘。
- 归档目录只读保留，不被任何启动脚本或构建流程引用。

## 验证

1. `cargo test`：4 个 Rust 单元测试通过（XOR 兼容、DNS A 记录解析、错误 DNS ID 拒绝、登录结果判定）。
2. `cargo build --release`：生成发布版可执行文件。
3. `bash -n start-linux.sh install-linux-autostart.sh`：脚本语法通过。
4. PowerShell `Parser::ParseFile` 检查两个 `.ps1` 脚本语法通过（环境具备 PowerShell 时）。
5. 若 crates.io 不可达或构建失败，停止自动降级，输出需要人工执行的完整命令。

## 不在范围内

- 不实现 systemd 服务或 Rust 内置 daemon 作为默认启动方式。
- 不改造校园网认证协议、重试常数或凭证加密算法。
- 不提交 git 变更。
