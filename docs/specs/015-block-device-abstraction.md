# Spec: 015-block-device-abstraction (Lớp trừu tượng hóa thiết bị khối)

- **Feature ID**: 015-block-device-abstraction
- **Tiêu đề**: Lớp trừu tượng hóa thiết bị khối (Block Device Abstraction)
- **Trạng thái**: DRAFT
- **Người phụ trách**: Kỹ sư trưởng AxiomOS
- **Ngày tạo**: 2026-07-07
- **Ngày cập nhật**: 2026-07-07

---

## 1. Vấn đề cần giải quyết
Để hỗ trợ đọc các hệ thống tệp tin (như FAT32) trên các phương tiện lưu trữ vật lý khác nhau (RAM disk, IDE, AHCI, NVMe), nhân kernel cần một lớp trừu tượng hóa thiết bị khối (Block Device Abstraction) độc lập với phần cứng.

## 2. Mục tiêu
- Thiết kế trait `BlockDevice` thống nhất trong nhân.
- Hiện thực một mock block device (RAM disk) sử dụng một vùng nhớ tĩnh hoặc Limine boot module để phục vụ kiểm thử hệ thống tệp tin.
- Đảm bảo an toàn luồng khi đọc/ghi thiết bị khối thông qua các cơ chế đồng bộ hóa (Spinlock).

## 3. Không thuộc phạm vi
- Chưa hiện thực các driver phần cứng thực tế như IDE/AHCI trong bản spec này.
- Chưa hiện thực tính năng ghi (write) cho block device nếu không cần thiết; bản đầu tiên ưu tiên read-only.
