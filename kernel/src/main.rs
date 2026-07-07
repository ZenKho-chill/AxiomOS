#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

pub mod arch;
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

    // Khởi tạo IDT và PIC
    // SAFETY: Các lệnh này thay đổi bảng ngắt của CPU ở mức đặc quyền Ring 0
    unsafe {
        arch::x86_64::idt::init();
        boot_log("[AXIOMOS] IDT initialized");

        drivers::pic::init();
    }

    // Kiểm chứng ngắt Breakpoint (int3)
    // SAFETY: int3 kích hoạt breakpoint exception hợp lệ đã đăng ký handler trong IDT
    unsafe {
        core::arch::asm!("int3");
    }

    run_panic_test_if_requested();

    // In chuỗi boot sequence theo đúng yêu cầu đặc tả
    boot_log("[AXIOMOS] Bootloader handoff complete");
    boot_log("[AXIOMOS] Kernel started");
    boot_log("[AXIOMOS] Serial logger initialized");
    boot_log("[AXIOMOS] System halted");

    // Bật ngắt phần cứng trên PIC và CPU sau khi đã in xong boot sequence
    // SAFETY: Bật ngắt thông qua unmask và sti yêu cầu đặc quyền Ring 0.
    unsafe {
        drivers::pic::unmask(0); // Timer (IRQ 0)
        drivers::pic::unmask(1); // Keyboard (IRQ 1)
        core::arch::asm!("sti");
    }

    // Vòng lặp chính xử lý nhập từ bàn phím
    let mut shift_pressed = false;
    let mut last_ticks = 0;
    loop {
        // Đọc và in ticks ngắt Timer an toàn trong main loop
        let current_ticks =
            arch::x86_64::idt::TIMER_TICKS.load(core::sync::atomic::Ordering::Relaxed);
        if current_ticks - last_ticks >= 100 {
            serial_println!("[AXIOMOS TIMER] Ticks: {}", current_ticks);
            last_ticks = current_ticks;
        }

        if let Some(event) = drivers::keyboard::poll_key_event() {
            if event.key_code == drivers::keyboard::KeyCode::LShift
                || event.key_code == drivers::keyboard::KeyCode::RShift
            {
                shift_pressed = event.pressed;
            }

            if event.pressed {
                if let Some(c) = drivers::keyboard::keycode_to_char(event.key_code, shift_pressed) {
                    serial_println!("[KEY] Pressed char: {}", c);
                    console::framebuffer::framebuffer_println(format_args!(
                        "[KEY] Pressed char: {}",
                        c
                    ));
                } else {
                    serial_println!("[KEY] Pressed special: {:?}", event.key_code);
                    console::framebuffer::framebuffer_println(format_args!(
                        "[KEY] Pressed special: {:?}",
                        event.key_code
                    ));
                }
            } else {
                serial_println!("[KEY] Released: {:?}", event.key_code);
            }
        }

        // SAFETY: hlt dừng CPU tạm thời cho tới khi ngắt tiếp theo xảy ra để tiết kiệm năng lượng
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
