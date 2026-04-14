#Requires -Version 5.1
<#
.SYNOPSIS
    Installs keep_wimesh_session and registers a Task Scheduler task
    that runs the watchdog on every network connection event.

.DESCRIPTION
    Copies the binary to %LOCALAPPDATA%\wimesh\,
    then creates a scheduled task that triggers on network-available events and at logon.

.EXAMPLE
    .\install.ps1
    .\install.ps1 -Uninstall
#>

param(
    [switch]$Uninstall
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Host "  [!!] Administrator privileges are required to register event-based Scheduled Tasks." -ForegroundColor Red
    Write-Host "       Please right-click PowerShell -> 'Run as Administrator', then run this script again." -ForegroundColor Yellow
    exit 1
}

$ScriptDir  = $PSScriptRoot
if (-not $ScriptDir) {
    Write-Host "  [!!] Cannot determine script directory. Run the script directly, not via dot-sourcing." -ForegroundColor Red
    exit 1
}

$TaskName   = "captive-login"
$InstallDir = "$env:LOCALAPPDATA\wimesh"
$LogFile    = "$InstallDir\task.log"

if (Test-Path "$ScriptDir\keep_wimesh_session.exe") {
    $BinarySrc = "$ScriptDir\keep_wimesh_session.exe"
} else {
    $BinarySrc = "$ScriptDir\target\release\keep_wimesh_session.exe"
}

$BinaryDest = "$InstallDir\keep_wimesh_session.exe"

function Write-Step ([string]$msg) { Write-Host "  [OK] $msg" -ForegroundColor Green }
function Write-Fail ([string]$msg) { Write-Host "  [!!] $msg" -ForegroundColor Red }

# ==============================================================================
# UNINSTALL
# ==============================================================================
if ($Uninstall) {
    Write-Host "Uninstalling keep_wimesh_session..."

    $task = Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
    if ($task) {
        Stop-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
        Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false
        Write-Step "scheduled task '$TaskName' removed"
    }

    # Kill the binary first (it holds a lock on keep_wimesh_session.exe).
    # The powershell wrapper exits on its own once the binary process ends.
    Get-Process -Name "keep_wimesh_session" -ErrorAction SilentlyContinue |
        Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 500

    if (Test-Path $InstallDir) {
        Remove-Item -Recurse -Force $InstallDir
        Write-Step "removed $InstallDir"
    }

    Write-Host ""
    Write-Host "Done."
    exit 0
}

# ==============================================================================
# INSTALL
# ==============================================================================
Write-Host "Installing keep_wimesh_session..."

# --- Stop existing task and process if running --------------------------------
$task = Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
if ($task) {
    Write-Host "  [*] Stopping existing task..."
    Stop-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 1
}
# Kill the binary process so it releases the file lock before Copy-Item.
Get-Process -Name "keep_wimesh_session" -ErrorAction SilentlyContinue |
    Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 500

# --- Preflight checks ---------------------------------------------------------
if (-not (Test-Path $BinarySrc)) {
    Write-Fail "binary not found at $BinarySrc"
    Write-Host "Make sure keep_wimesh_session.exe is in the same directory, or run: cargo build --release" -ForegroundColor Yellow
    exit 1
}

# --- Copy files ---------------------------------------------------------------
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir | Out-Null
}
Copy-Item -Force $BinarySrc $BinaryDest
Write-Step $BinaryDest

# --- Write a wrapper script ---------------------------------------------------
# A .ps1 file on disk avoids argument-escaping bugs that occur when redirection
# operators are embedded inside a Task Scheduler Argument string (they are passed
# to CreateProcess verbatim and may not be parsed as shell operators by cmd.exe).
# StreamWriter uses FileShare.Read so Get-Content -Wait can tail the log live.
$WrapperScript = "$InstallDir\run.ps1"
Set-Content -Path $WrapperScript -Encoding UTF8 -Value @"
`$w = [System.IO.File]::AppendText("$LogFile")
`$w.AutoFlush = `$true
try {
    & "$BinaryDest" watch 2>&1 | ForEach-Object { `$w.WriteLine([string]`$_) }
} finally {
    `$w.Dispose()
}
"@
Write-Step $WrapperScript

# --- Register scheduled task --------------------------------------------------
$action = New-ScheduledTaskAction `
    -Execute "powershell.exe" `
    -Argument "-NoProfile -NonInteractive -WindowStyle Hidden -ExecutionPolicy Bypass -File `"$WrapperScript`""

# Trigger 1: at logon
$triggerLogon = New-ScheduledTaskTrigger -AtLogOn

# Trigger 2: on network-state-change
# Event 10000 in Microsoft-Windows-NetworkProfile/Operational = network connected
$cimTrigger = (
    New-CimInstance -Namespace "Root/Microsoft/Windows/TaskScheduler" `
        -ClassName "MSFT_TaskEventTrigger" `
        -ClientOnly `
        -Property @{
            Enabled       = $true
            Subscription  = @"
<QueryList>
  <Query Id="0" Path="Microsoft-Windows-NetworkProfile/Operational">
    <Select Path="Microsoft-Windows-NetworkProfile/Operational">
      *[System[EventID=10000]]
    </Select>
  </Query>
</QueryList>
"@
        }
)

$settings = New-ScheduledTaskSettingsSet `
    -MultipleInstances IgnoreNew `
    -ExecutionTimeLimit (New-TimeSpan -Hours 0) `
    -RestartCount 10 `
    -RestartInterval (New-TimeSpan -Minutes 2)

# Run as the interactive user, not SYSTEM.
# SYSTEM cannot access the user's Wi-Fi session or write to the user's LOCALAPPDATA.
# RunLevel Limited: the binary needs no admin rights; Highest requires UAC auto-elevation
# which Task Scheduler cannot perform silently for background interactive tasks.
$principal = New-ScheduledTaskPrincipal `
    -UserId ([System.Security.Principal.WindowsIdentity]::GetCurrent().Name) `
    -LogonType Interactive `
    -RunLevel Limited

# Create task definition
$taskDef = New-ScheduledTask `
    -Action    $action `
    -Settings  $settings `
    -Principal $principal

# Assign triggers
$taskDef.Triggers = [Microsoft.Management.Infrastructure.CimInstance[]]($triggerLogon, $cimTrigger)

# Register task
Register-ScheduledTask -TaskName $TaskName -InputObject $taskDef -Force | Out-Null
Write-Step "scheduled task '$TaskName' registered (logon + network-connect triggers)"


# Start immediately
Start-ScheduledTask -TaskName $TaskName
Start-Sleep -Milliseconds 800
$info = Get-ScheduledTaskInfo -TaskName $TaskName -ErrorAction SilentlyContinue
if ($info -and $info.LastTaskResult -ne 0 -and $info.LastRunTime -lt (Get-Date).AddSeconds(-5)) {
    Write-Fail "Task may not have started cleanly. Check: Get-ScheduledTaskInfo -TaskName '$TaskName'"
} else {
    Write-Step "task started"
}

Write-Host ""
Write-Host "Done. Check status:"
Write-Host "  Get-ScheduledTask -TaskName '$TaskName'"
Write-Host "  Get-ScheduledTaskInfo -TaskName '$TaskName'"
Write-Host ""
Write-Host "View logs:"
Write-Host "  Get-Content -Path '$LogFile' -Tail 50 -Wait"
Write-Host ""
Write-Host "Uninstall:"
Write-Host "  .\install.ps1 -Uninstall"
