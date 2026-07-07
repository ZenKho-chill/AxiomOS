use crate::boot::limine::LimineMmapEntry;
use spin::Mutex;

pub const PAGE_SIZE: usize = 4096;
const BITS_PER_BYTE: usize = 8;
const LIMINE_USABLE_MEMORY: u32 = 0;

/// Lỗi xảy ra trong hệ thống quản lý bộ nhớ
#[derive(Debug, Clone, Copy)]
pub enum MemoryError {
    NoMemoryMap,
    NoHhdmOffset,
    OutOfFrames,
    BitmapTooLarge,
    FrameOutOfRange,
    FrameNotUsable,
    FrameAlreadyFree,
    UnalignedFrame,
    NotInitialized,
}

/// Khung trang vật lý 4 KiB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysFrame {
    start_address: u64,
}

impl PhysFrame {
    /// Tạo frame từ địa chỉ vật lý đã căn lề 4 KiB.
    pub const fn from_start_address(start_address: u64) -> Result<Self, MemoryError> {
        if start_address % PAGE_SIZE as u64 != 0 {
            return Err(MemoryError::UnalignedFrame);
        }

        Ok(Self { start_address })
    }

    /// Trả về địa chỉ vật lý đầu frame.
    pub const fn start_address(self) -> u64 {
        self.start_address
    }
}

/// Cấu trúc lưu trữ thống kê bộ nhớ vật lý
#[derive(Debug, Clone, Copy)]
pub struct MemoryStats {
    pub total_usable: usize,
    pub total_usable_frames: usize,
    pub allocated_frames: usize,
    pub free_frames: usize,
    pub region_count: usize,
}

impl MemoryStats {
    const fn empty() -> Self {
        Self {
            total_usable: 0,
            total_usable_frames: 0,
            allocated_frames: 0,
            free_frames: 0,
            region_count: 0,
        }
    }
}

pub struct BitmapFrameAllocator {
    bitmap_raw: *mut u8,
    entries_ptr: *const *const LimineMmapEntry,
    entry_count: usize,
    total_frames: usize,
    total_usable_frames: usize,
    allocated_frames: usize,
    last_searched_idx: usize,
    hhdm_offset: u64,
    stats: MemoryStats,
    initialized: bool,
}

// SAFETY: BitmapFrameAllocator sẽ được bọc trong spin::Mutex để đồng bộ hóa đa luồng an toàn.
unsafe impl Send for BitmapFrameAllocator {}

impl BitmapFrameAllocator {
    /// Tạo allocator thô chưa khởi tạo
    const fn empty() -> Self {
        Self {
            bitmap_raw: core::ptr::null_mut(),
            entries_ptr: core::ptr::null(),
            entry_count: 0,
            total_frames: 0,
            total_usable_frames: 0,
            allocated_frames: 0,
            last_searched_idx: 0,
            hhdm_offset: 0,
            stats: MemoryStats::empty(),
            initialized: false,
        }
    }

    /// Khởi tạo allocator từ memory map qua raw pointer
    ///
    /// # Safety
    /// Hàm này ghi trực tiếp lên bộ nhớ vật lý do Limine bàn giao và thiết lập bitmap.
    /// Chỉ được gọi duy nhất một lần trong quá trình khởi tạo kernel.
    /// Preconditions:
    /// - `entries_ptr` trỏ tới mảng con trỏ memory map hợp lệ do Limine bàn giao.
    /// - `entry_count` là số phần tử hợp lệ trong mảng đó.
    /// - `hhdm` là offset direct-map hợp lệ do Limine cung cấp.
    ///
    /// Postconditions:
    /// - Bitmap allocator được khởi tạo và frame chứa bitmap được đánh dấu allocated.
    /// - Chỉ các frame thuộc vùng Limine `usable` được đánh dấu có thể cấp phát.
    ///
    /// Memory safety assumptions:
    /// - Memory map và HHDM còn hợp lệ trong suốt giai đoạn kernel runtime hiện tại.
    /// - Hàm chỉ được gọi một lần trước khi allocator phục vụ subsystem khác.
    ///
    /// CPU state assumptions:
    /// - Paging của Limine vẫn active và HHDM mapping chưa bị thay đổi.
    pub unsafe fn init_raw(
        &mut self,
        entries_ptr: *const *const LimineMmapEntry,
        entry_count: usize,
        hhdm: u64,
    ) -> Result<MemoryStats, MemoryError> {
        // Tìm địa chỉ vật lý tối đa chỉ từ các vùng cần quản lý frame
        // (usable=0, reclaimable=5, kernel=6, framebuffer=7)
        // BỎ QUA: reserved=1, ACPI reclaimable=2, ACPI NVS=3, bad=4
        // Điều này tránh bitmap quá lớn do các vùng MMIO/reserved trên 4 GiB
        if entry_count == 0 || entries_ptr.is_null() {
            return Err(MemoryError::NoMemoryMap);
        }

        let mut max_usable_addr: u64 = 0;
        let mut total_usable_bytes: usize = 0;
        let mut total_usable_frames: usize = 0;
        for i in 0..entry_count {
            let entry = &**entries_ptr.add(i);
            if entry.entry_type == LIMINE_USABLE_MEMORY {
                let end_addr = entry.base + entry.length;
                if end_addr > max_usable_addr {
                    max_usable_addr = end_addr;
                }

                total_usable_bytes = total_usable_bytes.saturating_add(entry.length as usize);
                total_usable_frames =
                    total_usable_frames.saturating_add((entry.length as usize) / PAGE_SIZE);
            }
        }

        if max_usable_addr == 0 || total_usable_frames == 0 {
            return Err(MemoryError::OutOfFrames);
        }

        let total_frames = align_up(max_usable_addr as usize, PAGE_SIZE) / PAGE_SIZE;
        let bitmap_size = align_up(total_frames, BITS_PER_BYTE) / BITS_PER_BYTE;

        // Tìm vùng Usable đủ lớn để chứa bitmap
        let mut bitmap_phys_addr: u64 = u64::MAX;
        for i in 0..entry_count {
            let entry = &**entries_ptr.add(i);
            if entry.entry_type == LIMINE_USABLE_MEMORY && entry.length as usize >= bitmap_size {
                bitmap_phys_addr = entry.base;
                break;
            }
        }

        if bitmap_phys_addr == u64::MAX {
            return Err(MemoryError::BitmapTooLarge);
        }

        let bitmap_virt_addr = (bitmap_phys_addr + hhdm) as *mut u8;

        // Khởi tạo toàn bộ bitmap là 1 (tất cả frames Reserved)
        core::ptr::write_bytes(bitmap_virt_addr, 0xFF, bitmap_size);

        self.bitmap_raw = bitmap_virt_addr;
        self.entries_ptr = entries_ptr;
        self.entry_count = entry_count;
        self.total_frames = total_frames;
        self.total_usable_frames = total_usable_frames;
        self.allocated_frames = 0;
        self.last_searched_idx = 0;
        self.hhdm_offset = hhdm;
        self.initialized = true;

        // Đánh dấu các vùng usable là 0 (Free)
        let mut free_frames: usize = 0;
        for i in 0..entry_count {
            let entry = &**entries_ptr.add(i);
            if entry.entry_type == LIMINE_USABLE_MEMORY {
                let start_idx = (entry.base as usize) / PAGE_SIZE;
                let frame_count = (entry.length as usize) / PAGE_SIZE;
                for j in 0..frame_count {
                    let idx = start_idx + j;
                    if idx < total_frames {
                        self.set_bit(idx, false);
                        free_frames += 1;
                    }
                }
            }
        }

        // Đánh dấu các trang chứa bitmap là 1 (Used)
        let bitmap_start_idx = (bitmap_phys_addr as usize) / PAGE_SIZE;
        let bitmap_frame_count = align_up(bitmap_size, PAGE_SIZE) / PAGE_SIZE;
        for i in 0..bitmap_frame_count {
            self.set_bit(bitmap_start_idx + i, true);
        }
        // Trừ đi các frame bitmap (đã free rồi set lại used)
        let reclaimed_for_bitmap = bitmap_frame_count.min(free_frames);
        free_frames = free_frames.saturating_sub(reclaimed_for_bitmap);

        let allocated_frames = total_usable_frames.saturating_sub(free_frames);
        self.allocated_frames = allocated_frames;
        self.refresh_stats(total_usable_bytes, entry_count);
        Ok(self.stats)
    }

    /// Thiết lập trạng thái của một bit trong bitmap
    ///
    /// # Safety
    /// Hàm này ghi trực tiếp vào con trỏ bộ nhớ bitmap thô.
    /// Preconditions: `idx < self.total_frames` và `bitmap_raw` đã được khởi tạo.
    ///
    /// Postconditions: bit đại diện cho frame `idx` có giá trị `val`.
    ///
    /// Memory safety assumptions: vùng bitmap đủ lớn cho `self.total_frames` bit.
    ///
    /// CPU state assumptions: HHDM mapping còn cho phép ghi vùng bitmap.
    unsafe fn set_bit(&mut self, idx: usize, val: bool) {
        let byte_idx = idx / 8;
        let bit_idx = idx % 8;
        let ptr = self.bitmap_raw.add(byte_idx);
        let current = ptr.read();
        if val {
            ptr.write(current | (1 << bit_idx));
        } else {
            ptr.write(current & !(1 << bit_idx));
        }
    }

    /// Đọc trạng thái của một bit trong bitmap (true = Used, false = Free)
    ///
    /// # Safety
    /// Hàm này đọc trực tiếp từ con trỏ bộ nhớ bitmap thô.
    /// Preconditions: `idx < self.total_frames` và `bitmap_raw` đã được khởi tạo.
    ///
    /// Postconditions: không thay đổi trạng thái allocator.
    ///
    /// Memory safety assumptions: vùng bitmap đủ lớn cho `self.total_frames` bit.
    ///
    /// CPU state assumptions: HHDM mapping còn cho phép đọc vùng bitmap.
    unsafe fn get_bit(&self, idx: usize) -> bool {
        let byte_idx = idx / 8;
        let bit_idx = idx % 8;
        self.bitmap_raw.add(byte_idx).read() & (1 << bit_idx) != 0
    }

    /// Cấp phát một khung trang vật lý
    pub fn allocate(&mut self) -> Result<PhysFrame, MemoryError> {
        if !self.initialized || self.total_frames == 0 {
            return Err(MemoryError::NotInitialized);
        }

        let start = self.last_searched_idx;
        loop {
            // SAFETY: Đọc trực tiếp bộ nhớ bitmap thô đã khởi tạo an toàn
            unsafe {
                if !self.get_bit(self.last_searched_idx) {
                    self.set_bit(self.last_searched_idx, true);
                    let frame =
                        PhysFrame::from_start_address((self.last_searched_idx * PAGE_SIZE) as u64)?;
                    self.allocated_frames += 1;
                    self.refresh_stats(self.stats.total_usable, self.stats.region_count);
                    // Tối ưu hóa lần tìm kiếm tiếp theo
                    self.last_searched_idx = (self.last_searched_idx + 1) % self.total_frames;
                    return Ok(frame);
                }
            }

            self.last_searched_idx = (self.last_searched_idx + 1) % self.total_frames;
            if self.last_searched_idx == start {
                return Err(MemoryError::OutOfFrames);
            }
        }
    }

    /// Giải phóng một khung trang vật lý
    pub fn deallocate(&mut self, frame: PhysFrame) -> Result<(), MemoryError> {
        if !self.initialized || self.total_frames == 0 {
            return Err(MemoryError::NotInitialized);
        }

        let idx = (frame.start_address() as usize) / PAGE_SIZE;
        if idx >= self.total_frames {
            return Err(MemoryError::FrameOutOfRange);
        }
        if !self.is_usable_frame(idx) {
            return Err(MemoryError::FrameNotUsable);
        }
        // SAFETY: Ghi trực tiếp vào bitmap thô
        unsafe {
            if !self.get_bit(idx) {
                return Err(MemoryError::FrameAlreadyFree);
            }
            self.set_bit(idx, false);
        }
        self.allocated_frames = self.allocated_frames.saturating_sub(1);
        self.refresh_stats(self.stats.total_usable, self.stats.region_count);
        Ok(())
    }

    fn is_usable_frame(&self, idx: usize) -> bool {
        let frame_start = (idx * PAGE_SIZE) as u64;
        for i in 0..self.entry_count {
            // SAFETY: entries_ptr được Limine bàn giao và được lưu từ init_raw sau khi validate.
            let entry = unsafe { &**self.entries_ptr.add(i) };
            if entry.entry_type != LIMINE_USABLE_MEMORY {
                continue;
            }

            let frame_end = frame_start + PAGE_SIZE as u64;
            let region_end = entry.base + entry.length;
            if frame_start >= entry.base && frame_end <= region_end {
                return true;
            }
        }

        false
    }

    fn hhdm_offset(&self) -> Result<u64, MemoryError> {
        if self.initialized {
            Ok(self.hhdm_offset)
        } else {
            Err(MemoryError::NotInitialized)
        }
    }

    fn stats(&self) -> MemoryStats {
        self.stats
    }

    fn refresh_stats(&mut self, total_usable: usize, region_count: usize) {
        self.stats = MemoryStats {
            total_usable,
            total_usable_frames: self.total_usable_frames,
            allocated_frames: self.allocated_frames,
            free_frames: self
                .total_usable_frames
                .saturating_sub(self.allocated_frames),
            region_count,
        };
    }
}

/// Global Frame Allocator tĩnh bảo vệ bởi Mutex
pub static FRAME_ALLOCATOR: Mutex<BitmapFrameAllocator> = Mutex::new(BitmapFrameAllocator::empty());

/// Khởi tạo hệ thống quản lý bộ nhớ
pub fn init_memory() -> Result<MemoryStats, MemoryError> {
    let (entries_ptr, entry_count) =
        crate::boot::limine::memory_map_raw().ok_or(MemoryError::NoMemoryMap)?;
    let hhdm = crate::boot::limine::hhdm_offset().ok_or(MemoryError::NoHhdmOffset)?;

    let mut allocator = FRAME_ALLOCATOR.lock();
    // SAFETY: init_raw thiết lập bitmap thô, chỉ gọi duy nhất 1 lần lúc khởi động kernel
    let stats = unsafe { allocator.init_raw(entries_ptr, entry_count, hhdm)? };

    Ok(stats)
}

/// Wrapper cấp phát một khung trang vật lý
pub fn allocate_frame() -> Result<PhysFrame, MemoryError> {
    FRAME_ALLOCATOR.lock().allocate()
}

/// Wrapper giải phóng một khung trang vật lý
pub fn deallocate_frame(frame: PhysFrame) -> Result<(), MemoryError> {
    FRAME_ALLOCATOR.lock().deallocate(frame)
}

/// Trả về thống kê bộ nhớ hiện tại.
pub fn memory_stats() -> MemoryStats {
    FRAME_ALLOCATOR.lock().stats()
}

/// Trả về offset HHDM đã được xác thực khi khởi tạo memory.
pub fn hhdm_offset() -> Result<u64, MemoryError> {
    FRAME_ALLOCATOR.lock().hhdm_offset()
}

const fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phys_frame_rejects_unaligned_address() {
        assert!(matches!(
            PhysFrame::from_start_address(123),
            Err(MemoryError::UnalignedFrame)
        ));
    }

    #[test]
    fn align_up_rounds_to_next_boundary() {
        assert_eq!(align_up(0, PAGE_SIZE), 0);
        assert_eq!(align_up(1, PAGE_SIZE), PAGE_SIZE);
        assert_eq!(align_up(PAGE_SIZE, PAGE_SIZE), PAGE_SIZE);
    }
}
