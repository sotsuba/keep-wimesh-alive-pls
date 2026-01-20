#!/bin/bash
# Uninstall Wimesh systemd service

set -e

BLUE='\033[0;34m'
YELLOW='\033[0;33m'
GREEN='\033[0;32m'
RED='\033[0;31m'
RESET='\033[0m'

info()  { echo -e "${BLUE}[INFO]${RESET} $*"; }
warn()  { echo -e "${YELLOW}[WARN]${RESET} $*"; }
error() { echo -e "${RED}[ERROR]${RESET} $*"; }
ok()    { echo -e "${GREEN}[OK]${RESET} $*"; }

SYSTEMD_DIR="/etc/systemd/system"
SERVICE_NAME="wimesh.service"

info "==================================="
info "Wimesh Service Uninstaller"
info "==================================="
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    error "This script must be run as root (use sudo)"
    exit 1
fi

# Check if service exists
if [ ! -f "$SYSTEMD_DIR/$SERVICE_NAME" ]; then
    warn "Service not installed"
    exit 0
fi

# Stop service if running
info "Stopping wimesh service..."
systemctl stop wimesh.service 2>/dev/null || true

# Disable service
info "Disabling wimesh service..."
systemctl disable wimesh.service 2>/dev/null || true

# Remove service file
info "Removing service file..."
rm -f "$SYSTEMD_DIR/$SERVICE_NAME"

# Reload systemd
info "Reloading systemd daemon..."
systemctl daemon-reload

echo ""
info "==================================="
ok "Uninstallation complete!"
info "==================================="
echo ""
info "The wimesh service has been removed."
info "You can still run wimesh manually:"
echo "  ./target/release/wimesh --daemon"
echo ""
