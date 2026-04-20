<# : batch portion
@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~f0" %*
exit /b %ERRORLEVEL%
: end batch / begin PowerShell #>

#Requires -Version 5.1
<#
.SYNOPSIS
    Manages the installation and uninstallation of the captive_portal background task.

.DESCRIPTION
    This script deploys the captive_portal binary to the user's local AppData
    directory and registers a Scheduled Task.

    Self-elevation is handled automatically if the script is not run as Administrator,
    while preserving the original standard user's context (profile paths and username).

.EXAMPLE
    .\install.bat
    Installs or updates the binary and scheduled task.

.EXAMPLE
    .\install.bat -Uninstall
    Stops the task, unregisters it, and removes the deployed files.
#>

[CmdletBinding()]
param(
    [switch]$Uninstall,
    [string]$TargetUser = $env:USERNAME,
    [string]$TargetLocalAppData = $env:LOCALAPPDATA
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# ==============================================================================
# 1. CONTEXT-AWARE SELF-ELEVATION
# ==============================================================================
$isAdministrator = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdministrator) {
    Write-Host "Requesting administrative privileges..." -ForegroundColor Cyan

    # Pass standard user context explicitly to the elevated process
    $argList = @(
        "-NoProfile",
        "-ExecutionPolicy Bypass",
        "-File `"$PSCommandPath`"",
        "-TargetUser `"$TargetUser`"",
        "-TargetLocalAppData `"$TargetLocalAppData`""
    )
    if ($Uninstall) { $argList += "-Uninstall" }

    try {
        Start-Process powershell.exe -Verb RunAs -ArgumentList ($argList -join ' ') -WorkingDirectory $PSScriptRoot
    } catch {
        Write-Host "Elevation cancelled or failed. Script cannot continue." -ForegroundColor Red
    }
    exit 0
}

# ==============================================================================
# 2. CONFIGURATION & VARIABLES
# ==============================================================================
$TaskName       = "captive-login"
$BinaryName     = "captive_portal.exe"
$BinaryBaseName = "captive_portal"
$InstallDir     = Join-Path -Path $TargetLocalAppData -ChildPath "wimesh"
$BinaryDest     = Join-Path -Path $InstallDir -ChildPath $BinaryName

$BinarySrc = Join-Path -Path $PSScriptRoot -ChildPath $BinaryName
if (-not (Test-Path $BinarySrc)) {
    $BinarySrc = Join-Path -Path $PSScriptRoot -ChildPath "target\release\$BinaryName"
}

function Write-Step ([string]$Message) { Write-Host "  [OK] $Message" -ForegroundColor Green }
function Write-Fail ([string]$Message) { Write-Host "  [!!] $Message" -ForegroundColor Red }

# ==============================================================================
# 3. MAIN EXECUTION
# ==============================================================================
try {
    # --------------------------------------------------------------------------
    # UNINSTALLATION PATH
    # --------------------------------------------------------------------------
    if ($Uninstall) {
        Write-Host "Starting uninstallation for user: $TargetUser"

        $existingTask = Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
        if ($existingTask) {
            Stop-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
            Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false
            Write-Step "Scheduled task '$TaskName' removed."
        }

        # Kill any lingering processes before deleting directory
        $process = Get-Process -Name $BinaryBaseName -ErrorAction SilentlyContinue
        if ($process) {
            $process | Stop-Process -Force
            $process | Wait-Process -Timeout 5 -ErrorAction SilentlyContinue
        }

        if (Test-Path $InstallDir) {
            Remove-Item -Path $InstallDir -Recurse -Force
            Write-Step "Directory '$InstallDir' removed."
        }

        Write-Host "`nUninstallation completed successfully." -ForegroundColor Cyan
        Read-Host "Press Enter to exit"
        exit 0
    }

    # --------------------------------------------------------------------------
    # INSTALLATION PATH
    # --------------------------------------------------------------------------
    Write-Host "Starting installation for user: $TargetUser"

    if (-not (Test-Path $BinarySrc)) {
        Write-Fail "Binary not found. Please ensure '$BinaryName' is in the same folder as this script."
        Read-Host "Press Enter to exit"
        exit 1
    }

    # Stop existing task to release file locks
    $existingTask = Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
    if ($existingTask) {
        Stop-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
    }

    $process = Get-Process -Name $BinaryBaseName -ErrorAction SilentlyContinue
    if ($process) {
        $process | Stop-Process -Force
        $process | Wait-Process -Timeout 5 -ErrorAction SilentlyContinue
    }

    # Deploy Binary
    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir | Out-Null
    }
    Copy-Item -Path $BinarySrc -Destination $BinaryDest -Force
    Write-Step "Binary deployed to $BinaryDest"

    # Define Task Components
    $action = New-ScheduledTaskAction -Execute $BinaryDest -Argument "watch"

    $triggerLogon = New-ScheduledTaskTrigger -AtLogOn
    $triggerNet = New-CimInstance -Namespace "Root/Microsoft/Windows/TaskScheduler" `
        -ClassName "MSFT_TaskEventTrigger" `
        -ClientOnly `
        -Property @{
            Enabled      = $true
            Subscription = @"
<QueryList>
  <Query Id="0" Path="Microsoft-Windows-NetworkProfile/Operational">
    <Select Path="Microsoft-Windows-NetworkProfile/Operational">
      *[System[EventID=10000]]
    </Select>
  </Query>
</QueryList>
"@
        }

    $settings = New-ScheduledTaskSettingsSet `
        -MultipleInstances IgnoreNew `
        -ExecutionTimeLimit (New-TimeSpan -Hours 0) `
        -RestartCount 3 `
        -RestartInterval (New-TimeSpan -Minutes 1) `
        -Hidden

    $principal = New-ScheduledTaskPrincipal `
        -UserId $TargetUser `
        -LogonType Interactive `
        -RunLevel Limited

    # Assemble and Register Task
    $taskDef = New-ScheduledTask -Action $action -Settings $settings -Principal $principal
    $taskDef.Triggers = [Microsoft.Management.Infrastructure.CimInstance[]]($triggerLogon, $triggerNet)

    if ($existingTask) {
        Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false
    }

    Register-ScheduledTask -TaskName $TaskName -InputObject $taskDef | Out-Null
    Write-Step "Scheduled task '$TaskName' registered (Triggers: Logon, Network Connect)."

    # Start Task
    Start-ScheduledTask -TaskName $TaskName
    Write-Step "Task initiated successfully."

    Write-Host "`nInstallation completed successfully." -ForegroundColor Cyan
    Write-Host "To verify status, run:"
    Write-Host "  Get-ScheduledTask -TaskName '$TaskName'"

} catch {
    Write-Fail "An unexpected error occurred during execution:"
    Write-Host $_.Exception.Message -ForegroundColor Red
}

# Prevent the elevated window from closing immediately so logs can be reviewed
Write-Host "`nPress Enter to close this window..."
Read-Host
