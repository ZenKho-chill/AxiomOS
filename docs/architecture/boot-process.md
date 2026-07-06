# Tiến trình Boot (Boot Process)

AxiomOS sử dụng bootloader Limine để khởi chạy hệ điều hành.

## Sơ đồ các bước boot

1. **Khởi động Firmware (UEFI)**:
   Máy ảo hoặc PC thật được khởi chạy trong chế độ UEFI. UEFI load tệp tin EFI của Limine (`BOOTX64.EFI`) từ phân vùng EFI System Partition (ESP).

2. **Limine Bootloader**:
   Limine đọc file cấu hình `limine.cfg`, tải ảnh đĩa Kernel ELF64 (`kernel.elf`) vào bộ nhớ vật lý.

3. **Chuyển giao trạng thái (Handoff)**:
   Limine chuyển CPU sang chế độ 64-bit Long Mode, thiết lập phân trang cơ bản, và chuyển giao điều khiển cho hàm Entry Point của Kernel (`_start`) được xác định trong file linker.

4. **Kernel Entry**:
   Kernel nhận cấu hình bộ nhớ và thông tin phần cứng qua cấu trúc dữ liệu của Limine, khởi tạo COM1 serial port để bắt đầu in log chẩn đoán sớm.

5. **Framebuffer Console**:
   Nếu Limine cung cấp framebuffer RGB hợp lệ, kernel khởi tạo framebuffer console tối thiểu và mirror boot sequence lên màn hình QEMU. Nếu framebuffer không khả dụng, kernel tiếp tục boot bằng serial logging.
