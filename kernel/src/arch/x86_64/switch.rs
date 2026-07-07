//! Context switch mức thấp cho kiến trúc x86_64

use core::arch::naked_asm;

/// Thực hiện chuyển đổi ngữ cảnh (context switch) giữa hai kernel tasks.
///
/// # Safety
/// Hàm này trực tiếp thay đổi con trỏ stack `rsp` và các thanh ghi của CPU,
/// yêu cầu CPU chạy ở Ring 0.
///
/// Calling Convention: System V ABI
/// - `old_stack` (RDI): Địa chỉ con trỏ stack của task cũ (để lưu RSP hiện tại).
/// - `new_stack` (RSI): Giá trị con trỏ stack của task mới (để nạp vào RSP).
#[unsafe(naked)]
pub unsafe extern "C" fn switch_context(old_stack: *mut u64, new_stack: u64) {
    naked_asm!(
        // 1. Lưu các callee-saved registers của task cũ lên stack của nó
        "push rbp",
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        // 2. Lưu con trỏ stack hiện tại (rsp) vào old_stack
        "mov [rdi], rsp",
        // 3. Nạp con trỏ stack của task mới (new_stack) vào rsp
        "mov rsp, rsi",
        // 4. Khôi phục các callee-saved registers của task mới từ stack của nó
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "pop rbp",
        // 5. Trở về (nhảy tới RIP được lưu trên stack mới)
        "ret"
    );
}
