//! Các cấu trúc đồng bộ hóa tối giản của kernel (Spinlock, SpinlockIrqSave, Mutex).

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

/// Khóa Spinlock tối giản dựa trên AtomicBool.
pub struct Spinlock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

// SAFETY: Chúng ta đảm bảo truy cập độc quyền vào UnsafeCell thông qua atomic spin-lock,
// do đó việc chia sẻ và truyền Spinlock giữa các luồng/CPU là an toàn nếu dữ liệu T là Send.
unsafe impl<T: Send> Sync for Spinlock<T> {}
unsafe impl<T: Send> Send for Spinlock<T> {}

impl<T> Spinlock<T> {
    /// Tạo một Spinlock mới bọc dữ liệu.
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    /// Lấy khóa. Hàm này sẽ liên tục spin-wait cho tới khi lấy được khóa.
    pub fn lock(&self) -> SpinlockGuard<'_, T> {
        // Sử dụng compare_exchange để thử đặt trạng thái locked từ false thành true.
        // memory ordering Acquire đảm bảo các lệnh đọc ghi sau khi lấy khóa không bị CPU sắp xếp lại lên trước.
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // Spin-wait tối ưu trên CPU bằng chỉ thị hint loop (trong no_std ta có thể dùng spin_loop)
            core::hint::spin_loop();
        }

        SpinlockGuard { lock: self }
    }

    /// Kiểm tra xem khóa có đang bị giữ bởi bất kỳ luồng nào hay không.
    pub fn is_locked(&self) -> bool {
        self.locked.load(Ordering::Relaxed)
    }

    /// Giải phóng khóa một cách thủ công (không an sau vì bypass RAII guard).
    ///
    /// # Safety
    /// Hàm này trực tiếp đặt cờ khóa về false mà không kiểm tra ai đang giữ khóa.
    /// Chỉ dùng khi thực sự cần thiết (như trong trường hợp khẩn cấp, panic handler giải phóng khóa logger).
    pub unsafe fn force_unlock(&self) {
        // SAFETY: Giải phóng cờ khóa trực tiếp bằng release ordering.
        self.locked.store(false, Ordering::Release);
    }
}

/// RAII Guard cho Spinlock, tự động giải phóng khóa khi bị Drop.
pub struct SpinlockGuard<'a, T> {
    lock: &'a Spinlock<T>,
}

impl<'a, T> Deref for SpinlockGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: SpinlockGuard tồn tại đồng nghĩa với việc chúng ta đang giữ khóa độc quyền.
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T> DerefMut for SpinlockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: SpinlockGuard tồn tại đồng nghĩa với việc chúng ta đang giữ khóa độc quyền.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T> Drop for SpinlockGuard<'a, T> {
    fn drop(&mut self) {
        // Giải phóng khóa bằng Release ordering để đảm bảo các thay đổi bộ nhớ trong critical section
        // được cập nhật trước khi cờ locked chuyển về false.
        self.lock.locked.store(false, Ordering::Release);
    }
}

/// Khóa Spinlock an toàn với ngắt (Interrupt-safe Spinlock).
///
/// Tự động tắt ngắt trên CPU hiện tại khi lấy khóa và khôi phục lại trạng thái ngắt cũ khi giải phóng khóa.
pub struct SpinlockIrqSave<T> {
    lock: Spinlock<T>,
}

// SAFETY: Giống Spinlock, việc đồng bộ hóa dữ liệu độc quyền đảm bảo tính an toàn.
unsafe impl<T: Send> Sync for SpinlockIrqSave<T> {}
unsafe impl<T: Send> Send for SpinlockIrqSave<T> {}

impl<T> SpinlockIrqSave<T> {
    /// Tạo một SpinlockIrqSave mới bọc dữ liệu.
    pub const fn new(data: T) -> Self {
        Self {
            lock: Spinlock::new(data),
        }
    }

    /// Lấy khóa an toàn ngắt.
    pub fn lock(&self) -> SpinlockIrqSaveGuard<'_, T> {
        let interrupts_enabled = crate::arch::x86_64::instructions::are_interrupts_enabled();

        // SAFETY: Việc vô hiệu hóa ngắt cục bộ bằng cli yêu cầu CPU ở Ring 0.
        // Đây là thao tác an toàn nhằm bảo vệ tránh deadlock khi interrupt handler cố lấy khóa.
        unsafe {
            crate::arch::x86_64::instructions::cli();
        }

        // Thực hiện spin-wait lấy khóa
        while self
            .lock
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // Do ngắt đã bị tắt, nếu có spin-loop, CPU chỉ spin trong phạm vi nội bộ.
            core::hint::spin_loop();
        }

        SpinlockIrqSaveGuard {
            lock: &self.lock,
            interrupts_enabled,
        }
    }

    /// Kiểm tra xem khóa có đang bị giữ hay không.
    pub fn is_locked(&self) -> bool {
        self.lock.is_locked()
    }

    /// Giải phóng khóa thủ công và khôi phục trạng thái ngắt (không an toàn).
    ///
    /// # Safety
    /// Hàm này phá vỡ cơ chế RAII, chỉ dùng trong tình huống khẩn cấp của Kernel (ví dụ panic).
    pub unsafe fn force_unlock(&self, restore_interrupts: bool) {
        // SAFETY: Giải phóng cờ khóa trực tiếp.
        self.lock.force_unlock();
        if restore_interrupts {
            // SAFETY: Kích hoạt lại ngắt yêu cầu Ring 0.
            crate::arch::x86_64::instructions::sti();
        }
    }
}

/// RAII Guard cho SpinlockIrqSave, tự động giải phóng khóa và khôi phục ngắt khi bị Drop.
pub struct SpinlockIrqSaveGuard<'a, T> {
    lock: &'a Spinlock<T>,
    interrupts_enabled: bool,
}

impl<'a, T> Deref for SpinlockIrqSaveGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: Guard tồn tại chứng tỏ ta đang giữ khóa độc quyền.
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T> DerefMut for SpinlockIrqSaveGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: Guard tồn tại chứng tỏ ta đang giữ khóa độc quyền.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T> Drop for SpinlockIrqSaveGuard<'a, T> {
    fn drop(&mut self) {
        // 1. Giải phóng cờ khóa
        self.lock.locked.store(false, Ordering::Release);

        // 2. Khôi phục trạng thái ngắt cũ
        if self.interrupts_enabled {
            // SAFETY: Kích hoạt lại ngắt qua sti yêu cầu đặc quyền Ring 0 và IDT đã sẵn sàng.
            unsafe {
                crate::arch::x86_64::instructions::sti();
            }
        }
    }
}

/// Khóa Mutex cơ bản của kernel (hiện tại là wrapper xung quanh Spinlock).
///
/// Sau này khi scheduler hoàn chỉnh, Mutex sẽ hỗ trợ chặn luồng (blocking) thay vì spin-wait.
pub struct Mutex<T> {
    lock: Spinlock<T>,
}

// SAFETY: Mutex bảo vệ truy cập dữ liệu độc quyền an toàn.
unsafe impl<T: Send> Sync for Mutex<T> {}
unsafe impl<T: Send> Send for Mutex<T> {}

impl<T> Mutex<T> {
    /// Tạo Mutex mới bọc dữ liệu.
    pub const fn new(data: T) -> Self {
        Self {
            lock: Spinlock::new(data),
        }
    }

    /// Lấy khóa Mutex (spin-wait tạm thời).
    pub fn lock(&self) -> MutexGuard<'_, T> {
        let guard = self.lock.lock();
        // Giải phóng guard thô để trả về MutexGuard bọc trực tiếp Spinlock.
        core::mem::forget(guard);

        MutexGuard { lock: &self.lock }
    }

    /// Kiểm tra xem Mutex có đang bị khóa hay không.
    pub fn is_locked(&self) -> bool {
        self.lock.is_locked()
    }
}

/// RAII Guard cho Mutex.
pub struct MutexGuard<'a, T> {
    lock: &'a Spinlock<T>,
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: MutexGuard tồn tại đảm bảo giữ khóa độc quyền.
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: MutexGuard tồn tại đảm bảo giữ khóa độc quyền.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}
