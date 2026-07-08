//! Global Descriptor Table (GDT) cho x86_64
//!
//! Định nghĩa và nạp bảng phân đoạn tối thiểu cho chế độ 64-bit Long Mode.

use core::arch::asm;

pub const KERNEL_CODE_SELECTOR: u16 = 0x08;
pub const KERNEL_DATA_SELECTOR: u16 = 0x10;

#[repr(C, packed)]
struct DescriptorTablePointer {
    limit: u16,
    base: u64,
}

#[repr(C, align(8))]
struct Gdt {
    null: u64,
    kernel_code: u64,
    kernel_data: u64,
    user_data_dummy: u64, // Placeholder để CS/SS của user cách nhau đúng 8 bytes theo quy định của sysret
    user_data: u64,       // User Data Segment (Ring 3)
    user_code: u64,       // User Code Segment (Ring 3)
}

// Bảng GDT tĩnh của Kernel mở rộng cho Userspace
// CS Kernel Selector: 0x08 (Index 1)
// DS Kernel Selector: 0x10 (Index 2)
// DS User Selector Base: 0x1B (Index 3 | RPL=3 = 0x1B). sysret sẽ tự động set:
// - SS = Selector Base + 8 = 0x20 | RPL=3 = 0x23 (Index 4, User Data)
// - CS = Selector Base + 16 = 0x28 | RPL=3 = 0x2B (Index 5, User Code)
static GDT: Gdt = Gdt {
    null: 0,
    // Accessed bit được bật sẵn để CPU không phải ghi vào GDT nằm trong trang read-only.
    kernel_code: 0x00af9b000000ffff, // Kernel Code Segment (64-bit, Ring 0)
    kernel_data: 0x00cf93000000ffff, // Kernel Data Segment (64-bit, Ring 0)
    user_data_dummy: 0,
    user_data: 0x00cff3000000ffff, // User Data Segment (64-bit, Ring 3)
    user_code: 0x00affb000000ffff, // User Code Segment (64-bit, Ring 3)
};

/// Khởi tạo và nạp GDT của Kernel
///
/// # Safety
/// Hàm này thay đổi bảng phân đoạn của CPU, yêu cầu CPU ở trạng thái đặc quyền Ring 0.
///
/// Preconditions:
/// - CPU đang chạy ở long mode do Limine bàn giao.
/// - Stack hiện tại hợp lệ để thực hiện `push` và `retfq`.
///
/// Postconditions:
/// - GDTR trỏ tới GDT tĩnh của kernel.
/// - CS được nạp lại về selector kernel code `0x08`.
/// - Các thanh ghi đoạn dữ liệu dùng selector kernel data `0x10`.
///
/// Memory safety assumptions:
/// - Địa chỉ của `GDT` được Limine map trong không gian địa chỉ kernel.
/// - Descriptor đã bật Accessed bit để CPU không ghi vào trang read-only khi nạp selector.
///
/// CPU state assumptions:
/// - Interrupt chưa được bật trong khi thay đổi GDT.
pub unsafe fn init() {
    let descriptor = DescriptorTablePointer {
        limit: (core::mem::size_of::<Gdt>() - 1) as u16,
        base: &GDT as *const _ as u64,
    };

    // SAFETY: `descriptor` trỏ tới GDT tĩnh hợp lệ. Far return nạp lại CS để selector đang cache
    // của Limine không còn được dùng sau khi kernel thay GDTR.
    asm!(
        "lgdt [{gdt_descriptor}]",
        "push {kernel_code}",
        "lea rax, [rip + 2f]",
        "push rax",
        "retfq",
        "2:",

        "mov ax, {kernel_data}",
        "mov ds, ax",
        "mov es, ax",
        "mov ss, ax",
        "mov fs, ax",
        "mov gs, ax",
        gdt_descriptor = in(reg) &descriptor,
        kernel_code = const KERNEL_CODE_SELECTOR,
        kernel_data = const KERNEL_DATA_SELECTOR,
        out("rax") _
    );
}
