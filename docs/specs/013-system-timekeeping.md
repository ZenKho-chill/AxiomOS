# Spec: 013-system-timekeeping (Đồng hồ hệ thống và Timekeeping API)

- **Feature ID**: 013-system-timekeeping
- **Tiêu đề**: Đồng hồ hệ thống và Timekeeping API
- **Trạng thái**: COMPLETE
- **Người phụ trách**: Kỹ sư trưởng AxiomOS
- **Ngày tạo**: 2026-07-07
- **Ngày cập nhật**: 2026-07-07

---

## Vấn đề cần giải quyết

AxiomOS cần theo dõi thời gian hệ thống trôi qua (uptime, milliseconds, CPU ticks) để cung cấp các API đo lường thời gian và đặc biệt là API tạm dừng luồng (`sleep`) một cách chính xác cho scheduler ở các bước sau. Hiện tại ngắt Timer IRQ 0 chỉ tăng một biến đếm tick đơn giản mà không có lớp trừu tượng hóa và API rõ ràng.

## Mục tiêu

- Thiết lập lớp quản lý thời gian hệ thống (Timekeeper).
- Cung cấp API đọc thời gian hệ thống kể từ khi khởi động dưới dạng mili-giây (milliseconds) hoặc micro-giây (microseconds).
- Hiện thực API `sleep` không chặn (cooperative sleep) trả quyền kiểm soát lại cho scheduler khi task chưa hết thời gian chờ.
- Đồng bộ hóa an toàn biến đếm tick hệ thống khi đọc/ghi từ nhiều context khác nhau.

## Không thuộc phạm vi

- Không đồng bộ thời gian thực mạng (NTP).
- Không đọc Real-Time Clock (RTC) chip CMOS trong spec này (hoãn hỗ trợ đọc giờ thực của thế giới).
- Không hỗ trợ độ phân giải thời gian siêu cao (nanosecond-level) đòi hỏi HPET/TSC phức tạp ở milestone này.

## Ràng buộc

- Không sử dụng thư viện chuẩn `std`.
- Không cấp phát động (heap allocation) trong đường dẫn cập nhật thời gian.
- Biến đếm thời gian phải sử dụng kiểu dữ liệu nguyên tử hoặc khóa đồng bộ an toàn ngắt.

## Dependencies

- Spec 005: Interrupts and exceptions (đặc biệt là Timer interrupt IRQ 0).
- Spec 012: Synchronization primitives.

## ADR liên quan

- Chưa có ADR riêng. (Nếu cần chọn thiết bị đếm thời gian PIT vs HPET vs TSC, sẽ tạo ADR sau).

## Public interfaces

```rust
pub fn get_uptime_ms() -> u64;
pub fn get_uptime_ticks() -> u64;
pub fn sleep_ms(ms: u64);
```

## Internal interfaces

- Hàm `increment_ticks` được gọi từ Timer Interrupt Handler để cập nhật trạng thái thời gian.

## Data structures

- `Timekeeper`: Cấu trúc nội bộ chứa số tick hệ thống và các hằng số chuyển đổi sang mili-giây.

## Xử lý lỗi

- Lỗi tràn số (overflow) của biến đếm tick 64-bit được coi là không thể xảy ra trong thời gian vận hành thông thường (mất hàng triệu năm để tràn).

## Hành vi logging

- Không ghi log mỗi tick ngắt Timer. Chỉ ghi log khi khởi tạo thành công hệ thống Timekeeping.

## Security considerations

- Race condition: Đảm bảo thao tác đọc uptime từ các task không bị tranh chấp hoặc trả về giá trị không nhất quán khi ngắt Timer xảy ra giữa chừng.

## Kế hoạch test

- Unit test bộ chuyển đổi đơn vị ticks sang milliseconds.
- Kernel test gọi `sleep_ms(1000)` và đo lường khoảng thời gian trôi qua tương đối để xác minh tính chính xác.

## Acceptance criteria

- **Acceptance Criterion 1**:
  - **Given** hệ thống Timekeeper đã khởi chạy.
  - **When** đọc uptime hệ thống liên tiếp.
  - **Then** giá trị trả về sau phải lớn hơn hoặc bằng giá trị trước.

- **Acceptance Criterion 2**:
  - **Given** một task chạy kiểm thử.
  - **When** gọi `sleep_ms(500)`.
  - **Then** task đó phải bị dừng thực thi trong khoảng thời gian tương đương ít nhất 500 mili-giây trước khi tiếp tục.

## Kế hoạch rollback hoặc removal

- Có thể rollback về việc đọc trực tiếp biến đếm tĩnh không có wrapper API.
