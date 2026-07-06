# Spec: 003-framebuffer-console (Framebuffer console tối thiểu)

- **Feature ID**: 003-framebuffer-console
- **Tiêu đề**: Framebuffer console tối thiểu
- **Trạng thái**: DRAFT
- **Người phụ trách**: Kỹ sư trưởng AxiomOS
- **Ngày tạo**: 2026-07-06
- **Ngày cập nhật**: 2026-07-06

---

## Vấn đề cần giải quyết

Sau Spec 001 và Spec 002, kernel boot được và ghi log qua COM1, nhưng cửa sổ QEMU vẫn đen vì chưa có đường xuất text qua framebuffer. Điều này làm người dùng khó xác nhận trạng thái boot nếu không đọc serial log.

## Mục tiêu

- Nhận framebuffer do Limine cung cấp.
- Vẽ text ASCII tối thiểu lên framebuffer trong QEMU.
- Hiển thị boot sequence đã có trên màn hình QEMU.
- Cho phép panic handler ghi thông báo ra framebuffer nếu framebuffer đã sẵn sàng.
- Giữ serial COM1 là kênh logging chính trong giai đoạn đầu.

## Không thuộc phạm vi

- Không xây dựng GUI, compositor, window manager hoặc desktop shell.
- Không hỗ trợ font phức tạp, Unicode đầy đủ, anti-aliasing hoặc text shaping.
- Không thêm GPU acceleration.
- Không thêm double buffering hoặc renderer phức tạp nếu chưa cần cho boot diagnostics.
- Không thay thế serial logger bằng framebuffer logger.

## Ràng buộc

- Chỉ dùng framebuffer do Limine bàn giao trong QEMU UEFI.
- Không allocation trong đường ghi text sớm.
- Không panic nếu framebuffer không tồn tại; kernel vẫn phải boot và log qua serial.
- Không truy cập hardware-specific type ngoài module boot/console phù hợp.
- Mọi unsafe block phải có safety comment.

## Dependencies

- Spec 001: boot qua Limine và kernel entry.
- Spec 002: serial logging để chẩn đoán khi framebuffer lỗi.
- Limine framebuffer request.

## ADR liên quan

- Chưa có. Nếu chọn font format, pixel format abstraction hoặc renderer dài hạn thì cần ADR riêng.

## Public interfaces

```rust
pub fn init_framebuffer_console(info: FramebufferInfo) -> Result<(), ConsoleError>;
pub fn framebuffer_print(args: core::fmt::Arguments);
pub fn framebuffer_println(args: core::fmt::Arguments);
```

## Internal interfaces

```rust
struct FramebufferInfo {
    address: *mut u8,
    width: usize,
    height: usize,
    pitch: usize,
    bytes_per_pixel: usize,
}

struct TextConsole {
    cursor_x: usize,
    cursor_y: usize,
    foreground: Color,
    background: Color,
}
```

## Data structures

- `FramebufferInfo`: metadata framebuffer nhận từ Limine.
- `TextConsole`: trạng thái cursor và màu text.
- `Color`: màu RGB tối thiểu cho text và nền.
- `ConsoleError`: lỗi init hoặc pixel format không hỗ trợ.

## Xử lý lỗi

- Nếu Limine không trả framebuffer, trả `ConsoleError::Unavailable` và tiếp tục serial-only.
- Nếu pixel format không hỗ trợ, trả lỗi và log qua serial.
- Nếu text vượt màn hình, thực hiện newline hoặc clear màn hình tối thiểu; scrolling đầy đủ có thể hoãn.

## Hành vi logging

- Serial vẫn in đầy đủ log.
- Framebuffer console chỉ mirror các dòng boot quan trọng và panic message nếu đã init thành công.
- Mọi lỗi init framebuffer phải log qua serial với prefix `[AXIOMOS]`.

## Security considerations

- Framebuffer là vùng memory-mapped I/O do bootloader cung cấp; ghi ngoài pitch/height có thể phá memory.
- Mọi phép tính offset phải kiểm tra giới hạn trước khi ghi.
- Không nhận input từ người dùng ở spec này.

## Kế hoạch test

- Build image và chạy QEMU với cửa sổ GTK/WSLg.
- Chụp screenshot QEMU để xác nhận màn hình không còn đen.
- Chạy QEMU headless để xác nhận serial output không bị hồi quy.

## Acceptance criteria

- **Acceptance Criterion 1**:
  - **Given** AxiomOS boot trong QEMU UEFI qua Limine.
  - **When** framebuffer console được khởi tạo thành công.
  - **Then** cửa sổ QEMU phải hiển thị dòng `[AXIOMOS] Kernel started`.

- **Acceptance Criterion 2**:
  - **Given** framebuffer không khả dụng hoặc pixel format không hỗ trợ.
  - **When** kernel boot.
  - **Then** kernel vẫn phải in boot sequence qua serial và không panic vì thiếu framebuffer.

- **Acceptance Criterion 3**:
  - **Given** panic xảy ra sau khi framebuffer console đã sẵn sàng.
  - **When** panic handler chạy.
  - **Then** panic message phải được ghi ra serial và framebuffer.

## Kế hoạch rollback hoặc removal

- Có thể tắt framebuffer console bằng feature flag hoặc bỏ init call, giữ serial logger làm đường chẩn đoán chính.
- Không thay đổi boot protocol khi rollback spec này.

## Câu hỏi mở

- Font bitmap sẽ được nhúng tĩnh hay viết glyph tối thiểu cho ASCII cần thiết?
- Có cần scrolling ngay ở milestone này hay chỉ clear/newline đủ cho boot diagnostics?
