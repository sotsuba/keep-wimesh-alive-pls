#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY_SRC="$SCRIPT_DIR/target/release/keep_wimesh_session"
BINARY_DEST="/usr/local/bin/keep_wimesh_session"
PING_SCRIPT_SRC="$SCRIPT_DIR/check_ping.sh"
PING_SCRIPT_DEST="/usr/local/bin/check_ping.sh"
SERVICE_SRC="$SCRIPT_DIR/wimesh-ping.service"
SERVICE_DEST="/etc/systemd/system/wimesh-ping.service"
NM_DISPATCHER_SRC="$SCRIPT_DIR/99-wimesh"
NM_DISPATCHER_DEST="/etc/NetworkManager/dispatcher.d/99-wimesh"

if [[ $EUID -ne 0 ]]; then
    echo "ERROR: must run as root (sudo ./install.sh)" >&2
    exit 1
fi

if [[ ! -f "$BINARY_SRC" ]]; then
    echo "ERROR: release binary not found at $BINARY_SRC" >&2
    echo "Run: cargo build --release" >&2
    exit 1
fi

echo "Installing keep_wimesh_session..."

install -o root -g root -m 755 "$BINARY_SRC"       "$BINARY_DEST"
echo "  [OK] $BINARY_DEST"

install -o root -g root -m 755 "$PING_SCRIPT_SRC"  "$PING_SCRIPT_DEST"
echo "  [OK] $PING_SCRIPT_DEST"

install -o root -g root -m 644 "$SERVICE_SRC"      "$SERVICE_DEST"
echo "  [OK] $SERVICE_DEST"

install -o root -g root -m 755 "$NM_DISPATCHER_SRC" "$NM_DISPATCHER_DEST"
echo "  [OK] $NM_DISPATCHER_DEST"

if [[ ! -f /var/log/wimesh_ping.log ]]; then
    touch /var/log/wimesh_ping.log && chmod 644 /var/log/wimesh_ping.log
    echo "  [OK] created /var/log/wimesh_ping.log"
fi

systemctl daemon-reload
systemctl enable --now wimesh-ping.service
echo "  [OK] wimesh-ping.service enabled and started"

echo ""
echo "Done. Check status:"
echo "  systemctl status wimesh-ping.service"
echo "  journalctl -u wimesh-ping.service -f"
