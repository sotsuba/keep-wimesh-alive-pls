#!/bin/bash
set -euo pipefail

if [[ $EUID -ne 0 ]]; then
    echo "ERROR: must run as root (sudo ./uninstall.sh)" >&2
    exit 1
fi

echo "Uninstalling captive_portal..."

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
    /usr/local/bin/captive_portal \
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
