# Spec: 010-userspace-init (Tiến trình init userspace tối thiểu)

- **Feature ID**: 010-userspace-init
- **Tiêu đề**: Tiến trình init userspace tối thiểu
- **Trạng thái**: DRAFT
- **Người phụ trách**: Kỹ sư trưởng AxiomOS
- **Ngày tạo**: 2026-07-06
- **Ngày cập nhật**: 2026-07-06

---

## Vấn đề cần giải quyết

Sau khi kernel có filesystem, ELF loader và scheduler, hệ thống cần tiến trình userspace đầu tiên để chứng minh boundary kernel/userspace và làm nền cho shell tối thiểu.

## Mục tiêu

- Đóng gói binary `userspace/init` vào disk image.
- Kernel tìm và nạp init ELF từ FAT32.
- Tạo process/task đầu tiên cho init.
- Thiết lập syscall ABI tối thiểu hoặc exit/yield path tùy thiết kế đã được duyệt.
- Log lifecycle init qua serial.

## Không thuộc phạm vi

- Không chạy phần mềm Linux hoặc Windows.
- Không xây dựng shell đầy đủ trong spec này.
- Không hỗ trợ multi-user, permission model hoặc package manager.
- Không triển khai GUI.
- Không hỗ trợ dynamic linking.

## Ràng buộc

- ABI kernel/userspace phải được ghi trong `docs/design/kernel-api.md` trước khi COMPLETE.
- Không map kernel memory writable vào userspace.
- Không dùng fake init chỉ là kernel function giả danh userspace.
- Không claim userspace isolation hoàn chỉnh nếu paging/user mode chưa đủ.

## Dependencies

- Spec 004: memory management và address space.
- Spec 007: FAT32 read-only.
- Spec 008: ELF loader.
- Spec 009: scheduler/process model.

## ADR liên quan

- Cần ADR cho syscall ABI và userspace address layout trước implementation.

## Public interfaces

```rust
pub fn spawn_init(path: &Path) -> Result<ProcessId, InitError>;
pub fn enter_userspace(process: ProcessId) -> !;
```

## Internal interfaces

```rust
struct InitConfig {
    path: &'static str,
    argv: &'static [&'static str],
}

struct UserStackLayout;
struct SyscallFrame;
```

## Data structures

- `InitConfig`: đường dẫn và tham số init.
- `ProcessImage`: ELF đã load và metadata address space.
- `UserStackLayout`: bố trí stack ban đầu.
- `InitError`: lỗi file, ELF, memory, scheduler hoặc ABI.

## Xử lý lỗi

- Nếu không tìm thấy init, log lỗi và halt an toàn.
- Nếu init ELF không hợp lệ, trả `InitError::InvalidImage` và halt an toàn.
- Nếu init exit, kernel log exit code và chuyển sang idle/halt path.

## Hành vi logging

- Log đường dẫn init được nạp.
- Log PID init.
- Log exit code hoặc lỗi khi init không chạy được.
- Không log nội dung memory userspace.

## Security considerations

- Init là userspace đầu tiên nhưng vẫn là input không tin cậy từ disk image.
- Syscall boundary phải validate pointer và length nếu được thêm.
- Không cấp quyền kernel cho init.
- Không tuyên bố isolation production-grade ở milestone này.

## Kế hoạch test

- Build userspace init ELF tối thiểu.
- Đóng gói init vào FAT32 image.
- QEMU boot test xác nhận kernel load và transfer control hoặc spawn init.
- Test init exit path với exit code cố định.
- Test thiếu init file để xác nhận lỗi có kiểm soát.

## Acceptance criteria

- **Acceptance Criterion 1**:
  - **Given** disk image chứa `userspace/init` ELF hợp lệ.
  - **When** kernel gọi `spawn_init`.
  - **Then** serial log phải ghi PID của init process.

- **Acceptance Criterion 2**:
  - **Given** init ELF chạy và gọi exit với mã `0`.
  - **When** kernel nhận exit.
  - **Then** kernel phải log `[AXIOMOS] init exited with code 0`.

- **Acceptance Criterion 3**:
  - **Given** disk image thiếu init file.
  - **When** kernel boot đến bước spawn init.
  - **Then** kernel phải log lỗi rõ ràng và halt an toàn.

## Kế hoạch rollback hoặc removal

- Có thể rollback bằng cách không spawn init và quay về kernel diagnostics halt.
- Không được thay init bằng function kernel giả để claim userspace đã chạy.

## Câu hỏi mở

- Syscall tối thiểu đầu tiên là `exit`, `write`, hay `yield`?
- Init path chuẩn là `/bin/init`, `/init.elf`, hay `/system/init`?
