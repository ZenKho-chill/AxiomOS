# Spec: 012-synchronization-primitives (Cơ chế đồng bộ hóa Spinlock và Mutex)

- **Feature ID**: 012-synchronization-primitives
- **Tiêu đề**: Cơ chế đồng bộ hóa Spinlock và Mutex
- **Trạng thái**: COMPLETE
- **Người phụ trách**: Kỹ sư trưởng AxiomOS
- **Ngày tạo**: 2026-07-07
- **Ngày cập nhật**: 2026-07-07

---

## Vấn đề cần giải quyết

Khi kernel AxiomOS phát triển hệ thống lập lịch tiến trình và hỗ trợ nhiều driver truy cập bộ đệm dùng chung (như bàn phím, bộ nhớ), việc tranh chấp tài nguyên giữa các luồng thực thi và trình xử lý ngắt (interrupt handler) có thể xảy ra. Kernel cần các cơ chế khóa đồng bộ hóa cơ bản (Spinlock, Mutex) hoạt động an toàn trong môi trường `no_std` và hỗ trợ tắt ngắt khi giữ khóa để tránh deadlock.

## Mục tiêu

- Hiện thực cấu trúc `Spinlock` dựa trên nguyên tử (atomic operation `AtomicBool`).
- Hiện thực cấu trúc `Mutex` (hoặc bọc xung quanh `Spinlock` tạm thời) để đảm bảo đồng bộ hóa an toàn.
- Cung cấp cơ chế khóa an toàn với ngắt (Interrupt-safe Spinlock): tự động tắt ngắt trên CPU khi lấy khóa và khôi phục trạng thái ngắt ban đầu khi giải phóng khóa.
- Tích hợp thay thế cho các tài nguyên dùng chung hiện tại (ví dụ: ring buffer của driver bàn phím PS/2).

## Không thuộc phạm vi

- Không hỗ trợ luồng bị chặn (blocking queue) thực sự cho Mutex trong giai đoạn này (chỉ spin-wait vì chưa có scheduler hoàn chỉnh).
- Không giải quyết vấn đề đảo ngược độ ưu tiên (priority inversion).
- Không tuyên bố hỗ trợ đa lõi (SMP) đầy đủ trên bare-metal (chỉ kiểm thử trên QEMU đơn nhân).

## Ràng buộc

- Không sử dụng thư viện chuẩn `std`.
- Không cấp phát động (heap allocation) trong quá trình khóa/mở khóa.
- Unsafe code phải được bọc kỹ càng và ghi Safety comment đầy đủ.

## Dependencies

- Spec 005: Interrupts and exceptions.
- Spec 006: PS/2 Keyboard driver.

## ADR liên quan

- [adr-005-spinlock-mutex-implementation.md](../architecture/adr-005-spinlock-mutex-implementation.md): Quyết định tự hiện thực cơ chế Spinlock tối giản thay vì tích hợp các crate lớn từ ngoài.

## Public interfaces

```rust
pub struct Spinlock<T> { ... }
impl<T> Spinlock<T> {
    pub const fn new(data: T) -> Self;
    pub fn lock(&self) -> SpinlockGuard<'_, T>;
}

pub struct SpinlockIrqSave<T> { ... }
impl<T> SpinlockIrqSave<T> {
    pub const fn new(data: T) -> Self;
    pub fn lock(&self) -> SpinlockIrqSaveGuard<'_, T>;
}
```

## Internal interfaces

- Cơ chế bật/tắt ngắt CPU thông qua lệnh `cli` và `sti` (được bọc trong module `arch::x86_64`).

## Data structures

- `Spinlock<T>`: bọc dữ liệu và một cờ `AtomicBool`.
- `SpinlockGuard<T>`: RAII guard để tự động giải phóng khóa khi đi ra ngoài scope.
- `SpinlockIrqSaveGuard<T>`: RAII guard lưu trạng thái ngắt cũ và khôi phục khi giải phóng khóa.

## Xử lý lỗi

- Nếu xảy ra tình trạng cố gắng khóa đúp (double lock) trên cùng một luồng (deadlock), hệ thống có thể panic nếu bật debug_assertions hoặc treo máy an toàn.

## Hành vi logging

- Không ghi log trong đường dẫn thực thi khóa/mở khóa (lock/unlock path) để tránh re-entrancy deadlock (do logger cũng cần khóa).

## Security considerations

- Deadlock do ngắt: Nếu một ngắt xảy ra và handler cố gắng lấy một khóa đang bị giữ bởi luồng hiện tại, ngắt đó sẽ spin mãi mãi. SpinlockIrqSave bắt buộc phải tắt ngắt khi giữ khóa.

## Kế hoạch test

- Unit test thử nghiệm tranh chấp khóa giả lập trong môi trường an toàn.
- Tích hợp khóa mới vào driver bàn phím và kiểm tra gõ phím trong QEMU không bị lỗi/deadlock.

## Acceptance criteria

- **Acceptance Criterion 1**:
  - **Given** một `SpinlockIrqSave` bảo vệ tài nguyên bàn phím.
  - **When** luồng chính đang giữ khóa này.
  - **Then** ngắt bàn phím xảy ra không được gây ra deadlock (do ngắt đã bị vô hiệu hóa tạm thời).

- **Acceptance Criterion 2**:
  - **Given** hai luồng thực thi (giả lập).
  - **When** tranh chấp cùng một `Spinlock`.
  - **Then** chỉ có duy nhất một luồng có thể vào vùng găng (critical section) tại một thời điểm.

## Kế hoạch rollback hoặc removal

- Có thể rollback về việc sử dụng crate `spin` đơn giản hoặc bọc static thô không đồng bộ nếu cần debug.
