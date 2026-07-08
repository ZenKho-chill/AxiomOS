# Chiến lược Kiểm thử (Testing Strategy)

AxiomOS áp dụng quy trình kiểm thử phân tầng để đảm bảo tính ổn định của hệ điều hành.

## Các loại hình kiểm thử

1. **Unit Tests (Kiểm thử đơn vị)**:
   - Viết trực tiếp trong các module Rust không phụ thuộc phần cứng (như các thuật toán hàng đợi, cấu trúc dữ liệu, phân tích định dạng file ELF, FAT32 parser).
   - Chạy trên môi trường host thông thường thông qua:
     ```bash
     make test
     ```

2. **Integration Tests (Kiểm thử tích hợp)**:
   - Các bài kiểm thử chạy bên trong môi trường giả lập QEMU.
   - Ví dụ: Xác minh xem kernel có in đúng chuỗi chào mừng ra cổng COM1 hay không, kiểm tra xem bộ cấp phát heap hoạt động chính xác không.
   - Các bài test này sẽ tự động hóa qua script và tích hợp trên Github Actions.

3. **Userspace Tests (Kiểm thử userspace)**:
   - Shell core Milestone 7 được kiểm thử bằng runtime giả trong `userspace/shell`.
   - Test xác nhận shell in prompt `axiomsh> ls /`, liệt kê `INIT.ELF`, `HELLO.TXT` và in nội dung `/HELLO.TXT`.
   - Chạy cùng bộ test host bằng:
     ```bash
     make test
     ```

## Kiểm thử framebuffer panic path

Spec 003 có feature test-only `panic-test` để ép kernel panic ngay sau khi framebuffer console sẵn sàng. Feature này không được bật trong image mặc định.

```bash
KERNEL_FEATURES=panic-test bash scripts/build-image.sh
timeout 18s qemu-system-x86_64 \
    -drive format=raw,file=target/axiomOS.img \
    -serial file:qemu_serial.log \
    -display gtk \
    -m 256M \
    -no-reboot \
    -no-shutdown
```

Sau khi kiểm thử xong, chạy lại `make image` để trả image về cấu hình boot mặc định.
