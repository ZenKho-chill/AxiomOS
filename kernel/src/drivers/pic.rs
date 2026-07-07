//! Driver cho bộ điều khiển ngắt 8259 PIC (Programmable Interrupt Controller)
//!
//! Remap các cổng ngắt phần cứng IRQ để tránh xung đột với CPU Exceptions.

use core::arch::asm;

const MASTER_COMMAND: u16 = 0x20;
const MASTER_DATA: u16 = 0x21;
const SLAVE_COMMAND: u16 = 0xA0;
const SLAVE_DATA: u16 = 0xA1;

const ICW1_INIT: u8 = 0x11;
const ICW4_8086: u8 = 0x01;

unsafe fn outb(port: u16, value: u8) {
    // SAFETY: Viết trực tiếp byte ra I/O port yêu cầu quyền Ring 0
    asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
}

unsafe fn io_delay() {
    // SAFETY: Ghi cổng rác 0x80 để tạo độ trễ ngắn cho các chip cũ
    outb(0x80, 0);
}

/// Khởi tạo và remap bộ ngắt 8259 PIC
///
/// # Safety
/// Hàm này tương tác trực tiếp với các I/O ports phần cứng, yêu cầu CPU chạy ở Ring 0.
pub unsafe fn init() {
    // Khởi đầu quá trình khởi tạo (ICW1)
    outb(MASTER_COMMAND, ICW1_INIT);
    io_delay();
    outb(SLAVE_COMMAND, ICW1_INIT);
    io_delay();

    // Thiết lập vector ngắt offset (ICW2)
    // Master PIC remap sang IRQ 0-7 -> Vector 0x20 - 0x27
    // Slave PIC remap sang IRQ 8-15 -> Vector 0x28 - 0x2F
    outb(MASTER_DATA, 0x20);
    io_delay();
    outb(SLAVE_DATA, 0x28);
    io_delay();

    // Thiết lập kết nối Master-Slave (ICW3)
    outb(MASTER_DATA, 0x04); // Master báo có Slave ở IRQ2
    io_delay();
    outb(SLAVE_DATA, 0x02); // Slave báo ID kết nối là 2
    io_delay();

    // Thiết lập chế độ 8086 (ICW4)
    outb(MASTER_DATA, ICW4_8086);
    io_delay();
    outb(SLAVE_DATA, ICW4_8086);
    io_delay();

    // Mask (tắt) tất cả ngắt IRQ phần cứng ở giai đoạn này để tránh ngắt rác gây crash
    outb(MASTER_DATA, 0xFF);
    outb(SLAVE_DATA, 0xFF);
}
