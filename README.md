# keep_wimesh_session

Công cụ sẽ thực hiện các bước đăng nhập lằng nhằng vào `Free WiMesh` dùm bạn tại KTX Khu B - ĐHQG TP.HCM.

## Miễn trừ trách nhiệm
Công cụ này được viết ra nhằm tự động hóa quy trình đăng nhập và bỏ qua quảng cáo của hệ thống WiMesh. Việc sử dụng tool có thể coi là hành vi lách quy trình vận hành thông thường của nhà cung cấp dịch vụ.

## Build & run

```bash
cargo build --release
./target/release/keep_wimesh_session
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
