@echo off
cd /d "%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0Start-AutoNetPsmux.ps1"
if errorlevel 1 (
    echo Failed to start or restart the psmux net session.
    pause
)
