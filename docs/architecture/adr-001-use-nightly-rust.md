# ADR 001: Sử dụng Rust Nightly cho Phát triển Kernel AxiomOS

- **Trạng thái**: APPROVED
- **Người phụ trách**: Kỹ sư trưởng AxiomOS
- **Ngày quyết định**: 2026-07-06

---

## Bối cảnh

Hệ điều hành AxiomOS nhắm tới biên dịch bare-metal target `x86_64-unknown-none`. Trong quá trình thiết lập bootloader Limine (Spec 001) và ghi log sớm qua Serial (Spec 002), chúng tôi sử dụng crate `limine` để tương tác trực tiếp với các cấu trúc dữ liệu của bootloader. 

Tuy nhiên, bắt đầu từ phiên bản `0.5.x` và `0.6.x` (các phiên bản duy nhất không bị gỡ - yanked trên crates.io), crate `limine` yêu cầu tính năng unstable `#![feature(ptr_metadata)]` của compiler. Nếu sử dụng kênh biên dịch Rust Stable, chúng tôi không thể biên dịch thành công.

Hơn nữa, trong các Milestone tiếp theo (Milestone 2 - CPU IDT/Exceptions và Milestone 3 - Allocator), việc lập trình hệ điều hành bare-metal đòi hỏi rất nhiều tính năng unstable khác của Rust như `naked_functions` (cho interrupt handler stubs), `asm_const`, `allocator_api` để tránh việc phải viết quá nhiều mã Assembly thuần phức tạp.

## Quyết định

Chuyển đổi Rust Toolchain của dự án từ Stable sang **Nightly** (phiên bản cập nhật mới nhất). Quyết định này được áp dụng thông qua tệp tin cấu hình `rust-toolchain.toml` ở thư mục gốc của dự án.

## Hệ quả

- **Tích cực**:
  - Biên dịch thành công crate `limine` phiên bản mới nhất (`0.6.x`) trên crates.io.
  - Cho phép sử dụng các tính năng unstable mạnh mẽ và an toàn hơn cho lập trình OS như `naked_functions`, `asm_const` ở Milestone 2.
  - Đơn giản hóa việc tích hợp bộ cấp phát bộ nhớ tùy chỉnh bằng `allocator_api`.
- **Tiêu cực**:
  - Kênh Nightly có thể chứa các thay đổi phá vỡ tương thích (breaking changes) của compiler trong tương lai. Điều này sẽ được kiểm soát bằng cách cố định phiên bản nightly cụ thể trong `rust-toolchain.toml` nếu xảy ra xung đột lớn.
