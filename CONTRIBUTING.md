# Hướng dẫn Đóng góp (Contributing Guidelines)

Chào mừng bạn đến với dự án AxiomOS! Chúng tôi rất hoan nghênh sự đóng góp của cộng đồng nhằm hoàn thiện và phát triển hệ điều hành modular này.

## Quy tắc chung

1. **Tuân thủ Context Lock Protocol**: Mọi thay đổi hoặc giao tiếp liên quan đến mã nguồn/tài liệu phải khai báo mục tiêu ảnh hưởng theo đúng mẫu quy định tại [AGENTS.md](AGENTS.md).
2. **Quy trình Spec Kit**: Không tự ý lập trình tính năng mới khi chưa có đặc tả kỹ thuật tương ứng ở trạng thái `APPROVED`.
3. **Chất lượng mã nguồn**: Mọi dòng code Rust phải được format bằng `cargo fmt` và kiểm tra lỗi thông qua `cargo clippy`. Warning sẽ được coi là Error trên hệ thống CI.
4. **Thông điệp Commit**: Sử dụng định dạng Conventional Commits. Các loại commit hợp lệ bao gồm: `feat`, `fix`, `docs`, `refactor`, `test`, `build`, `ci`, `chore`, `perf`, `security`.

## Quy trình đóng góp

1. Fork dự án và tạo nhánh mới có tiền tố phù hợp:
   - `feature/` cho tính năng mới.
   - `fix/` cho các bản sửa lỗi.
   - `docs/` cho việc bổ sung hoặc cải thiện tài liệu.
2. Thực hiện các chỉnh sửa và viết kiểm thử đầy đủ.
3. Chạy `make fmt` và `make lint` để kiểm tra.
4. Tạo Pull Request (PR) đính kèm mã số Spec ID liên quan, tóm tắt thay đổi và kết quả chạy thử trong trình giả lập QEMU.
