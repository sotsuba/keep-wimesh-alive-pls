#!/bin/bash
# Install Wimesh systemd service

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

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SERVICE_TEMPLATE="${SCRIPT_DIR}/wimesh.service.template"
SERVICE_FILE="${SCRIPT_DIR}/wimesh.service"
SYSTEMD_DIR="/etc/systemd/system"
SERVICE_NAME="wimesh.service"

info "==================================="
info "Wimesh Service Installer"
info "==================================="
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    error "This script must be run as root (use sudo)"
    exit 1
fi

# Check if binary exists
WIMESH_BINARY="${SCRIPT_DIR}/target/release/wimesh"
if [ ! -f "$WIMESH_BINARY" ]; then
    error "Wimesh binary not found at: $WIMESH_BINARY"
    info "Please build it first: cargo build --release"
    exit 1
fi

# Check if template exists
if [ ! -f "$SERVICE_TEMPLATE" ]; then
    error "Service template not found: $SERVICE_TEMPLATE"
    exit 1
fi

# Get current user info
CURRENT_USER="${SUDO_USER:-$USER}"
CURRENT_GROUP=$(id -gn "$CURRENT_USER")

info "Configuration:"
echo "   User:        $CURRENT_USER"
echo "   Group:       $CURRENT_GROUP"
echo "   Working Dir: $SCRIPT_DIR"
echo "   Binary:      $WIMESH_BINARY"
echo ""

# Generate service file from template
info "Generating service file..."
sed -e "s|WIMESH_BINARY_PATH|${WIMESH_BINARY}|g" \
    -e "s|WIMESH_USER|${CURRENT_USER}|g" \
    -e "s|WIMESH_GROUP|${CURRENT_GROUP}|g" \
    -e "s|WIMESH_WORKDIR|${SCRIPT_DIR}|g" \
    "$SERVICE_TEMPLATE" > "$SERVICE_FILE"

# Copy service file to systemd
info "Installing service file..."
cp "$SERVICE_FILE" "$SYSTEMD_DIR/$SERVICE_NAME"
chmod 644 "$SYSTEMD_DIR/$SERVICE_NAME"

# Reload systemd
info "Reloading systemd daemon..."
systemctl daemon-reload

# Enable service
info "Enabling wimesh service..."
systemctl enable wimesh.service

echo ""
info "==================================="
ok "Installation complete!"
info "==================================="
echo ""
info "Service commands:"
echo "  Start:   sudo systemctl start wimesh"
echo "  Stop:    sudo systemctl stop wimesh"
echo "  Status:  sudo systemctl status wimesh"
echo "  Logs:    sudo journalctl -u wimesh -f"
echo "  Disable: sudo systemctl disable wimesh"
echo ""
info "Or use the service manager:"
echo "  ./service.sh start|stop|status|logs"
echo ""
info "To start the service now, run:"
echo "  sudo systemctl start wimesh"
echo ""
