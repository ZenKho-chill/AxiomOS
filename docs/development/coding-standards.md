# Quy chuẩn Viết code (Coding Standards)

Để duy trì chất lượng mã nguồn AxiomOS, mọi nhà phát triển phải tuân thủ nghiêm ngặt các quy tắc sau.

## Quy tắc Rust

1. **no_std và no_main**:
   - Tất cả mã nguồn trong kernel phải bắt đầu bằng `#![no_std]` và `#![no_main]`.

2. **Quản lý mã Unsafe**:
   - Tối thiểu hóa việc sử dụng `unsafe`.
   - Mỗi khối `unsafe` bắt buộc phải có comment giải thích tính an toàn (`// SAFETY: <giải thích lý do an toàn>`) ngay phía trên.

3. **Tránh Panic ngầm định**:
   - Không được dùng `.unwrap()` hoặc `.expect()` trong luồng xử lý runtime của Kernel.
   - Thay vào đó, hãy sử dụng cơ chế xử lý lỗi `Result` và propagate lỗi bằng toán tử `?`.

4. **Định dạng code**:
   - Bắt buộc chạy `cargo fmt` trước khi commit.

5. **Linter**:
   - Chạy `cargo clippy` để phát hiện các đoạn code chưa tối ưu. Warnings sẽ được coi là Errors trên CI.
