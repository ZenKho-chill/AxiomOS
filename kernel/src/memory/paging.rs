use crate::memory::frame::{allocate_frame, MemoryError, PAGE_SIZE};

#[repr(C, align(4096))]
struct PageTable {
    entries: [u64; 512],
}

impl PageTable {
    fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            *entry = 0;
        }
    }
}

const PHYSICAL_ADDRESS_MASK: u64 = 0x000F_FFFF_FFFF_F000;
const FLAG_PRESENT: u64 = 1 << 0;
const FLAG_WRITABLE: u64 = 1 << 1;

/// Thực hiện ánh xạ một trang ảo sang một khung trang vật lý
///
/// # Safety
/// Hàm này can thiệp trực tiếp vào cấu trúc Page Table của CPU và thay đổi sơ đồ ánh xạ ảo-vật lý.
/// Việc ánh xạ không chính xác có thể dẫn đến General Protection Fault hoặc Page Fault sập hệ thống.
/// Preconditions:
/// - `virt_addr` và `phys_addr` đại diện cho trang 4 KiB hợp lệ.
/// - `hhdm` là offset direct-map hợp lệ để truy cập bảng trang vật lý.
///
/// Postconditions:
/// - `virt_addr` được map tới `phys_addr` với `flags | PRESENT`.
/// - TLB entry tương ứng được invalidate.
///
/// Memory safety assumptions:
/// - Caller bảo đảm mapping mới không ghi đè vùng đang được sở hữu bởi subsystem khác.
///
/// CPU state assumptions:
/// - CPU đang chạy ở Ring 0, paging active và CR3 trỏ tới bảng trang chỉnh sửa được.
pub unsafe fn map_page(
    virt_addr: u64,
    phys_addr: u64,
    flags: u64,
    hhdm: u64,
) -> Result<(), MemoryError> {
    // Căn lề địa chỉ ảo và vật lý
    let virt_addr = virt_addr & !(PAGE_SIZE as u64 - 1);
    let phys_addr = phys_addr & !(PAGE_SIZE as u64 - 1);

    // Trích xuất các chỉ mục
    let l4_idx = ((virt_addr >> 39) & 0x1FF) as usize;
    let l3_idx = ((virt_addr >> 30) & 0x1FF) as usize;
    let l2_idx = ((virt_addr >> 21) & 0x1FF) as usize;
    let l1_idx = ((virt_addr >> 12) & 0x1FF) as usize;

    // Đọc CR3 để lấy địa chỉ vật lý của bảng L4
    let mut cr3: u64;
    core::arch::asm!("mov {}, cr3", out(reg) cr3);
    let l4_table_phys = cr3 & PHYSICAL_ADDRESS_MASK;
    let l4_table = &mut *((l4_table_phys + hhdm) as *mut PageTable);

    // Tra cứu L4 -> L3
    let l3_table_phys = get_or_create_next_table(&mut l4_table.entries[l4_idx], hhdm)?;
    let l3_table = &mut *((l3_table_phys + hhdm) as *mut PageTable);

    // Tra cứu L3 -> L2
    let l2_table_phys = get_or_create_next_table(&mut l3_table.entries[l3_idx], hhdm)?;
    let l2_table = &mut *((l2_table_phys + hhdm) as *mut PageTable);

    // Tra cứu L2 -> L1
    let l1_table_phys = get_or_create_next_table(&mut l2_table.entries[l2_idx], hhdm)?;
    let l1_table = &mut *((l1_table_phys + hhdm) as *mut PageTable);

    // Ánh xạ L1 -> Physical Frame đích
    l1_table.entries[l1_idx] = phys_addr | flags | FLAG_PRESENT;

    // Invalidate TLB cho địa chỉ ảo vừa map
    core::arch::asm!("invlpg [{}]", in(reg) virt_addr);

    Ok(())
}

/// Lấy địa chỉ vật lý của bảng trang cấp dưới từ entry hiện tại, tạo mới nếu chưa tồn tại
///
/// # Safety
/// Hàm này tự động cấp phát và ghi đè dữ liệu thô của bảng trang mới.
/// Preconditions:
/// - `entry` thuộc bảng trang hiện tại và có thể ghi.
/// - `hhdm` là offset direct-map hợp lệ.
///
/// Postconditions:
/// - Trả về địa chỉ vật lý của bảng trang cấp dưới đã tồn tại hoặc vừa tạo.
///
/// Memory safety assumptions:
/// - Frame cấp phát mới chưa được dùng cho mục đích khác và có thể zero-init.
///
/// CPU state assumptions:
/// - Paging active và CPU đang chạy ở Ring 0.
unsafe fn get_or_create_next_table(entry: &mut u64, hhdm: u64) -> Result<u64, MemoryError> {
    if (*entry & FLAG_PRESENT) == 0 {
        // Chưa tồn tại, tiến hành cấp phát 1 frame vật lý mới cho bảng trang
        let new_frame_phys = allocate_frame()?.start_address();
        let new_table_virt = (new_frame_phys + hhdm) as *mut PageTable;

        // Xóa sạch dữ liệu rác trong bảng trang mới
        (*new_table_virt).clear();

        // Ghi địa chỉ vật lý vào entry hiện tại với các cờ Present + Writable
        *entry = new_frame_phys | FLAG_PRESENT | FLAG_WRITABLE;
        Ok(new_frame_phys)
    } else {
        Ok(*entry & PHYSICAL_ADDRESS_MASK)
    }
}
