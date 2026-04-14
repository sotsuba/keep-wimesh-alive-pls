#!/bin/bash
set -euo pipefail

if [[ $EUID -ne 0 ]]; then
    echo "ERROR: must run as root (sudo ./uninstall.sh)" >&2
    exit 1
fi

echo "Uninstalling keep_wimesh_session..."

if systemctl is-active --quiet captive-login.service; then
    systemctl stop captive-login.service
    echo "  [OK] service stopped"
fi

if systemctl is-enabled --quiet captive-login.service 2>/dev/null; then
    systemctl disable captive-login.service
    echo "  [OK] service disabled"
fi

for F in \
    /etc/systemd/system/captive-login.service \
    /usr/local/bin/keep_wimesh_session \
    /tmp/wimesh_watchdog.lock
do
    if [[ -f "$F" ]]; then
        rm -f "$F"
        echo "  [OK] removed $F"
    fi
done

systemctl daemon-reload

echo ""
echo "Done."
