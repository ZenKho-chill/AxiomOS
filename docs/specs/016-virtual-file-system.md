# Spec: 016-virtual-file-system (Hệ thống tệp tin ảo VFS)

- **Feature ID**: 016-virtual-file-system
- **Tiêu đề**: Hệ thống tệp tin ảo (VFS) tối giản
- **Trạng thái**: TESTING
- **Người phụ trách**: Kỹ sư trưởng AxiomOS
- **Ngày tạo**: 2026-07-07
- **Ngày cập nhật**: 2026-07-07

---

## Vấn đề cần giải quyết

AxiomOS cần một lớp VFS tối giản để kernel có thể đọc file mà không phụ thuộc trực tiếp vào chi tiết FAT32. Nếu FAT32, ELF loader và kernel file API gọi trực tiếp nhau, Milestone 5 và Milestone 6 sẽ khó mở rộng khi thêm mount point, nhiều filesystem hoặc userspace file descriptor sau này.

## Mục tiêu

- Định nghĩa boundary giữa block device, FAT32 read-only và kernel file API.
- Cung cấp mô hình root mount duy nhất cho giai đoạn đầu.
- Định nghĩa `FileSystem`, `FileHandle`, `DirEntry`, `FileType` và `DirEntrySink`.
- Cung cấp API mở file, đọc file và liệt kê thư mục cho kernel service.
- Chuẩn bị đường nâng cấp sang syscall file descriptor ở Milestone 6 mà chưa công bố ABI userspace.

## Không thuộc phạm vi

- Không triển khai syscall `open`, `read`, `write`, `close` cho userspace trong spec này.
- Không hỗ trợ ghi file.
- Không hỗ trợ permissions, owner, hard link, symlink hoặc long filename policy ngoài filesystem backend.
- Không thêm block cache.
- Không hỗ trợ nhiều mount point động.
- Không thêm driver disk phần cứng thật.

## Ràng buộc

- Bản đầu chỉ cần root mount `/`.
- Backend FAT32 read-only là filesystem đầu tiên.
- Không allocation trong interrupt handler.
- Không panic nếu path không tồn tại hoặc filesystem chưa mount.
- Không expose FAT32-specific type ra API VFS public.
- Không claim ABI userspace ổn định.

## Dependencies

- Spec 007: FAT32 read-only.
- Spec 015: block device abstraction.
- Spec 004: memory management nếu implementation cần buffer động có kiểm soát.
- Spec 012: synchronization primitives nếu root mount registry cần lock.

## ADR liên quan

- [adr-006-minimal-vfs-root-mount.md](../architecture/adr-006-minimal-vfs-root-mount.md): Quyết định dùng VFS tối giản với một root mount read-only trước khi có syscall ABI.

## Public interfaces

Các interface này là internal kernel API, chưa phải ABI userspace:

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

## Internal interfaces

```rust
pub struct FileHandle {
    node: FileNode,
    offset: u64,
}

pub struct FileNode {
    file_type: FileType,
    size: u64,
    backend_id: u32,
    backend_node: u64,
}

pub trait DirEntrySink {
    fn push(&mut self, entry: DirEntry) -> Result<(), VfsError>;
}
```

## Data structures

- `FileSystem`: trait backend filesystem.
- `FileHandle`: trạng thái đọc tuần tự của một file đã mở.
- `FileNode`: metadata nội bộ không chứa type FAT32-specific.
- `DirEntry`: tên entry, loại file và kích thước nếu có.
- `FileType`: `RegularFile`, `Directory`.
- `VfsError`: lỗi path, mount, filesystem hoặc buffer.
- `RootMount`: registry root filesystem duy nhất trong bản đầu.

## Xử lý lỗi

- Chưa mount root trả `VfsError::NoRootMount`.
- Path rỗng hoặc không bắt đầu bằng `/` trả `VfsError::InvalidPath`.
- File không tồn tại trả `VfsError::NotFound`.
- Gọi `read` trên directory trả `VfsError::IsDirectory`.
- Backend FAT32 trả lỗi metadata thì map sang `VfsError::BackendError`.
- Buffer quá nhỏ không được panic; `read` trả số byte đã đọc hoặc lỗi cụ thể nếu không thể đọc byte nào.

## Hành vi logging

- Log khi root filesystem được mount thành công.
- Log lỗi mount root nghiêm trọng qua logging subsystem.
- Không log nội dung path nhạy cảm hoặc nội dung file.
- Không log lặp liên tục khi `open` hoặc `read` lỗi do user/kernel caller truyền path sai.

## Security considerations

- Path là input không tin cậy và phải normalize tối thiểu trước khi chuyển cho backend.
- Không cho phép path traversal vượt root mount.
- Không dùng metadata filesystem làm offset nếu chưa kiểm tra overflow.
- Không công bố ABI userspace; mọi interface trong spec này là kernel-internal.

## Kế hoạch test

- Unit test path validation: `/`, `/BOOT/KERNEL.ELF`, path rỗng và path tương đối.
- Unit test root mount chưa có filesystem trả `NoRootMount`.
- Unit test `read` tăng offset theo số byte đã đọc.
- Integration test sau khi Spec 007 implementation có FAT32 fixture: mount root FAT32 và đọc file marker.
- QEMU boot test sau khi có implementation phải giữ `[AXIOMOS] Kernel started`.

## Ghi chú triển khai hiện tại

- Module hiện thực: `kernel/src/fs/vfs.rs`.
- Root mount registry hiện chỉ hỗ trợ một filesystem read-only.
- Unit test dùng `VfsRoot` cục bộ để tránh phụ thuộc global state khi test chạy song song.
- Integration test FAT32 fixture nằm trong `kernel/src/fs/fat32.rs`.

## Acceptance criteria

- **Acceptance Criterion 1**:
  - **Given** root filesystem chưa được mount.
  - **When** kernel gọi `open("/AXIOMOS.TXT")`.
  - **Then** VFS phải trả `VfsError::NoRootMount`.

- **Acceptance Criterion 2**:
  - **Given** root filesystem FAT32 read-only đã được mount.
  - **When** kernel gọi `open("/AXIOMOS.TXT")` rồi `read`.
  - **Then** dữ liệu file marker phải được đọc vào buffer mà không expose type FAT32 ra caller.

- **Acceptance Criterion 3**:
  - **Given** caller truyền path tương đối `AXIOMOS.TXT`.
  - **When** VFS validate path.
  - **Then** VFS phải trả `VfsError::InvalidPath`.

- **Acceptance Criterion 4**:
  - **Given** caller gọi `list_dir("/")`.
  - **When** backend trả nhiều directory entry.
  - **Then** VFS phải chuyển từng entry qua `DirEntrySink` theo thứ tự backend mà không allocation không giới hạn.

## Kế hoạch rollback hoặc removal

- Có thể rollback bằng cách để kernel file API gọi trực tiếp FAT32 backend trong một module duy nhất.
- Không được rollback bằng hardcode path hoặc fake file map.

## Câu hỏi mở

- Root mount registry nên dùng static slot duy nhất hay bảng mount cố định nhiều slot ở Milestone 5?
- VFS có nên normalize case-insensitive path ở layer VFS hay để FAT32 backend xử lý?
- `FileHandle` nên là value type kernel-internal hay được lưu trong bảng descriptor khi Milestone 6 thêm syscall?
