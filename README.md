# keep_wimesh_session

Công cụ sẽ thực hiện các bước đăng nhập lằng nhằng vào `Free WiMesh` dùm bạn tại KTX Khu B - ĐHQG TP.HCM.

Hostspot được hỗ trợ:

- `Free-WiMesh` tại KTX Khu B.
- `HCMUS-STUDENT` tại CS2.

## Miễn trừ trách nhiệm
Công cụ này được viết ra nhằm tự động hóa quy trình đăng nhập và bỏ qua quảng cáo của hệ thống WiMesh. Việc sử dụng tool có thể coi là hành vi lách quy trình vận hành thông thường của nhà cung cấp dịch vụ.

## Build & run

```bash
cargo build --release
./target/release/keep_wimesh_session 1.Free-WiMesh
```

## Cài đặt tự động

Tự động đăng nhập lại khi vào mạng hoặc khi session hết hạn (kiểm tra mỗi 5 giây).

```bash
cargo build --release
sudo ./install.sh
```

Gỡ cài đặt:

```bash
sudo ./uninstall.sh
```

> Chỉnh `TARGET_SSID` trong `99-wimesh` thành tên wifi nếu cần (mặc định `1.Free WiMesh`).

## Tinh chinh watchdog service

`wimesh-ping.service` + `wimesh_ping_check.sh` ho tro da SSID theo co che:

- Neu set `WIMESH_SSID`, script se dung SSID nay (override).
- Neu khong set, script tu dong lay SSID dang ket noi qua NetworkManager.
- Script chi thu login voi SSID khop regex `WIMESH_SUPPORTED_SSID_REGEX` (mac dinh: `WiMesh|HCMUS-STUDENT|HCMUS-PUBLIC`).

Co the override cac bien moi truong trong service:

```ini
Environment=WIMESH_CHECK_URL=http://connectivitycheck.gstatic.com/generate_204
Environment=WIMESH_SSID=
Environment=WIMESH_SUPPORTED_SSID_REGEX=(WiMesh|HCMUS-STUDENT|HCMUS-PUBLIC)
Environment=WIMESH_RETRY_BASE_SECONDS=10
Environment=WIMESH_RETRY_MAX_SECONDS=120
```

Sau khi sua service:

```bash
sudo systemctl daemon-reload
sudo systemctl restart wimesh-ping
sudo journalctl -u wimesh-ping -f
```
