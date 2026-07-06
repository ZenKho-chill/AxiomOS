#![no_std]
#![no_main]

use core::panic::PanicInfo;

/// Điểm vào Kernel (Kernel Entry Point)
///
/// # Safety
/// Hàm này được gọi trực tiếp bởi bootloader Limine. Chúng ta tắt mangling để trình liên kết (linker)
/// có thể định vị chính xác nhãn `_start`.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {
        // SAFETY: Chỉ thị asm hlt dừng CPU tạm thời cho đến khi có ngắt.
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

/// Trình xử lý Panic khi xảy ra lỗi không thể phục hồi
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        // SAFETY: Dừng CPU khi xảy ra panic.
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}
