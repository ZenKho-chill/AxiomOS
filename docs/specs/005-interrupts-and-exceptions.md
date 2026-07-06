# Spec: 005-interrupts-and-exceptions (Ngắt và ngoại lệ CPU)

- **Feature ID**: 005-interrupts-and-exceptions
- **Tiêu đề**: Ngắt và ngoại lệ CPU
- **Trạng thái**: DRAFT
- **Người phụ trách**: Kỹ sư trưởng AxiomOS
- **Ngày tạo**: 2026-07-06
- **Ngày cập nhật**: 2026-07-06

---

## Vấn đề cần giải quyết

Kernel cần xử lý CPU exceptions thay vì triple fault hoặc reboot im lặng. Đây là nền tảng bắt buộc trước khi thêm timer interrupt, keyboard interrupt, scheduler và driver model.

## Mục tiêu

- Thiết lập GDT tối thiểu nếu cần cho interrupt stack và segment state.
- Thiết lập IDT cho CPU exceptions x86_64.
- Log exception name, vector, error code và instruction pointer qua serial.
- Cung cấp interrupt stubs an toàn cho Rust handler.
- Đưa ra quyết định PIC hoặc APIC cho giai đoạn đầu.

## Không thuộc phạm vi

- Không triển khai preemptive scheduler.
- Không xử lý USB, network hoặc storage interrupt.
- Không hỗ trợ SMP/multiple CPU.
- Không thêm signal/userspace exception handling.

## Ràng buộc

- Không allocation trong interrupt handler.
- Không blocking trong interrupt handler.
- Assembly chỉ dùng cho interrupt stubs và CPU instruction đặc biệt.
- Mọi ABI assembly phải được tài liệu hóa.
- Exception không recoverable phải halt an toàn sau khi log.

## Dependencies

- Spec 001: kernel entry.
- Spec 002: serial logging.
- Spec 004: memory foundation nếu cần stack/descriptor allocation rõ ràng.

## ADR liên quan

- Cần ADR cho quyết định PIC/APIC nếu triển khai external interrupts.

## Public interfaces

```rust
pub fn init_interrupts() -> Result<(), InterruptError>;
pub fn enable_interrupts();
pub fn disable_interrupts();
```

## Internal interfaces

```rust
struct InterruptFrame {
    instruction_pointer: u64,
    code_segment: u64,
    cpu_flags: u64,
    stack_pointer: u64,
    stack_segment: u64,
}

type ExceptionHandler = extern "x86-interrupt" fn(InterruptFrame);
```

## Data structures

- `IdtEntry`: descriptor IDT x86_64.
- `InterruptFrame`: snapshot CPU khi exception xảy ra.
- `ExceptionVector`: enum vector exception.
- `InterruptError`: lỗi init descriptor hoặc vector.

## Xử lý lỗi

- Nếu IDT init thất bại, log qua serial và halt.
- Nếu exception không recoverable xảy ra, log context rồi halt.
- Nếu vector không có handler, dùng fallback handler log `Unhandled exception`.

## Hành vi logging

- Log khi IDT init thành công.
- Log mỗi exception với vector, tên, error code nếu có.
- Không log vòng lặp liên tục trong interrupt handler.

## Security considerations

- Handler sai ABI có thể phá stack.
- Không được để interrupt handler ghi vượt stack.
- Không expose raw interrupt mutation cho subsystem không liên quan.

## Kế hoạch test

- Boot QEMU và xác nhận IDT init log.
- Tạo test path gây breakpoint exception (`int3`) trong chế độ debug.
- Tạo page fault có kiểm soát sau khi memory subsystem đủ sẵn sàng.
- Xác nhận QEMU không triple fault khi exception test chạy.

## Acceptance criteria

- **Acceptance Criterion 1**:
  - **Given** AxiomOS boot trong QEMU.
  - **When** `init_interrupts` chạy.
  - **Then** serial log phải có dòng `[AXIOMOS] IDT initialized`.

- **Acceptance Criterion 2**:
  - **Given** breakpoint exception được kích hoạt có kiểm soát.
  - **When** handler chạy.
  - **Then** serial log phải ghi vector breakpoint và kernel không triple fault.

- **Acceptance Criterion 3**:
  - **Given** exception không recoverable xảy ra.
  - **When** fallback handler xử lý.
  - **Then** kernel phải log lỗi và halt an toàn.

## Kế hoạch rollback hoặc removal

- Có thể tắt init interrupts và quay về boot diagnostics serial-only.
- Không được giữ handler giả chỉ in success nếu IDT chưa thực sự load.

## Câu hỏi mở

- Milestone đầu dùng PIC legacy hay chuyển thẳng sang APIC?
- Có cần Interrupt Stack Table ngay trong spec này không?
