#!/usr/bin/env bash
# Mirrors upstream ~/.profile login-shell autostart, but invokes the Rust launcher.
set -euo pipefail

cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
root=$(pwd -P)
[[ "$root" != *$'\n'* ]] || { echo '项目路径不能包含换行符。' >&2; exit 1; }

command -v tmux >/dev/null 2>&1 || { echo '找不到 tmux，请先安装。' >&2; exit 1; }
[[ -x "$root/target/release/tyut-keepalive" ]] || { echo '请先运行 cargo build --release。' >&2; exit 1; }
[[ -f "$root/config.enc" ]] || { echo '请先执行 Rust 程序的 --once --config ./config.enc 保存凭证。' >&2; exit 1; }

profile="$HOME/.profile"
marker='# AutoNet login startup'
if [[ -f "$profile" ]] && grep -Fqx -- "$marker" "$profile"; then
    echo '发现原版或当前版 AutoNet 开机脚本配置，未修改 ~/.profile；请检查它是否指向此 Rust 项目。'
    exit 0
fi

# Escape single quotes in the absolute path for the POSIX shell invocation.
escaped=${root//\'/\'\\\'\'}
printf '\n%s\n%s\n' "$marker" "bash '$escaped/start-linux.sh'" >> "$profile"
echo '已写入 ~/.profile；下次使用读取该文件的登录 Shell 时运行。'
