# Spec: 000-project-charter (Hiến chương dự án AxiomOS)

- **Feature ID**: 000-project-charter
- **Tiêu đề**: Hiến chương dự án AxiomOS
- **Trạng thái**: APPROVED
- **Người phụ trách**: Kỹ sư trưởng AxiomOS
- **Ngày tạo**: 2026-07-06
- **Ngày cập nhật**: 2026-07-06

---

## Vấn đề cần giải quyết

Xác định tầm nhìn, phạm vi, định hướng kiến trúc, và các quy tắc phát triển cốt lõi cho hệ điều hành AxiomOS để ngăn chặn sự chệch hướng mục tiêu (scope creep) và đảm bảo tính thống nhất trong suốt quá trình phát triển.

## Mục tiêu

- Thiết lập một lộ trình phát triển modular gồm 8 milestone rõ ràng.
- Ràng buộc chặt chẽ quy trình phát triển dựa trên tài liệu đặc tả (Spec Kit).
- Định nghĩa các tiêu chuẩn về an toàn bộ nhớ và chất lượng mã nguồn bằng Rust.

## Không thuộc phạm vi (Non-goals)

- Khả năng tương thích nhị phân với Windows hoặc Linux.
- Khả năng chạy trực tiếp trên các phần cứng bare-metal thực tế không được kiểm chứng qua ảo hóa.
- Hệ thống giao diện đồ họa (GUI) hay các subsystem phức tạp (USB, Audio, Network) trong các giai đoạn đầu.

## Ràng buộc

- Ngôn ngữ lập trình chính cho Kernel: Rust stable (`no_std`, `no_main`).
- Target platform: `x86_64` (UEFI bootloader Limine).

## Dependencies

- Cần trình giả lập QEMU và các công cụ đóng gói đĩa ảo (`parted`, `mtools`, `dosfstools`) trên môi trường host.

## ADR liên quan

- Chưa có.

## Public interfaces

- Không có (Đây là tài liệu đặc tả hiến chương dự án).

## Internal interfaces

- Không có.

## Data structures

- Không có.

## Xử lý lỗi

- Không có.

## Hành vi logging

- Không có.

## Security considerations

- Đây là hệ điều hành học tập, không thiết kế để chịu các cuộc tấn công mạng hay đảm bảo an ninh cấp độ cao ở các Milestone đầu tiên.

## Kế hoạch test

- Xác minh cấu trúc repository phải khớp hoàn chỉnh 100% với quy chuẩn cấu trúc thư mục quy định tại [AGENTS.md](../../AGENTS.md).

## Acceptance criteria

- **Acceptance Criterion 1**:
  - **Given** Repository AxiomOS được khởi tạo.
  - **When** Kiểm tra cấu trúc cây thư mục.
  - **Then** Toàn bộ các thư mục `kernel/`, `userspace/`, `docs/`, `scripts/`, `tools/`, `assets/` phải tồn tại đầy đủ.

## Kế hoạch rollback hoặc removal

- Không áp dụng.

## Câu hỏi mở

- Không có.
