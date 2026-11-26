# Registers a logon task that starts/reuses the existing `net` psmux session.
# Will not overwrite a task from the original Python implementation.
$ErrorActionPreference = 'Stop'
$TaskName = 'AutoNet校园网自动登录'
$UserId = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name
$StartScript = Join-Path $PSScriptRoot 'Start-AutoNetPsmux.ps1'
$Executable = Join-Path $PSScriptRoot 'target\release\tyut-keepalive.exe'
$ConfigPath = Join-Path $PSScriptRoot 'config.enc'

if (Get-ScheduledTask -TaskName $TaskName -TaskPath '\' -ErrorAction SilentlyContinue) {
    throw "任务 '$TaskName' 已存在，未覆盖；请先在任务计划程序中检查旧版 Python 任务。"
}
if (-not (Get-Command psmux.exe -CommandType Application -ErrorAction SilentlyContinue)) {
    throw '找不到 psmux.exe，请将其加入 PATH。'
}
foreach ($Path in @($StartScript, $Executable, $ConfigPath)) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "找不到必需文件：$Path。请先编译并在前台保存凭证。"
    }
}

$PowerShell = Join-Path $PSHOME 'powershell.exe'
if (-not (Test-Path -LiteralPath $PowerShell)) {
    $PowerShell = (Get-Command powershell.exe -CommandType Application -ErrorAction Stop).Source
}
$Action = New-ScheduledTaskAction -Execute $PowerShell -Argument "-NoProfile -ExecutionPolicy Bypass -File `"$StartScript`"" -WorkingDirectory $PSScriptRoot
$Trigger = New-ScheduledTaskTrigger -AtLogOn -User $UserId
$Principal = New-ScheduledTaskPrincipal -UserId $UserId -LogonType Interactive
$Settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -MultipleInstances IgnoreNew
Register-ScheduledTask -TaskName $TaskName -Action $Action -Trigger $Trigger -Principal $Principal -Settings $Settings | Out-Null
Write-Output "已配置登录后通过 psmux 启动 Rust AutoNet：$TaskName"
