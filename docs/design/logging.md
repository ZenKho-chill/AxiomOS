# Design: Logging Subsystem

Tài liệu này đặc tả hệ thống ghi log và định dạng thông điệp chẩn đoán trong AxiomOS.

## Kênh logging hiện tại

- Serial COM1 là kênh logging chính cho boot diagnostics.
- Framebuffer console là kênh mirror tối thiểu khi Limine cung cấp framebuffer hợp lệ.
- Panic handler ghi ra serial và framebuffer nếu framebuffer console đã khởi tạo thành công.

## Facade Milestone 4

Milestone 4 thêm `kernel::logging` làm facade logging nội bộ. Facade này nhận `LogRecord` gồm level, subsystem, message và cờ mirror framebuffer, sau đó ghi ra serial và mirror framebuffer khi được yêu cầu.

Thiết kế hiện tại cố ý giữ định dạng legacy cho các log quan trọng:

- Boot diagnostics: `[AXIOMOS] <message>`
- Panic diagnostics: `[AXIOMOS PANIC] <message>`
- Subsystem diagnostics: `[AXIOMOS <SUBSYSTEM>] <message>`

Việc giữ format legacy giúp boot test, tài liệu QEMU và các bằng chứng serial hiện có tiếp tục hợp lệ trong khi code chuyển dần sang API logging tập trung.

## Định dạng

- Boot diagnostics dùng prefix `[AXIOMOS]`.
- Panic diagnostics dùng prefix `[AXIOMOS PANIC]`.
- Timer diagnostics dùng prefix `[AXIOMOS TIMER]`.
- Memory diagnostics dùng prefix `[AXIOMOS MEMORY]`.
- Framebuffer console không thay thế serial log; nếu framebuffer lỗi, serial vẫn phải tiếp tục hoạt động.

## Quy tắc implementation

- Không allocation khi ghi log.
- Không thêm dependency logging bên ngoài trong Milestone 4.
- Không dùng logging facade trong timer interrupt handler; timer interrupt chỉ tăng counter.
- Log trong main loop có thể đi qua `logging::info`.
- Log boot và panic nên đi qua `logging::boot` và `logging::panic`.

## Giới hạn

- Chưa có log level runtime.
- Chưa có ring buffer log.
- Chưa lưu log vào disk.
- Chưa có chính sách logging cho SMP.
