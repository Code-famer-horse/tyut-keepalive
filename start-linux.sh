#!/usr/bin/env bash
# Keep the original upstream `autonet` tmux session semantics.
set -euo pipefail

cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
root=$(pwd -P)

command -v tmux >/dev/null 2>&1 || { echo '找不到 tmux，请先安装。' >&2; exit 1; }

exe="$root/target/release/tyut-keepalive"
config="$root/config.enc"
[[ -x "$exe" ]] || { echo "找不到 Rust 可执行文件：$exe；请运行 cargo build --release。" >&2; exit 1; }
[[ -f "$config" ]] || { echo "找不到凭证：$config；请先在终端执行 '$exe' --once --config '$config'。" >&2; exit 1; }

# An existing session is left untouched, exactly as in the Python project.
if tmux has-session -t '=autonet' 2>/dev/null; then
    echo 'tmux autonet 会话已经存在，不重复启动。'
    exit 0
fi

tmux new-session -d -s autonet -c "$root" -- "$exe" --config "$config"
echo '已创建 tmux 会话 autonet。使用 tmux attach -t autonet 查看。'
