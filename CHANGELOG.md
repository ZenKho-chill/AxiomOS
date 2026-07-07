# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Thêm lớp trừu tượng hóa thiết bị khối (Block Device Abstraction) hỗ trợ trait BlockDevice và mock RamDisk.
- Cập nhật Spec 015-block-device-abstraction sang trạng thái COMPLETE.
- Thêm tài liệu đặc tả thiết kế trình lập lịch trưng dụng (Preemptive Scheduler spec) dưới dạng Spec 014-preemptive-scheduler-design ở trạng thái COMPLETE.
- Thêm trình lập lịch tiến trình cộng tác (Cooperative Scheduler) hỗ trợ chuyển đổi ngữ cảnh bằng Assembly x86_64, TCB và API yield_now().
- Cập nhật Spec 009-process-scheduler sang trạng thái COMPLETE.
- Thêm hệ thống quản lý thời gian (Timekeeping) hỗ trợ cấu hình PIT 1000Hz, API uptime_ms() và sleep_ms().
- Cập nhật Spec 013-system-timekeeping sang trạng thái COMPLETE.
- Thêm cơ chế lọc log động ở runtime và bộ đệm xoay vòng ring buffer tĩnh trong kernel cho Milestone 4.
- Thêm các thành phần đồng bộ hóa luồng cơ bản tự viết gồm Spinlock, SpinlockIrqSave và Mutex an toàn cho ngắt CPU.
- Thêm Spec 012 và tài liệu kiến trúc ADR-005 cho cơ chế đồng bộ hóa tối giản.
- Tích hợp SpinlockIrqSave vào driver bàn phím PS/2 để thay thế hoàn toàn thư viện ngoài spin.
- Khởi tạo cấu trúc repository ban đầu theo quy chuẩn [AGENTS.md](AGENTS.md).
- Thiết lập Cargo workspace và cấu hình `rust-toolchain.toml` sử dụng phiên bản Rust nightly với target `x86_64-unknown-none`.
- Thêm tài liệu quyết định kiến trúc [adr-001-use-nightly-rust.md](./docs/architecture/adr-001-use-nightly-rust.md).
- Thêm các file workflow CI bằng Github Actions và Issue/PR templates trong thư mục `.github/`.
- Thêm `Makefile` cơ bản để tự động hóa build, format và kiểm tra lint mã nguồn.
- Tạo hệ thống tài liệu nền tảng, roadmap dự án và tài liệu pháp lý (`README.md`, `LICENSE`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`).
- Thêm `userspace/Cargo.toml` và issue template `subsystem_propesal.md` để hoàn thiện cấu trúc repository bắt buộc của Spec 000.
- Thêm marker `.gitkeep` cho các thư mục kernel rỗng bắt buộc để cấu trúc repository Spec 000 được lưu trong git mà không tạo implementation giả.
- Soạn thảo và duyệt (`APPROVED`) 3 spec đặc tả kỹ thuật đầu tiên:
  - `000-project-charter.md` (Hiến chương dự án)
  - `001-boot-and-kernel-entry.md` (Quá trình khởi động và điểm vào Kernel)
  - `002-serial-logging.md` (Hệ thống ghi log sớm qua cổng nối tiếp COM1)
- Hoàn thiện nội dung Spec Kit cho các spec DRAFT `003` đến `010`, bao gồm mục tiêu, non-goals, dependency, interfaces, test plan và acceptance criteria.
- Thêm framebuffer console tối thiểu để hiển thị boot sequence trong cửa sổ QEMU khi Limine cung cấp framebuffer hợp lệ.
- Thêm feature test-only `panic-test` để kiểm chứng panic handler ghi ra framebuffer sau khi console sẵn sàng.
- Thêm hệ thống ngắt và ngoại lệ CPU (Spec 005) bao gồm Interrupt Descriptor Table (IDT), các handlers cho exceptions (Divide-by-zero, Breakpoint, Double Fault, General Protection Fault, Page Fault) và driver remap 8259 PIC.
- Thêm tài liệu quyết định kiến trúc [adr-002-use-8259-pic.md](./docs/architecture/adr-002-use-8259-pic.md) giải thích việc lựa chọn bộ ngắt 8259 PIC thay vì APIC.
- Thêm driver bàn phím PS/2 (Spec 006) hỗ trợ giải mã Scancode Set 1, tích hợp ring buffer tĩnh đồng bộ bằng spinlock Mutex.
- Bật ngắt Timer (IRQ 0) của hệ thống và xử lý tăng tick định kỳ kiểm chứng ngắt phần cứng.
- Thêm nền tảng quản lý bộ nhớ theo Spec 004: đọc Limine memory map, lấy HHDM offset, bitmap physical frame allocator, paging helper, kernel heap 8 MiB và memory diagnostics qua serial/framebuffer.
- Thêm unit test kernel cho `PhysFrame::from_start_address` và helper căn lề bitmap trong memory foundation.
- Thêm cấu hình CodeRabbit để review tự động các PR target `main`, `milestone-*` và `feature/*` theo quy tắc AxiomOS.
- Thêm Spec 011, ADR 004 và logging facade nội bộ cho Milestone 4 với `LogRecord`, level, subsystem và mirror framebuffer tùy chọn.

### Fixed
- Sửa lỗi tranh chấp và RefCell already borrowed trong các unit test của module logging bằng cách tối ưu hóa scope của lock guard.
- Sửa cấu hình CI chỉ chạy push trên main nhằm loại bỏ trùng lặp workflow kiểm thử khi đẩy commit lên các nhánh feature đang có PR mở.
- Sửa cấu hình CodeRabbit để tự động review cả PR nháp (Draft PR) trên mọi nhánh.
- Sửa cấu hình Limine 7.x để entry AxiomOS dùng `PROTOCOL=limine` và `KERNEL_PATH=boot:///boot/kernel.elf`, tránh lỗi `[config file contains no valid entries]`.
- Sửa linker script để các `PT_LOAD` segment khác quyền không dùng chung một memory page khi Limine nạp kernel ELF.
- Sửa serial boot sequence để khớp đúng các dòng log bắt buộc của Milestone 1.
- Sửa lỗi phân quyền thực thi (Permission Denied) cho binary limine trong quá trình build image trên CI.
- Sửa cấu hình timeout của bootloader và thời gian chờ kiểm thử QEMU trên CI để tránh việc log serial trống do CI chạy chậm.
- Sửa thống kê frame allocator để `allocated_frames` chỉ tính frame thuộc vùng Limine `usable`, tránh log số frame đã dùng bị phóng đại bởi framebuffer hoặc vùng reserved cao.
- Sửa `deallocate_frame` để từ chối frame không thuộc vùng usable và frame đã free, tránh cấp phát lại vùng reserved/kernel/MMIO do caller truyền sai.
- Sửa khởi tạo heap để không fallback HHDM về `0`; kernel giờ chỉ init heap khi HHDM offset đã được memory module xác thực.

### Changed
- Chuyển trạng thái specs `000-project-charter`, `001-boot-and-kernel-entry` và `002-serial-logging` sang `COMPLETE` sau khi acceptance criteria đã được xác minh.
- Chuyển trạng thái spec `003-framebuffer-console` sang `COMPLETE` sau khi xác minh serial, screenshot QEMU và panic-test.
- Chuyển trạng thái đặc tả `005-interrupts-and-exceptions` sang `COMPLETE` sau khi đã hiện thực hóa IDT/PIC và kiểm chứng thành công breakpoint exception (int3) trên local.
- Chuyển trạng thái đặc tả `006-keyboard-input` sang `COMPLETE` sau khi đã hiện thực hóa driver bàn phím, ngắt Timer/Keyboard và kiểm chứng thành công qua QEMU monitor.
