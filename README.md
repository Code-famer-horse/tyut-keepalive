# TYUT 校园网自动登录

自动检测网络状态，在断线后尝试重新登录太原理工大学校园网。

## 环境要求

- Windows
- Python 3.10 或更高版本
- [uv](https://docs.astral.sh/uv/)
- 使用后台会话时，需要安装 `psmux` 并将其加入 `PATH`

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

## 凭证安全

账号和密码以固定密钥异或后的形式保存在项目目录的 `config.enc` 中，可以被还原。请勿分享或上传该文件；项目的 `.gitignore` 已将其排除。运行日志和 `debug` 目录也可能包含网络信息，请妥善保管。
