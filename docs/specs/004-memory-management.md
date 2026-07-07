# Spec: 004-memory-management (Nền tảng quản lý bộ nhớ)

- **Feature ID**: 004-memory-management
- **Tiêu đề**: Nền tảng quản lý bộ nhớ
- **Trạng thái**: APPROVED
- **Người phụ trách**: Kỹ sư trưởng AxiomOS
- **Ngày tạo**: 2026-07-06
- **Ngày cập nhật**: 2026-07-07

---

## Vấn đề cần giải quyết

Kernel cần biết vùng nhớ nào an toàn để dùng, cấp phát frame vật lý, thiết lập paging rõ ràng và cung cấp heap allocator tối thiểu cho các subsystem sau. Nếu không có memory foundation, interrupts, filesystem, loader và scheduler không có nền tảng an toàn.

## Mục tiêu

- Đọc và chuẩn hóa memory map từ Limine.
- Xây dựng physical frame allocator tối thiểu.
- Định nghĩa interface paging cho x86_64.
- Thiết lập kernel heap allocator nhỏ, có giới hạn rõ ràng.
- In memory diagnostics qua serial và framebuffer nếu có.

## Không thuộc phạm vi

- Không triển khai demand paging.
- Không swap.
- Không hỗ trợ NUMA.
- Không hỗ trợ userspace address space trong spec này.
- Không tối ưu allocator cho hiệu năng cao.

## Ràng buộc

- Không cấp phát từ vùng không được Limine đánh dấu usable.
- Không ghi đè kernel image hoặc dữ liệu bootloader chưa được phép reclaim.
- Không dùng `unwrap` hoặc `expect` trong kernel runtime path.
- Mọi global mutable state phải được bọc bởi primitive đồng bộ có tài liệu.
- Mọi unsafe block phải có safety comment.

## Dependencies

- Spec 001: Limine handoff.
- Spec 002: serial logging.
- Spec 003: framebuffer console là optional cho diagnostics.

## ADR liên quan

- [ADR-003: Chiến Lược Quản Lý Bộ Nhớ (Physical & Heap Allocator)](../architecture/adr-003-memory-management-strategy.md)

## Public interfaces

```rust
pub fn init_memory() -> Result<MemoryStats, MemoryError>;
pub fn allocate_frame() -> Result<PhysFrame, MemoryError>;
pub fn deallocate_frame(frame: PhysFrame) -> Result<(), MemoryError>;
pub fn memory_stats() -> MemoryStats;
pub fn hhdm_offset() -> Result<u64, MemoryError>;
```

## Internal interfaces

```rust
struct MemoryRegion {
    start: PhysAddr,
    length: u64,
    kind: MemoryRegionKind,
}

trait FrameAllocator {
    fn allocate(&mut self) -> Result<PhysFrame, MemoryError>;
    fn deallocate(&mut self, frame: PhysFrame) -> Result<(), MemoryError>;
}
```

## Data structures

- `PhysAddr`, `VirtAddr`: wrapper địa chỉ rõ loại.
- `PhysFrame`: frame vật lý 4 KiB.
- `MemoryRegion`: vùng memory đã chuẩn hóa.
- `MemoryStats`: tổng usable bytes, usable frames, allocated frames, free frames và số memory-map region đã đọc.
- `MemoryError`: lỗi memory map, HHDM, hết frame, frame ngoài vùng usable, double-free, địa chỉ frame không căn lề hoặc allocator chưa khởi tạo.

## Xử lý lỗi

- Nếu memory map trống hoặc không hợp lệ, log lỗi và halt an toàn.
- Nếu hết frame, trả `MemoryError::OutOfFrames`.
- Nếu `deallocate_frame` nhận frame không thuộc vùng Limine `usable`, trả `MemoryError::FrameNotUsable`.
- Nếu `deallocate_frame` nhận frame đã free, trả `MemoryError::FrameAlreadyFree`.
- Nếu heap init thất bại, kernel không được tiếp tục vào subsystem cần allocation.

## Hành vi logging

- Log số lượng memory region, usable memory và reserved memory.
- Log địa chỉ heap start/end khi heap được bật.
- Không dump toàn bộ memory map nếu output quá dài; chỉ bật dump chi tiết qua debug mode sau này.

## Security considerations

- Memory map sai hoặc xử lý sai có thể phá kernel image.
- Frame allocator phải tránh double-free và cấp phát trùng frame.
- Không expose physical address thô ra subsystem không cần biết hardware detail.

## Kế hoạch test

- Unit test parser memory map bằng dữ liệu giả lập trong crate test được phép.
- Unit test `PhysFrame::from_start_address` từ chối địa chỉ không căn lề.
- Unit test helper căn lề bitmap để tránh tính sai kích thước bitmap.
- QEMU boot test xác nhận diagnostics không crash.
- Test allocator cấp phát liên tiếp và không trả frame trùng.
- Test hết frame bằng vùng memory nhỏ giả lập.

## Acceptance criteria

- **Acceptance Criterion 1**:
  - **Given** Limine cung cấp memory map trong QEMU.
  - **When** `init_memory` chạy.
  - **Then** kernel phải log tổng usable memory và số region đã parse.

- **Acceptance Criterion 2**:
  - **Given** frame allocator đã init.
  - **When** kernel cấp phát nhiều frame liên tiếp.
  - **Then** mỗi frame trả về phải khác nhau và nằm trong vùng usable.

- **Acceptance Criterion 3**:
  - **Given** memory map không hợp lệ trong test.
  - **When** parser chạy.
  - **Then** parser phải trả `MemoryError` thay vì panic.

## Kế hoạch rollback hoặc removal

- Có thể rollback về trạng thái không init heap và chỉ dùng stack/static memory.
- Không được rollback bằng fake allocator trả địa chỉ hardcode.

## Câu hỏi mở

- Chọn bitmap allocator hay free-list allocator cho frame vật lý đầu tiên?
  - **Trả lời**: Chọn Bitmap Allocator để hiện thực đơn giản, trực quan và dễ gỡ lỗi (xem [ADR-003](../architecture/adr-003-memory-management-strategy.md)).
- Heap allocator dùng implementation tự viết hay crate đã được ADR phê duyệt?
  - **Trả lời**: Sử dụng crate `linked_list_allocator` đã được phê duyệt trong [ADR-003](../architecture/adr-003-memory-management-strategy.md) nhằm đảm bảo an toàn bộ nhớ.
