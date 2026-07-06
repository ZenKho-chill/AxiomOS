# Quản lý Bộ nhớ (Memory Management)

Tài liệu này phác thảo thiết kế hệ thống quản lý bộ nhớ của AxiomOS.

## Các tầng quản lý bộ nhớ

1. **Physical Frame Allocator (Bộ cấp phát khung trang vật lý)**:
   - Đọc thông tin bản đồ bộ nhớ (Memory Map) từ Limine.
   - Quản lý các khung trang vật lý kích thước 4KB bằng cấu trúc dữ liệu Bitmap hoặc Free List.

2. **Virtual Memory Paging (Phân trang bộ nhớ ảo)**:
   - Sử dụng bảng phân trang 4 tầng của kiến trúc x86_64.
   - Ánh xạ bộ nhớ ảo của Kernel và Userspace độc lập để đảm bảo an toàn.

3. **Kernel Heap Allocator (Bộ cấp phát Heap)**:
   - Cung cấp cơ chế cấp phát động trong Kernel (sử dụng crate `alloc` của Rust) thông qua các thuật toán như Linked List Allocator hoặc Fixed-size Block Allocator.
