# Spec: 011-kernel-logging-subsystem (Hệ thống logging kernel có cấu trúc)

- **Feature ID**: 011-kernel-logging-subsystem
- **Tiêu đề**: Hệ thống logging kernel có cấu trúc
- **Trạng thái**: COMPLETE
- **Người phụ trách**: Kỹ sư trưởng AxiomOS
- **Ngày tạo**: 2026-07-07
- **Ngày cập nhật**: 2026-07-07

---

## Vấn đề cần giải quyết

Sau Milestone 3, kernel đã có nhiều điểm ghi log trực tiếp qua `serial_println!` và framebuffer console. Cách gọi phân tán này làm khó việc chuẩn hóa prefix, tách sink serial/framebuffer, thêm log level và chuẩn bị cho scheduler/timekeeping ở Milestone 4.

## Mục tiêu

- Thêm một facade logging tập trung trong kernel.
- Chuẩn hóa metadata tối thiểu gồm level, subsystem, message và chính sách mirror framebuffer.
- Giữ nguyên định dạng log boot/panic quan trọng để không phá boot test và bằng chứng QEMU hiện có.
- Không cấp phát động khi ghi log.
- Không thêm dependency mới.
- Cung cấp unit test cho formatter prefix.

## Không thuộc phạm vi

- Không thêm ring buffer log.
- Không thêm runtime log filtering.
- Không ghi log ra disk hoặc filesystem.
- Không thay thế toàn bộ log trong interrupt handler.
- Không thêm subsystem tracing hoặc telemetry.
- Không tuyên bố logging an toàn cho SMP.

## Ràng buộc

- Serial COM1 vẫn là sink chính trong giai đoạn này.
- Framebuffer chỉ là mirror tùy chọn và không được thay thế serial.
- Logging phải hoạt động trong `no_std`.
- Logging không được allocation trong kernel runtime path.
- Không dùng `unwrap` hoặc `expect` trong kernel runtime path.
- Không log liên tục trong interrupt handler.

## Dependencies

- Spec 002: Serial logging.
- Spec 003: Framebuffer console.
- Spec 005: Interrupts and exceptions.

## ADR liên quan

- [adr-004-kernel-logging-facade.md](../architecture/adr-004-kernel-logging-facade.md): Quyết định thêm logging facade tập trung, không thêm dependency mới.

## Public interfaces

```rust
pub enum LogLevel {
    Boot,
    Info,
    Warn,
    Error,
    Panic,
}

pub struct LogRecord<'a> {
    pub level: LogLevel,
    pub subsystem: Option<&'a str>,
    pub message: core::fmt::Arguments<'a>,
    pub mirror_framebuffer: bool,
}

pub fn write(record: LogRecord<'_>);
pub fn boot(message: &str);
pub fn panic(args: core::fmt::Arguments<'_>);
pub fn info(subsystem: &str, args: core::fmt::Arguments<'_>, mirror_framebuffer: bool);
```

## Internal interfaces

```rust
fn write_record(writer: &mut impl core::fmt::Write, record: &LogRecord<'_>) -> core::fmt::Result;
fn write_serial(record: &LogRecord<'_>);
fn write_framebuffer(record: &LogRecord<'_>);
```

## Data structures

- `LogLevel`: mức log tối thiểu, phục vụ formatter và mở đường cho filter sau này.
- `LogRecord`: bản ghi log không sở hữu dữ liệu, không allocation.
- Formatter prefix: render prefix `[AXIOMOS]`, `[AXIOMOS PANIC]` hoặc `[AXIOMOS <SUBSYSTEM>]` theo metadata.

## Xử lý lỗi

- Lỗi ghi serial được bỏ qua theo chính sách best-effort giống serial logging hiện tại.
- Lỗi ghi framebuffer không làm kernel panic.
- Formatter trả `fmt::Result` cho unit test và helper nội bộ, nhưng API public không panic khi format thất bại.

## Hành vi logging

- Boot diagnostics tiếp tục dùng prefix `[AXIOMOS]`.
- Panic diagnostics tiếp tục dùng prefix `[AXIOMOS PANIC]`.
- Subsystem diagnostics dùng prefix `[AXIOMOS <SUBSYSTEM>]`, ví dụ `[AXIOMOS TIMER]`.
- Không thêm log trong timer interrupt handler.

## Security considerations

- Log message có thể chứa dữ liệu debug nhạy cảm sau này; Milestone 4 chưa ghi log ra disk.
- Logging không được giữ lock lâu trong interrupt path.
- Không expose writer thô cho subsystem ngoài logging nếu không cần.
- Không claim logging thread-safe cho SMP; hiện chỉ dựa trên spinlock trong môi trường single-core QEMU.

## Kế hoạch test

- Unit test formatter cho boot prefix.
- Unit test formatter cho subsystem prefix.
- Unit test formatter cho panic prefix.
- Build kernel target `x86_64-unknown-none`.
- Boot QEMU và xác nhận serial vẫn có `[AXIOMOS] Kernel started`.

## Acceptance criteria

- **Acceptance Criterion 1**:
  - **Given** kernel gọi `logging::boot("Kernel started")`.
  - **When** formatter render record boot.
  - **Then** output phải là `[AXIOMOS] Kernel started`.

- **Acceptance Criterion 2**:
  - **Given** kernel gọi `logging::info("TIMER", format_args!("Ticks: {}", 100), false)`.
  - **When** formatter render record subsystem.
  - **Then** output phải là `[AXIOMOS TIMER] Ticks: 100`.

- **Acceptance Criterion 3**:
  - **Given** panic handler gọi `logging::panic(format_args!("{}", info))`.
  - **When** formatter render record panic.
  - **Then** output phải có prefix `[AXIOMOS PANIC]`.

- **Acceptance Criterion 4**:
  - **Given** AxiomOS boot trong QEMU.
  - **When** boot sequence chạy qua logging facade.
  - **Then** serial log vẫn phải có dòng `[AXIOMOS] Kernel started`.

## Kế hoạch rollback hoặc removal

- Có thể rollback về helper `boot_log` cục bộ trong `main.rs` và các macro `serial_println!` trực tiếp.
- Không được rollback bằng cách xóa serial boot diagnostics bắt buộc.

## Bằng chứng hoàn tất

- `cargo +nightly fmt --all --check` pass trong WSL.
- `cargo +nightly test --manifest-path kernel/Cargo.toml` pass với 5 unit tests, gồm 3 test formatter logging.
- `cargo +nightly build --manifest-path kernel/Cargo.toml --target x86_64-unknown-none` pass.
- `./scripts/build-image.sh` tạo thành công `target/axiomOS.img`.
- QEMU headless xác nhận serial log vẫn có:
  ```text
  [AXIOMOS] Kernel started
  [AXIOMOS TIMER] Ticks: 100
  ```

## Câu hỏi mở

- Runtime log level sẽ dùng compile-time feature hay cấu hình bootloader?
- Ring buffer log nên nằm trong memory subsystem hay logging subsystem?
- Khi scheduler xuất hiện, log lock contention sẽ được xử lý thế nào?
