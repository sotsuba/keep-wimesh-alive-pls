#!/bin/bash
set -euo pipefail

if [[ $EUID -ne 0 ]]; then
    echo "ERROR: must run as root (sudo ./uninstall.sh)" >&2
    exit 1
fi

echo "Uninstalling keep_wimesh_session..."

if systemctl is-active --quiet wimesh-ping.service; then
    systemctl stop wimesh-ping.service
    echo "  [OK] service stopped"
fi

if systemctl is-enabled --quiet wimesh-ping.service 2>/dev/null; then
    systemctl disable wimesh-ping.service
    echo "  [OK] service disabled"
fi

for F in \
    /etc/systemd/system/wimesh-ping.service \
    /usr/local/bin/keep_wimesh_session \
    /usr/local/bin/check_ping.sh \
    /etc/NetworkManager/dispatcher.d/99-wimesh \
    /tmp/wimesh_ping.lock
do
    if [[ -f "$F" ]]; then
        rm -f "$F"
        echo "  [OK] removed $F"
    fi
done

systemctl daemon-reload

echo ""
echo "Done. Log file kept at:"
echo "  /var/log/wimesh_ping.log"
echo "Remove manually if desired: sudo rm /var/log/wimesh_ping.log"
