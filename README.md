# AxiomOS

*(Logo AxiomOS - Placeholder)*

**Tagline**: Hệ điều hành desktop modular được xây dựng từ đầu.

AxiomOS là một hệ điều hành thử nghiệm được thiết kế và xây dựng hoàn toàn từ đầu cho nền tảng phần cứng PC x86_64, sử dụng ngôn ngữ lập trình Rust. Mục tiêu chính của dự án là phục vụ học tập, nghiên cứu kỹ thuật hệ điều hành, quản lý bộ nhớ, lập lịch tiến trình và phát triển hệ thống modular.

> [!WARNING]
> **CẢNH BÁO AN TOÀN QUAN TRỌNG:**
> - AxiomOS KHÔNG PHẢI là Linux.
> - AxiomOS KHÔNG PHẢI là Windows.
> - AxiomOS KHÔNG hướng tới việc tương thích hay chạy phần mềm Windows/Linux ở các milestone đầu.
> - AxiomOS hiện tại là dự án thử nghiệm (experimental). **KHÔNG dùng AxiomOS trên máy tính chứa dữ liệu quan trọng hoặc máy sản xuất (production).**
> - Chỉ nên kiểm thử AxiomOS bên trong trình giả lập QEMU hoặc các máy ảo có thể xóa cho đến khi cơ chế hỗ trợ phần cứng đủ trưởng thành. AxiomOS có thể làm hỏng dữ liệu hoặc không boot được nếu chạy trực tiếp trên phần cứng thật.

---

## Tổng Quan Kiến Trúc & Lý Do Chọn Công Nghệ

- **Target Platform**: PC x86_64
- **Firmware & Bootloader**: UEFI sử dụng bootloader Limine với giao thức Limine protocol, giúp đơn giản hóa quá trình chuyển giao trạng thái CPU sang chế độ 64-bit Long Mode và nhận thông tin cấu hình phần cứng.
- **Ngôn ngữ chính**: Rust (`no_std`, `no_main`), giúp đảm bảo an toàn bộ nhớ và cung cấp các tính năng hiện đại mà không cần runtime mặc định. Assembly được sử dụng tối thiểu khi xử lý CPU entry, interrupt stubs hoặc context switching.
- **Hiển thị**: Framebuffer console.
- **Giao tiếp**: Serial port COM1 để debug và in log sớm.

---

## Cấu Trúc Repository

```text
axiomOS/
├── README.md
├── CHANGELOG.md
├── LICENSE
├── CONTRIBUTING.md
├── CODE_OF_CONDUCT.md
├── SECURITY.md
├── Makefile
├── Cargo.toml
├── rust-toolchain.toml
├── docs/               # Tài liệu thiết kế, roadmap và đặc tả
├── scripts/            # Script xây dựng đĩa ảo và khởi chạy QEMU
├── kernel/             # Mã nguồn Kernel viết bằng Rust
├── userspace/          # Mã nguồn các chương trình userspace
├── tools/              # Công cụ hỗ trợ build image và tác vụ phụ
├── assets/             # File cấu hình bootloader Limine
└── target/             # Build output (do Cargo tự sinh)
```

---

## Yêu Cầu Môi Trường & Thiết Lập

### Trên Linux (Ubuntu/Debian) / Windows WSL2

1. **Cài đặt các gói phụ trợ**:
   ```bash
   sudo apt update
   sudo apt install -y build-essential qemu-system-x86 llvm clang mtools parted dosfstools git
   ```

2. **Cài đặt Rust toolchain**:
   Truy cập [rustup.rs](https://rustup.rs) và cài đặt phiên bản Rust stable mới nhất.

3. **Cài đặt rust-src**:
   ```bash
   rustup component add rust-src
   ```

---

## Hướng Dẫn Build & Chạy Thử

Hệ thống sử dụng `Makefile` để tự động hóa:

- **Biên dịch kernel**:
  ```bash
  make build
  ```
- **Xây dựng đĩa ảo bootable (RAW image)**:
  ```bash
  make image
  ```
- **Chạy thử trên giả lập QEMU**:
  ```bash
  make run
  ```
- **Debug bằng GDB**:
  ```bash
  make debug
  ```
- **Kiểm tra định dạng (format)**:
  ```bash
  make fmt
  ```
- **Kiểm tra chất lượng mã nguồn (linter)**:
  ```bash
  make lint
  ```
- **Dọn dẹp build files**:
  ```bash
  make clean
  ```

---

## Quy Trình Phát Triển & Spec Kit

Dự án áp dụng quy trình **Spec Kit** nghiêm ngặt cho mọi tính năng mới:
1. Soạn thảo tài liệu đặc tả (Spec) trong thư mục `docs/specs/` dưới trạng thái `DRAFT`.
2. Trải qua các bước `REVIEW` -> `APPROVED`.
3. Chỉ tiến hành lập trình (`IMPLEMENTING`) khi spec đã được phê duyệt.
4. Mọi mô tả kiểm thử phải sử dụng mẫu `Given / When / Then`.

Tất cả các thay đổi phải tuân thủ nghiêm ngặt **Context Lock Protocol** được mô tả trong tài liệu [AGENTS.md](AGENTS.md).

---

## Lịch Trình Phát Triển (Roadmap)

Xem lộ trình phát triển chi tiết tại tài liệu [roadmap.md](./docs/architecture/roadmap.md). Các cột mốc lớn bao gồm:
- **Milestone 0**: Thiết lập Repository và Môi trường.
- **Milestone 1**: Boot Kernel qua Limine và xuất log ra COM1 Serial.
- **Milestone 2**: Cấu trúc CPU Exception, GDT, IDT và ngắt ngắt bàn phím.
- **Milestone 3**: Hệ thống quản lý bộ nhớ (Frame Allocator, Paging, Heap).
- **Milestone 4**: Scheduler hợp tác (Cooperative Scheduler) và dịch vụ Kernel.
- **Milestone 5**: Hệ thống tệp tin FAT32 Read-only.
- **Milestone 6**: Trình nạp ELF64 và kích hoạt Userspace.
- **Milestone 7**: Hệ thống Userspace tối giản (init, shell).

---

## Quy Tắc Đóng Góp (Contributing)

Xem chi tiết tại [CONTRIBUTING.md](CONTRIBUTING.md). Chúng tôi tuân thủ quy tắc ứng xử [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) và báo cáo bảo mật tại [SECURITY.md](SECURITY.md).

---

## Giấy Phép (License)

Dự án được phân phối dưới giấy phép MIT. Xem thêm chi tiết tại file [LICENSE](./LICENSE).
