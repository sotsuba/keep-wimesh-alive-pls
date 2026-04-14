# keep_wimesh_session

**Hotspot được hỗ trợ:**

- `Free-WiMesh` tại KTX Khu B.
- `HCMUS-STUDENT` tại CS2 (chưa stable).
- `Highlands Coffee`.


## Trước khi làm gì khác, hãy đọc kỹ phần này
Captive Portal như các hotspot được hỗ trợ đều là open network, dữ liệu bạn gửi đi sẽ không được mã hóa. Do đó, hãy cẩn thận khi sử dụng các dịch vụ nhạy cảm như ngân hàng trực tuyến hoặc mua sắm trực tuyến khi kết nối với các mạng này.

Do đó, khuyến khích dùng VPN như WireGuard, OpenVPN khi sử dụng tool cũng như khi kết nối với mạng công cộng.

## Miễn trừ trách nhiệm
Tool được viết ra nhằm tự động hóa quy trình đăng nhập và bỏ qua quảng cáo của các Captive Portal. Việc sử dụng tool có thể coi là hành vi lách quy trình vận hành thông thường của nhà cung cấp dịch vụ.

## Cài đặt

### Từ bản release

Tải bản dựng sẵn từ [Releases](../../releases/latest) — không cần Rust toolchain.

**GNU/Linux**

```bash
tar -xzf keep_wimesh_session-linux-x86_64.tar.gz
# Cài đặt
sudo ./install.sh
# Gỡ cài đặt
sudo ./uninstall.sh
```

Kiểm tra trạng thái sau khi cài:

```bash
systemctl status captive-login
journalctl -t captive-login -f
```

**Windows**

```powershell
Expand-Archive keep_wimesh_session-windows-x86_64.zip .
.\install.ps1
# Gỡ cài đặt
.\install.ps1 -Uninstall
```

Kiểm tra logs sau khi cài:

```powershell
# Xem 50 dòng log cuối cùng (live update)
Get-Content -Path "$env:LOCALAPPDATA\wimesh\task.log" -Tail 50 -Wait

# Hoặc xem toàn bộ logs
Get-Content -Path "$env:LOCALAPPDATA\wimesh\task.log"

# Xóa logs cũ
Clear-Content -Path "$env:LOCALAPPDATA\wimesh\task.log"
```

---

### Tự build từ mã nguồn

Yêu cầu: Rust toolchain.

**GNU/Linux**

```bash
cargo build --release
sudo ./install.sh
# Gỡ cài đặt
sudo ./uninstall.sh
```

**Windows**

```powershell
cargo build --release
.\install.ps1
# Gỡ cài đặt
.\install.ps1 -Uninstall
```

## Đóng góp

Xem [CONTRIBUTING.md](CONTRIBUTING.md) để biết cách thêm hỗ trợ cho một captive portal mới.
