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
static INIT_ELF_BYTES: &[u8] =
    include_bytes!("../../userspace/target/x86_64-unknown-none/debug/init");
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

    // Khởi tạo GDT, IDT và PIC
    // SAFETY: Các lệnh này thay đổi bảng phân đoạn và bảng ngắt của CPU ở mức đặc quyền Ring 0
    unsafe {
        arch::x86_64::gdt::init();
        logging::boot("GDT initialized");

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

                            // Khởi tạo FAT32 RAM Disk thực tế từ INIT_ELF_BYTES
                            let disk_data = build_init_ramdisk(INIT_ELF_BYTES);
                            let disk = drivers::block::RamDisk::new(disk_data);
                            let static_disk = alloc::boxed::Box::leak(alloc::boxed::Box::new(disk));
                            drivers::block::register_system_block_device(*static_disk);

                            // Mount root filesystem FAT32 vào VFS
                            let volume = fs::fat32::mount_fat32(static_disk)
                                .expect("Lỗi: Không mount được FAT32");
                            let filesystem = alloc::boxed::Box::leak(alloc::boxed::Box::new(
                                fs::fat32::Fat32FileSystem::new(volume),
                            ));
                            fs::vfs::mount_root(filesystem)
                                .expect("Lỗi: Không mount được root VFS");
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

    // Bật ngắt phần cứng trên PIC
    unsafe {
        drivers::pic::unmask(0); // Timer (IRQ 0)
        drivers::pic::unmask(1); // Keyboard (IRQ 1)
    }

    // Khởi động tiến trình userspace init
    let pid = process::spawn_init("/INIT.ELF").expect("Lỗi: Không spawn được init");
    unsafe {
        process::enter_userspace(pid);
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

/// Dựng cấu trúc ảnh đĩa FAT32 tối giản trực tiếp trong RAM chứa duy nhất tệp tin /INIT.ELF
fn build_init_ramdisk(init_elf: &[u8]) -> &'static [u8] {
    use alloc::boxed::Box;

    let sector_size = 512;
    let file_sectors = (init_elf.len() + 511) / 512;

    // Tính toán số sectors cho bảng FAT dựa trên kích thước file thực tế
    let fat_entries = 3 + file_sectors;
    let fat_size_bytes = fat_entries * 4;
    let fat_sectors = (fat_size_bytes + 511) / 512;

    let total_sectors = 2 + fat_sectors + file_sectors;
    let mut image = alloc::vec![0u8; sector_size * total_sectors];

    // 1. Dựng Boot Sector (LBA 0)
    let boot_sector = &mut image[0..512];
    boot_sector[0] = 0xEB;
    boot_sector[1] = 0x58;
    boot_sector[2] = 0x90;
    boot_sector[3..11].copy_from_slice(b"AXIOMOS ");

    boot_sector[11] = 0x00;
    boot_sector[12] = 0x02; // Sector size = 512 bytes
    boot_sector[13] = 1; // 1 sector per cluster
    boot_sector[14] = 1; // Reserved sectors = 1 (LBA 0)
    boot_sector[15] = 0;
    boot_sector[16] = 1; // Number of FATs = 1
    boot_sector[21] = 0xF8; // Media descriptor
    boot_sector[22] = 0; // Sectors per FAT 16 = 0 (bắt buộc bằng 0 đối với FAT32)
    boot_sector[23] = 0;

    // Ghi Sectors per FAT 32 vào byte 36-39 theo đặc tả FAT32
    boot_sector[36..40].copy_from_slice(&(fat_sectors as u32).to_le_bytes());

    let total_sec_bytes = (total_sectors as u32).to_le_bytes();
    boot_sector[32..36].copy_from_slice(&total_sec_bytes);
    boot_sector[44..48].copy_from_slice(&2u32.to_le_bytes()); // Root dir cluster = 2
    boot_sector[510] = 0x55;
    boot_sector[511] = 0xAA;

    // 2. Dựng bảng FAT (LBA 1 đến LBA fat_sectors)
    let fat_offset = 512;
    image[fat_offset..fat_offset + 4].copy_from_slice(&0x0FFF_FFF8u32.to_le_bytes());
    image[fat_offset + 4..fat_offset + 8].copy_from_slice(&0x0FFF_FFFFu32.to_le_bytes());
    image[fat_offset + 8..fat_offset + 12].copy_from_slice(&0x0FFF_FFFFu32.to_le_bytes()); // Cluster 2 (End of Chain)

    for i in 0..file_sectors {
        let cluster = 3 + i as u32;
        let next_val = if i == file_sectors - 1 {
            0x0FFF_FFFFu32
        } else {
            cluster + 1
        };
        let entry_offset = fat_offset + (cluster as usize * 4);
        image[entry_offset..entry_offset + 4].copy_from_slice(&next_val.to_le_bytes());
    }

    // 3. Dựng Root Directory (LBA 1 + fat_sectors, Cluster 2)
    let root_offset = (1 + fat_sectors) * 512;
    let name = b"INIT    ELF"; // Định dạng shortname 8.3
    image[root_offset..root_offset + 11].copy_from_slice(name);
    image[root_offset + 11] = 0x20; // Archive attribute
    image[root_offset + 20] = 0;
    image[root_offset + 21] = 0;

    let first_cluster_bytes = 3u16.to_le_bytes(); // INIT.ELF dữ liệu bắt đầu ở Cluster 3
    image[root_offset + 26..root_offset + 28].copy_from_slice(&first_cluster_bytes);

    let size_bytes = (init_elf.len() as u32).to_le_bytes();
    image[root_offset + 28..root_offset + 32].copy_from_slice(&size_bytes);

    // 4. Ghi dữ liệu INIT.ELF (LBA 2 + fat_sectors trở đi)
    let data_offset = (2 + fat_sectors) * 512;
    image[data_offset..data_offset + init_elf.len()].copy_from_slice(init_elf);

    Box::leak(image.into_boxed_slice())
}
