//! Logging facade nội bộ của kernel.
//!
//! Serial COM1 vẫn là sink chính. Framebuffer chỉ được dùng như mirror tùy chọn.
//! Hỗ trợ bộ lọc log động và ghi log vào ring buffer tĩnh của kernel.

use core::fmt::{self, Write};

#[cfg(not(test))]
use crate::utils::sync::{Spinlock, SpinlockIrqSave};

/// Mức log tối thiểu dùng để chuẩn hóa metadata logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Boot,
    Info,
    Warn,
    Error,
    Panic,
}

impl LogLevel {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Boot => "BOOT",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Panic => "PANIC",
        }
    }
}

/// Bản ghi log không sở hữu dữ liệu và không cấp phát.
pub struct LogRecord<'a> {
    pub level: LogLevel,
    pub subsystem: Option<&'a str>,
    pub message: fmt::Arguments<'a>,
    pub mirror_framebuffer: bool,
}

impl<'a> LogRecord<'a> {
    pub fn boot(message: fmt::Arguments<'a>) -> Self {
        Self {
            level: LogLevel::Boot,
            subsystem: None,
            message,
            mirror_framebuffer: true,
        }
    }

    pub fn info(subsystem: &'a str, message: fmt::Arguments<'a>, mirror_framebuffer: bool) -> Self {
        Self {
            level: LogLevel::Info,
            subsystem: Some(subsystem),
            message,
            mirror_framebuffer,
        }
    }

    pub fn panic(message: fmt::Arguments<'a>) -> Self {
        Self {
            level: LogLevel::Panic,
            subsystem: None,
            message,
            mirror_framebuffer: true,
        }
    }
}

const BUFFER_LINE_COUNT: usize = 64;
const BUFFER_LINE_LENGTH: usize = 128;

/// Bộ đệm xoay vòng lưu trữ log thô
struct LogRingBuffer {
    lines: [[u8; BUFFER_LINE_LENGTH]; BUFFER_LINE_COUNT],
    lens: [usize; BUFFER_LINE_COUNT],
    head: usize,
    count: usize,
}

impl LogRingBuffer {
    const fn new() -> Self {
        Self {
            lines: [[0; BUFFER_LINE_LENGTH]; BUFFER_LINE_COUNT],
            lens: [0; BUFFER_LINE_COUNT],
            head: 0,
            count: 0,
        }
    }

    fn push(&mut self, text: &str) {
        let idx = (self.head + self.count) % BUFFER_LINE_COUNT;
        let bytes = text.as_bytes();
        let len = core::cmp::min(bytes.len(), BUFFER_LINE_LENGTH);

        self.lines[idx][..len].copy_from_slice(&bytes[..len]);
        self.lens[idx] = len;

        if self.count < BUFFER_LINE_COUNT {
            self.count += 1;
        } else {
            self.head = (self.head + 1) % BUFFER_LINE_COUNT;
        }
    }
}

/// Trình hỗ trợ ghi chuỗi vào bộ đệm tĩnh của log sử dụng con trỏ thô để tránh xung đột mượn bộ nhớ
struct BufferWriter {
    ptr: *mut u8,
    capacity: usize,
    cursor: usize,
}

impl Write for BufferWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let bytes = s.as_bytes();
        let len = bytes.len();
        let avail = self.capacity - self.cursor;
        let copy_len = core::cmp::min(len, avail);
        if copy_len > 0 {
            // SAFETY: ptr trỏ tới vùng nhớ đệm temp_buf hợp lệ trên stack của caller,
            // copy_len được kiểm chứng không vượt quá giới hạn capacity của buffer.
            unsafe {
                core::ptr::copy_nonoverlapping(bytes.as_ptr(), self.ptr.add(self.cursor), copy_len);
            }
            self.cursor += copy_len;
        }
        if copy_len < len {
            return Err(fmt::Error);
        }
        Ok(())
    }
}

// Trong chế độ test (chạy trên host), ta không có crate::utils::sync do kernel crate bị loại trừ,
// nên ta dùng lock cục bộ giả lập để chạy test an toàn.
#[cfg(test)]
mod mock_lock {
    use core::cell::RefCell;
    pub struct Spinlock<T> {
        cell: RefCell<T>,
    }
    impl<T> Spinlock<T> {
        pub const fn new(data: T) -> Self {
            Self {
                cell: RefCell::new(data),
            }
        }
        pub fn lock(&self) -> core::cell::RefMut<'_, T> {
            self.cell.borrow_mut()
        }
    }
    // SAFETY: Giả lập Sync trong môi trường unit test đơn luồng để pass borrow checker
    unsafe impl<T> Sync for Spinlock<T> {}

    pub struct SpinlockIrqSave<T> {
        cell: RefCell<T>,
    }
    impl<T> SpinlockIrqSave<T> {
        pub const fn new(data: T) -> Self {
            Self {
                cell: RefCell::new(data),
            }
        }
        pub fn lock(&self) -> core::cell::RefMut<'_, T> {
            self.cell.borrow_mut()
        }
    }
    // SAFETY: Giả lập Sync trong môi trường unit test đơn luồng để pass borrow checker
    unsafe impl<T> Sync for SpinlockIrqSave<T> {}
}

#[cfg(test)]
use mock_lock::{Spinlock, SpinlockIrqSave};

static MINIMUM_LOG_LEVEL: Spinlock<LogLevel> = Spinlock::new(LogLevel::Boot);
static LOG_RING_BUFFER: SpinlockIrqSave<LogRingBuffer> = SpinlockIrqSave::new(LogRingBuffer::new());

/// Ghi một record log ra serial và mirror framebuffer nếu record yêu cầu.
pub fn write(record: LogRecord<'_>) {
    // 1. Kiểm tra bộ lọc log động
    let min_level = {
        let guard = MINIMUM_LOG_LEVEL.lock();
        *guard
    };
    if record.level < min_level {
        return;
    }

    // 2. Ghi ra serial (chỉ thực hiện khi không chạy test host để tránh import crate::drivers)
    #[cfg(not(test))]
    write_serial(&record);

    #[cfg(not(test))]
    if record.mirror_framebuffer {
        write_framebuffer(&record);
    }

    // 3. Ghi vào ring buffer
    let mut temp_buf = [0u8; BUFFER_LINE_LENGTH];
    let mut writer = BufferWriter {
        ptr: temp_buf.as_mut_ptr(),
        capacity: BUFFER_LINE_LENGTH,
        cursor: 0,
    };
    if write_record(&mut writer, &record).is_ok() {
        if let Ok(s) = core::str::from_utf8(&temp_buf[..writer.cursor]) {
            LOG_RING_BUFFER.lock().push(s);
        }
    }
}

/// Ghi boot diagnostics với prefix legacy `[AXIOMOS]`.
pub fn boot(message: &str) {
    write(LogRecord::boot(format_args!("{}", message)));
}

/// Ghi panic diagnostics với prefix legacy `[AXIOMOS PANIC]`.
pub fn panic(args: fmt::Arguments<'_>) {
    write(LogRecord::panic(args));
}

/// Ghi log subsystem với prefix legacy `[AXIOMOS <SUBSYSTEM>]`.
pub fn info(subsystem: &str, args: fmt::Arguments<'_>, mirror_framebuffer: bool) {
    write(LogRecord::info(subsystem, args, mirror_framebuffer));
}

/// Cập nhật mức lọc log tối thiểu ở runtime.
pub fn set_filter_level(level: LogLevel) {
    let mut guard = MINIMUM_LOG_LEVEL.lock();
    *guard = level;
}

/// Đọc mức lọc log hiện tại.
pub fn filter_level() -> LogLevel {
    let guard = MINIMUM_LOG_LEVEL.lock();
    *guard
}

/// In trực tiếp toàn bộ log từ ring buffer ra cổng nối tiếp COM1.
#[cfg(not(test))]
pub fn dump_log_buffer() {
    let buffer = LOG_RING_BUFFER.lock();
    let mut serial = crate::drivers::serial::SERIAL1.lock();
    let _ = serial.write_str("=== DUMPING KERNEL LOG RING BUFFER ===\n");
    for i in 0..buffer.count {
        let idx = (buffer.head + i) % BUFFER_LINE_COUNT;
        let len = buffer.lens[idx];
        if let Ok(s) = core::str::from_utf8(&buffer.lines[idx][..len]) {
            let _ = serial.write_str(s);
            let _ = serial.write_str("\n");
        }
    }
    let _ = serial.write_str("=== END OF DUMP ===\n");
}

#[cfg(not(test))]
fn write_serial(record: &LogRecord<'_>) {
    let mut serial = crate::drivers::serial::SERIAL1.lock();
    let _ = write_record(&mut *serial, record);
    let _ = serial.write_str("\n");
}

#[cfg(not(test))]
fn write_framebuffer(record: &LogRecord<'_>) {
    match (record.level, record.subsystem) {
        (LogLevel::Boot, None) => {
            crate::console::framebuffer::framebuffer_println(format_args!(
                "[AXIOMOS] {}",
                record.message
            ));
        }
        (LogLevel::Panic, _) => {
            crate::console::framebuffer::framebuffer_println(format_args!(
                "[AXIOMOS PANIC] {}",
                record.message
            ));
        }
        (_, Some(subsystem)) => {
            crate::console::framebuffer::framebuffer_println(format_args!(
                "[AXIOMOS {}] {}",
                subsystem, record.message
            ));
        }
        (_, None) => {
            crate::console::framebuffer::framebuffer_println(format_args!(
                "[AXIOMOS {}] {}",
                record.level.as_str(),
                record.message
            ));
        }
    }
}

fn write_record<W: Write + ?Sized>(writer: &mut W, record: &LogRecord<'_>) -> fmt::Result {
    write_prefix(writer, record)?;
    writer.write_char(' ')?;
    writer.write_fmt(record.message)
}

fn write_prefix<W: Write + ?Sized>(writer: &mut W, record: &LogRecord<'_>) -> fmt::Result {
    match (record.level, record.subsystem) {
        (LogLevel::Boot, None) => writer.write_str("[AXIOMOS]"),
        (LogLevel::Panic, _) => writer.write_str("[AXIOMOS PANIC]"),
        (_, Some(subsystem)) => write!(writer, "[AXIOMOS {}]", subsystem),
        (_, None) => write!(writer, "[AXIOMOS {}]", record.level.as_str()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedBuffer {
        bytes: [u8; 128],
        len: usize,
    }

    impl FixedBuffer {
        const fn new() -> Self {
            Self {
                bytes: [0; 128],
                len: 0,
            }
        }

        fn as_bytes(&self) -> &[u8] {
            &self.bytes[..self.len]
        }
    }

    impl Write for FixedBuffer {
        fn write_str(&mut self, value: &str) -> fmt::Result {
            let end = self.len + value.len();
            if end > self.bytes.len() {
                return Err(fmt::Error);
            }

            self.bytes[self.len..end].copy_from_slice(value.as_bytes());
            self.len = end;
            Ok(())
        }
    }

    #[test]
    fn boot_record_uses_legacy_prefix() {
        let record = LogRecord::boot(format_args!("Kernel started"));
        let mut output = FixedBuffer::new();

        assert!(write_record(&mut output, &record).is_ok());
        assert_eq!(output.as_bytes(), b"[AXIOMOS] Kernel started");
    }

    #[test]
    fn subsystem_record_uses_subsystem_prefix() {
        let record = LogRecord::info("TIMER", format_args!("Ticks: {}", 100), false);
        let mut output = FixedBuffer::new();

        assert!(write_record(&mut output, &record).is_ok());
        assert_eq!(output.as_bytes(), b"[AXIOMOS TIMER] Ticks: 100");
    }

    #[test]
    fn panic_record_uses_panic_prefix() {
        let record = LogRecord::panic(format_args!("fatal error"));
        let mut output = FixedBuffer::new();

        assert!(write_record(&mut output, &record).is_ok());
        assert_eq!(output.as_bytes(), b"[AXIOMOS PANIC] fatal error");
    }

    #[test]
    fn log_filtering_drops_lower_level_logs() {
        set_filter_level(LogLevel::Warn);
        assert_eq!(filter_level(), LogLevel::Warn);

        // Reset ring buffer
        {
            let mut buf = LOG_RING_BUFFER.lock();
            *buf = LogRingBuffer::new();
        }

        // Info log (must be filtered)
        info("TEST", format_args!("Info level message"), false);
        {
            let buf = LOG_RING_BUFFER.lock();
            assert_eq!(buf.count, 0, "Info log should be filtered out!");
        }

        // Warn log (must be recorded)
        write(LogRecord {
            level: LogLevel::Warn,
            subsystem: Some("TEST"),
            message: format_args!("Warn level message"),
            mirror_framebuffer: false,
        });
        {
            let buf = LOG_RING_BUFFER.lock();
            assert_eq!(buf.count, 1, "Warn log should not be filtered out!");
            let len = buf.lens[buf.head];
            let s = core::str::from_utf8(&buf.lines[buf.head][..len]).unwrap();
            assert!(s.contains("[AXIOMOS TEST] Warn level message"));
        }

        // Reset level
        set_filter_level(LogLevel::Boot);
    }

    #[test]
    fn log_ring_buffer_wraps_correctly() {
        // Reset ring buffer
        {
            let mut buf = LOG_RING_BUFFER.lock();
            *buf = LogRingBuffer::new();
        }

        set_filter_level(LogLevel::Boot);

        // Push BUFFER_LINE_COUNT + 5 logs
        for i in 0..(BUFFER_LINE_COUNT + 5) {
            info("TEST", format_args!("Log message {}", i), false);
        }

        let buf = LOG_RING_BUFFER.lock();
        assert_eq!(buf.count, BUFFER_LINE_COUNT);
        assert_eq!(buf.head, 5); // Đã xoay vòng và đè 5 dòng đầu tiên

        // Dòng đầu tiên đọc ra phải là log index 5
        let len = buf.lens[buf.head];
        let s = core::str::from_utf8(&buf.lines[buf.head][..len]).unwrap();
        assert!(s.contains("[AXIOMOS TEST] Log message 5"));
    }
}
