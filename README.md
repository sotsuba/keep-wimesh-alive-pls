# keep_wimesh_session

Công cụ sẽ thực hiện các bước đăng nhập lằng nhằng vào `Free WiMesh` dùm bạn tại KTX Khu B - ĐHQG TP.HCM.

**Hostspot được hỗ trợ:**

- `Free-WiMesh` tại KTX Khu B.
- `HCMUS-STUDENT` tại CS2.
- `Highlands Coffee`.

## Miễn trừ trách nhiệm
Công cụ này được viết ra nhằm tự động hóa quy trình đăng nhập và bỏ qua quảng cáo của hệ thống WiMesh. Việc sử dụng tool có thể coi là hành vi lách quy trình vận hành thông thường của nhà cung cấp dịch vụ.

## Cài đặt

Yêu cầu: Rust toolchain.

### GNU/Linux

```bash
# Cài đặt
cargo build --release
sudo ./install.sh
# Gỡ cài đặt
sudo ./uninstall.sh
```

### Windows


```powershell
# Cài đặt
cargo build --release
.\install.ps1          
# Gỡ cài đặt
.\install.ps1 -Uninstall 
```