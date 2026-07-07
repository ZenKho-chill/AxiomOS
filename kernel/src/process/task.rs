//! Cấu trúc Task và khởi tạo vùng nhớ Stack cho tiến trình nhân (Kernel Task)

use alloc::boxed::Box;

/// Kích thước mặc định của vùng nhớ stack cho mỗi task (16 KiB)
pub const STACK_SIZE: usize = 16 * 1024;

/// Các trạng thái có thể có của một task
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
}

/// Task Control Block (TCB) lưu trữ ngữ cảnh của một luồng thực thi trong nhân
pub struct Task {
    pub id: u32,
    pub stack: Box<[u8]>,      // Vùng nhớ stack được cấp phát động trên heap
    pub stack_ptr: u64,        // Giá trị thanh ghi RSP hiện tại của task này
    pub state: TaskState,
}

impl Task {
    /// Tạo một task mới với hàm entry tương ứng.
    ///
    /// # Safety
    /// Hàm này chuẩn bị cấu trúc stack ban đầu giả lập cho việc khôi phục ngữ cảnh sau này.
    pub fn new(id: u32, entry: fn()) -> Self {
        #[cfg(test)]
        extern crate std;
        #[cfg(test)]
        std::println!("DEBUG: Task::new start, id = {}", id);

        // Cấp phát stack động trên heap của kernel
        let mut stack = Box::new([0u8; STACK_SIZE]);
        let stack_top = stack.as_mut_ptr() as usize + STACK_SIZE;

        #[cfg(test)]
        std::println!("DEBUG: Stack allocated, top = {:#x}", stack_top);

        // Thiết lập stack ban đầu
        let stack_ptr = unsafe {
            // Định vị con trỏ ghi ngược từ đỉnh stack
            let mut ptr = stack_top as *mut u64;

            // 1. Địa chỉ trả về khi hàm entry kết thúc (hàm exit)
            ptr = ptr.sub(1);
            ptr.write(task_exit as *const () as u64);

            // 2. RIP ban đầu của task (hàm entry)
            ptr = ptr.sub(1);
            ptr.write(entry as *const () as u64);

            // 3. Các thanh ghi callee-saved giả lập (r15, r14, r13, r12, rbx, rbp)
            for _ in 0..6 {
                ptr = ptr.sub(1);
                ptr.write(0);
            }

            ptr as u64
        };

        Self {
            id,
            stack,
            stack_ptr,
            state: TaskState::Ready,
        }
    }
}

/// Hàm exit mặc định khi một task chạy xong.
/// Chuyển quyền điều khiển lại cho scheduler và giải phóng tài nguyên.
extern "C" fn task_exit() -> ! {
    crate::process::scheduler::exit_current_task();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_entry() {}

    #[test]
    fn test_task_creation() {
        let task = Task::new(1, dummy_entry);
        assert_eq!(task.id, 1);
        assert_eq!(task.state, TaskState::Ready);
        assert_eq!(task.stack.len(), STACK_SIZE);
        
        // RSP phải trỏ vào trong stack
        let stack_start = task.stack.as_ptr() as u64;
        let stack_end = stack_start + STACK_SIZE as u64;
        assert!(task.stack_ptr >= stack_start);
        assert!(task.stack_ptr < stack_end);
    }
}
