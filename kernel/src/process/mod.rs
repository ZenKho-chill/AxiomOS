//! Phân hệ lập lịch và quản lý tiến trình (task/process subsystem)
//!
//! Mô-đun này quản lý vòng đời của các tasks trong nhân và tiến trình userspace đầu tiên (init).

pub mod elf;
pub mod scheduler;
pub mod task;

use crate::fs::kernel_file::{kernel_open_file, kernel_read};
use crate::memory::paging::{map_user_page, FLAG_USER, FLAG_WRITABLE};
use crate::memory::user_space::UserAddressSpace;
use crate::process::elf::load_elf64;
use spin::Mutex;

/// Các lỗi có thể xảy ra khi khởi tạo tiến trình userspace
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitError {
    /// Không tìm thấy tệp tin init.elf
    FileNotFound,
    /// Định dạng tệp ELF không hợp lệ
    InvalidElf,
    /// Lỗi quản lý bộ nhớ (cấp phát/ánh xạ trang)
    MemoryError,
    /// Chưa cấu hình direct-map HHDM
    NoHhdm,
}

impl InitError {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FileNotFound => "Không tìm thấy tệp tin init.elf",
            Self::InvalidElf => "Tệp tin ELF không hợp lệ hoặc không được hỗ trợ",
            Self::MemoryError => "Lỗi cấp phát hoặc ánh xạ trang bộ nhớ",
            Self::NoHhdm => "Chưa cấu hình direct-map HHDM",
        }
    }
}

/// Lưu trữ thông tin ngữ cảnh nạp của tiến trình userspace
pub struct ProcessImage {
    pub id: u32,
    pub address_space: UserAddressSpace,
    pub entry_point: u64,
    pub user_stack_top: u64,
}

/// Tiến trình init toàn cục của hệ thống
pub static INIT_PROCESS: Mutex<Option<ProcessImage>> = Mutex::new(None);

/// Nạp và chuẩn bị tiến trình userspace init đầu tiên từ ổ đĩa
pub fn spawn_init(path: &str) -> Result<u32, InitError> {
    // 1. Mở file init.elf
    let mut handle = kernel_open_file(path).map_err(|_| InitError::FileNotFound)?;
    let size = handle.size() as usize;

    // Cấp phát buffer tạm thời để đọc file
    let mut buffer = alloc::vec![0u8; size];
    kernel_read(&mut handle, &mut buffer).map_err(|_| InitError::FileNotFound)?;

    // 2. Phân tích và nạp ELF vào các frames vật lý mới
    let loaded_image = load_elf64(&buffer).map_err(|_| InitError::InvalidElf)?;
    let entry_point = loaded_image.entry_point;

    // 3. Khởi tạo Address Space ảo riêng biệt cho userspace
    let mut address_space = UserAddressSpace::new().map_err(|_| InitError::MemoryError)?;
    let l4_phys = address_space.l4_table_phys();

    // 4. Chuẩn bị stack ảo cho userspace (16 KiB - 4 trang ảo tại đỉnh bộ nhớ người dùng)
    let user_stack_limit = 0x0000_7FFF_FFFF_E000u64; // Dưới stack top 8 KiB
    let user_stack_top = 0x0000_7FFF_FFFF_F000u64; // Căn lề trang
    let num_stack_pages = 4; // 16 KiB stack
    let hhdm = crate::memory::frame::hhdm_offset().map_err(|_| InitError::NoHhdm)?;

    for i in 0..num_stack_pages {
        let frame = crate::memory::frame::allocate_frame().map_err(|_| InitError::MemoryError)?;
        let page_vaddr = user_stack_limit + (i as u64 * 4096);

        // Ánh xạ stack ảo sang frame vật lý với cờ USER + WRITABLE
        unsafe {
            map_user_page(
                l4_phys,
                page_vaddr,
                frame.start_address(),
                FLAG_USER | FLAG_WRITABLE,
                hhdm,
                true,
            )
            .map_err(|_| InitError::MemoryError)?;
        }
    }

    // 5. Ánh xạ các segments của ELF vào bảng trang userspace
    unsafe {
        address_space
            .load_image(loaded_image)
            .map_err(|_| InitError::MemoryError)?;
    }

    // 6. Lưu trữ ProcessImage vào biến tĩnh toàn cục
    let pid = 1;
    let mut init_guard = INIT_PROCESS.lock();
    *init_guard = Some(ProcessImage {
        id: pid,
        address_space,
        entry_point,
        user_stack_top,
    });

    crate::serial_println!(
        "[AXIOMOS] Spawning init process (PID: {}, Entry Point: 0x{:X})",
        pid,
        entry_point
    );

    Ok(pid)
}

/// Chuyển đổi phân quyền sang Ring 3 để bắt đầu chạy tiến trình userspace
///
/// # Safety
/// Hàm này can thiệp vào các segment registers và sử dụng `iretq` để nhảy phân quyền Ring 3.
/// Yêu cầu bảng trang userspace đã được map hợp lệ và chứa mã độc lập.
pub unsafe fn enter_userspace(_pid: u32) -> ! {
    let process = {
        let mut guard = INIT_PROCESS.lock();
        guard
            .take()
            .expect("Lỗi: Không tìm thấy INIT_PROCESS để khởi chạy!")
    };

    let entry_point = process.entry_point;
    let user_stack_top = process.user_stack_top;
    let l4_phys = process.address_space.l4_table_phys();

    // Do chúng ta dùng take(), struct process sẽ bị drop và giải phóng bộ nhớ khi ra khỏi scope hiện tại.
    // Để giữ cho bộ nhớ vật lý của bảng trang và ELF segments không bị giải phóng khi drop,
    // chúng ta sử dụng mem::forget đối với address_space của process.
    // Bộ nhớ này sẽ được sở hữu vĩnh viễn bởi CPU trong suốt vòng đời tiến trình.
    core::mem::forget(process.address_space);

    crate::serial_println!("[AXIOMOS] Switched to Ring 3 (userspace)");

    // 1. Tắt ngắt trước khi chuyển đổi
    core::arch::asm!("cli");

    // 2. Kích hoạt bảng trang ảo của userspace (ghi CR3)
    core::arch::asm!("mov cr3, {}", in(reg) l4_phys);

    // 3. Chuẩn bị stack frame cho iretq
    let ss = 0x23u64; // User Data Selector (Index 4, RPL=3)
    let cs = 0x2Bu64; // User Code Selector (Index 5, RPL=3)
    let rflags = 0x202u64; // Bật cờ IF (bit 9) để nhận ngắt và cờ mặc định (bit 1)

    // Lấy RSP hiện tại của kernel để lưu làm KERNEL_RSP
    // Khi userspace gọi syscall, CPU tự động chuyển sang stack này
    let curr_rsp: u64;
    core::arch::asm!("mov {}, rsp", out(reg) curr_rsp);
    crate::syscall::KERNEL_RSP = curr_rsp;

    // Thực hiện iretq để nhảy sang Ring 3
    core::arch::asm!(
        "push {ss}",
        "push {rsp}",
        "push {rflags}",
        "push {cs}",
        "push {rip}",
        "iretq",
        ss = in(reg) ss,
        rsp = in(reg) user_stack_top,
        rflags = in(reg) rflags,
        cs = in(reg) cs,
        rip = in(reg) entry_point,
        options(noreturn)
    );
}
