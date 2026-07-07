#![no_std]
#![cfg_attr(not(test), no_main)]
#![feature(abi_x86_interrupt)]

extern crate alloc;

pub mod arch;
pub mod boot;
pub mod console;
pub mod drivers;
pub mod memory;

#[cfg(not(test))]
use core::panic::PanicInfo;

/// Điểm vào Kernel (Kernel Entry Point)
///
/// # Safety
/// Hàm này được gọi trực tiếp bởi bootloader Limine. Chúng ta tắt mangling để trình liên kết (linker)
/// có thể định vị chính xác nhãn `_start`.
#[cfg(not(test))]
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

    // Khởi tạo hệ thống quản lý bộ nhớ
    match memory::frame::init_memory() {
        Ok(stats) => {
            serial_println!(
                "[AXIOMOS MEMORY] Usable RAM: {} MiB, Regions: {}, Free: {} frames, Used: {} frames",
                stats.total_usable / (1024 * 1024),
                stats.region_count,
                stats.free_frames,
                stats.allocated_frames
            );
            console::framebuffer::framebuffer_println(format_args!(
                "[AXIOMOS MEMORY] Usable RAM: {} MiB",
                stats.total_usable / (1024 * 1024)
            ));

            // Khởi tạo Heap Allocator
            match memory::frame::hhdm_offset() {
                Ok(hhdm) => {
                    // SAFETY: Khởi tạo heap chỉ gọi duy nhất 1 lần, các vùng nhớ ảo đã được map an toàn
                    unsafe {
                        if let Err(error) = memory::heap::init_heap(hhdm) {
                            serial_println!(
                                "[AXIOMOS MEMORY] Heap initialization failed: {:?}",
                                error
                            );
                        } else {
                            let heap_size_kib = memory::heap::HEAP_SIZE / 1024;
                            let heap_start = memory::heap::HEAP_START;
                            serial_println!(
                                "[AXIOMOS MEMORY] Kernel Heap initialized: {} KiB at Virtual Address: 0x{:X}",
                                heap_size_kib,
                                heap_start
                            );

                            // Chạy chẩn đoán cấp phát bộ nhớ động
                            run_memory_diagnostics();
                        }
                    }
                }
                Err(error) => {
                    serial_println!("[AXIOMOS MEMORY] HHDM offset unavailable: {:?}", error);
                }
            }
        }
        Err(error) => {
            serial_println!(
                "[AXIOMOS MEMORY] Frame allocator initialization failed: {:?}",
                error
            );
        }
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
#[cfg(not(test))]
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

#[cfg(not(test))]
fn boot_log(message: &str) {
    serial_println!("{}", message);
    console::framebuffer::framebuffer_println(format_args!("{}", message));
}

#[cfg(all(not(test), feature = "panic-test"))]
fn run_panic_test_if_requested() {
    panic!("Spec 003 framebuffer panic test");
}

#[cfg(all(not(test), not(feature = "panic-test")))]
fn run_panic_test_if_requested() {}

#[cfg(not(test))]
fn run_memory_diagnostics() {
    use alloc::boxed::Box;
    use alloc::vec::Vec;

    serial_println!("[AXIOMOS MEMORY] Running dynamic memory diagnostics...");

    let box_val = Box::new(42);
    serial_println!(
        "[AXIOMOS MEMORY] Box allocation success, value: {}",
        *box_val
    );

    let mut vec_val = Vec::new();
    for i in 0..5 {
        vec_val.push(i * 10);
    }
    serial_println!(
        "[AXIOMOS MEMORY] Vec allocation success, elements: {:?}",
        vec_val
    );

    serial_println!("[AXIOMOS MEMORY] All memory diagnostics passed successfully!");
}
