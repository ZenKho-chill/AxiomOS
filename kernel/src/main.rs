#![no_std]
#![no_main]

pub mod boot;
pub mod console;
pub mod drivers;

use core::panic::PanicInfo;

/// Điểm vào Kernel (Kernel Entry Point)
///
/// # Safety
/// Hàm này được gọi trực tiếp bởi bootloader Limine. Chúng ta tắt mangling để trình liên kết (linker)
/// có thể định vị chính xác nhãn `_start`.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    boot::limine::keep_requests_alive();

    // Khởi tạo cổng nối tiếp COM1 sớm để chẩn đoán
    drivers::serial::init();
    if let Some(info) = boot::limine::framebuffer_info() {
        if let Err(error) = console::framebuffer::init_framebuffer_console(info) {
            serial_println!(
                "[AXIOMOS] Framebuffer console unavailable: {}",
                error.as_str()
            );
        }
    } else {
        serial_println!("[AXIOMOS] Framebuffer console unavailable: no framebuffer");
    }

    run_panic_test_if_requested();

    // In chuỗi boot sequence theo đúng yêu cầu đặc tả
    boot_log("[AXIOMOS] Bootloader handoff complete");
    boot_log("[AXIOMOS] Kernel started");
    boot_log("[AXIOMOS] Serial logger initialized");
    boot_log("[AXIOMOS] System halted");

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
    console::framebuffer::framebuffer_println(format_args!("[AXIOMOS PANIC] {}", info));
    loop {
        // SAFETY: Dừng CPU khi xảy ra panic.
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

fn boot_log(message: &str) {
    serial_println!("{}", message);
    console::framebuffer::framebuffer_println(format_args!("{}", message));
}

#[cfg(feature = "panic-test")]
fn run_panic_test_if_requested() {
    panic!("Spec 003 framebuffer panic test");
}

#[cfg(not(feature = "panic-test"))]
fn run_panic_test_if_requested() {}
