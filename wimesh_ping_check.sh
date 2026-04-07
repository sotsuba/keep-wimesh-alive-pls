#!/bin/bash
log() { echo "$(date -Iseconds) $*" >> "$LOGFILE"; }
readonly BINARY="/usr/local/bin/keep_wimesh_session" LOGFILE="/var/log/wimesh_ping.log"
readonly LOCKFILE="/tmp/wimesh_ping.lock"
readonly CHECK_URL="${WIMESH_CHECK_URL:-http://connectivitycheck.gstatic.com/generate_204}"
readonly TARGET_SSID_OVERRIDE="${WIMESH_SSID:-}"
readonly SUPPORTED_SSID_REGEX="${WIMESH_SUPPORTED_SSID_REGEX:-(WiMesh|HCMUS-STUDENT|HCMUS-PUBLIC)}"
readonly CHECK_INTERVAL=5 POST_LOGIN_WAIT=10
readonly RETRY_BASE_SECONDS="${WIMESH_RETRY_BASE_SECONDS:-10}"
readonly RETRY_MAX_SECONDS="${WIMESH_RETRY_MAX_SECONDS:-120}"

PROBE_LAST_ERR="init"
LOGIN_FAIL_COUNT=0

probe_connectivity() {
  local http_code
  http_code="$(curl --silent --connect-timeout 2 --max-time 4 --output /dev/null \
    --write-out '%{http_code}' --proto '=http' "$CHECK_URL" 2>/dev/null)"
  local rc=$?
  if [[ "$rc" -eq 0 && "$http_code" == "204" ]]; then
    PROBE_LAST_ERR="ok"
    return 0
  fi

  case "$rc" in
    0) PROBE_LAST_ERR="http_${http_code:-000}" ;;
    6) PROBE_LAST_ERR="dns" ;;
    7) PROBE_LAST_ERR="connect" ;;
    28) PROBE_LAST_ERR="timeout" ;;
    35) PROBE_LAST_ERR="tls" ;;
    *) PROBE_LAST_ERR="curl_${rc}" ;;
  esac
  return "$rc"
}

detect_current_ssid() {
  nmcli -t -f ACTIVE,SSID dev wifi 2>/dev/null | awk -F: '$1=="yes" {print $2; exit}'
}

pick_target_ssid() {
  if [[ -n "$TARGET_SSID_OVERRIDE" ]]; then
    printf '%s\n' "$TARGET_SSID_OVERRIDE"
    return 0
  fi

  detect_current_ssid
}

is_supported_ssid() {
  local ssid="$1"
  [[ "$ssid" =~ $SUPPORTED_SSID_REGEX ]]
}

next_backoff_seconds() {
  local fail_count="$1"
  local delay="$RETRY_BASE_SECONDS"
  local i
  for ((i=1; i<fail_count; i++)); do
    delay=$((delay * 2))
    if ((delay >= RETRY_MAX_SECONDS)); then
      echo "$RETRY_MAX_SECONDS"
      return
    fi
  done
  echo "$delay"
}

exec 9>"$LOCKFILE" || exit 0
flock -n 9 || { log "already running"; exit 0; }
trap "flock -u 9" EXIT
log "watchdog started (pid $$) ssid_override=${TARGET_SSID_OVERRIDE:-auto} check_url=$CHECK_URL ssid_regex=$SUPPORTED_SSID_REGEX"
while true; do
  if probe_connectivity; then
    LOGIN_FAIL_COUNT=0
    sleep "$CHECK_INTERVAL"
    continue
  fi

  log "connectivity probe failed type=$PROBE_LAST_ERR"
  TARGET_SSID="$(pick_target_ssid)"
  if [[ -z "$TARGET_SSID" ]]; then
    log "connectivity lost; cannot determine active SSID, skipping login"
    sleep "$CHECK_INTERVAL"
    continue
  fi

  if ! is_supported_ssid "$TARGET_SSID"; then
    log "connectivity lost; ssid '$TARGET_SSID' not in supported regex '$SUPPORTED_SSID_REGEX', skipping login"
    sleep "$CHECK_INTERVAL"
    continue
  fi

  log "connectivity lost; running login for ssid=$TARGET_SSID"
  "$BINARY" "$TARGET_SSID" >> "$LOGFILE" 2>&1
  LOGIN_STATUS=$?
  log "login exited with status $LOGIN_STATUS"

  if [[ "$LOGIN_STATUS" -eq 0 ]]; then
    LOGIN_FAIL_COUNT=0
    sleep "$POST_LOGIN_WAIT"
  else
    LOGIN_FAIL_COUNT=$((LOGIN_FAIL_COUNT + 1))
    BACKOFF_SECONDS="$(next_backoff_seconds "$LOGIN_FAIL_COUNT")"
    log "login failed (count=$LOGIN_FAIL_COUNT); backing off ${BACKOFF_SECONDS}s"
    sleep "$BACKOFF_SECONDS"
  fi

  probe_connectivity && log "recovered" || log "still offline"
  sleep "$CHECK_INTERVAL"
done
