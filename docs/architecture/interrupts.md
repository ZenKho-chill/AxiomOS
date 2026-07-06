# Hệ thống Ngắt (Interrupts & Exceptions)

AxiomOS xử lý ngắt và ngoại lệ thông qua Bảng mô tả ngắt (Interrupt Descriptor Table - IDT).

## Các thành phần ngắt

1. **CPU Exceptions (Ngoại lệ CPU)**:
   - Các lỗi xảy ra trong quá trình thực thi lệnh (như Page Fault, Divide by Zero, Double Fault, General Protection Fault).
   - Mỗi ngoại lệ có một hàm handler riêng để in thông tin debug hoặc thực hiện khôi phục.

2. **Hardware Interrupts (Ngắt phần cứng)**:
   - Nhận tín hiệu từ các thiết bị ngoại vi thông qua APIC (Advanced Programmable Interrupt Controller).
   - Ví dụ: Ngắt từ Timer (để lập lịch) và bàn phím PS/2.
