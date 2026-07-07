//! Bộ lập lịch tiến trình cộng tác (Cooperative Task Scheduler) toàn cục

use crate::process::task::{Task, TaskState};
use crate::serial_println;
use crate::utils::sync::SpinlockIrqSave;
use alloc::boxed::Box;
use alloc::collections::VecDeque;

/// Quản lý danh sách ready tasks và task hiện tại của Scheduler
struct Scheduler {
    ready_queue: VecDeque<Box<Task>>,
    current_task: Option<Box<Task>>,
    next_id: u32,
}

impl Scheduler {
    const fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
            current_task: None,
            next_id: 1,
        }
    }

    fn spawn(&mut self, entry: fn()) {
        let id = self.next_id;
        self.next_id += 1;
        let task = Box::new(Task::new(id, entry));
        self.ready_queue.push_back(task);
    }
}

/// Instance toàn cục của Scheduler được bảo vệ bởi SpinlockIrqSave
static SCHEDULER: SpinlockIrqSave<Scheduler> = SpinlockIrqSave::new(Scheduler::new());

/// Khởi tạo scheduler và đăng ký luồng chạy hiện tại làm main task
pub fn init() {
    let mut sched = SCHEDULER.lock();

    // Main task đại diện cho luồng chính khởi động của kernel.
    // Stack của nó chính là stack khởi động đang chạy, nên ta dùng mảng rỗng để không cấp phát thừa.
    let main_task = Box::new(Task {
        id: 0,
        stack: Box::new([]),
        stack_ptr: 0,
        state: TaskState::Running,
    });

    sched.current_task = Some(main_task);
}

/// Spawn một task nhân mới
pub fn spawn(entry: fn()) {
    SCHEDULER.lock().spawn(entry);
}

/// Nhường quyền điều khiển CPU cho task tiếp theo trong hàng đợi
pub fn yield_now() {
    // 1. Lưu trạng thái ngắt và tắt ngắt để đảm bảo an toàn context switch
    let interrupts_enabled = crate::arch::x86_64::instructions::are_interrupts_enabled();
    if interrupts_enabled {
        // SAFETY: Việc điều khiển ngắt qua cli yêu cầu CPU chạy ở Ring 0.
        unsafe {
            crate::arch::x86_64::instructions::cli();
        }
    }

    let mut switch_args = None;

    // 2. Logic cập nhật Scheduler nằm trong scope lock ngắn để giải phóng khóa trước khi switch
    {
        let mut sched = SCHEDULER.lock();
        if !sched.ready_queue.is_empty() {
            if let Some(mut current) = sched.current_task.take() {
                current.state = TaskState::Ready;
                if let Some(mut next) = sched.ready_queue.pop_front() {
                    next.state = TaskState::Running;

                    // Do current và next bọc trong Box cố định địa chỉ heap,
                    // việc push/pop Box không làm lệch địa chỉ của stack_ptr.
                    let old_stack_ptr_ref = &mut current.stack_ptr as *mut u64;
                    let new_stack_ptr = next.stack_ptr;

                    sched.ready_queue.push_back(current);
                    sched.current_task = Some(next);

                    switch_args = Some((old_stack_ptr_ref, new_stack_ptr));
                } else {
                    // Khôi phục lại nếu không lấy được next task
                    sched.current_task = Some(current);
                }
            }
        }
    } // Khóa của scheduler được giải phóng tự động tại đây!

    // 3. Thực hiện context switch
    if let Some((old_stack_ptr_ref, new_stack_ptr)) = switch_args {
        // SAFETY: Thay đổi con trỏ stack RSP yêu cầu đặc quyền Ring 0.
        // Con trỏ old_stack_ptr_ref và địa chỉ new_stack_ptr trỏ tới các vùng nhớ stack hợp lệ.
        unsafe {
            crate::arch::x86_64::switch::switch_context(old_stack_ptr_ref, new_stack_ptr);
        }
    }

    // 4. Khôi phục lại trạng thái ngắt ban đầu của task sau khi switch trở lại
    if interrupts_enabled {
        // SAFETY: sti yêu cầu CPU chạy ở Ring 0.
        unsafe {
            crate::arch::x86_64::instructions::sti();
        }
    }
}

/// Giải phóng task hiện tại và switch sang task tiếp theo
pub fn exit_current_task() -> ! {
    // Luôn tắt ngắt khi kết thúc task để tránh ngắt chen ngang giữa lúc dọn dẹp
    unsafe {
        crate::arch::x86_64::instructions::cli();
    }

    let new_stack_ptr;

    {
        let mut sched = SCHEDULER.lock();
        // Drop current_task (tự động giải phóng vùng nhớ heap của stack Box)
        let _exited = sched.current_task.take();

        if let Some(mut next) = sched.ready_queue.pop_front() {
            next.state = TaskState::Running;
            new_stack_ptr = next.stack_ptr;
            sched.current_task = Some(next);
        } else {
            // Không còn task nào hoạt động
            drop(sched);
            serial_println!("[AXIOMOS SCHEDULER] Không còn task nào hoạt động. Hệ thống dừng.");
            loop {
                // SAFETY: Dừng CPU an toàn khi hệ thống nhàn rỗi hoàn toàn
                unsafe {
                    core::arch::asm!("hlt");
                }
            }
        }
    } // Giải phóng khóa scheduler trước khi switch

    let mut junk_stack = 0u64;
    // SAFETY: Chuyển đổi stack sang task mới. Task cũ đã bị drop hoàn toàn.
    unsafe {
        crate::arch::x86_64::switch::switch_context(&mut junk_stack, new_stack_ptr);
    }

    unreachable!();
}

#[cfg(test)]
mod tests {
    use super::*;

    static mut TEST_VAL: u32 = 0;
    fn test_task_1() {
        unsafe {
            TEST_VAL = 10;
        }
        yield_now();
        unsafe {
            TEST_VAL = 20;
        }
    }

    fn test_task_2() {
        unsafe {
            TEST_VAL = 30;
        }
        yield_now();
    }

    #[test]
    fn test_scheduler_spawn_and_queue() {
        // Reset scheduler
        {
            let mut sched = SCHEDULER.lock();
            sched.ready_queue.clear();
            sched.current_task = None;
            sched.next_id = 1;
        }

        init();
        spawn(test_task_1);
        spawn(test_task_2);

        let sched = SCHEDULER.lock();
        assert_eq!(sched.ready_queue.len(), 2);
        assert_eq!(sched.ready_queue[0].id, 1);
        assert_eq!(sched.ready_queue[1].id, 2);
    }
}
