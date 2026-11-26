# TYUT Keepalive（Rust，保留 tmux / psmux）

Rust 版校园网自动登录核心，参考 [NoRegret-L/tyut-keepalive](https://github.com/NoRegret-L/tyut-keepalive) 的 ePortal 协议实现。**保留原版管理方式：Windows 使用 psmux，Linux 使用 tmux。** Rust 只替换 Python 常驻进程，不替换终端会话管理器。

> 原 Python 版已归档到 `archive/python/`，不参与构建与启动。

## 1. 最傻瓜式使用（Windows）

适用：只想要一个能自动保活的程序，不想编译、不关心终端会话。

1. 在桌面新建文件夹，例如 `D:\AutoNet`。
2. 从 [v0.1.0 Release](https://github.com/NoRegret-L/tyut-keepalive/releases/tag/v0.1.0) 下载 `tyut-keepalive-windows-x86_64.exe`，放进该文件夹。
3. 双击 exe。首次会打开命令行窗口，按提示输入校园网账号和密码；程序把混淆后的凭证保存为同目录的 `config.enc`，随后开始前台保活。
4. 以后直接双击 exe 即可自动读取 `config.enc`，无需再次输入密码。

注意：exe 窗口关闭即停止保活。需要“登录后常驻、不显示窗口”时，安装 psmux 后使用本仓库的 `启动校园网登录.bat`，见第 5 节。`config.enc` 仅作 XOR 混淆，不是安全加密，不要分享。

### Linux（预编译版）

1. 新建目录，把 `tyut-keepalive-linux-x86_64` 放进去，执行 `chmod +x tyut-keepalive-linux-x86_64`。
2. 前台首次运行：`./tyut-keepalive-linux-x86_64 --once --config ./config.enc`，按提示输入账号密码，生成 `config.enc`。
3. 后台常驻：`nohup ./tyut-keepalive-linux-x86_64 --config ./config.enc > autonet.log 2>&1 &`；停止用 `pkill -f tyut-keepalive-linux-x86_64`。需要会话管理时用仓库的 `start-linux.sh`（tmux），见第 6 节。

## 2. 发布版下载与校验

下载地址：https://github.com/NoRegret-L/tyut-keepalive/releases/tag/v0.1.0

| 平台 | Release 资产 | SHA-256 |
|---|---|---|
| Windows x86_64 | `tyut-keepalive-windows-x86_64.exe` | `917f1a39d1daff77d650c23b258272a5bcae075581d9fef467e312d15757fd59` |
| Linux x86_64 | `tyut-keepalive-linux-x86_64` | `9820455bcb4de030553cc169b02397b9a30a4b66d76fd6275d63bea2e9ae2e76` |

校验命令（在二进制所在目录）：

```powershell
Get-FileHash .\tyut-keepalive.exe -Algorithm SHA256
```

```bash
sha256sum ./tyut-keepalive-linux-x86_64
```

Linux 下载后需要 `chmod +x tyut-keepalive-linux-x86_64`。浏览器若把文件保存为 `.exe.zip` 或 `.exe.crdownload`，请先改回 `.exe` 再运行。

## 3. 编译（Windows / Linux）

```bash
cargo build --release
cargo test
```

编译后：Windows 为 `target\release\tyut-keepalive.exe`，Linux 为 `target/release/tyut-keepalive`。

## 4. 保存凭证（只需首次进行）

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

## 5. Windows：psmux 会话

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

## 6. Linux：tmux 会话

```bash
bash start-linux.sh
tmux attach -t autonet
```

已存在 `autonet` 会话时脚本直接退出。登录 Shell 自启：`bash install-linux-autostart.sh`（检测 `# AutoNet login startup` 标记，不覆盖现有配置）。

## 7. 兼容性与风险

| 功能 | Rust + tmux / psmux |
|---|---|
| `Portal → loadConfig → login`、Cookie、校园 DNS | 保留移植逻辑 |
| `config.enc` XOR 混淆格式 | 兼容原版；非安全加密 |
| 断线检测、指数退避 | 沿用先前实现 |
| 认证超时 | 不直接视为成功，改用网络实测确认 |
| 调试 HTML 落盘 | 已移除，避免会话信息落盘 |
| Windows 内置 `--daemon` | 取消，由 psmux 会话取代 |

部署时先退出旧 Python 常驻进程，避免双进程同时重连；不要同时启用旧的 systemd 服务或直启 EXE 的计划任务。
