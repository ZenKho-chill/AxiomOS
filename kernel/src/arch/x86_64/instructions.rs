//! Các lệnh CPU x86_64 mức thấp.

/// Vô hiệu hóa ngắt trên CPU cục bộ.
///
/// # Safety
/// Hàm này sử dụng assembly trực tiếp để thực hiện lệnh `cli`. Caller phải đảm bảo
/// rằng việc tắt ngắt không gây mất mát dữ liệu quan trọng hoặc gây ra tình trạng khóa chết (deadlock)
/// trong các tiến trình cần xử lý ngắt khẩn cấp.
///
/// - Preconditions: CPU phải chạy ở đặc quyền Ring 0.
/// - Postconditions: Ngắt bị vô hiệu hóa cục bộ trên CPU hiện tại.
/// - Memory safety assumptions: Không có tác động phụ lên bộ nhớ.
/// - CPU state assumptions: Cờ Interrupt Flag (IF) trong thanh ghi RFLAGS bị xóa (về 0).
#[inline(always)]
pub unsafe fn cli() {
    // SAFETY: Thực hiện lệnh cli vô hiệu hóa ngắt yêu cầu Ring 0.
    unsafe {
        core::arch::asm!("cli", options(nomem, nostack, preserves_flags));
    }
}

/// Kích hoạt lại ngắt trên CPU cục bộ.
///
/// # Safety
/// Hàm này sử dụng assembly trực tiếp để thực hiện lệnh `sti`. Caller phải đảm bảo
/// rằng việc bật ngắt là an toàn và IDT đã được khởi tạo chính xác.
///
/// - Preconditions: CPU chạy ở đặc quyền Ring 0 và IDT đã được thiết lập.
/// - Postconditions: Ngắt được kích hoạt cục bộ trên CPU hiện tại.
/// - Memory safety assumptions: Không có tác động phụ lên bộ nhớ.
/// - CPU state assumptions: Cờ Interrupt Flag (IF) trong thanh ghi RFLAGS được set (về 1).
#[inline(always)]
pub unsafe fn sti() {
    // SAFETY: Thực hiện lệnh sti kích hoạt ngắt yêu cầu Ring 0 và IDT sẵn sàng.
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }
}

/// Đọc thanh ghi RFLAGS của CPU.
///
/// # Safety
/// Hàm này đọc trực tiếp thanh ghi hệ thống RFLAGS bằng cách đẩy lên stack và pop ra.
///
/// - Preconditions: Không có.
/// - Postconditions: Trả về giá trị của RFLAGS tại thời điểm đọc.
/// - Memory safety assumptions: Sử dụng stack tạm thời thông qua pushfq và pop, an toàn bộ nhớ.
/// - CPU state assumptions: Không thay đổi các cờ trạng thái của CPU.
#[inline(always)]
pub unsafe fn read_rflags() -> u64 {
    let rflags: u64;
    // SAFETY: Đọc cờ RFLAGS qua stack, an toàn và không gây tác động phụ.
    unsafe {
        core::arch::asm!("pushfq; pop {}", out(reg) rflags, options(nomem, preserves_flags));
    }
    rflags
}

/// Kiểm tra xem ngắt trên CPU cục bộ có đang được kích hoạt hay không.
///
/// Trả về `true` nếu ngắt đang bật, ngược lại trả về `false`.
#[inline(always)]
pub fn are_interrupts_enabled() -> bool {
    // SAFETY: Việc đọc RFLAGS để kiểm tra trạng thái cờ ngắt không thay đổi trạng thái CPU.
    let rflags = unsafe { read_rflags() };
    (rflags & (1 << 9)) != 0
}
