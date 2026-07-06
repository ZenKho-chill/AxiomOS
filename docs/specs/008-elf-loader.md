# Spec: 008-elf-loader (Bộ nạp ELF64)

- **Feature ID**: 008-elf-loader
- **Tiêu đề**: Bộ nạp ELF64
- **Trạng thái**: DRAFT
- **Người phụ trách**: Kỹ sư trưởng AxiomOS
- **Ngày tạo**: 2026-07-06
- **Ngày cập nhật**: 2026-07-06

---

## Vấn đề cần giải quyết

Để chạy userspace, kernel cần đọc file ELF64, kiểm tra header, map các segment hợp lệ vào address space và xác định entry point. Đây là bước nối giữa filesystem, memory management và process model.

## Mục tiêu

- Parse ELF64 little-endian cho x86_64.
- Validate ELF header và program headers.
- Load `PT_LOAD` segments vào memory được cấp phát.
- Trả metadata entry point và layout chương trình cho process subsystem.
- Từ chối ELF không hợp lệ bằng error rõ ràng.

## Không thuộc phạm vi

- Không hỗ trợ ELF32.
- Không hỗ trợ dynamic linker hoặc shared library.
- Không hỗ trợ relocation phức tạp trong bản đầu.
- Không chạy binary Linux hoặc Windows.
- Không triển khai syscall ABI trong spec này.

## Ràng buộc

- Không trust ELF input.
- Mọi offset, size, alignment phải kiểm tra overflow.
- Không dùng `unwrap` hoặc `expect` trong kernel runtime path.
- Không map segment executable và writable cùng lúc nếu không cần thiết.
- ABI userspace phải được ghi trong `docs/design/kernel-api.md` trước khi expose.

## Dependencies

- Spec 004: memory management và paging.
- Spec 007: FAT32 read-only để đọc ELF từ disk image.
- Spec 009: process scheduler hoặc process model tối thiểu để chứa loaded image.

## ADR liên quan

- Cần ADR nếu chọn ABI userspace hoặc layout address space dài hạn.

## Public interfaces

```rust
pub fn load_elf64(bytes: &[u8], address_space: &mut AddressSpace) -> Result<LoadedImage, ElfError>;
pub fn validate_elf64(bytes: &[u8]) -> Result<ElfMetadata, ElfError>;
```

## Internal interfaces

```rust
struct ElfHeader64;
struct ProgramHeader64;
struct LoadedSegment;
```

## Data structures

- `ElfMetadata`: class, machine, entry, program header count.
- `LoadedImage`: entry point, mapped segments, initial permissions.
- `ElfError`: lỗi magic, class, machine, bounds, alignment hoặc permissions.
- `SegmentPermissions`: read/write/execute.

## Xử lý lỗi

- Magic sai trả `ElfError::InvalidMagic`.
- Machine không phải x86_64 trả `ElfError::UnsupportedMachine`.
- Segment vượt file size trả `ElfError::OutOfBounds`.
- Segment permission không hợp lệ trả `ElfError::InvalidPermissions`.

## Hành vi logging

- Log tên file ELF, entry point và số segment khi load thành công.
- Log lỗi validate ở mức ngắn gọn, không dump toàn bộ binary.
- Không log dữ liệu chương trình.

## Security considerations

- ELF là input không tin cậy từ disk image.
- Loader phải ngăn integer overflow và mapping ngoài userspace range.
- Không map kernel memory vào userspace.
- Không cấp quyền write+execute nếu policy không cho phép.

## Kế hoạch test

- Unit test valid ELF64 tối thiểu.
- Unit test ELF sai magic, sai machine, segment out-of-bounds.
- Integration test đọc ELF từ FAT32 image và validate metadata.
- QEMU test load init ELF nhưng chưa cần chuyển quyền nếu Spec 010 chưa implement.

## Acceptance criteria

- **Acceptance Criterion 1**:
  - **Given** file ELF64 x86_64 hợp lệ trong FAT32 image.
  - **When** `load_elf64` chạy.
  - **Then** loader phải trả `LoadedImage` có entry point và ít nhất một mapped segment hợp lệ.

- **Acceptance Criterion 2**:
  - **Given** file không phải ELF.
  - **When** `validate_elf64` chạy.
  - **Then** loader phải trả `ElfError::InvalidMagic`.

- **Acceptance Criterion 3**:
  - **Given** ELF có segment vượt quá kích thước file.
  - **When** loader parse program headers.
  - **Then** loader phải từ chối file và không map segment đó.

## Kế hoạch rollback hoặc removal

- Có thể rollback bằng cách không gọi ELF loader và không spawn userspace.
- Không thay thế bằng loader giả chỉ nhảy tới địa chỉ hardcode.

## Câu hỏi mở

- Userspace address range đầu tiên sẽ đặt ở đâu?
- Có cần hỗ trợ relocation cho init ở milestone đầu không?
