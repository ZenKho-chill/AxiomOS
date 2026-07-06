# Hệ thống Tệp tin (Filesystem)

AxiomOS xây dựng hệ thống tệp tin ảo và hỗ trợ định dạng FAT32.

## Các tầng thiết kế

1. **VFS (Virtual Filesystem)**:
   - Cung cấp giao diện trừu tượng hóa cho các thao tác tệp tin (`open`, `read`, `write`, `close`, `readdir`).
   - Cho phép mount các hệ thống tệp tin khác nhau vào một cây thư mục chung.

2. **FAT32 Driver (Read-Only)**:
   - Trình đọc hệ thống tệp tin FAT32 trên phân vùng đĩa ảo.
   - Hỗ trợ đường dẫn ngắn (8.3 filename) và đọc nội dung file.

3. **Block Cache**:
   - Bộ đệm dữ liệu khối của đĩa để tối ưu hóa tốc độ đọc.
