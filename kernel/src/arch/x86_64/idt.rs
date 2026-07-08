//! Interrupt Descriptor Table (IDT) cho x86_64
//!
//! Định nghĩa cấu trúc IDT và đăng ký các CPU Exception Handlers.

use crate::serial_println;
use core::arch::asm;
use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy)]
#[repr(C, packed)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    attributes: u8,
    offset_middle: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
    const fn missing() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            attributes: 0,
            offset_middle: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    fn set_handler(&mut self, handler_address: u64, cs_selector: u16) {
        self.offset_low = handler_address as u16;
        self.selector = cs_selector;
        self.ist = 0;
        self.attributes = 0x8E; // Present, DPL=0, Type=Interrupt Gate (0xE)
        self.offset_middle = (handler_address >> 16) as u16;
        self.offset_high = (handler_address >> 32) as u32;
        self.reserved = 0;
    }
}

#[repr(C, align(16))]
struct Idt {
    entries: [IdtEntry; 256],
}

// Bảng IDT tĩnh của Kernel
static mut IDT: Idt = Idt {
    entries: [IdtEntry::missing(); 256],
};

/// Stack snapshot do CPU đẩy vào khi ngắt xảy ra
#[derive(Debug)]
#[repr(C)]
pub struct InterruptStackFrame {
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

/// Khởi tạo và nạp IDT
///
/// # Safety
/// Hàm này thay đổi bảng IDT của CPU, yêu cầu CPU ở đặc quyền Ring 0.
pub unsafe fn init() {
    let cs = crate::arch::x86_64::gdt::KERNEL_CODE_SELECTOR;

    // Đăng ký các exception handlers thiết yếu
    IDT.entries[0].set_handler(divide_by_zero_handler as *const () as u64, cs);
    IDT.entries[3].set_handler(breakpoint_handler as *const () as u64, cs);
    IDT.entries[8].set_handler(double_fault_handler as *const () as u64, cs);
    IDT.entries[13].set_handler(general_protection_fault_handler as *const () as u64, cs);
    IDT.entries[14].set_handler(page_fault_handler as *const () as u64, cs);

    // Đăng ký các ngắt phần cứng (IRQ)
    IDT.entries[0x20].set_handler(timer_interrupt_handler as *const () as u64, cs);
    IDT.entries[0x21].set_handler(keyboard_interrupt_handler as *const () as u64, cs);

    let base = core::ptr::addr_of!(IDT) as u64;
    let limit = (core::mem::size_of::<Idt>() - 1) as u16;

    // SAFETY: Dựng descriptor trực tiếp trên stack để nạp IDT an toàn, tránh lỗi cắt địa chỉ 64-bit của compiler
    asm!(
        "sub rsp, 16",
        "mov [rsp + 2], {base}",
        "mov [rsp], {limit:x}",
        "lidt [rsp]",
        "add rsp, 16",
        base = in(reg) base,
        limit = in(reg) limit,
    );
}

// --- Exception Handlers ---

extern "x86-interrupt" fn divide_by_zero_handler(frame: &mut InterruptStackFrame) {
    serial_println!(
        "[AXIOMOS EXCEPTION] Divide by Zero at RIP: {:#x}",
        frame.rip
    );
    loop {
        // SAFETY: Dừng CPU sau khi log lỗi không thể phục hồi
        unsafe {
            asm!("hlt");
        }
    }
}

extern "x86-interrupt" fn breakpoint_handler(frame: &mut InterruptStackFrame) {
    serial_println!("[AXIOMOS EXCEPTION] Breakpoint at RIP: {:#x}", frame.rip);
    // Ngoại lệ này có thể phục hồi, ta return để CPU tiếp tục chạy lệnh tiếp theo
}

extern "x86-interrupt" fn double_fault_handler(
    frame: &mut InterruptStackFrame,
    error_code: u64,
) -> ! {
    serial_println!(
        "[AXIOMOS EXCEPTION] Double Fault! Error code: {}, RIP: {:#x}",
        error_code,
        frame.rip
    );
    loop {
        // SAFETY: Dừng CPU sau khi gặp lỗi nghiêm trọng
        unsafe {
            asm!("hlt");
        }
    }
}

extern "x86-interrupt" fn general_protection_fault_handler(
    frame: &mut InterruptStackFrame,
    error_code: u64,
) {
    serial_println!(
        "[AXIOMOS EXCEPTION] General Protection Fault! Error code: {}, RIP: {:#x}",
        error_code,
        frame.rip
    );
    loop {
        // SAFETY: GPF không phục hồi được trong kernel mode
        unsafe {
            asm!("hlt");
        }
    }
}

extern "x86-interrupt" fn page_fault_handler(frame: &mut InterruptStackFrame, error_code: u64) {
    let cr2: u64;
    // SAFETY: Lấy địa chỉ gây ra page fault từ thanh ghi CR2
    unsafe {
        asm!("mov {}, cr2", out(reg) cr2);
    }
    serial_println!(
        "[AXIOMOS EXCEPTION] Page Fault accessing address: {:#x}, Error code: {}, RIP: {:#x}",
        cr2,
        error_code,
        frame.rip
    );
    loop {
        // SAFETY: Page fault không phục hồi được ở giai đoạn này
        unsafe {
            asm!("hlt");
        }
    }
}

// --- Hardware Interrupt Handlers ---

/// Bộ đếm tick tĩnh cho ngắt Timer
pub static TIMER_TICKS: AtomicU64 = AtomicU64::new(0);

extern "x86-interrupt" fn timer_interrupt_handler(_frame: &mut InterruptStackFrame) {
    TIMER_TICKS.fetch_add(1, Ordering::Relaxed);

    // SAFETY: Gửi tín hiệu EOI cho PIC IRQ 0
    unsafe {
        crate::drivers::pic::send_eoi(0);
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_frame: &mut InterruptStackFrame) {
    // SAFETY: Đọc dữ liệu scan code từ cổng bàn phím 0x60 và gửi EOI cho PIC IRQ 1
    unsafe {
        let scancode = crate::drivers::pic::inb(0x60);
        crate::drivers::keyboard::handle_scancode(scancode);
        crate::drivers::pic::send_eoi(1);
    }
}
