$ErrorActionPreference = 'Stop'
$TaskName = 'AutoNet校园网自动登录'
$UserId = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name
$StartScript = Join-Path $PSScriptRoot 'Start-AutoNetPsmux.ps1'
$PythonPath = Join-Path $PSScriptRoot '.venv\Scripts\python.exe'
$ConfigPath = Join-Path $PSScriptRoot 'config.enc'

if (Get-ScheduledTask -TaskName $TaskName -TaskPath '\' -ErrorAction SilentlyContinue) {
    throw "任务 '$TaskName' 已存在，未进行任何更改。请在任务计划程序中检查该任务。"
}
if (-not (Get-Command psmux.exe -CommandType Application -ErrorAction SilentlyContinue)) {
    throw '找不到 psmux.exe，请先将其加入 PATH。'
}
foreach ($Path in @($StartScript, $PythonPath, $ConfigPath)) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "找不到必需文件：$Path。请先完成安装及首次登录。"
    }
}
$PowerShell = Join-Path $PSHOME 'powershell.exe'
$Action = New-ScheduledTaskAction -Execute $PowerShell -Argument "-NoProfile -ExecutionPolicy Bypass -File `"$StartScript`"" -WorkingDirectory $PSScriptRoot
$Trigger = New-ScheduledTaskTrigger -AtLogOn -User $UserId
$Principal = New-ScheduledTaskPrincipal -UserId $UserId -LogonType Interactive
$Settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
Register-ScheduledTask -TaskName $TaskName -Action $Action -Trigger $Trigger -Principal $Principal -Settings $Settings | Out-Null
Write-Output "已配置用户登录时自动启动：$TaskName"
