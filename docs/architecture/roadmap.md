# Lộ trình Phát triển (Development Roadmap)

Tài liệu này xác định các cột mốc phát triển (Milestones) của AxiomOS từ nền tảng đầu tiên cho đến một hệ thống userspace tối giản.

## Các Milestone

### Milestone 0: Nền tảng Repository (Hoàn thành)
- [x] Thiết lập Workspace Cargo.
- [x] Cấu hình Formatting, Linting và Github CI.
- [x] Viết và duyệt 3 Spec cốt lõi ban đầu (`000-project-charter`, `001-boot-and-kernel-entry`, `002-serial-logging`).
- [x] Cấu trúc repository hoàn chỉnh.

### Milestone 1: Kernel Có Thể Boot (Hoàn thành)
- [x] Tải bootloader Limine và thiết lập cấu hình.
- [x] Điểm vào Kernel (`_start`) bằng Rust `no_std`, `no_main`.
- [x] Giao tiếp sớm qua Serial COM1 để in log.
- [x] Framebuffer console tối thiểu mirror boot sequence lên màn hình QEMU.
- [x] Cơ chế xử lý Panic Kernel sơ khởi xuất ra Serial và framebuffer nếu sẵn sàng.
- [x] Viết script xây dựng đĩa ảo raw IMG và chạy thử nghiệm bằng QEMU.
- [x] Tích hợp kiểm thử tự động boot bằng QEMU trên CI.

### Milestone 2: Nền tảng CPU (Hoàn thành)
- [x] Thiết lập Bảng mô tả phân đoạn toàn cục (GDT).
- [x] Thiết lập Bảng mô tả ngắt (IDT).
- [x] Xử lý các CPU Exception cơ bản (như Page Fault, Double Fault).
- [x] Kích hoạt bộ điều khiển ngắt (APIC hoặc PIC) và Timer interrupt.
- [x] Driver bàn phím PS/2 cơ bản thông qua ngắt I/O.

### Milestone 3: Nền tảng Quản lý Bộ nhớ
- [x] Phân tích bản đồ bộ nhớ (Memory Map) cung cấp từ Limine.
- [x] Trình quản lý khung trang vật lý (Physical Frame Allocator) dạng Bitmap.
- [x] Ánh xạ bộ nhớ ảo tối thiểu cho heap kernel.
- [x] Trình cấp phát bộ nhớ Heap của Kernel (Kernel Heap Allocator).
- [x] Viết chẩn đoán bộ nhớ tối thiểu qua serial/framebuffer trong QEMU.

### Milestone 4: Dịch Vụ Kernel & Scheduler
- [x] Hệ thống ghi log có cấu trúc nâng cao.
  - [x] Facade logging kernel giai đoạn 1 với `LogRecord`, level, subsystem và mirror framebuffer tùy chọn.
  - [x] Runtime log filtering và ring buffer log.
- [x] Các thành phần đồng bộ hóa luồng cơ bản (Spinlock, Mutex).
- [x] Trình lập lịch tiến trình cộng tác (Cooperative Task Scheduler) cơ bản.
- [x] Đặc tả thiết kế trình lập lịch trưng dụng (Preemptive Scheduler).
- [x] Đồng hồ thời gian hệ thống (Timekeeping).

### Milestone 5: Hệ thống Tệp tin & Lưu trữ
- [x] Lớp trừu tượng hóa thiết bị khối (Block Device Abstraction).
- [x] Trình đọc hệ thống tệp tin FAT32 Read-only qua `BlockDevice` kernel-internal; driver đĩa QEMU trực tiếp chưa thuộc phạm vi mục này.
- [x] Thiết kế Hệ thống tệp tin ảo (VFS).
- [x] API đọc tệp tin từ Kernel qua VFS read-only với caller-provided buffer.

### Milestone 6: Nạp Chương Trình & Userspace (Đang thực hiện)
- [ ] Trình phân tích định dạng ELF64.
- [ ] Trình nạp chương trình ELF64 (ELF64 Loader).
- [ ] Không gian địa chỉ người dùng (Userspace Address Space).
- [ ] Giao diện cuộc gọi hệ thống (Syscall ABI).
- [ ] Tiến trình khởi tạo đầu tiên (`init`).

### Milestone 7: Môi trường Userspace Tối giản
- [ ] Tiến trình `init` chạy thành công ở chế độ User Mode (Ring 3).
- [ ] Chương trình dòng lệnh `shell` cơ bản.
- [ ] Hỗ trợ các lệnh cơ bản: liệt kê file (`ls`), đọc nội dung file (`cat`).
- [ ] Thư viện chuẩn C tối giản cho userspace (`libc`).

### Milestone 8: Nghiên Cứu Môi Trường Desktop
- [ ] Đặc tả thiết kế cho bộ tổng hợp đồ họa (Compositor).
- [ ] Nghiên cứu mô hình Window Server.
- [ ] Thiết lập lớp trừu tượng hóa đầu vào đồ họa (Mouse/Keyboard).
- [ ] Lựa chọn API đồ họa phù hợp (Framebuffer, GPU).
