# Spec: 009-process-scheduler (Process model và cooperative scheduler)

- **Feature ID**: 009-process-scheduler
- **Tiêu đề**: Process model và cooperative scheduler
- **Trạng thái**: DRAFT
- **Người phụ trách**: Kỹ sư trưởng AxiomOS
- **Ngày tạo**: 2026-07-06
- **Ngày cập nhật**: 2026-07-06

---

## Vấn đề cần giải quyết

AxiomOS cần một mô hình process/task tối thiểu để chạy init và các chương trình userspace sau này. Cooperative scheduler là bước đầu an toàn hơn preemptive scheduler vì giảm độ phức tạp interrupt và context switching.

## Mục tiêu

- Định nghĩa `ProcessId`, `ThreadId`, trạng thái task và lifecycle cơ bản.
- Tạo cooperative scheduler chạy một hoặc nhiều kernel task/userspace task tùy milestone.
- Cung cấp API yield tự nguyện.
- Chuẩn bị boundary cho preemptive scheduler sau này mà chưa bật preemption.
- Log state transition cơ bản để debug.

## Không thuộc phạm vi

- Không triển khai preemptive scheduling trong spec này.
- Không hỗ trợ SMP.
- Không triển khai priority scheduling phức tạp.
- Không hỗ trợ process isolation hoàn chỉnh nếu memory/userspace chưa sẵn sàng.
- Không chạy phần mềm Linux hoặc Windows.

## Ràng buộc

- Không allocation trong interrupt handler.
- Không context switch từ interrupt nếu chưa có ADR/preemptive spec.
- Không dùng global mutable state trần; scheduler state phải có synchronization rõ ràng.
- Context switching assembly phải có tài liệu ABI nếu được thêm.

## Dependencies

- Spec 004: memory management.
- Spec 005: interrupts/exceptions cho foundation CPU.
- Spec 008: ELF loader nếu scheduler chạy userspace process.

## ADR liên quan

- Cần ADR cho calling convention context switch và quyết định cooperative trước preemptive.

## Public interfaces

```rust
pub fn init_scheduler() -> Result<(), SchedulerError>;
pub fn spawn_task(entry: TaskEntry) -> Result<TaskId, SchedulerError>;
pub fn yield_now();
pub fn run_scheduler() -> !;
```

## Internal interfaces

```rust
struct TaskControlBlock {
    id: TaskId,
    state: TaskState,
    context: CpuContext,
}

enum TaskState {
    Ready,
    Running,
    Blocked,
    Exited,
}
```

## Data structures

- `TaskId`, `ProcessId`: định danh không tái sử dụng tùy tiện.
- `TaskControlBlock`: metadata task.
- `CpuContext`: register cần lưu khi switch.
- `RunQueue`: hàng đợi ready task.
- `SchedulerError`: lỗi spawn, queue full, context invalid.

## Xử lý lỗi

- Nếu run queue đầy, `spawn_task` trả `SchedulerError::QueueFull`.
- Nếu task panic, kernel log và đánh dấu task exited nếu có thể.
- Nếu scheduler không có task runnable, kernel halt hoặc idle loop rõ ràng.

## Hành vi logging

- Log khi scheduler init.
- Log task spawn và exit.
- Không log mỗi lần yield trong chế độ bình thường nếu gây nhiễu serial.

## Security considerations

- Context switch sai có thể phá stack hoặc nhảy vào memory không hợp lệ.
- Userspace isolation chưa hoàn chỉnh không được claim an toàn.
- Không expose primitive scheduler cho driver tùy tiện nếu chưa có policy.

## Kế hoạch test

- Unit test run queue push/pop.
- Kernel test tạo hai task cooperative cùng increment counter tĩnh.
- QEMU serial test xác nhận task A/B yield theo thứ tự mong đợi.
- Test queue full trả lỗi thay vì panic.

## Acceptance criteria

- **Acceptance Criterion 1**:
  - **Given** scheduler đã init với hai task test.
  - **When** mỗi task gọi `yield_now`.
  - **Then** serial log phải cho thấy cả hai task đều được chạy.

- **Acceptance Criterion 2**:
  - **Given** run queue đạt giới hạn.
  - **When** kernel gọi `spawn_task` thêm task mới.
  - **Then** API phải trả `SchedulerError::QueueFull`.

- **Acceptance Criterion 3**:
  - **Given** không có task runnable.
  - **When** scheduler loop chạy.
  - **Then** kernel phải vào idle/halt path có log rõ ràng.

## Kế hoạch rollback hoặc removal

- Có thể rollback về single execution path trong `_start`.
- Không được giữ scheduler giả chỉ in log task switch mà không chuyển quyền thực tế.

## Câu hỏi mở

- Context switch đầu tiên sẽ chỉ hỗ trợ kernel task hay hỗ trợ userspace ngay?
- Kích thước stack mặc định cho task là bao nhiêu?
