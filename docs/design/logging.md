# Design: Logging Subsystem

Tài liệu này đặc tả hệ thống ghi log và định dạng thông điệp chẩn đoán trong AxiomOS.

## Kênh logging hiện tại

- Serial COM1 là kênh logging chính cho boot diagnostics.
- Framebuffer console là kênh mirror tối thiểu khi Limine cung cấp framebuffer hợp lệ.
- Panic handler ghi ra serial và framebuffer nếu framebuffer console đã khởi tạo thành công.

## Định dạng

- Boot diagnostics dùng prefix `[AXIOMOS]`.
- Panic diagnostics dùng prefix `[AXIOMOS PANIC]`.
- Framebuffer console không thay thế serial log; nếu framebuffer lỗi, serial vẫn phải tiếp tục hoạt động.

## Giới hạn

- Chưa có log level runtime.
- Chưa có ring buffer log.
- Chưa lưu log vào disk.
