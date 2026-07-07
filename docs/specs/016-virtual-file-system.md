# Spec: 016-virtual-file-system (Hệ thống tệp tin ảo VFS)

- **Feature ID**: 016-virtual-file-system
- **Tiêu đề**: Hệ thống tệp tin ảo (VFS) tối giản
- **Trạng thái**: DRAFT
- **Người phụ trách**: Kỹ sư trưởng AxiomOS
- **Ngày tạo**: 2026-07-07
- **Ngày cập nhật**: 2026-07-07

---

## 1. Vấn đề cần giải quyết
Để nhân kernel và các ứng dụng userspace trong tương lai có thể tương tác với các hệ thống tệp tin và thiết bị khác nhau thông qua một giao diện thống nhất, AxiomOS cần một lớp Hệ thống tệp tin ảo (Virtual File System - VFS) tối giản.

## 2. Mục tiêu
- Định nghĩa các cấu trúc dữ liệu cơ bản của VFS: `Vnode` (hoặc `Inode`), `FileDescriptor`, `FileSystem`.
- Hiện thực API mở, đọc, ghi và đóng tệp tin thống nhất: `sys_open`, `sys_read`, `sys_write`, `sys_close`.
- Hỗ trợ mount hệ thống tệp tin FAT32 vào thư mục root `/` của VFS.

## 3. Không thuộc phạm vi
- Chưa hiện thực các tính năng nâng cao như định quyền truy cập file (permissions), hard links, soft links.
- Chưa hiện thực cơ chế đồng bộ buffer cache phức tạp.
