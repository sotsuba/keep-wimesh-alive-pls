#Requires -Version 5.1
<#
.SYNOPSIS
    Installs keep_wimesh_session and registers a Task Scheduler task
    that runs the watchdog on every network connection event.

.DESCRIPTION
    Copies the binary and check_ping.ps1 to %LOCALAPPDATA%\wimesh\,
    then creates a user-scope scheduled task (no admin required) that
    triggers on network-available events and at logon.

.EXAMPLE
    .\install.ps1
    .\install.ps1 -Uninstall
#>

param(
    [switch]$Uninstall
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$TaskName   = "wimesh-ping"
$InstallDir = "$env:LOCALAPPDATA\wimesh"
$BinarySrc  = ".\target\release\keep_wimesh_session.exe"
$ScriptSrc  = ".\check_ping.ps1"
$BinaryDest = "$InstallDir\keep_wimesh_session.exe"
$ScriptDest = "$InstallDir\check_ping.ps1"

function Write-Step ([string]$msg) { Write-Host "  [OK] $msg" -ForegroundColor Green }
function Write-Fail ([string]$msg) { Write-Host "  [!!] $msg" -ForegroundColor Red }

# ==============================================================================
# UNINSTALL
# ==============================================================================
if ($Uninstall) {
    Write-Host "Uninstalling keep_wimesh_session..."

    $task = Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
    if ($task) {
        Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false
        Write-Step "scheduled task '$TaskName' removed"
    }

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

# --- Preflight checks ---------------------------------------------------------
if (-not (Test-Path $BinarySrc)) {
    Write-Fail "binary not found at $BinarySrc"
    Write-Host "Run: cargo build --release" -ForegroundColor Yellow
    exit 1
}
if (-not (Test-Path $ScriptSrc)) {
    Write-Fail "check_ping.ps1 not found in current directory"
    exit 1
}

# --- Copy files ---------------------------------------------------------------
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir | Out-Null
}
Copy-Item -Force $BinarySrc $BinaryDest
Write-Step $BinaryDest
Copy-Item -Force $ScriptSrc $ScriptDest
Write-Step $ScriptDest

# --- Register scheduled task (user-scope, no admin) ---------------------------
$pwsh = (Get-Command powershell.exe).Source

$action = New-ScheduledTaskAction `
    -Execute $pwsh `
    -Argument "-NonInteractive -WindowStyle Hidden -ExecutionPolicy Bypass -File `"$ScriptDest`""

# Trigger 1: at logon
$triggerLogon = New-ScheduledTaskTrigger -AtLogOn

# Trigger 2: on network-state-change (using event log)
# Event 10000 in Microsoft-Windows-NetworkProfile/Operational = network connected
$triggerNet = New-ScheduledTaskTrigger -AtStartup   # fallback; real event below
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
    -RestartCount 3 `
    -RestartInterval (New-TimeSpan -Minutes 1)

$principal = New-ScheduledTaskPrincipal `
    -UserId $env:USERNAME `
    -LogonType Interactive `
    -RunLevel Limited

# Register with logon trigger first, then patch in the event trigger
$taskDef = New-ScheduledTask `
    -Action   $action `
    -Trigger  @($triggerLogon, $cimTrigger) `
    -Settings $settings `
    -Principal $principal

Register-ScheduledTask -TaskName $TaskName -InputObject $taskDef -Force | Out-Null
Write-Step "scheduled task '$TaskName' registered (logon + network-connect triggers)"

# --- Start immediately --------------------------------------------------------
Start-ScheduledTask -TaskName $TaskName
Write-Step "task started"

Write-Host ""
Write-Host "Done. Check status:"
Write-Host "  Get-ScheduledTask -TaskName '$TaskName'"
Write-Host "  Get-ScheduledTaskInfo -TaskName '$TaskName'"
Write-Host ""
Write-Host "Uninstall:"
Write-Host "  .\install.ps1 -Uninstall"
