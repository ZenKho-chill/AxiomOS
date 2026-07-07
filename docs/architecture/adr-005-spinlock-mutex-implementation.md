# ADR-005: Chiến Lược Hiện Thực Spinlock và Mutex tối giản

- **Trạng thái**: APPROVED
- **Người đề xuất**: Kỹ sư trưởng AxiomOS
- **Ngày tạo**: 2026-07-07

## Bối cảnh

Để hỗ trợ phát triển scheduler và tránh tranh chấp dữ liệu giữa các luồng thực thi và interrupt handlers, AxiomOS cần các cơ chế khóa đồng bộ. Có hai lựa chọn chính:
1. Tích hợp crate ngoài có sẵn (ví dụ: `spin` hoặc `lock_api`).
2. Tự hiện thực cấu trúc Spinlock tối giản dựa trên kiểu dữ liệu nguyên tử nguyên bản của Rust (`core::sync::atomic::AtomicBool`).

## Quyết định

Chúng tôi quyết định chọn phương án tự hiện thực cấu trúc Spinlock tối giản.

## Lý do lựa chọn

- **Mục tiêu giáo dục**: Tự hiện thực Spinlock dựa trên các atomic instruction giúp hiểu sâu sắc cách thức CPU thực thi đồng bộ hóa ở mức phần cứng.
- **Tính kiểm soát và an toàn ngắt (Interrupt Safety)**: Để tránh deadlock khi interrupt handler cố lấy khóa đang bị giữ bởi luồng hiện tại, chúng ta cần một phiên bản Spinlock tự động tắt ngắt trên CPU cục bộ (`SpinlockIrqSave`). Việc tích hợp logic tắt ngắt của kiến trúc x86_64 (`cli`/`sti`) trực tiếp vào vòng lặp spin-lock sẽ dễ kiểm soát và tối ưu hơn khi tự viết.
- **Độc lập và no_std**: Tránh phụ thuộc vào các crate bên ngoài khi không cần thiết, giữ cho kernel nhẹ và độc lập tối đa.

## Hệ quả

- Chúng ta phải viết và bảo trì mã nguồn liên quan đến atomic spin-lock và kiểm tra kỹ lưỡng các vấn đề về memory ordering (`Ordering::Acquire`, `Ordering::Release`).
- Mã nguồn driver và scheduler sau này sẽ sử dụng trực tiếp interface đồng bộ tự viết này.
