# Spec: 002-serial-logging (Ghi log qua cổng nối tiếp COM1)

- **Feature ID**: 002-serial-logging
- **Tiêu đề**: Ghi log qua cổng nối tiếp COM1
- **Trạng thái**: COMPLETE
- **Người phụ trách**: Kỹ sư trưởng AxiomOS
- **Ngày tạo**: 2026-07-06
- **Ngày cập nhật**: 2026-07-06

---

## Vấn đề cần giải quyết

Khi phát triển hệ điều hành từ đầu, việc chẩn đoán và theo dõi hoạt động của Kernel là cực kỳ quan trọng. Trước khi có hệ thống hiển thị màn hình (framebuffer) hoàn chỉnh, cổng nối tiếp (Serial Port) là kênh xuất thông tin duy nhất và đáng tin cậy nhất để gửi thông tin chẩn đoán từ máy ảo/máy thật ra môi trường host.

## Mục tiêu

- Cấu hình và khởi tạo cổng nối tiếp COM1 (địa chỉ cổng I/O mặc định `0x3F8`).
- Viết driver gửi ký tự qua cổng nối tiếp bằng phương pháp Polling (truy vấn vòng trạng thái thanh ghi).
- Định nghĩa macro định dạng log sớm (như `serial_print!` và `serial_println!`).
- Đưa thông tin log của Kernel ra stdout của trình giả lập QEMU.
- Định dạng log bắt buộc đầu ra chứa tiền tố `[AXIOMOS]`.

## Không thuộc phạm vi (Non-goals)

- Xây dựng driver serial hỗ trợ ngắt (Interrupt-driven Serial Driver) ở giai đoạn này (chỉ dùng Polling/Blocking write).
- Hỗ trợ ghi log ra các cổng COM khác ngoài COM1.
- Định tuyến log qua mạng hoặc lưu vào đĩa ảo ở cột mốc này.

## Ràng buộc

- Địa chỉ cổng COM1 được hardcode tại `0x3F8`.
- Giao tiếp với I/O port sử dụng các chỉ thị Assembly `in` và `out`. Có thể sử dụng crate `uart_16550` hoặc tự viết mã tương tác thanh ghi cổng I/O để tối ưu hóa tính độc lập.
- Không cấp phát bộ nhớ động (no allocation) trong quá trình ghi log.

## Dependencies

- Crate `uart_16550` (hoặc crate tương tác cổng I/O cơ bản như `x86_64` hoặc `cpu_io`) để viết driver COM1.

## ADR liên quan

- Chưa có.

## Public interfaces

- Các macro logging:
  - `serial_print!(format, ...)`
  - `serial_println!(format, ...)`
- Hàm khởi tạo:
  ```rust
  pub fn init_serial()
  ```

## Internal interfaces

- Hàm viết byte trực tiếp:
  ```rust
  fn write_byte(port: u16, byte: u8)
  ```

## Data structures

- Cấu trúc đại diện cho cổng Serial:
  ```rust
  pub struct SerialPort {
      port: u16,
  }
  ```

## Xử lý lỗi

- Nếu cổng serial chưa được khởi tạo, việc ghi log có thể bị bỏ qua hoặc dẫn đến vòng lặp vô hạn nếu thanh ghi truyền tin không bao giờ trống (ví dụ: chạy trên phần cứng không có cổng serial mà không giả lập).

## Hành vi logging

- Mọi log xuất ra phải có cấu trúc thống nhất. Các mức độ log cơ bản: `INFO`, `WARN`, `ERROR` dưới dạng text thô.

## Security considerations

- Cổng I/O `0x3F8` có thể được truy cập bởi bất kỳ mã Ring 0 nào. Không có nguy cơ bảo mật lớn trong môi trường thực thi thử nghiệm hiện tại.

## Kế hoạch test

- **Test case 1**: Gọi `init_serial()` và xuất ký tự thử nghiệm ra COM1.
- **Test case 2**: Xác minh QEMU bắt được dữ liệu nối tiếp từ COM1 và hiển thị ra màn hình host.

## Acceptance criteria

- **Acceptance Criterion 1**:
  - **Given** AxiomOS được khởi chạy trong QEMU với tùy chọn `-serial stdio`.
  - **When** Kernel entry point hoạt động và khởi tạo serial.
  - **Then** Trên terminal của máy host phải xuất hiện chính xác dòng chữ:
    `[AXIOMOS] Serial logger initialized`

## Kế hoạch rollback hoặc removal

- Không áp dụng.

## Bằng chứng hoàn tất

- Kernel khởi tạo COM1 qua module `drivers::serial`.
- QEMU headless với serial file đã ghi đủ boot sequence.
- Acceptance criterion đã được xác nhận bằng dòng serial bắt buộc:

```text
[AXIOMOS] Serial logger initialized
```
- Không còn `unwrap` hoặc `expect` trong kernel runtime path của serial logger.

## Câu hỏi mở

- Đã chốt dùng crate `uart_16550` cho COM1 polling logger ở milestone đầu.
