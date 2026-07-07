# Spec: 017-kernel-file-api (API đọc tệp tin từ Kernel)

- **Feature ID**: 017-kernel-file-api
- **Tiêu đề**: API đọc tệp tin từ Kernel
- **Trạng thái**: APPROVED
- **Người phụ trách**: Kỹ sư trưởng AxiomOS
- **Ngày tạo**: 2026-07-07
- **Ngày cập nhật**: 2026-07-07

---

## Vấn đề cần giải quyết

Các dịch vụ kernel như ELF loader sau này cần đọc file cấu hình hoặc binary từ storage mà không gọi trực tiếp FAT32 backend. Nếu caller kernel phụ thuộc vào `kernel/src/fs/fat32.rs`, Milestone 6 sẽ khó thay backend, thêm VFS hoặc thêm syscall ABI.

## Mục tiêu

- Cung cấp API đọc file kernel-internal qua VFS tối giản.
- Dùng caller-provided buffer để tránh hidden allocation trong runtime path.
- Cho phép caller mở file, đọc tuần tự và liệt kê thư mục root thông qua abstraction VFS.
- Map lỗi FAT32/backend sang lỗi VFS/kernel file API rõ ràng.
- Không expose type FAT32-specific ra caller của Kernel File API.

## Không thuộc phạm vi

- Không thiết kế syscall `open`, `read`, `write`, `close` cho userspace.
- Không cung cấp API ghi file.
- Không thêm file descriptor table cho process/userspace.
- Không thêm block cache.
- Không thêm driver đĩa QEMU hoặc hardware storage driver.
- Không hỗ trợ long filename, permissions, symlink hoặc nhiều mount point động.

## Ràng buộc

- Spec 016 VFS tối giản là boundary bắt buộc giữa Kernel File API và FAT32.
- API public của spec này phải nhận `&mut [u8]` từ caller, không tự cấp phát `Vec`.
- Path kernel-internal phải bắt đầu bằng `/`.
- Nếu root filesystem chưa mount, API phải trả lỗi thay vì panic.
- Không allocation trong interrupt handler.
- Không thêm dependency mới.
- Không thay đổi ABI userspace.

## Dependencies

- Spec 007: FAT32 read-only.
- Spec 015: Block device abstraction.
- Spec 016: VFS tối giản.
- Spec 004: Heap/memory foundation chỉ dùng nếu caller chọn buffer động bên ngoài API này.
- Spec 012: Synchronization primitives cho root mount registry.

## ADR liên quan

- [ADR-006: VFS tối giản với một root mount read-only](../architecture/adr-006-minimal-vfs-root-mount.md).

## Public interfaces

Các interface này là kernel-internal API, chưa phải userspace ABI:

```rust
pub fn kernel_read_file(path: &str, buffer: &mut [u8]) -> Result<usize, VfsError>;
pub fn kernel_open_file(path: &str) -> Result<FileHandle, VfsError>;
pub fn kernel_read(handle: &mut FileHandle, buffer: &mut [u8]) -> Result<usize, VfsError>;
pub fn kernel_list_dir(path: &str, sink: &mut dyn DirEntrySink) -> Result<(), VfsError>;
```

## Internal interfaces

```rust
pub fn mount_root(filesystem: &'static dyn FileSystem) -> Result<(), VfsError>;
pub fn open(path: &str) -> Result<FileHandle, VfsError>;
pub fn read(handle: &mut FileHandle, buffer: &mut [u8]) -> Result<usize, VfsError>;
pub fn list_dir(path: &str, sink: &mut dyn DirEntrySink) -> Result<(), VfsError>;

pub trait FileSystem {
    fn open(&self, path: &str) -> Result<FileNode, VfsError>;
    fn read_at(&self, node: &FileNode, offset: u64, buffer: &mut [u8]) -> Result<usize, VfsError>;
    fn list_dir(&self, path: &str, sink: &mut dyn DirEntrySink) -> Result<(), VfsError>;
}
```

## Data structures

- `FileHandle`: handle kernel-internal chứa `FileNode` và offset đọc tuần tự.
- `FileNode`: metadata backend-neutral gồm loại file, kích thước và node id nội bộ.
- `FileSystem`: trait backend filesystem read-only.
- `DirEntry`: entry thư mục backend-neutral, tên byte bounded, loại file và kích thước.
- `DirEntrySink`: sink nhận entry không cần allocation không giới hạn.
- `VfsRoot`: root mount registry duy nhất cho Milestone 5.
- `VfsError`: lỗi path, mount, type file, backend hoặc buffer.

## Xử lý lỗi

- Root filesystem chưa mount trả `VfsError::NoRootMount`.
- Path rỗng hoặc không bắt đầu bằng `/` trả `VfsError::InvalidPath`.
- File không tồn tại trả `VfsError::NotFound`.
- Gọi `read` trên directory trả `VfsError::IsDirectory`.
- Backend FAT32 lỗi metadata hoặc cluster chain trả `VfsError::BackendError`.
- Buffer rỗng trả `Ok(0)` nếu file hợp lệ.
- Không dùng `panic`, `unwrap` hoặc `expect` trong runtime path.

## Hành vi logging

- Log mount root thành công có thể thêm sau khi logging facade có policy ổn định cho filesystem.
- Không log nội dung file.
- Không log lặp liên tục với path lỗi để tránh spam serial.

## Security considerations

- Path là input không tin cậy và phải validate trước khi gọi backend.
- Không cho phép path tương đối trong Kernel File API.
- Không dùng metadata backend làm offset nếu chưa kiểm tra overflow.
- Không expose type FAT32-specific hoặc pointer backend ra caller.
- API này không cấp quyền userspace và không phải security boundary cho process.

## Kế hoạch test

- Unit test path tương đối trả `InvalidPath`.
- Unit test root chưa mount trả `NoRootMount`.
- Unit test `read` tăng offset tuần tự.
- Integration test VFS + FAT32 fixture đọc marker `AXIOMOS.TXT`.
- Unit test `list_dir("/")` chuyển nhiều entry qua `DirEntrySink`.
- QEMU boot regression phải giữ `[AXIOMOS] Kernel started`.

## Acceptance criteria

- **Acceptance Criterion 1**:
  - **Given** root filesystem chưa được mount.
  - **When** kernel gọi `open("/AXIOMOS.TXT")`.
  - **Then** VFS phải trả `VfsError::NoRootMount`.

- **Acceptance Criterion 2**:
  - **Given** root filesystem FAT32 read-only đã được mount bằng test fixture.
  - **When** kernel gọi `kernel_open_file("/AXIOMOS.TXT")` rồi `kernel_read`.
  - **Then** dữ liệu marker phải được đọc vào buffer mà không expose type FAT32 ra caller.

- **Acceptance Criterion 3**:
  - **Given** caller truyền path tương đối `AXIOMOS.TXT`.
  - **When** VFS validate path.
  - **Then** VFS phải trả `VfsError::InvalidPath`.

- **Acceptance Criterion 4**:
  - **Given** caller gọi `list_dir("/")`.
  - **When** backend trả nhiều directory entry.
  - **Then** VFS phải chuyển từng entry qua `DirEntrySink` theo thứ tự backend mà không allocation không giới hạn.

## Kế hoạch rollback hoặc removal

- Có thể rollback bằng cách tắt module `kernel_file` và giữ FAT32 backend trực tiếp cho test nội bộ.
- Không được rollback bằng hardcode path hoặc fake file map.
- Nếu VFS root mount gây vấn đề runtime, giữ trait/interface và vô hiệu hóa global mount registry cho tới khi có synchronization policy tốt hơn.

## Câu hỏi mở

- Khi Milestone 6 thêm process model đầy đủ, `FileHandle` có chuyển thành descriptor table theo process không?
- VFS nên normalize case-insensitive ở layer chung hay để FAT32 backend xử lý?
- Có cần thêm giới hạn tên file chung lớn hơn 32 byte khi hỗ trợ long filename sau này không?
