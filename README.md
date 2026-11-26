# TYUT Keepalive（Rust）

校园网自动登录与保活工具。Windows 与 Linux x86_64 均提供单文件可执行程序。

## Windows

1. 从 [Releases](https://github.com/NoRegret-L/tyut-keepalive/releases) 下载 `tyut-keepalive-windows-x86_64.exe`，放到任意目录。
2. 双击运行，按提示输入校园网账号密码。
3. 程序自动安装到 `%LOCALAPPDATA%\tyut-keepalive\`、注册登录自启并驻留系统托盘。托盘菜单可「打开界面」或「退出」。

界面提供：查看在线状态与最近日志、立即重连、重置凭据、启用/禁用开机自启。
「退出」只结束当前运行；关闭自启需单独点按按钮。

## Linux

1. 下载 `tyut-keepalive-linux-x86_64`，执行 `chmod +x tyut-keepalive-linux-x86_64`。
2. 双击/终端运行一次，按提示输入账号密码；程序复制到 `~/.local/share/tyut-keepalive/`，并写入 `systemd --user` 服务，登录后自启且崩溃后自动重启。
3. 常用命令：`tyut-keepalive --status` 查看状态，`--reset` 删除凭证，`--disable-autostart` 关闭自启。

无 systemd 的桌面环境可手动以后台方式运行安装目录中的程序。

## 说明

`config.enc` 使用与原 Python 版兼容的 XOR 混淆，不是安全加密，不要分享该文件。
源码构建：`cargo build --release`；测试：`cargo test`。
