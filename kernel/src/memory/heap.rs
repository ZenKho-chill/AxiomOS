use crate::memory::frame::{allocate_frame, MemoryError, PAGE_SIZE};
use crate::memory::paging::map_page;
#[cfg(not(test))]
use linked_list_allocator::LockedHeap;

#[cfg(not(test))]
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 8 * 1024 * 1024; // 8 MiB heap

const FLAG_WRITABLE: u64 = 1 << 1;

/// Khởi tạo và ánh xạ bộ nhớ Heap ảo cho Kernel
///
/// # Safety
/// Hàm này tự động ánh xạ vùng ảo sang vật lý và kích hoạt heap allocator toàn cục.
/// Chỉ được gọi một lần trong quá trình khởi chạy kernel.
/// Preconditions:
/// - Frame allocator đã được khởi tạo và `hhdm` là offset HHDM hợp lệ.
/// - Vùng `HEAP_START..HEAP_START + HEAP_SIZE` chưa được map cho mục đích khác.
///
/// Postconditions:
/// - Toàn bộ vùng heap ảo được map writable và global allocator sẵn sàng.
///
/// Memory safety assumptions:
/// - Mỗi frame vật lý cấp phát cho heap là duy nhất và không bị subsystem khác sở hữu.
///
/// CPU state assumptions:
/// - Paging đang active và bảng trang hiện tại có thể chỉnh sửa qua HHDM.
pub unsafe fn init_heap(hhdm: u64) -> Result<(), MemoryError> {
    let page_count = HEAP_SIZE / PAGE_SIZE;

    // Duyệt qua từng trang ảo trong vùng heap và thực hiện ánh xạ
    for i in 0..page_count {
        let virt_page_addr = (HEAP_START + i * PAGE_SIZE) as u64;
        let phys_frame_addr = allocate_frame()?.start_address();

        // Ánh xạ trang ảo sang khung trang vật lý
        map_page(virt_page_addr, phys_frame_addr, FLAG_WRITABLE, hhdm)?;
    }

    // Khởi tạo Heap Allocator toàn cục với vùng nhớ ảo vừa map
    #[cfg(not(test))]
    init_global_allocator();

    Ok(())
}

#[cfg(not(test))]
fn init_global_allocator() {
    // SAFETY: `init_heap` đã map toàn bộ vùng `HEAP_START..HEAP_START + HEAP_SIZE`
    // trước khi gọi helper này, và global allocator chỉ được init một lần trong boot path.
    unsafe {
        ALLOCATOR.lock().init(HEAP_START as *mut u8, HEAP_SIZE);
    }
}
