//! Global Descriptor Table (GDT) cho x86_64
//!
//! Định nghĩa và nạp bảng phân đoạn tối thiểu cho chế độ 64-bit Long Mode.

use core::arch::asm;

#[repr(C, align(8))]
struct Gdt {
    null: u64,
    kernel_code: u64,
    kernel_data: u64,
}

// Bảng GDT tĩnh của Kernel
// CS Selector: 0x08 (Index 1)
// DS Selector: 0x10 (Index 2)
static GDT: Gdt = Gdt {
    null: 0,
    kernel_code: 0x00af9a000000ffff, // Kernel Code Segment (64-bit, Ring 0)
    kernel_data: 0x00cf92000000ffff, // Kernel Data Segment (64-bit, Ring 0)
};

/// Khởi tạo và nạp GDT của Kernel
///
/// # Safety
/// Hàm này thay đổi bảng phân đoạn của CPU, yêu cầu CPU ở trạng thái đặc quyền Ring 0.
pub unsafe fn init() {
    let base = &GDT as *const _ as u64;
    let limit = (core::mem::size_of::<Gdt>() - 1) as u16;

    // SAFETY: Dựng descriptor trực tiếp trên stack để nạp GDT an toàn, sau đó reload CS và các segment registers.
    asm!(
        "sub rsp, 16",
        "mov [rsp + 2], {base}",
        "mov [rsp], {limit:x}",
        "lgdt [rsp]",
        "add rsp, 16",
        
        "push 0x08",           // CS selector trong GDT mới
        "lea rax, [2f]",       // Địa chỉ nhãn 2
        "push rax",
        "retfq",               // Far return để reload CS và RIP
        "2:",
        "mov ax, 0x10",        // Data segment selector trong GDT mới
        "mov ds, ax",
        "mov es, ax",
        "mov ss, ax",
        "mov fs, ax",
        "mov gs, ax",
        base = in(reg) base,
        limit = in(reg) limit,
        out("rax") _
    );
}
