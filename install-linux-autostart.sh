#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
root=$(pwd -P)
[[ "$root" != *$'\n'* ]] || { echo '项目路径不能包含换行符。' >&2; exit 1; }
command -v tmux >/dev/null || { echo '请先安装 tmux。' >&2; exit 1; }
[[ -x "$root/.venv/bin/python" && -f "$root/config.enc" ]] || { echo '请先安装依赖并在前台保存凭证。' >&2; exit 1; }
profile="$HOME/.profile"
marker='# AutoNet login startup'
if [[ -f "$profile" ]] && grep -Fqx -- "$marker" "$profile"; then
    echo '登录自启配置已存在；如需更改路径，请先手动检查 ~/.profile。'
    exit 0
fi
escaped=${root//\'/\'\\\'\'}
printf '\n%s\n%s\n' "$marker" "bash '$escaped/start-linux.sh'" >> "$profile"
echo '已写入 ~/.profile；下次登录 Shell 时启动。'
