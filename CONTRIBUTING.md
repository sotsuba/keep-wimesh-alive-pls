# Cách thêm hotspot mới 

## 1. Thu thập log mạng

Kết nối vào hotspot, mở DevTools trình duyệt (F12) → tab **Network**, bật **Preserve log**.  
Thực hiện đăng nhập thủ công và ghi lại toàn bộ các request.  
Xuất file HAR để phân tích offline nếu cần.

## 2. Phân tích luồng đăng nhập

Xác định các bước chính:
- URL redirect ban đầu captive portal đưa về đâu.
- Các request trung gian (quảng cáo, token, v.v.).
- Request đăng nhập cuối cùng: method, URL, payload, cookie cần thiết.

Trước hết mô phỏng lại toàn bộ luồng. Sau đó tối ưu dần bằng cách bỏ qua các bước mà có khả năng không cần thiết và bỏ từng bước một thôi.


## 3. Triển khai strategy

1. Tạo `src/strategies/<tên>/mod.rs`, implement trait `LoginStrategy` (xem `src/strategies/hcmus/` cho luồng đơn giản, `src/strategies/wimesh/` cho luồng nhiều bước có cookie).
2. Khai báo `REGISTRY_ENTRY` với `name`, `predicate` (hàm khớp SSID), `factory`.
3. Thêm `pub mod <tên>;` và entry vào mảng `REGISTRY` trong `src/strategies/mod.rs`.
4. Viết test kiểm tra SSID khớp đúng strategy.

## 4. Kiểm thử

```bash
cargo run -- login "<SSID>"
```
## 5. Để hủy session sau khi đăng nhập thành công, chạy:

Captive portal ghi nhớ session qua MAC address. Để đổi MAC address, chạy:

```bash
    scripts/rotate_mac.sh 
```