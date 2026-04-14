# keep_wimesh_session

**Hostspot được hỗ trợ:**

- `Free-WiMesh` tại KTX Khu B.
- `HCMUS-STUDENT` tại CS2.
- `Highlands Coffee`.


## Trước khi làm gì khác, hãy đọc kỹ phần này
Captive Portal như các hotspot được hỗ trợ đều là open network, dữ liệu bạn gửi đi sẽ không được mã hóa. Do đó, hãy cẩn thận khi sử dụng các dịch vụ nhạy cảm như ngân hàng trực tuyến hoặc mua sắm trực tuyến khi kết nối với các mạng này.

Do đó, khuyến khích dùng VPN như WireGuard, OpenVPN khi sử dụng tool cũng như khi kết nối với mạng công cộng.

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

