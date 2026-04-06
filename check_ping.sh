#!/bin/bash
# Ping watchdog for WiMesh captive portal session.
# Checks internet connectivity every 5 seconds.
# Runs the login binary only when session is detected as expired.

BINARY="/usr/local/bin/keep_wimesh_session"
LOGFILE="/var/log/wimesh_ping.log"
LOCKFILE="/tmp/wimesh_ping.lock"
INTERNET_HOST="8.8.8.8"
PORTAL_HOST="172.172.0.1"
CHECK_INTERVAL=5    # seconds between checks
POST_LOGIN_WAIT=10  # seconds to wait after login before rechecking

log() {
    echo "$(date -Iseconds) $*" >> "$LOGFILE"
}

# Single-instance guard
exec 9>"$LOCKFILE"
if ! flock -n 9; then
    log "already running, exiting"
    exit 0
fi

log "watchdog started (pid $$)"

while true; do
    if ping -c 1 -W 1 "$INTERNET_HOST" &>/dev/null; then
        : # internet OK — no log needed
    elif ping -c 1 -W 1 "$PORTAL_HOST" &>/dev/null; then
        log "session expired — running login binary"
        "$BINARY" >> "$LOGFILE" 2>&1
        log "binary exited $?"
        sleep "$POST_LOGIN_WAIT"
        if ping -c 1 -W 1 "$INTERNET_HOST" &>/dev/null; then
            log "post-login check: OK"
        else
            log "post-login check: STILL NO INTERNET"
        fi
    fi
    # else: portal not reachable → not on WiMesh, skip silently
    sleep "$CHECK_INTERVAL"
done
