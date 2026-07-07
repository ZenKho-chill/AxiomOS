# Spec: 017-kernel-file-api (API đọc tệp tin từ Kernel)

- **Feature ID**: 017-kernel-file-api
- **Tiêu đề**: API đọc tệp tin từ Kernel
- **Trạng thái**: DRAFT
- **Người phụ trách**: Kỹ sư trưởng AxiomOS
- **Ngày tạo**: 2026-07-07
- **Ngày cập nhật**: 2026-07-07

---

## 1. Vấn đề cần giải quyết
Để các dịch vụ khác của nhân kernel (chẳng hạn như ELF loader) có thể đọc cấu hình và dữ liệu chương trình từ đĩa lưu trữ, nhân kernel cần một bộ API đơn giản, an toàn và dễ sử dụng để thao tác trên tệp tin mà không cần gọi trực tiếp driver phần cứng hay cấu trúc FAT32 nội bộ.

## 2. Mục tiêu
- Cung cấp API đọc tệp tin ở mức kernel: `pub fn kernel_read_file(path: &str) -> Result<Vec<u8>, FsError>`.
- Tích hợp gọi thông qua lớp VFS để xác định phân vùng và định tuyến yêu cầu đọc dữ liệu đến đúng trình điều khiển hệ thống tệp tin (FAT32).

## 3. Không thuộc phạm vi
- Không thiết kế giao diện syscall cho userspace trong spec này (được hoãn sang Milestone 6).
- Không cung cấp API ghi file (`kernel_write_file`).
