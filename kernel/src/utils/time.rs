//! Quản lý thời gian hệ thống và bộ đếm ticks của CPU.
//!
//! Tích hợp với Programmable Interval Timer (PIT) chạy ở tần số 1000Hz (1ms/tick).

use core::sync::atomic::Ordering;

#[cfg(test)]
use core::sync::atomic::AtomicU64;

/// Tần số PIT đích (1000Hz tức là 1ms mỗi tick)
#[cfg(not(test))]
const PIT_FREQUENCY_HZ: u32 = 1000;
#[cfg(not(test))]
const PIT_BASE_FREQUENCY: u32 = 1193182;

#[cfg(not(test))]
const PIT_CHANNEL_0: u16 = 0x40;
#[cfg(not(test))]
const PIT_COMMAND: u16 = 0x43;
#[cfg(not(test))]
const PIT_COMMAND_VAL: u8 = 0x36; // Channel 0, lobyte/hibyte, Mode 3, Binary

#[cfg(test)]
static MOCK_TICKS: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
fn increment_mock_ticks(n: u64) {
    MOCK_TICKS.fetch_add(n, Ordering::Relaxed);
}

/// Khởi tạo Programmable Interval Timer (PIT) ở tần số 1000Hz (1ms mỗi ngắt).
///
/// # Safety
/// Hàm này sử dụng các lệnh I/O port của CPU để cấu hình chip phần cứng PIT,
/// yêu cầu CPU chạy ở Ring 0 và IDT đã sẵn sàng.
pub unsafe fn init() {
    #[cfg(not(test))]
    {
        let divisor = PIT_BASE_FREQUENCY / PIT_FREQUENCY_HZ;

        // Ghi command byte 0x36 ra port 0x43
        outb(PIT_COMMAND, PIT_COMMAND_VAL);
        // Ghi low byte divisor ra port 0x40
        outb(PIT_CHANNEL_0, (divisor & 0xFF) as u8);
        // Ghi high byte divisor ra port 0x40
        outb(PIT_CHANNEL_0, ((divisor >> 8) & 0xFF) as u8);
    }
}

/// Đọc số lượng ticks tích lũy từ khi khởi động hệ thống.
pub fn ticks() -> u64 {
    #[cfg(not(test))]
    {
        crate::arch::x86_64::idt::TIMER_TICKS.load(Ordering::Relaxed)
    }
    #[cfg(test)]
    {
        MOCK_TICKS.load(Ordering::Relaxed)
    }
}

/// Trả về thời gian hệ thống hoạt động (uptime) tính bằng mili-giây.
pub fn uptime_ms() -> u64 {
    // Vì PIT được cấu hình chạy ở 1000Hz, 1 tick tương đương với đúng 1ms.
    ticks()
}

/// Tạm dừng thực thi luồng hiện tại trong khoảng thời gian mili-giây chỉ định (busy-wait).
pub fn sleep_ms(ms: u64) {
    let start = uptime_ms();
    while uptime_ms() - start < ms {
        core::hint::spin_loop();
    }
}

#[cfg(not(test))]
unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(nomem, nostack, preserves_flags)
    );
}

#[cfg(test)]
mod tests {
    extern crate std;
    use self::std::thread;
    use self::std::time::Duration;
    use super::*;

    #[test]
    fn test_uptime_increases() {
        MOCK_TICKS.store(0, Ordering::Relaxed);
        assert_eq!(uptime_ms(), 0);
        increment_mock_ticks(10);
        assert_eq!(uptime_ms(), 10);
        increment_mock_ticks(5);
        assert_eq!(uptime_ms(), 15);
        MOCK_TICKS.store(0, Ordering::Relaxed);
    }

    #[test]
    fn test_sleep_ms() {
        MOCK_TICKS.store(0, Ordering::Relaxed);

        // Spawn một thread phụ để tăng mock ticks giả lập cho busy-wait
        thread::spawn(|| {
            for _ in 0..20 {
                thread::sleep(Duration::from_millis(1));
                increment_mock_ticks(1);
            }
        });

        sleep_ms(5);
        assert!(uptime_ms() >= 5);
        MOCK_TICKS.store(0, Ordering::Relaxed);
    }
}
