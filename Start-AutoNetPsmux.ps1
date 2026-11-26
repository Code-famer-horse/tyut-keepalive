# Requires psmux.exe in PATH. Reuses upstream session `net` and window `AutoNet`.
$ErrorActionPreference = 'Stop'

$PsmuxCommand = Get-Command psmux.exe -CommandType Application -ErrorAction SilentlyContinue
if (-not $PsmuxCommand) { throw '找不到 psmux.exe，请先安装 psmux 并将其加入 PATH。' }
$PsmuxPath = $PsmuxCommand.Source

$WorkingDirectory = $PSScriptRoot
$Executable = Join-Path $WorkingDirectory 'target\release\tyut-keepalive.exe'
$ConfigPath = Join-Path $WorkingDirectory 'config.enc'
foreach ($Path in @($Executable, $ConfigPath)) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "找不到文件：$Path。请先编译 Rust 并在前台保存凭证。"
    }
}

$SessionName = 'net'
$WindowName = 'AutoNet'
$SessionTarget = "${SessionName}:${WindowName}"

function Invoke-Psmux {
    param([string[]]$PsmuxArguments)
    & $PsmuxPath @PsmuxArguments
    if ($LASTEXITCODE -ne 0) {
        throw ("psmux 执行失败 (exit $LASTEXITCODE)：" + ($PsmuxArguments -join ' '))
    }
}

# Use the documented exit status, not translated error text.
& $PsmuxPath has-session -t "=$SessionName" *> $null
$SessionExists = ($LASTEXITCODE -eq 0)
$BufferWindow = $null

if ($SessionExists) {
    # Match window names exactly; do not depend on psmux's decorative list formatting.
    $Windows = @(& $PsmuxPath list-windows -t $SessionName -F '#{window_name}')
    if ($LASTEXITCODE -ne 0) { throw "无法列出 psmux $SessionName 的窗口。" }

    if ($Windows -contains $WindowName) {
        # Preserve the session even when AutoNet is its only window.
        $BufferWindow = "__net_restart_$PID"
        Invoke-Psmux -PsmuxArguments @('new-window', '-d', '-t', $SessionName, '-n', $BufferWindow)
        Invoke-Psmux -PsmuxArguments @('kill-window', '-t', $SessionTarget)
    }
    Invoke-Psmux -PsmuxArguments @(
        'new-window', '-d', '-t', $SessionName, '-n', $WindowName,
        '-c', $WorkingDirectory, '--', $Executable, '--config', $ConfigPath
    )
} else {
    Invoke-Psmux -PsmuxArguments @(
        'new-session', '-d', '-s', $SessionName, '-n', $WindowName,
        '-c', $WorkingDirectory, '--', $Executable, '--config', $ConfigPath
    )
}

# Upstream behavior: keep pane contents visible if the program exits.
Invoke-Psmux -PsmuxArguments @('set-window-option', '-t', $SessionTarget, 'remain-on-exit', 'on')
if ($BufferWindow) {
    Invoke-Psmux -PsmuxArguments @('kill-window', '-t', "${SessionName}:${BufferWindow}")
}
Invoke-Psmux -PsmuxArguments @('select-window', '-t', $SessionTarget)
Write-Output 'Rust AutoNet 已在 psmux net:AutoNet 窗口中启动/重启。使用 psmux attach -t net 查看。'
