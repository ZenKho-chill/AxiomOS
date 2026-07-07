# ADR-004: Logging facade tập trung cho kernel

- **Trạng thái**: APPROVED
- **Người phụ trách**: Kỹ sư trưởng AxiomOS
- **Ngày tạo**: 2026-07-07
- **Ngày cập nhật**: 2026-07-07

---

## Bối cảnh và vấn đề cần giải quyết

Từ Milestone 1 đến Milestone 3, kernel ghi log trực tiếp qua `serial_println!` và một số chỗ mirror sang framebuffer. Cách này đủ cho boot diagnostics, nhưng khi bước sang Milestone 4, scheduler, timekeeping và synchronization cần một điểm logging thống nhất để tránh format phân tán và để chuẩn bị cho filter/ring buffer sau này.

## Các phương án cân nhắc

### Phương án A: Giữ nguyên macro serial trực tiếp

- **Ưu điểm**: Không thay đổi code hiện có, ít rủi ro.
- **Nhược điểm**: Prefix và sink phân tán; khó thêm log level, mirror policy và test formatter.

### Phương án B: Thêm crate logging bên ngoài

- **Ưu điểm**: Có API quen thuộc và có thể mở rộng.
- **Nhược điểm**: Thêm dependency mới vào kernel, cần đánh giá tương thích `no_std` và chính sách allocation.

### Phương án C: Tự viết facade nhỏ trong kernel

- **Ưu điểm**: Không thêm dependency, giữ kiểm soát `no_std`, không allocation và có thể giữ nguyên format boot hiện tại.
- **Nhược điểm**: Chưa có filter/ring buffer đầy đủ; cần mở rộng dần khi scheduler xuất hiện.

## Quyết định lựa chọn

Chọn **Phương án C: tự viết facade nhỏ trong kernel**.

Facade logging đầu tiên sẽ có:

- `LogLevel` tối thiểu (`Boot`, `Info`, `Warn`, `Error`, `Panic`).
- `LogRecord` không sở hữu dữ liệu và không cấp phát.
- Sink serial chính và framebuffer mirror tùy chọn.
- Formatter prefix có unit test.
- Định dạng legacy được giữ cho boot/panic/subsystem log để không phá bằng chứng QEMU và CI.

## Hệ quả và ảnh hưởng

- **Không thêm dependency mới**.
- **Không thay đổi ABI userspace** vì đây là internal kernel API.
- **Không thay đổi boot protocol**.
- Các log mới nên đi qua `kernel::logging` thay vì tự thêm helper cục bộ.
- Runtime filtering và ring buffer vẫn là công việc tương lai của Milestone 4, chưa được claim hoàn tất trong ADR này.
