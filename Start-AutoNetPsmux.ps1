$ErrorActionPreference = "Stop"

$PsmuxCommand = Get-Command psmux.exe -CommandType Application -ErrorAction SilentlyContinue
if (-not $PsmuxCommand) {
    throw "找不到 psmux.exe，请安装 psmux 并将其添加到 PATH。"
}
$PsmuxPath = $PsmuxCommand.Source
$WorkingDirectory = $PSScriptRoot
$PythonPath = Join-Path $WorkingDirectory ".venv\Scripts\python.exe"
$ScriptPath = Join-Path $WorkingDirectory "AutoNet.py"
if (-not (Test-Path -LiteralPath $PythonPath -PathType Leaf)) {
    throw "找不到项目虚拟环境中的 Python：$PythonPath"
}
if (-not (Test-Path -LiteralPath $ScriptPath -PathType Leaf)) {
    throw "找不到项目脚本：$ScriptPath"
}
$SessionName = "net"
$WindowName = "AutoNet"

function Invoke-Psmux {
    param([string[]]$PsmuxArguments)
    & $PsmuxPath @PsmuxArguments
    if ($LASTEXITCODE -ne 0) {
        throw ("psmux command failed (exit " + $LASTEXITCODE + "): " + ($PsmuxArguments -join " "))
    }
}

& $PsmuxPath has-session -t $SessionName *> $null
$SessionExists = $LASTEXITCODE -eq 0
$BufferWindow = $null
$SessionTarget = $SessionName + ":" + $WindowName

if ($SessionExists) {
    $Windows = & $PsmuxPath list-windows -t $SessionName
    if ($LASTEXITCODE -ne 0) {
        throw ("Unable to list windows in psmux session " + $SessionName)
    }

    $HasAutoNetWindow = $Windows | Where-Object { $_ -match "^\s*\d+:\s+AutoNet(?:\*|-|\s|\()" }
    if ($HasAutoNetWindow) {
        $BufferWindow = "__net_restart_$PID"
        Invoke-Psmux -PsmuxArguments @("new-window", "-d", "-t", $SessionName, "-n", $BufferWindow)
        Invoke-Psmux -PsmuxArguments @("kill-window", "-t", $SessionTarget)
    }

    Invoke-Psmux -PsmuxArguments @("new-window", "-d", "-t", $SessionName, "-n", $WindowName, "-c", $WorkingDirectory, "--", $PythonPath, "-u", $ScriptPath)
} else {
    Invoke-Psmux -PsmuxArguments @("new-session", "-d", "-s", $SessionName, "-n", $WindowName, "--", $PythonPath, "-u", $ScriptPath)
}

Invoke-Psmux -PsmuxArguments @("set-window-option", "-t", $SessionTarget, "remain-on-exit", "on")
if ($BufferWindow) {
    Invoke-Psmux -PsmuxArguments @("kill-window", "-t", ($SessionName + ":" + $BufferWindow))
}
Invoke-Psmux -PsmuxArguments @("select-window", "-t", $SessionTarget)
