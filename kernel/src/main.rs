#![no_std]
#![no_main]

pub mod drivers;

use core::panic::PanicInfo;

// =====================================================================
// LIMINE PROTOCOL v7 - Magic bytes viết tay từ đặc tả chính thức
// Nguồn: https://github.com/limine-bootloader/limine-protocol
// =====================================================================

/// Start marker: Limine scan kernel ELF tìm chuỗi magic này
/// để biết requests bắt đầu từ đây.
#[used]
#[link_section = ".requests_start_marker"]
static REQUESTS_START_MARKER: [u64; 4] = [
    0xf6b8f4b39de7d1ae,
    0xfab91a6940fcb9cf,
    0x785c6ed015d3e316,
    0x181e920a7852b9d9,
];

/// Base Revision request: Yêu cầu bootloader dùng revision 3 (phiên bản tối thiểu).
/// Layout: [magic0, magic1, revision]
/// COMMON_MAGIC = [0xc7b1dd30df4c8b88, 0x0a82e883a194f07b]
/// BASE_REVISION_ID = [0xf9562b2d5c95a6c8, 0x6a7b384944536bdc]
#[used]
#[link_section = ".requests"]
static BASE_REVISION: [u64; 3] = [
    // ID
    0xf9562b2d5c95a6c8,
    0x6a7b384944536bdc,
    // Revision yêu cầu (0 = tối thiểu, bootloader sẽ ghi đè nếu hỗ trợ)
    3,
];

/// End marker: Limine dừng tìm kiếm requests sau chuỗi magic này.
#[used]
#[link_section = ".requests_end_marker"]
static REQUESTS_END_MARKER: [u64; 2] = [0xadc0e0531bb10d03, 0x9572709f31764c62];

/// Điểm vào Kernel (Kernel Entry Point)
///
/// # Safety
/// Hàm này được gọi trực tiếp bởi bootloader Limine. Chúng ta tắt mangling để trình liên kết (linker)
/// có thể định vị chính xác nhãn `_start`.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Tham chiếu tường minh đến các marker để ép linker giữ lại chúng
    let _ = &REQUESTS_START_MARKER;
    let _ = &REQUESTS_END_MARKER;
    let _ = &BASE_REVISION;

    // Khởi tạo cổng nối tiếp COM1 sớm để chẩn đoán
    drivers::serial::init();

    // In chuỗi boot sequence theo đúng yêu cầu đặc tả
    serial_println!("[AXIOMOS] Bootloader handoff complete");
    serial_println!("[AXIOMOS] Kernel started");
    serial_println!("[AXIOMOS] Serial logger initialized");
    serial_println!("[AXIOMOS] System halted");

    loop {
        // SAFETY: Dừng CPU sau khi hoàn tất boot chẩn đoán.
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

/// Trình xử lý Panic khi xảy ra lỗi không thể phục hồi
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("[AXIOMOS PANIC] {}", info);
    loop {
        // SAFETY: Dừng CPU khi xảy ra panic.
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}
