# Spec: 007-fat32-readonly (FAT32 read-only)

- **Feature ID**: 007-fat32-readonly
- **Tiêu đề**: Hệ thống tệp tin FAT32 read-only
- **Trạng thái**: DRAFT
- **Người phụ trách**: Kỹ sư trưởng AxiomOS
- **Ngày tạo**: 2026-07-06
- **Ngày cập nhật**: 2026-07-06

---

## Vấn đề cần giải quyết

AxiomOS cần đọc file từ disk image để nạp chương trình userspace và tài nguyên kiểm thử. FAT32 read-only là bước nhỏ nhất vì boot image hiện đã dùng FAT32/ESP trong QEMU.

## Mục tiêu

- Đọc BIOS Parameter Block và metadata FAT32.
- Liệt kê thư mục root hoặc thư mục cấu hình được chỉ định.
- Đọc file thường theo cluster chain.
- Cung cấp API read-only cho kernel loader.
- Xử lý lỗi filesystem mà không panic.

## Không thuộc phạm vi

- Không ghi file.
- Không tạo, xóa hoặc rename file.
- Không hỗ trợ long filename nếu chưa có test cụ thể; có thể chỉ hỗ trợ 8.3 ban đầu.
- Không hỗ trợ nhiều filesystem.
- Không triển khai VFS đầy đủ nếu chưa có spec riêng.

## Ràng buộc

- Chỉ đọc từ disk image QEMU đã được build bởi script dự án.
- Không cache không giới hạn.
- Không allocation trong interrupt handler.
- Không tin tưởng metadata FAT32; mọi offset/size phải kiểm tra giới hạn.

## Dependencies

- Spec 004: memory management cho buffer/cấp phát có kiểm soát.
- Spec 005: interrupts nếu block device dùng interrupt sau này; polling được phép ở bản đầu nếu ghi rõ.
- Block device abstraction tối thiểu phải có spec hoặc được giới hạn trong implementation của spec này.

## ADR liên quan

- Cần ADR nếu thêm VFS abstraction hoặc crate parser FAT32 bên ngoài.

## Public interfaces

```rust
pub fn mount_fat32(device: BlockDeviceId) -> Result<Fat32Volume, FsError>;
pub fn read_file(path: &Path, buffer: &mut [u8]) -> Result<usize, FsError>;
pub fn list_dir(path: &Path, sink: &mut dyn DirEntrySink) -> Result<(), FsError>;
```

## Internal interfaces

```rust
trait BlockDevice {
    fn read_sector(&self, lba: u64, buffer: &mut [u8; 512]) -> Result<(), BlockError>;
}

struct Fat32BootSector;
struct FatEntry(u32);
struct DirectoryEntry;
```

## Data structures

- `Fat32Volume`: thông tin volume, FAT offset, data offset, cluster size.
- `DirectoryEntry`: entry file/directory đã parse.
- `ClusterChain`: iterator cluster của file.
- `FsError`: lỗi filesystem, path, bounds hoặc block device.

## Xử lý lỗi

- BPB không hợp lệ trả `FsError::InvalidBootSector`.
- Cluster chain vòng lặp hoặc vượt giới hạn trả `FsError::CorruptFat`.
- File không tồn tại trả `FsError::NotFound`.
- Buffer quá nhỏ trả số byte đọc được hoặc `FsError::BufferTooSmall` tùy API cụ thể.

## Hành vi logging

- Log khi mount FAT32 thành công: sector size, cluster size, FAT count.
- Log lỗi metadata nghiêm trọng qua serial.
- Không log nội dung file.

## Security considerations

- FAT metadata là dữ liệu không tin cậy, phải kiểm tra overflow số học.
- Không đọc vượt kích thước image hoặc block device.
- Không thực thi file trực tiếp trong spec này.

## Kế hoạch test

- Tạo FAT32 test image với file 8.3 nhỏ.
- Unit test parser BPB và directory entry bằng byte fixture.
- QEMU boot test đọc file `/boot/kernel.elf` hoặc file marker read-only.
- Test image corrupt FAT để xác nhận lỗi có kiểm soát.

## Acceptance criteria

- **Acceptance Criterion 1**:
  - **Given** disk image FAT32 chứa file marker `AXIOMOS.TXT`.
  - **When** kernel gọi `read_file`.
  - **Then** nội dung file marker phải được đọc đúng vào buffer.

- **Acceptance Criterion 2**:
  - **Given** FAT32 metadata bị sửa sai trong test fixture.
  - **When** parser mount volume.
  - **Then** parser phải trả `FsError` thay vì panic.

- **Acceptance Criterion 3**:
  - **Given** thư mục root có nhiều entry.
  - **When** `list_dir("/")` chạy.
  - **Then** API phải trả danh sách entry read-only theo thứ tự trên disk.

## Kế hoạch rollback hoặc removal

- Có thể rollback bằng cách tắt mount FAT32 và nạp chương trình userspace từ blob nhúng tĩnh nếu spec loader cho phép.
- Không được thay bằng fake file map hardcode trừ khi được ghi rõ là test-only.

## Câu hỏi mở

- Có hỗ trợ long filename ở bản đầu không?
- Block device abstraction nên nằm trong spec này hay cần spec riêng trước khi implement?
