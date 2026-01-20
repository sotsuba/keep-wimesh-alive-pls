#!/bin/bash
# Wimesh Auto-Login Daemon Wrapper
# Simple wrapper to run wimesh in daemon mode

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WIMESH_BIN="${SCRIPT_DIR}/target/release/wimesh"

# Check if binary exists
if [ ! -f "$WIMESH_BIN" ]; then
    echo "Error: Wimesh binary not found at $WIMESH_BIN"
    echo "Please build it first: cargo build --release"
    exit 1
fi

# Run in daemon mode
cd "$SCRIPT_DIR" && exec "$WIMESH_BIN" --daemon