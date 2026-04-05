# keep_wimesh_session

The free WiFi at KTX Khu B ĐHQG-HCM runs on wi-mesh.com and is genuinely useful — but each session has a time limit, and every login makes you sit through a 5-second ad wait for no good reason. This tool automates the whole captive portal flow so reconnecting is instant and hands-free.

## How it works

1. Probes the network to trigger the captive portal redirect
2. Fetches login credentials from the Awing backend
3. Waits out the ad timer (configurable, default 7 s)
4. POSTs the final login to the MikroTik hotspot

## Build & run

```bash
cargo build --release
./target/release/keep_wimesh_session
```

Dry run (skips the final login POST):

```bash
./target/release/keep_wimesh_session --dry-run
```

## NetworkManager integration

Auto-login whenever you connect to the hotspot:

```bash
sudo cp 99-wimesh /etc/NetworkManager/dispatcher.d/
sudo chmod 755 /etc/NetworkManager/dispatcher.d/99-wimesh
sudo chown root:root /etc/NetworkManager/dispatcher.d/99-wimesh
```

Edit `TARGET_SSID` in the script to match your connection profile name (`nmcli connection show`).

## Options

| Flag | Default | Description |
|------|---------|-------------|
| `--probe-url` | `http://login.net.vn/` | URL used to trigger portal redirect |
| `--device-name` | `MyDevice` | Device name sent to the backend |
| `--ad-wait-seconds` | `7` | Seconds to wait before final login |
| `--place-id` | *(venue default)* | Awing place ID |
| `--domain-id` | *(venue default)* | Awing domain ID |
| `--dry-run` | false | Skip the final login POST |

## Logging

```bash
RUST_LOG=debug ./target/release/keep_wimesh_session
```
