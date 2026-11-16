# TYUT 校园网自动登录

自动检测网络状态，在断线后尝试重新登录太原理工大学校园网。

## 环境要求

- Windows 或 Linux
- Python 3.10 或更高版本
- [uv](https://docs.astral.sh/uv/)
- 后台会话：Windows 使用 [psmux](https://github.com/psmux/psmux)（需加入 `PATH`），Linux 使用 [tmux](https://github.com/tmux/tmux)；前台运行不需要它们

## 安装

在项目目录下执行：

```powershell
uv sync
```

## 使用

首次运行时，在前台输入校园网账号和密码：

```powershell
.\.venv\Scripts\python.exe .\AutoNet.py
```

凭证保存后，按 `Ctrl+C` 停止前台程序。双击 `启动校园网登录.bat`，即可在 `psmux` 的 `net` 会话中启动或重启 `AutoNet` 窗口。

查看会话：

```powershell
psmux attach -t net
```

单次检查和重连：

```powershell
.\.venv\Scripts\python.exe .\AutoNet.py --once
```

删除已保存的凭证，下次运行时重新输入：

```powershell
.\.venv\Scripts\python.exe .\AutoNet.py --reset
```

## 登录时自动启动

可使用 Windows 任务计划程序，在用户登录后启动后台会话。先按上面的步骤保存校园网凭证，并确认 `psmux.exe` 已加入用户的 `PATH`。

在项目目录打开 PowerShell，执行：

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\Register-AutoNetAutostart.ps1
```

脚本为当前用户创建“AutoNet校园网自动登录”计划任务，允许电池供电时运行；若同名任务已存在，不会覆盖。任务在用户登录后运行，移动项目后需要在任务计划程序中更新路径或删除旧任务后重新注册。可在任务计划程序中右键任务选择“运行”，再通过 `psmux attach -t net` 检查会话。

## Linux 使用

安装 tmux 和项目依赖后，在项目目录前台运行一次，输入并保存凭证：

```bash
uv sync
.venv/bin/python AutoNet.py
```

按 `Ctrl+C` 停止前台程序后，启动 tmux 会话：

```bash
bash start-linux.sh
tmux attach -t autonet
```

配置用户登录 Shell 后自动启动：

```bash
bash install-linux-autostart.sh
```

安装脚本会向 `~/.profile` 追加启动命令，不会覆盖现有内容。已存在 `autonet` 会话时不会重复启动；修改项目路径后需要手动更新 `~/.profile`。只有读取 `~/.profile` 的登录 Shell 才会触发，纯桌面登录或不读取该文件的 Shell 可能不会启动。Linux 不支持 `--daemon`，请使用 tmux。

## 凭证安全

账号和密码以固定密钥异或后的形式保存在项目目录的 `config.enc` 中，可以被还原。请勿分享或上传该文件；项目的 `.gitignore` 已将其排除。运行日志和 `debug` 目录也可能包含网络信息，请妥善保管。
