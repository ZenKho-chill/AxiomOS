# Spec: 001-boot-and-kernel-entry (Khởi động và điểm vào Kernel)

- **Feature ID**: 001-boot-and-kernel-entry
- **Tiêu đề**: Khởi động và điểm vào Kernel (Limine boot handoff)
- **Trạng thái**: COMPLETE
- **Người phụ trách**: Kỹ sư trưởng AxiomOS
- **Ngày tạo**: 2026-07-06
- **Ngày cập nhật**: 2026-07-06

---

## Vấn đề cần giải quyết

Khởi động hệ điều hành bare-metal trên phần cứng x86_64 từ chế độ UEFI, chuyển giao CPU sang chế độ 64-bit Long Mode và chuyển quyền điều khiển an toàn sang mã nguồn Rust (hàm entry point của Kernel) mà không cần phụ thuộc vào BIOS cũ.

## Mục tiêu

- Sử dụng bootloader Limine để quản lý giai đoạn khởi động ban đầu.
- Cấu hình tệp tin cấu hình `limine.cfg` và nạp kernel ELF64.
- Định nghĩa hàm entry point của Kernel trong Rust (`no_std`, `no_main`) sử dụng giao thức Limine protocol.
- Đảm bảo Kernel có thể boot ổn định trong trình giả lập QEMU.

## Không thuộc phạm vi (Non-goals)

- Tự xây dựng một bootloader riêng từ đầu.
- Hỗ trợ chế độ boot BIOS kế thừa (legacy BIOS/CSM).
- Thiết lập GDT, IDT hoặc Paging tùy chỉnh ngay tại giai đoạn này (sử dụng cấu hình phân trang mặc định do Limine thiết lập).

## Ràng buộc

- Kernel phải được build với target `x86_64-unknown-none`.
- Entry point phải được định nghĩa bằng hàm `_start` không bao bọc bởi ABI mặc định (sử dụng extern "C" và attribute `#[no_mangle]`).
- Phải khai báo cấu trúc giao thức Limine (Limine Requests) để giao tiếp với bootloader.

## Dependencies

- Crate `limine` của Rust để định nghĩa các cấu trúc dữ liệu giao thức.
- Limine bootloader nhị phân (`limine.sys`, `BOOTX64.EFI`).

## ADR liên quan

- Chưa có.

## Public interfaces

- Điểm vào Kernel:
  ```rust
  #[no_mangle]
  pub extern "C" fn _start() -> !
  ```

## Internal interfaces

- Limine Handoff Requests:
  ```rust
  static FRAMEBUFFER_REQUEST: limine::request::FramebufferRequest = ...;
  ```

## Data structures

- Sử dụng các struct định nghĩa trong crate `limine` như `FramebufferRequest`, `Framebuffer`.

## Xử lý lỗi

- Nếu Limine không thể tìm thấy kernel hoặc cấu hình lỗi, CPU sẽ dừng (halt) hoặc restart do lỗi của bootloader.
- Nếu kernel entry thành công nhưng gặp lỗi không thể phục hồi trước khi serial logger sẵn sàng, kernel sẽ rơi vào vòng lặp vô hạn hoặc dừng CPU (`loop { hlt() }`).

## Hành vi logging

- Giai đoạn này chỉ hỗ trợ log ra màn hình qua Framebuffer tối giản (nếu có) hoặc dừng hệ thống khi hoàn thành. Việc ghi log thực tế sẽ được bàn giao cho module Serial Logger ở Spec 002.

## Security considerations

- Kernel chạy ở Ring 0 (đặc quyền cao nhất) ngay từ đầu. Chưa có cơ chế bảo vệ phân vùng bộ nhớ đặc quyền ở giai đoạn này.

## Kế hoạch test

- **Test case 1**: Tạo đĩa ảo chứa tệp tin `kernel.elf` và cấu hình Limine, chạy trên QEMU.
- **Test case 2**: Xác minh CPU không bị crash (triple fault) sau khi bootloader chuyển giao quyền điều khiển.

## Acceptance criteria

- **Acceptance Criterion 1**:
  - **Given** AxiomOS được đóng gói và nạp thông qua Limine trong QEMU.
  - **When** Máy ảo khởi động.
  - **Then** Trình giả lập QEMU chạy bình thường mà không tự động khởi động lại hay crash, và đi vào trạng thái dừng (halt) an toàn ở cuối hàm entry point.

## Kế hoạch rollback hoặc removal

- Cấu hình bootloader Limine rất khó thay thế mà không thay đổi toàn bộ mã nguồn entry point. Rollback sẽ đồng nghĩa với việc hoàn tác toàn bộ commits của Milestone 1.

## Bằng chứng hoàn tất

- `make image` đã tạo thành công `target/axiomOS.img` trong WSL Ubuntu.
- QEMU headless đã boot qua Limine và kernel đi vào vòng lặp halt an toàn.
- Serial log xác nhận kernel entry chạy đến cuối boot diagnostics:

```text
[AXIOMOS] Bootloader handoff complete
[AXIOMOS] Kernel started
[AXIOMOS] Serial logger initialized
[AXIOMOS] System halted
```

## Câu hỏi mở

- Đã chốt dùng Limine 7.x; binary hiện tại trong repository báo phiên bản 7.13.3.
