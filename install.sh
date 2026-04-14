#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [[ -f "$SCRIPT_DIR/keep_wimesh_session" ]]; then
    BINARY_SRC="$SCRIPT_DIR/keep_wimesh_session"
elif [[ -f "$SCRIPT_DIR/target/release/keep_wimesh_session" ]]; then
    BINARY_SRC="$SCRIPT_DIR/target/release/keep_wimesh_session"
else
    echo "ERROR: binary not found in current directory or target/release/" >&2
    exit 1
fi

BINARY_DEST="/usr/local/bin/keep_wimesh_session"
SERVICE_SRC="$SCRIPT_DIR/captive-login.service"
SERVICE_DEST="/etc/systemd/system/captive-login.service"

if [[ $EUID -ne 0 ]]; then
    echo "ERROR: must run as root (sudo ./install.sh)" >&2
    exit 1
fi

echo "Installing keep_wimesh_session..."

install -o root -g root -m 755 "$BINARY_SRC"  "$BINARY_DEST"
echo "  [OK] $BINARY_DEST"

install -o root -g root -m 644 "$SERVICE_SRC" "$SERVICE_DEST"
echo "  [OK] $SERVICE_DEST"

systemctl daemon-reload
systemctl enable --now captive-login.service
echo "  [OK] captive-login.service enabled and started"

echo ""
echo "Done. Check status:"
echo "  systemctl status captive-login"
echo "  journalctl -t captive-login -f"
