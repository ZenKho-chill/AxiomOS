# ADR-006: VFS tối giản với một root mount read-only

- **Trạng thái**: APPROVED
- **Người phụ trách**: Kỹ sư trưởng AxiomOS
- **Ngày tạo**: 2026-07-07
- **Ngày cập nhật**: 2026-07-07

---

## Bối cảnh và vấn đề cần giải quyết

Milestone 5 cần kết nối block device, FAT32 read-only và kernel file API. Nếu kernel file API gọi trực tiếp FAT32, các milestone sau sẽ khó thêm filesystem khác, mount table hoặc file descriptor cho userspace. Tuy nhiên, một VFS đầy đủ với permissions, cache, nhiều mount point và syscall ABI là quá rộng cho giai đoạn hiện tại.

## Các phương án cân nhắc

### Phương án A: Kernel file API gọi trực tiếp FAT32

- **Ưu điểm**: Ít abstraction, nhanh triển khai.
- **Nhược điểm**: Coupling chặt vào FAT32; khó thay backend hoặc thêm mount point.

### Phương án B: VFS đầy đủ ngay từ Milestone 5

- **Ưu điểm**: Gần với kiến trúc hệ điều hành hoàn chỉnh.
- **Nhược điểm**: Quá rộng, kéo theo permissions, descriptor table, cache và syscall ABI trước khi userspace ổn định.

### Phương án C: VFS tối giản với một root mount read-only

- **Ưu điểm**: Tách kernel file API khỏi FAT32 mà vẫn giữ scope nhỏ; đủ để đọc file phục vụ ELF loader sau này.
- **Nhược điểm**: Chưa hỗ trợ nhiều mount point, write, permissions hoặc userspace file descriptor.

## Quyết định lựa chọn

Chọn **Phương án C: VFS tối giản với một root mount read-only**.

Thiết kế ban đầu chỉ có root mount `/`, backend đầu tiên là FAT32 read-only. VFS cung cấp internal kernel API để mở file, đọc file và liệt kê thư mục. Syscall ABI và descriptor table sẽ được thiết kế ở Milestone 6, không nằm trong ADR này.

## Hệ quả và ảnh hưởng

- Kernel file API của Spec 017 sẽ phụ thuộc vào VFS thay vì phụ thuộc trực tiếp FAT32.
- FAT32 backend không expose type nội bộ ra caller VFS.
- Không thêm dependency mới.
- Không thay đổi ABI userspace vì AxiomOS chưa công bố ABI ổn định.
- Roadmap Milestone 5 hoàn tất phần thiết kế VFS khi Spec 016 được APPROVED, nhưng implementation VFS vẫn phải có PR riêng nếu cần code runtime.
