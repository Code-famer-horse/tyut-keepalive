#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
root=$(pwd -P)
command -v tmux >/dev/null || { echo '请先安装 tmux。' >&2; exit 1; }
[[ -x "$root/.venv/bin/python" ]] || { echo '找不到虚拟环境，请先运行 uv sync。' >&2; exit 1; }
[[ -f "$root/config.enc" ]] || { echo '请先在前台运行 AutoNet.py 并保存凭证。' >&2; exit 1; }
if tmux has-session -t '=autonet' 2>/dev/null; then
    exit 0
fi
tmux new-session -d -s autonet -c "$root" "$root/.venv/bin/python" -u "$root/AutoNet.py"
