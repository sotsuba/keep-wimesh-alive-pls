#!/bin/bash
log() { echo "$(date -Iseconds) $*" >> "$LOGFILE"; }
readonly BINARY="/usr/local/bin/keep_wimesh_session" LOGFILE="/var/log/wimesh_ping.log"
readonly LOCKFILE="/tmp/wimesh_ping.lock" CHECK_HOST="https://www.google.com"
readonly CHECK_INTERVAL=5 POST_LOGIN_WAIT=10
exec 9>"$LOCKFILE" || exit 0
flock -n 9 || { log "already running"; exit 0; }
trap "flock -u 9" EXIT
log "watchdog started (pid $$)"
while true; do
  curl -sS -m 2 -o/dev/null "$CHECK_HOST" && { sleep "$CHECK_INTERVAL"; continue; }
  log "connectivity lost — running login"
  "$BINARY" >> "$LOGFILE" 2>&1
  log "login exited with status $?"
  sleep "$POST_LOGIN_WAIT"
  curl -sS -m 2 -o/dev/null "$CHECK_HOST" && log "recovered" || log "still offline"
  sleep "$CHECK_INTERVAL"
done
