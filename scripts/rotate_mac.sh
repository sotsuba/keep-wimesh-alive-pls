#!/bin/bash

if [[ $EUID -ne 0 ]]; then
  echo "Please run this script as admin/root."
  echo "Example: sudo $0"
  exit 1
fi

DEVICE=$(iw dev | awk '$1=="Interface"{print $2; exit}')

if [[ -z "$DEVICE" ]]; then
  echo "No wireless interface found."
  exit 1
fi

echo "Rotating MAC on $DEVICE..."
ip link set "$DEVICE" down
macchanger -r "$DEVICE"
ip link set "$DEVICE" up
