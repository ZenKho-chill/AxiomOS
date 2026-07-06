use core::fmt::{self, Write};
use spin::Mutex;
use uart_16550::SerialPort;

/// Cổng nối tiếp COM1 được bảo vệ bởi Mutex
pub static SERIAL1: Mutex<SerialPort> = Mutex::new({
    // SAFETY: COM1 tại I/O port 0x3F8 là cổng serial chuẩn trong QEMU x86_64;
    // mọi truy cập được tuần tự hóa qua Mutex trước khi ghi dữ liệu.
    unsafe { SerialPort::new(0x3F8) }
});

/// Khởi tạo cổng Serial COM1
pub fn init() {
    SERIAL1.lock().init();
}

/// Ghi dữ liệu định dạng ra cổng Serial
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    let _ = SERIAL1.lock().write_fmt(args);
}

/// Macro in ra Serial không xuống dòng
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::drivers::serial::_print(format_args!($($arg)*));
    };
}

/// Macro in ra Serial kèm xuống dòng
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(concat!($fmt, "\n"), $($arg)*));
}
