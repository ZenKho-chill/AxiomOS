# Hệ thống Tệp tin (Filesystem)

AxiomOS xây dựng hệ thống tệp tin ảo và hỗ trợ định dạng FAT32.

## Các tầng thiết kế

1. **VFS (Virtual Filesystem)**:
   - Bản Milestone 5 dùng thiết kế tối giản với một root mount `/`.
   - Cung cấp giao diện kernel-internal cho `open`, `read` và `list_dir`.
   - Không công bố syscall ABI hoặc userspace file descriptor trong Milestone 5.

2. **FAT32 Driver (Read-Only)**:
   - Module hiện tại là `kernel/src/fs/fat32.rs`.
   - Mount metadata FAT32 từ `BlockDevice`, kiểm tra BPB, FAT offset và data offset.
   - Hỗ trợ đường dẫn root ngắn 8.3, liệt kê root directory và đọc regular file theo cluster chain.
   - Không hỗ trợ ghi file, long filename, thư mục lồng nhau hoặc driver đĩa QEMU trực tiếp trong phạm vi này.
   - Test hiện tại dùng `RamDisk` fixture để xác minh parser và read-only API mà không tuyên bố hỗ trợ phần cứng thật.

3. **Block Cache**:
   - Hoãn sau bản VFS tối giản; nếu thêm block cache cần ADR hoặc spec cập nhật.

## Quyết định Milestone 5

- Spec 016 định nghĩa VFS read-only tối giản để tách kernel file API khỏi FAT32 backend.
- Root mount duy nhất là đủ cho giai đoạn đọc file marker và chuẩn bị ELF loader.
- Ghi file, permissions, nhiều mount point và syscall file descriptor được hoãn sang milestone sau.
- FAT32 driver hiện chỉ là backend read-only trên trait `BlockDevice`; việc nối vào driver block thật của QEMU sẽ cần spec hoặc PR riêng.
