#!/bin/bash

# ==============================================================================
# CONFIGURATION & CONSTANTS
# ==============================================================================
readonly BINARY="/usr/local/bin/keep_wimesh_session"
readonly LOGFILE="/var/log/wimesh_ping.log"
readonly LOCKFILE="/tmp/wimesh_ping.lock"

readonly CHECK_URL="${WIMESH_CHECK_URL:-http://connectivitycheck.gstatic.com/generate_204}"

readonly CHECK_INTERVAL=5
readonly POST_LOGIN_WAIT=5
readonly RETRY_BASE_SECONDS="${WIMESH_RETRY_BASE_SECONDS:-10}"
readonly RETRY_MAX_SECONDS="${WIMESH_RETRY_MAX_SECONDS:-120}"

# ==============================================================================
# GLOBAL STATE
# ==============================================================================
PROBE_LAST_ERR="init"
LOGIN_FAIL_COUNT=0

# ==============================================================================
# FUNCTIONS
# ==============================================================================
log() {
  printf '%(%Y-%m-%dT%H:%M:%S%z)T %s\n' -1 "$*" >> "$LOGFILE"
}

probe_connectivity() {
  local gateway http_code rc

  gateway=$(ip -4 route show default | awk '/default/ {print $3; exit}')
  if [[ -z "$gateway" ]]; then
    PROBE_LAST_ERR="no_gateway"
    return 1
  fi

  if ! ping -c 1 -W 1 "$gateway" >/dev/null 2>&1; then
    PROBE_LAST_ERR="gateway_unreachable"
    return 1
  fi

  http_code="$(curl -4 --silent --connect-timeout 2 --max-time 4 --output /dev/null \
    --write-out '%{http_code}' --proto '=http' "$CHECK_URL" 2>/dev/null)"
  rc=$?

  if [[ "$rc" -eq 0 ]]; then
    if [[ "$http_code" == "204" ]]; then
      PROBE_LAST_ERR="ok"
      return 0
    elif [[ "$http_code" == "200" || "$http_code" =~ ^30[0-9]$ ]]; then
      PROBE_LAST_ERR="captive_portal_intercept_http_${http_code}"
      return 1
    else
      PROBE_LAST_ERR="http_${http_code}"
      return 1
    fi
  fi

  case "$rc" in
    6)  PROBE_LAST_ERR="dns_blocked_by_portal" ;;
    7)  PROBE_LAST_ERR="connect_refused" ;;
    28) PROBE_LAST_ERR="timeout" ;;
    35) PROBE_LAST_ERR="tls_error" ;;
    *)  PROBE_LAST_ERR="curl_${rc}" ;;
  esac
  return 1
}

detect_current_ssid() {
  iw dev 2>/dev/null | awk '/ssid/ { sub(/^[ \t]*ssid[ \t]+/, ""); print; exit }'
}

next_backoff_seconds() {
  local fail_count="$1"
  
  local shift_amount=$(( fail_count - 1 ))
  local delay=$(( RETRY_BASE_SECONDS << shift_amount ))

  if (( delay >= RETRY_MAX_SECONDS || delay <= 0 )); then
    echo "$RETRY_MAX_SECONDS"
  else
    echo "$delay"
  fi
}

handle_login() {
  local target_ssid="$1"
  local login_status
  local backoff_seconds

  log "connectivity lost; running login for ssid=$target_ssid"
  "$BINARY" "$target_ssid" >> "$LOGFILE" 2>&1
  login_status=$?

  log "login exited with status $login_status"

  if [[ "$login_status" -eq 0 ]]; then
    LOGIN_FAIL_COUNT=0
    sleep "$POST_LOGIN_WAIT"
  else
    LOGIN_FAIL_COUNT=$(( LOGIN_FAIL_COUNT + 1 ))
    backoff_seconds="$(next_backoff_seconds "$LOGIN_FAIL_COUNT")"
    log "login failed (count=$LOGIN_FAIL_COUNT); backing off ${backoff_seconds}s"
    sleep "$backoff_seconds"
  fi
}

# ==============================================================================
# MAIN ENTRY POINT
# ==============================================================================
main() {
  exec 9>"$LOCKFILE" || exit 0
  flock -n 9 || { log "already running"; exit 0; }
  trap 'flock -u 9' EXIT

  log "watchdog started (pid $$) ssid_override=${TARGET_SSID_OVERRIDE:-auto} check_url=$CHECK_URL"

  while true; do
    if probe_connectivity; then
      LOGIN_FAIL_COUNT=0
      sleep "$CHECK_INTERVAL"
      continue
    fi

    log "connectivity probe failed type=$PROBE_LAST_ERR"

    local current_target_ssid
    current_target_ssid=$(detect_current_ssid)

    if [[ -z "$current_target_ssid" ]]; then
      log "connectivity lost; cannot determine active SSID, skipping login"
    else
      handle_login "$current_target_ssid"
    fi

    sleep "$CHECK_INTERVAL"
  done
}

main "$@"