# TYUT Keepalive（Rust，保留 tmux / psmux）

Rust 版校园网自动登录核心，参考 [NoRegret-L/tyut-keepalive](https://github.com/NoRegret-L/tyut-keepalive) 的 ePortal 协议实现。**保留原版管理方式：Windows 使用 psmux，Linux 使用 tmux。** Rust 只替换 Python 常驻进程，不替换终端会话管理器。

> 原 Python 版已归档到 `archive/python/`，不参与构建与启动。

## 1. 编译（Windows / Linux）

```bash
cargo build --release
cargo test
```

编译后：Windows 为 `target\release\tyut-keepalive.exe`，Linux 为 `target/release/tyut-keepalive`。

## 2. 保存凭证（只需首次进行）

本方案固定使用项目根目录 `config.enc`，以便与原 Python 项目保持一致。也可以将原版 `config.enc` 放到本目录复用；不要把它上传到 Git。

Windows PowerShell（从项目根目录运行）：

```powershell
.\target\release\tyut-keepalive.exe --once --config .\config.enc
```

Linux：

```bash
./target/release/tyut-keepalive --once --config ./config.enc
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
