# ==============================================================================
# CONFIGURATION & CONSTANTS
# ==============================================================================
$Binary       = "$env:LOCALAPPDATA\wimesh\keep_wimesh_session.exe"
$LogFile      = "$env:LOCALAPPDATA\wimesh\wimesh_ping.log"
$CheckUrl     = if ($env:WIMESH_CHECK_URL) { $env:WIMESH_CHECK_URL } else { "http://connectivitycheck.gstatic.com/generate_204" }
$CheckInterval     = 5
$PostLoginWait     = 5
$RetryBaseSeconds  = if ($env:WIMESH_RETRY_BASE_SECONDS) { [int]$env:WIMESH_RETRY_BASE_SECONDS } else { 10 }
$RetryMaxSeconds   = if ($env:WIMESH_RETRY_MAX_SECONDS)  { [int]$env:WIMESH_RETRY_MAX_SECONDS  } else { 120 }

# ==============================================================================
# GLOBAL STATE
# ==============================================================================
$script:ProbeLastErr    = "init"
$script:LoginFailCount  = 0

# ==============================================================================
# FUNCTIONS
# ==============================================================================
function Write-Log {
    param([string]$Message)
    $ts = (Get-Date).ToString("yyyy-MM-ddTHH:mm:sszzz")
    Add-Content -LiteralPath $LogFile -Value "$ts $Message"
}

function Get-DefaultGateway {
    $route = Get-NetRoute -DestinationPrefix "0.0.0.0/0" -ErrorAction SilentlyContinue |
             Sort-Object RouteMetric |
             Select-Object -First 1
    return $route?.NextHop
}

function Test-ProbeConnectivity {
    $gateway = Get-DefaultGateway
    if (-not $gateway) {
        $script:ProbeLastErr = "no_gateway"
        return $false
    }

    $ping = Test-Connection -ComputerName $gateway -Count 1 -TimeoutSeconds 1 -ErrorAction SilentlyContinue
    if (-not $ping) {
        $script:ProbeLastErr = "gateway_unreachable"
        return $false
    }

    try {
        $resp = Invoke-WebRequest -Uri $CheckUrl -UseBasicParsing `
                    -TimeoutSec 4 -MaximumRedirection 0 `
                    -ErrorAction SilentlyContinue -SkipHttpErrorCheck
        $code = [int]$resp.StatusCode

        if ($code -eq 204) {
            $script:ProbeLastErr = "ok"
            return $true
        } elseif ($code -eq 200 -or ($code -ge 300 -and $code -le 399)) {
            $script:ProbeLastErr = "captive_portal_intercept_http_$code"
            return $false
        } else {
            $script:ProbeLastErr = "http_$code"
            return $false
        }
    } catch {
        $msg = $_.Exception.Message
        if ($msg -match "name.*could not be resolved|DNS") {
            $script:ProbeLastErr = "dns_blocked_by_portal"
        } elseif ($msg -match "refused") {
            $script:ProbeLastErr = "connect_refused"
        } elseif ($msg -match "timed out|timeout") {
            $script:ProbeLastErr = "timeout"
        } else {
            $script:ProbeLastErr = "error"
        }
        return $false
    }
}

function Get-CurrentSsid {
    $out = netsh wlan show interfaces 2>$null
    foreach ($line in $out) {
        if ($line -match '^\s+SSID\s+:\s+(.+)$') {
            return $Matches[1].Trim()
        }
    }
    return $null
}

function Get-NextBackoffSeconds {
    param([int]$FailCount)
    $shift  = $FailCount - 1
    $delay  = $RetryBaseSeconds * [math]::Pow(2, $shift)
    if ($delay -ge $RetryMaxSeconds -or $delay -le 0) { return $RetryMaxSeconds }
    return [int]$delay
}

function Invoke-HandleLogin {
    param([string]$TargetSsid)

    Write-Log "connectivity lost; running login for ssid=$TargetSsid"
    & $Binary $TargetSsid >> $LogFile 2>&1
    $status = $LASTEXITCODE

    Write-Log "login exited with status $status"

    if ($status -eq 0) {
        $script:LoginFailCount = 0
        Start-Sleep -Seconds $PostLoginWait
    } else {
        $script:LoginFailCount++
        $backoff = Get-NextBackoffSeconds -FailCount $script:LoginFailCount
        Write-Log "login failed (count=$($script:LoginFailCount)); backing off ${backoff}s"
        Start-Sleep -Seconds $backoff
    }
}

# ==============================================================================
# MAIN ENTRY POINT
# ==============================================================================
$mutex = New-Object System.Threading.Mutex($false, "Global\wimesh_ping")
$acquired = $mutex.WaitOne(0)
if (-not $acquired) {
    Write-Log "already running"
    exit 0
}

try {
    Write-Log "watchdog started (pid $PID) check_url=$CheckUrl"

    while ($true) {
        if (Test-ProbeConnectivity) {
            $script:LoginFailCount = 0
            Start-Sleep -Seconds $CheckInterval
            continue
        }

        Write-Log "connectivity probe failed type=$($script:ProbeLastErr)"

        $ssid = Get-CurrentSsid
        if (-not $ssid) {
            Write-Log "connectivity lost; cannot determine active SSID, skipping login"
        } else {
            Invoke-HandleLogin -TargetSsid $ssid
        }

        Start-Sleep -Seconds $CheckInterval
    }
} finally {
    $mutex.ReleaseMutex()
    $mutex.Dispose()
}
