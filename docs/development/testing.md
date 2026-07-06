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
