# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
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

### Fixed
- Sửa cấu hình Limine 7.x để entry AxiomOS dùng `PROTOCOL=limine` và `KERNEL_PATH=boot:///boot/kernel.elf`, tránh lỗi `[config file contains no valid entries]`.
- Sửa linker script để các `PT_LOAD` segment khác quyền không dùng chung một memory page khi Limine nạp kernel ELF.
- Sửa serial boot sequence để khớp đúng các dòng log bắt buộc của Milestone 1.
- Sửa lỗi phân quyền thực thi (Permission Denied) cho binary limine trong quá trình build image trên CI.

### Changed
- Chuyển trạng thái specs `000-project-charter`, `001-boot-and-kernel-entry` và `002-serial-logging` sang `COMPLETE` sau khi acceptance criteria đã được xác minh.
- Chuyển trạng thái spec `003-framebuffer-console` sang `COMPLETE` sau khi xác minh serial, screenshot QEMU và panic-test.
