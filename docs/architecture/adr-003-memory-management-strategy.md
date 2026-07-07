# ADR-003: Chiến Lược Quản Lý Bộ Nhớ (Physical & Heap Allocator)

- **Trạng thái**: APPROVED
- **Người phụ trách**: Kỹ sư trưởng AxiomOS
- **Ngày tạo**: 2026-07-07
- **Ngày cập nhật**: 2026-07-07

---

## Bối cảnh và vấn đề cần giải quyết

Để AxiomOS có thể hỗ trợ các subsystem phức tạp hơn như nạp tệp tin (FAT32), tạo tiến trình (scheduler), và tương tác với người dùng (shell), Kernel bắt buộc phải có khả năng cấp phát bộ nhớ động (Heap Allocation) và quản lý các khung trang vật lý (Physical Frame Allocation). 

Mục tiêu của Milestone 3 là xây dựng một nền tảng quản lý bộ nhớ vững chắc, an toàn và dễ debug. Chúng ta cần đưa ra các quyết định kiến trúc cụ thể cho:
1. Thuật toán và cấu trúc dữ liệu cho **Physical Frame Allocator** (Trình quản lý trang vật lý).
2. Thư viện / Cơ chế cho **Kernel Heap Allocator** (Trình cấp phát heap của kernel).

## Các phương án cân nhắc

### 1. Đối với Physical Frame Allocator
- **Phương án A: Bitmap Allocator**
  - *Mô tả*: Sử dụng một mảng bit (bitmap), trong đó mỗi bit đại diện cho trạng thái của một frame 4 KiB (0 = Free, 1 = Used).
  - *Ưu điểm*: Cực kỳ trực quan, dễ hiện thực, dễ gỡ lỗi (debug) thông qua in trạng thái bitmap. Không có hiện tượng phân mảnh dữ liệu cấu trúc vật lý.
  - *Nhược điểm*: Cần một vùng nhớ vật lý liên tục ban đầu để lưu trữ chính bitmap đó. Thời gian tìm kiếm frame trống là $O(N)$ nếu không được tối ưu.
- **Phương án B: Linked List / Free List Allocator**
  - *Mô tả*: Lưu trữ các con trỏ tới vùng nhớ trống trực tiếp trong chính các trang vật lý trống (không tốn thêm bộ nhớ lưu bitmap).
  - *Ưu điểm*: Cấp phát và giải phóng cực nhanh $O(1)$.
  - *Nhược điểm*: Phức tạp hơn khi khởi tạo, dễ gặp lỗi tham chiếu bộ nhớ chưa được map (Virtual Paging) nếu không cẩn thận. Khó kiểm tra và debug trạng thái bộ nhớ từ bên ngoài.

### 2. Đối với Kernel Heap Allocator
- **Phương án A: Tự viết thuật toán cấp phát Heap (ví dụ: Linked List Allocator đơn giản)**
  - *Mô tả*: Tự thiết kế một danh sách liên kết các block trống và thực hiện thuật toán First-Fit/Best-Fit.
  - *Ưu điểm*: Hiểu sâu sắc cơ chế hoạt động của heap allocator.
  - *Nhược điểm*: Dễ gặp lỗi an toàn bộ nhớ (memory safety), lỗi phân mảnh dữ liệu (fragmentation), và mất nhiều thời gian debug các trường hợp biên của cấp phát/giải phóng.
- **Phương án B: Sử dụng crate bên ngoài đã được kiểm chứng (ví dụ: `linked_list_allocator`)**
  - *Mô tả*: Tích hợp crate `linked_list_allocator` (một heap allocator nhỏ gọn, an sau, hỗ trợ khóa và được dùng phổ biến trong bare-metal Rust).
  - *Ưu điểm*: Cực kỳ an toàn, đã được cộng đồng kiểm chứng qua nhiều năm, tích hợp hoàn hảo với global allocator của Rust (`#[global_allocator]`), tiết kiệm thời gian phát triển để tập trung vào cơ chế ảo hóa và paging.
  - *Nhược điểm*: Thêm một dependency ngoài vào kernel workspace.

## Quyết định lựa chọn

1. **Physical Frame Allocator**: Chọn **Phương án A (Bitmap Allocator)**.
   - Vị trí lưu bitmap sẽ được tính toán động và đặt tại vùng bộ nhớ `usable` đầu tiên có kích thước đủ lớn được trả về từ Limine Memory Map.
   - Bitmap sẽ quản lý toàn bộ dải bộ nhớ vật lý của hệ thống.
2. **Kernel Heap Allocator**: Chọn **Phương án B (Sử dụng crate `linked_list_allocator`)**.
   - Quyết định này được đưa ra nhằm đảm bảo an toàn tối đa cho Kernel Runtime, tránh các lỗi undefined behavior do lỗi cấp phát heap tự viết gây ra.
   - Crate `linked_list_allocator` được thêm vào `Cargo.toml` của kernel dưới dạng dependency cho no_std.

## Hệ quả và ảnh hưởng

- **Dependencies mới**: Dự án sẽ thêm crate `linked_list_allocator` phiên bản `^0.10.6` vào kernel dependencies.
- **Sử dụng bộ nhớ**: Kernel sẽ tiêu tốn một phần nhỏ bộ nhớ vật lý ban đầu (khoảng vài chục KiB tùy thuộc vào tổng dung lượng RAM) để lưu trữ bitmap của Frame Allocator.
- **Tính an toàn**: Tích hợp global allocator của Rust cho phép chúng ta sử dụng các cấu trúc dữ liệu động quen thuộc như `Box`, `Vec`, `BTreeMap` từ thư viện `alloc` của Rust cho các milestone tiếp theo một cách an toàn và dễ dàng.
