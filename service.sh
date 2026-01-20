#!/bin/bash
# Wimesh Service Manager - Easy control script

SERVICE_NAME="wimesh.service"

show_usage() {
    echo "Wimesh Service Manager"
    echo ""
    echo "Usage: $0 {start|stop|restart|status|enable|disable|logs}"
    echo ""
    echo "Commands:"
    echo "  start    - Start the wimesh service"
    echo "  stop     - Stop the wimesh service"
    echo "  restart  - Restart the wimesh service"
    echo "  status   - Show service status"
    echo "  enable   - Enable service to start on boot"
    echo "  disable  - Disable service from starting on boot"
    echo "  logs     - Show service logs (live)"
    echo ""
}

check_sudo() {
    if [ "$EUID" -ne 0 ]; then 
        echo "[ERROR] This command requires sudo privileges"
        echo "Please run: sudo $0 $1"
        exit 1
    fi
}

case "$1" in
    start)
        check_sudo "start"
        echo "[INFO] Starting wimesh service..."
        systemctl start $SERVICE_NAME
        systemctl status $SERVICE_NAME --no-pager
        ;;
    stop)
        check_sudo "stop"
        echo "[INFO] Stopping wimesh service..."
        systemctl stop $SERVICE_NAME
        echo "[OK] Service stopped"
        ;;
    restart)
        check_sudo "restart"
        echo "[INFO] Restarting wimesh service..."
        systemctl restart $SERVICE_NAME
        systemctl status $SERVICE_NAME --no-pager
        ;;
    status)
        systemctl status $SERVICE_NAME --no-pager
        ;;
    enable)
        check_sudo "enable"
        echo "[INFO] Enabling wimesh service..."
        systemctl enable $SERVICE_NAME
        echo "Service will start automatically on boot"
        ;;
    disable)
        check_sudo "disable"
        echo "[INFO] Disabling wimesh service..."
        systemctl disable $SERVICE_NAME
        echo "Service will not start automatically on boot"
        ;;
    logs)
        echo "[INFO] Showing wimesh logs (Ctrl+C to exit)..."
        journalctl -u $SERVICE_NAME -f
        ;;
    *)
        show_usage
        exit 1
        ;;
esac
