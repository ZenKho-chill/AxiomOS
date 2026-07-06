# Mô hình Tiến trình & Lập lịch (Process Model & Scheduling)

Tài liệu này định nghĩa mô hình thực thi của tiến trình và luồng trong AxiomOS.

## Mô hình Tiến trình

- **Kernel Thread**: Thực thi trong không gian địa chỉ của Kernel (Ring 0) và không có không gian địa chỉ ảo riêng biệt.
- **User Process**: Thực thi trong không gian địa chỉ riêng biệt (Ring 3), giao tiếp với Kernel qua Syscalls.

## Bộ lập lịch (Scheduler)

1. **Cooperative Scheduler (Lập lịch cộng tác)**:
   - Các luồng chủ động nhường CPU bằng cách gọi hàm yield.
   - Thích hợp cho giai đoạn phát triển ban đầu vì tính đơn giản.

2. **Preemptive Scheduler (Lập lịch trưng dụng)**:
   - Sử dụng ngắt từ timer hệ thống để luân chuyển CPU giữa các luồng.
   - Phân chia mức độ ưu tiên (Priority-based Round Robin).
