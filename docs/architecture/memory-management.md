# Quản lý Bộ nhớ (Memory Management)

Tài liệu này phác thảo thiết kế hệ thống quản lý bộ nhớ của AxiomOS.

## Các tầng quản lý bộ nhớ

1. **Physical Frame Allocator (Bộ cấp phát khung trang vật lý)**:
   - Đọc thông tin bản đồ bộ nhớ (Memory Map) từ Limine.
   - Quản lý các khung trang vật lý kích thước 4 KiB bằng bitmap allocator.
   - Chỉ cấp phát frame thuộc vùng Limine `usable`; `deallocate_frame` phải từ chối frame reserved, framebuffer, kernel/module hoặc frame đã free.

2. **Virtual Memory Paging (Phân trang bộ nhớ ảo)**:
   - Sử dụng bảng phân trang 4 tầng của kiến trúc x86_64.
   - Ở Spec 004, chỉ ánh xạ tối thiểu vùng heap ảo của kernel.
   - Userspace address space là công việc tương lai của Spec 008/010 và chưa được triển khai trong phạm vi này.

3. **Kernel Heap Allocator (Bộ cấp phát Heap)**:
   - Cung cấp cơ chế cấp phát động trong Kernel thông qua crate `alloc` của Rust và `linked_list_allocator`.
   - Heap đầu tiên có kích thước cố định 8 MiB và chỉ được khởi tạo sau khi frame allocator cùng HHDM offset đã hợp lệ.
   - Khi tạo page table userspace, mapping L4 của kernel heap được copy dạng supervisor-only để syscall handler có thể truy cập object kernel/VFS trong lúc CR3 đang là page table userspace. Mapping này không bật cờ `USER`.

## Interface nội bộ hiện tại

- `init_memory()`: đọc memory map và HHDM từ Limine, khởi tạo bitmap allocator.
- `allocate_frame()`: trả `PhysFrame` đã căn lề 4 KiB hoặc `MemoryError`.
- `deallocate_frame(frame)`: chỉ giải phóng frame đã cấp phát và thuộc vùng usable.
- `memory_stats()`: trả tổng usable bytes, usable frames, allocated frames, free frames và số region đã đọc.
- `hhdm_offset()`: trả HHDM offset đã được xác thực trong quá trình khởi tạo memory.

Các API này là internal kernel API, chưa phải ABI ổn định cho userspace.
