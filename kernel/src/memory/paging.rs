use crate::memory::frame::{allocate_frame, MemoryError, PAGE_SIZE};
use crate::memory::heap::{HEAP_SIZE, HEAP_START};

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

/// Các cờ phân quyền bảng trang ảo
pub const FLAG_PRESENT: u64 = 1 << 0;
pub const FLAG_WRITABLE: u64 = 1 << 1;
pub const FLAG_USER: u64 = 1 << 2;

/// Thực hiện ánh xạ một trang ảo sang một khung trang vật lý trong bảng trang hiện tại (CR3)
///
/// # Safety
/// Hàm này can thiệp trực tiếp vào cấu trúc Page Table của CPU và thay đổi sơ đồ ánh xạ ảo-vật lý.
/// Việc ánh xạ không chính xác có thể dẫn đến General Protection Fault hoặc Page Fault sập hệ thống.
pub unsafe fn map_page(
    virt_addr: u64,
    phys_addr: u64,
    flags: u64,
    hhdm: u64,
) -> Result<(), MemoryError> {
    // Đọc CR3 để lấy địa chỉ vật lý của bảng L4 hiện tại
    let mut cr3: u64;
    core::arch::asm!("mov {}, cr3", out(reg) cr3);
    let l4_table_phys = cr3 & PHYSICAL_ADDRESS_MASK;

    map_user_page(l4_table_phys, virt_addr, phys_addr, flags, hhdm, false)
}

/// Tạo một bảng trang L4 mới dành cho userspace, sao chép không gian kernel để cô lập
pub fn create_user_page_table(hhdm: u64) -> Result<u64, MemoryError> {
    // Cấp phát 1 frame vật lý mới cho bảng L4 của userspace
    let new_frame = allocate_frame()?;
    let new_l4_phys = new_frame.start_address();
    let new_l4_virt = (new_l4_phys + hhdm) as *mut PageTable;

    // SAFETY: Chúng ta sở hữu độc quyền frame vừa được cấp phát
    unsafe {
        (*new_l4_virt).clear();

        // Lấy địa chỉ vật lý L4 hiện tại (bảng trang kernel)
        let mut cr3: u64;
        core::arch::asm!("mov {}, cr3", out(reg) cr3);
        let curr_l4_phys = cr3 & PHYSICAL_ADDRESS_MASK;
        let curr_l4_virt = (curr_l4_phys + hhdm) as *const PageTable;

        // Sao chép các entries từ 256 đến 511 (nửa trên của address space dành cho Kernel)
        // Điều này đảm bảo khi chuyển sang Ring 3, CPU vẫn có thể tra cứu và thực thi mã kernel
        // khi có ngắt hoặc syscall xảy ra. Các entries userspace (0..256) được giữ bằng 0.
        for i in 256..512 {
            (*new_l4_virt).entries[i] = (*curr_l4_virt).entries[i];
        }

        // Kernel heap hiện nằm ở lower-half nhưng vẫn là supervisor-only mapping.
        // Syscall handler cần truy cập VFS/alloc object trên heap khi CR3 đang là userspace page table.
        copy_supervisor_l4_range(
            &mut *new_l4_virt,
            &*curr_l4_virt,
            HEAP_START as u64,
            HEAP_SIZE as u64,
        );
    }

    Ok(new_l4_phys)
}

fn copy_supervisor_l4_range(target: &mut PageTable, source: &PageTable, start_addr: u64, len: u64) {
    if len == 0 {
        return;
    }

    let end_addr = start_addr.saturating_add(len - 1);
    let start_idx = l4_index(start_addr);
    let end_idx = l4_index(end_addr);

    for index in start_idx..=end_idx {
        target.entries[index] = source.entries[index] & !FLAG_USER;
    }
}

fn l4_index(virt_addr: u64) -> usize {
    ((virt_addr >> 39) & 0x1FF) as usize
}

/// Ánh xạ một trang ảo vào một bảng trang L4 cụ thể (sử dụng địa chỉ vật lý L4)
///
/// # Safety
/// Hàm này thay đổi trực tiếp cấu trúc bảng trang được chỉ định. Nếu bảng trang đang hoạt động,
/// nó sẽ thực hiện invalidate TLB để cập nhật ánh xạ ảo.
pub unsafe fn map_user_page(
    l4_table_phys: u64,
    virt_addr: u64,
    phys_addr: u64,
    flags: u64,
    hhdm: u64,
    is_user: bool,
) -> Result<(), MemoryError> {
    // Căn lề địa chỉ ảo và vật lý
    let virt_addr = virt_addr & !(PAGE_SIZE as u64 - 1);
    let phys_addr = phys_addr & !(PAGE_SIZE as u64 - 1);

    // Trích xuất các chỉ mục
    let l4_idx = ((virt_addr >> 39) & 0x1FF) as usize;
    let l3_idx = ((virt_addr >> 30) & 0x1FF) as usize;
    let l2_idx = ((virt_addr >> 21) & 0x1FF) as usize;
    let l1_idx = ((virt_addr >> 12) & 0x1FF) as usize;

    let l4_table = &mut *((l4_table_phys + hhdm) as *mut PageTable);

    // Tra cứu L4 -> L3
    let l3_table_phys = get_or_create_next_table(&mut l4_table.entries[l4_idx], hhdm, is_user)?;
    let l3_table = &mut *((l3_table_phys + hhdm) as *mut PageTable);

    // Tra cứu L3 -> L2
    let l2_table_phys = get_or_create_next_table(&mut l3_table.entries[l3_idx], hhdm, is_user)?;
    let l2_table = &mut *((l2_table_phys + hhdm) as *mut PageTable);

    // Tra cứu L2 -> L1
    let l1_table_phys = get_or_create_next_table(&mut l2_table.entries[l2_idx], hhdm, is_user)?;
    let l1_table = &mut *((l1_table_phys + hhdm) as *mut PageTable);

    // Ánh xạ L1 -> Physical Frame đích
    l1_table.entries[l1_idx] = phys_addr | flags | FLAG_PRESENT;

    // Kiểm tra xem bảng trang được sửa đổi có đang active (CR3) hay không.
    // Nếu có, thực hiện invalidate TLB cho trang ảo vừa map.
    let mut cr3: u64;
    core::arch::asm!("mov {}, cr3", out(reg) cr3);
    if (cr3 & PHYSICAL_ADDRESS_MASK) == l4_table_phys {
        core::arch::asm!("invlpg [{}]", in(reg) virt_addr);
    }

    Ok(())
}

/// Lấy địa chỉ vật lý của bảng trang cấp dưới từ entry hiện tại, tạo mới nếu chưa tồn tại
///
/// # Safety
/// Hàm này tự động cấp phát và ghi đè dữ liệu thô của bảng trang mới.
unsafe fn get_or_create_next_table(
    entry: &mut u64,
    hhdm: u64,
    is_user: bool,
) -> Result<u64, MemoryError> {
    if (*entry & FLAG_PRESENT) == 0 {
        // Chưa tồn tại, tiến hành cấp phát 1 frame vật lý mới cho bảng trang
        let new_frame_phys = allocate_frame()?.start_address();
        let new_table_virt = (new_frame_phys + hhdm) as *mut PageTable;

        // Xóa sạch dữ liệu rác trong bảng trang mới
        (*new_table_virt).clear();

        // Ghi địa chỉ vật lý vào entry hiện tại với các cờ Present + Writable.
        // Đối với không gian userspace, bắt buộc phải set cờ USER trên các entry trung gian.
        let mut flags = FLAG_PRESENT | FLAG_WRITABLE;
        if is_user {
            flags |= FLAG_USER;
        }

        *entry = new_frame_phys | flags;
        Ok(new_frame_phys)
    } else {
        // Nếu entry đã tồn tại và chúng ta đang map cho user, đảm bảo cờ USER được set
        if is_user && (*entry & FLAG_USER) == 0 {
            *entry |= FLAG_USER;
        }
        Ok(*entry & PHYSICAL_ADDRESS_MASK)
    }
}
