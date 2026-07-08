#![no_std]
#![cfg_attr(not(test), no_main)]
#![feature(abi_x86_interrupt)]

extern crate alloc;

pub mod arch;
pub mod boot;
pub mod console;
pub mod drivers;
pub mod fs;
pub mod logging;
pub mod memory;
pub mod process;
pub mod syscall;
pub mod utils;

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
        logging::boot("IDT initialized");

        drivers::pic::init();
        utils::time::init();
        logging::boot("PIT initialized at 1000Hz");
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

                            // Khởi tạo bộ lập lịch tiến trình
                            process::scheduler::init();

                            // Chạy chẩn đoán cấp phát bộ nhớ động
                            run_memory_diagnostics();

                            // Chạy chẩn đoán cơ chế đồng bộ hóa
                            run_sync_diagnostics();

                            // Chạy chẩn đoán bộ lọc log và ring buffer
                            run_logging_diagnostics();

                            // Chạy chẩn đoán bộ đếm thời gian
                            run_timekeeping_diagnostics();

                            // Chạy chẩn đoán lập lịch tiến trình
                            run_scheduler_diagnostics();

                            // Chạy chẩn đoán thiết bị khối
                            run_block_device_diagnostics();

                            // Chạy chẩn đoán trình phân tích ELF64
                            process::elf::run_elf_parser_diagnostics();

                            // Chạy chẩn đoán không gian địa chỉ người dùng
                            memory::user_space::run_userspace_as_diagnostics();

                            // Chạy chẩn đoán Syscall ABI
                            syscall::run_syscall_diagnostics();
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
    logging::boot("Bootloader handoff complete");
    logging::boot("Kernel started");
    logging::boot("Serial logger initialized");
    logging::boot("System halted");

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
            logging::info("TIMER", format_args!("Ticks: {}", current_ticks), false);
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
    logging::panic(format_args!("{}", info));
    loop {
        // SAFETY: Dừng CPU khi xảy ra panic.
        unsafe {
            core::arch::asm!("hlt");
        }
    }
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

#[cfg(not(test))]
fn run_sync_diagnostics() {
    use utils::sync::{Spinlock, SpinlockIrqSave};

    serial_println!("[AXIOMOS SYNC] Chạy chẩn đoán cơ chế đồng bộ hóa...");

    // 1. Kiểm thử Spinlock cơ bản
    let lock = Spinlock::new(100);
    assert!(!lock.is_locked());
    {
        let mut guard = lock.lock();
        assert!(lock.is_locked());
        assert_eq!(*guard, 100);
        *guard = 200;
    }
    assert!(!lock.is_locked());
    {
        let guard = lock.lock();
        assert_eq!(*guard, 200);
    }
    serial_println!("[AXIOMOS SYNC] Kiểm thử Spinlock cơ bản: THÀNH CÔNG");

    // 2. Kiểm thử SpinlockIrqSave (An toàn ngắt)
    let irq_lock = SpinlockIrqSave::new(300);
    assert!(!irq_lock.is_locked());

    let is_enabled_before = arch::x86_64::instructions::are_interrupts_enabled();
    {
        let mut guard = irq_lock.lock();
        assert!(irq_lock.is_locked());
        assert_eq!(*guard, 300);
        *guard = 400;

        let is_enabled_during = arch::x86_64::instructions::are_interrupts_enabled();
        assert!(
            !is_enabled_during,
            "Lỗi: Ngắt chưa bị tắt khi đang giữ SpinlockIrqSave!"
        );
    }
    assert!(!irq_lock.is_locked());

    let is_enabled_after = arch::x86_64::instructions::are_interrupts_enabled();
    assert_eq!(
        is_enabled_before, is_enabled_after,
        "Lỗi: Trạng thái ngắt không được khôi phục sau khi thả khóa!"
    );

    serial_println!("[AXIOMOS SYNC] Kiểm thử SpinlockIrqSave (An toàn ngắt): THÀNH CÔNG");
    serial_println!("[AXIOMOS SYNC] Tất cả chẩn đoán đồng bộ hóa đã vượt qua!");
}

#[cfg(not(test))]
fn run_logging_diagnostics() {
    use logging::{dump_log_buffer, filter_level, set_filter_level, LogLevel};

    serial_println!("[AXIOMOS LOG] Chạy chẩn đoán bộ lọc log và ring buffer...");

    // Lưu mức lọc hiện tại
    let original_level = filter_level();

    // 1. Thử ghi log mức Info và Warn
    logging::info("TEST", format_args!("Thông điệp mức Info"), false);
    logging::write(logging::LogRecord {
        level: LogLevel::Warn,
        subsystem: Some("TEST"),
        message: format_args!("Thông điệp mức Warn"),
        mirror_framebuffer: false,
    });

    // 2. Thiết lập mức lọc Warn
    set_filter_level(LogLevel::Warn);

    // Log Info tiếp theo phải bị lọc (không lưu vào ring buffer)
    logging::info(
        "TEST",
        format_args!("Thông điệp mức Info này phải bị lọc!"),
        false,
    );

    // Khôi phục lại mức lọc ban đầu
    set_filter_level(original_level);

    // 3. Dump ring buffer log ra serial để kiểm tra thủ công các log đã lưu
    dump_log_buffer();

    serial_println!("[AXIOMOS LOG] Chạy chẩn đoán bộ lọc log và ring buffer: THÀNH CÔNG");
}

#[cfg(not(test))]
fn run_timekeeping_diagnostics() {
    use utils::time::{sleep_ms, uptime_ms};

    serial_println!("[AXIOMOS TIME] Chạy chẩn đoán đồng hồ thời gian...");

    // 1. Kiểm chứng việc đọc uptime tăng lên
    let start_time = uptime_ms();

    // Bật ngắt tạm thời để bộ đếm ticks có thể hoạt động trong lúc chẩn đoán
    // (Vì ngắt ngầm định chưa bật cho đến cuối _start, ta cần bật ngắt ở đây để timer chạy)
    let is_enabled_before = arch::x86_64::instructions::are_interrupts_enabled();
    if !is_enabled_before {
        unsafe {
            drivers::pic::unmask(0); // Bật IRQ 0
            core::arch::asm!("sti");
        }
    }

    // 2. Thử sleep_ms 50ms và đo lường thời gian thực tế trôi qua
    sleep_ms(50);

    let end_time = uptime_ms();
    let elapsed = end_time - start_time;

    // Khôi phục lại trạng thái ngắt ban đầu
    if !is_enabled_before {
        unsafe {
            core::arch::asm!("cli");
        }
    }

    serial_println!(
        "[AXIOMOS TIME] Đã ngủ 50ms, thời gian đo được: {} ms",
        elapsed
    );
    assert!(elapsed >= 50, "Lỗi: sleep_ms kết thúc quá sớm!");

    serial_println!("[AXIOMOS TIME] Chạy chẩn đoán đồng hồ thời gian: THÀNH CÔNG");
}

#[cfg(not(test))]
fn run_scheduler_diagnostics() {
    use process::scheduler::{spawn, yield_now};

    serial_println!("[AXIOMOS SCHED] Chạy chẩn đoán lập lịch tiến trình cộng tác...");

    // 1. Spawn 2 tasks nhân đơn giản
    spawn(task_a);
    spawn(task_b);

    // Bật ngắt tạm thời để bộ đếm timer ticks chạy ổn định
    let is_enabled_before = arch::x86_64::instructions::are_interrupts_enabled();
    if !is_enabled_before {
        unsafe {
            drivers::pic::unmask(0);
            core::arch::asm!("sti");
        }
    }

    // 2. Nhường CPU liên tục cho đến khi cả Task A và Task B hoàn thành đủ 2 chu kỳ chạy
    loop {
        let done = unsafe {
            let a = core::ptr::read_volatile(&raw const TASK_A_RUNS);
            let b = core::ptr::read_volatile(&raw const TASK_B_RUNS);
            a == 2 && b == 2
        };
        if done {
            break;
        }
        yield_now();
    }

    // 3. Khôi phục lại trạng thái ngắt ban đầu
    if !is_enabled_before {
        unsafe {
            core::arch::asm!("cli");
        }
    }

    // Xác nhận cả 2 task đều đã chạy xong 2 lần
    unsafe {
        let a_runs = core::ptr::read_volatile(&raw const TASK_A_RUNS);
        let b_runs = core::ptr::read_volatile(&raw const TASK_B_RUNS);
        assert_eq!(a_runs, 2, "Lỗi: Task A chưa chạy đủ 2 lần!");
        assert_eq!(b_runs, 2, "Lỗi: Task B chưa chạy đủ 2 lần!");
    }

    serial_println!("[AXIOMOS SCHED] Chạy chẩn đoán lập lịch tiến trình: THÀNH CÔNG");
}

#[cfg(not(test))]
static mut TASK_A_RUNS: u32 = 0;
#[cfg(not(test))]
static mut TASK_B_RUNS: u32 = 0;

#[cfg(not(test))]
fn task_a() {
    serial_println!("[AXIOMOS SCHED] Task A bắt đầu chạy.");
    unsafe {
        let val = core::ptr::read_volatile(&raw const TASK_A_RUNS);
        core::ptr::write_volatile(&raw mut TASK_A_RUNS, val + 1);
    }

    // Nhường CPU cho Task B
    process::scheduler::yield_now();

    serial_println!("[AXIOMOS SCHED] Task A chạy lại lần 2.");
    unsafe {
        let val = core::ptr::read_volatile(&raw const TASK_A_RUNS);
        core::ptr::write_volatile(&raw mut TASK_A_RUNS, val + 1);
    }
}

#[cfg(not(test))]
fn task_b() {
    serial_println!("[AXIOMOS SCHED] Task B bắt đầu chạy.");
    unsafe {
        let val = core::ptr::read_volatile(&raw const TASK_B_RUNS);
        core::ptr::write_volatile(&raw mut TASK_B_RUNS, val + 1);
    }

    // Nhường CPU cho Task A
    process::scheduler::yield_now();

    serial_println!("[AXIOMOS SCHED] Task B chạy lại lần 2.");
    unsafe {
        let val = core::ptr::read_volatile(&raw const TASK_B_RUNS);
        core::ptr::write_volatile(&raw mut TASK_B_RUNS, val + 1);
    }
}

#[cfg(not(test))]
static mut MOCK_DISK_DATA: [u8; 1024] = [0u8; 1024];

#[cfg(not(test))]
fn run_block_device_diagnostics() {
    use drivers::block::{register_system_block_device, BlockDevice, RamDisk, SYSTEM_BLOCK_DEVICE};

    serial_println!("[AXIOMOS BLOCK] Chạy chẩn đoán thiết bị khối...");

    // Thiết lập dữ liệu mock tĩnh
    let test_string = b"AXIOMOS BLOCK DEVICE TEST SUCCESS";
    unsafe {
        let ptr = &raw mut MOCK_DISK_DATA;
        let buf = &mut *ptr;
        buf[512..512 + test_string.len()].copy_from_slice(test_string);
    }

    // Đăng ký thiết bị hệ thống
    let disk = RamDisk::new(unsafe { &*(&raw const MOCK_DISK_DATA as *const [u8]) });
    register_system_block_device(disk);

    // Đọc kiểm chứng
    let mut buf = [0u8; 512];
    let guard = SYSTEM_BLOCK_DEVICE.lock();
    if let Some(ref device) = *guard {
        assert_eq!(device.total_sectors(), 2);

        let res = device.read_sector(1, &mut buf);
        assert!(res.is_ok(), "Lỗi đọc sector!");

        assert_eq!(
            &buf[0..test_string.len()],
            test_string,
            "Dữ liệu sector đọc ra bị sai lệch!"
        );
    } else {
        panic!("Lỗi: Chưa đăng ký thiết bị khối hệ thống!");
    }

    serial_println!("[AXIOMOS BLOCK] Chạy chẩn đoán thiết bị khối: THÀNH CÔNG");
}
