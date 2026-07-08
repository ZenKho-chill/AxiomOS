# Hệ thống Ngắt (Interrupts & Exceptions)

AxiomOS xử lý ngắt và ngoại lệ thông qua Bảng mô tả ngắt (Interrupt Descriptor Table - IDT).

## GDT và selector kernel

Kernel nạp GDT riêng ngay sau khi serial và framebuffer sẵn sàng. Các descriptor trong GDT bật sẵn Accessed bit vì bảng GDT nằm trong segment read-only của kernel; nếu bit này tắt, CPU sẽ cố ghi lại descriptor khi nạp selector và có thể gây page fault trước khi IDT được nạp.

Sau `lgdt`, kernel nạp lại CS bằng far return về selector kernel code `0x08`, sau đó nạp DS/ES/SS/FS/GS bằng selector kernel data `0x10`. IDT luôn đăng ký handler với selector kernel code `0x08`, không dùng selector còn lại từ bootloader.

## Các thành phần ngắt

1. **CPU Exceptions (Ngoại lệ CPU)**:
   - Các lỗi xảy ra trong quá trình thực thi lệnh (như Page Fault, Divide by Zero, Double Fault, General Protection Fault).
   - Mỗi ngoại lệ có một hàm handler riêng để in thông tin debug hoặc thực hiện khôi phục.

2. **Hardware Interrupts (Ngắt phần cứng)**:
   - Nhận tín hiệu từ các thiết bị ngoại vi thông qua 8259 PIC trong milestone hiện tại.
   - Ví dụ: Ngắt từ Timer (để lập lịch) và bàn phím PS/2.
