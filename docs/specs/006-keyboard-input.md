# Spec: 006-keyboard-input (Đầu vào bàn phím PS/2)

- **Feature ID**: 006-keyboard-input
- **Tiêu đề**: Đầu vào bàn phím PS/2
- **Trạng thái**: DRAFT
- **Người phụ trách**: Kỹ sư trưởng AxiomOS
- **Ngày tạo**: 2026-07-06
- **Ngày cập nhật**: 2026-07-06

---

## Vấn đề cần giải quyết

AxiomOS cần nhận input cơ bản trong QEMU trước khi có shell hoặc UI. PS/2 keyboard là lựa chọn tối thiểu phù hợp milestone đầu vì QEMU hỗ trợ tốt và không cần USB stack.

## Mục tiêu

- Nhận scancode từ PS/2 keyboard trong QEMU.
- Chuyển scancode set cơ bản thành key event.
- Log key event qua serial và optionally framebuffer console.
- Cung cấp buffer input tối thiểu cho shell/userspace sau này.

## Không thuộc phạm vi

- Không hỗ trợ USB keyboard.
- Không hỗ trợ layout quốc tế đầy đủ.
- Không xử lý tổ hợp phím phức tạp ngoài modifier cơ bản.
- Không tạo terminal line editor hoàn chỉnh.
- Không thêm GUI input abstraction.

## Ràng buộc

- Không allocation trong interrupt handler.
- Không blocking trong keyboard interrupt handler.
- Không đọc I/O port ngoài module driver hoặc arch phù hợp.
- Buffer phải có giới hạn cố định và xử lý overflow rõ ràng.

## Dependencies

- Spec 005: interrupts and exceptions.
- Spec 002: serial logging.
- Spec 003: framebuffer console optional.

## ADR liên quan

- Chưa có. Nếu chọn input event model dài hạn thì cần ADR.

## Public interfaces

```rust
pub fn init_keyboard() -> Result<(), KeyboardError>;
pub fn poll_key_event() -> Option<KeyEvent>;
```

## Internal interfaces

```rust
struct KeyEvent {
    key_code: KeyCode,
    pressed: bool,
    modifiers: Modifiers,
}

struct KeyboardBuffer<const N: usize> {
    // ring buffer cố định cho key events
}
```

## Data structures

- `KeyCode`: mã phím nội bộ.
- `KeyEvent`: sự kiện press/release.
- `Modifiers`: trạng thái Shift/Ctrl/Alt tối thiểu.
- `KeyboardBuffer`: ring buffer không allocation.
- `KeyboardError`: lỗi init hoặc controller không phản hồi.

## Xử lý lỗi

- Nếu PS/2 controller không phản hồi trong QEMU, log lỗi và tiếp tục boot không keyboard.
- Nếu buffer đầy, drop event mới hoặc cũ theo chính sách được ghi trong code.
- Scancode không hỗ trợ phải log ở debug level hoặc bỏ qua an toàn.

## Hành vi logging

- Log khi keyboard init thành công.
- Log key event ở chế độ diagnostic ban đầu, có thể tắt sau khi shell dùng input.
- Không spam log trong interrupt handler nếu buffer overflow lặp lại.

## Security considerations

- Input là dữ liệu không tin cậy, không được dùng làm index trực tiếp vào bảng không kiểm tra.
- Interrupt handler phải ngắn và không cấp phát.
- Không xử lý command nguy hiểm từ keyboard ở milestone này.

## Kế hoạch test

- Chạy QEMU với PS/2 keyboard mặc định.
- Gửi phím từ cửa sổ QEMU và kiểm tra serial log.
- Unit test scancode translation bằng dữ liệu mẫu.
- Test buffer overflow bằng chuỗi key event giả lập.

## Acceptance criteria

- **Acceptance Criterion 1**:
  - **Given** AxiomOS boot trong QEMU và interrupts đã bật.
  - **When** người dùng nhấn phím `A`.
  - **Then** serial log phải ghi một key event tương ứng với phím `A`.

- **Acceptance Criterion 2**:
  - **Given** keyboard buffer đầy.
  - **When** interrupt handler nhận thêm scancode.
  - **Then** kernel không panic và overflow policy được áp dụng.

- **Acceptance Criterion 3**:
  - **Given** scancode chưa hỗ trợ.
  - **When** decoder nhận scancode đó.
  - **Then** decoder bỏ qua an toàn hoặc trả lỗi có kiểm soát.

## Kế hoạch rollback hoặc removal

- Có thể tắt `init_keyboard` và giữ hệ thống serial-only.
- Không được rollback bằng fake key event không đến từ QEMU input.

## Câu hỏi mở

- Sử dụng scancode set nào làm mặc định trong QEMU?
- Chính sách buffer overflow là drop newest hay drop oldest?
