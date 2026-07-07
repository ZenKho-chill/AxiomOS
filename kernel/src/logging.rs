//! Logging facade nội bộ của kernel.
//!
//! Serial COM1 vẫn là sink chính. Framebuffer chỉ được dùng như mirror tùy chọn.

use core::fmt::{self, Write};

/// Mức log tối thiểu dùng để chuẩn hóa metadata logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// Ghi một record log ra serial và mirror framebuffer nếu record yêu cầu.
pub fn write(record: LogRecord<'_>) {
    write_serial(&record);

    if record.mirror_framebuffer {
        write_framebuffer(&record);
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

fn write_serial(record: &LogRecord<'_>) {
    let mut serial = crate::drivers::serial::SERIAL1.lock();
    let _ = write_record(&mut *serial, record);
    let _ = serial.write_str("\n");
}

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
}
