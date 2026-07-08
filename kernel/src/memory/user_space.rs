//! Quản lý Không gian địa chỉ người dùng (Userspace Address Space)
//!
//! Mô-đun này cung cấp trừu tượng `UserAddressSpace` đại diện cho bảng trang L4 riêng
//! của một tiến trình userspace, quản lý việc ánh xạ mã lệnh và giải phóng bộ nhớ.

use crate::memory::frame::{hhdm_offset, MemoryError};
use crate::memory::paging::{create_user_page_table, map_user_page, FLAG_USER, FLAG_WRITABLE};
use crate::process::elf::LoadedImage;

/// Đại diện cho Không gian địa chỉ bộ nhớ ảo của một tiến trình người dùng
pub struct UserAddressSpace {
    /// Địa chỉ vật lý của bảng trang L4 của tiến trình
    l4_table_phys: u64,
    /// Ảnh chương trình đã nạp vào Address Space này (nếu có)
    loaded_image: Option<LoadedImage>,
}

impl UserAddressSpace {
    /// Khởi tạo một không gian địa chỉ userspace mới với bảng trang ảo riêng
    pub fn new() -> Result<Self, MemoryError> {
        let hhdm = hhdm_offset()?;
        let l4_table_phys = create_user_page_table(hhdm)?;
        Ok(Self {
            l4_table_phys,
            loaded_image: None,
        })
    }

    /// Trả về địa chỉ vật lý L4 của bảng trang này
    pub fn l4_table_phys(&self) -> u64 {
        self.l4_table_phys
    }

    /// Ánh xạ ảnh chương trình (LoadedImage) vào không gian địa chỉ này
    ///
    /// # Safety
    /// Hàm này chỉnh sửa cấu trúc bảng trang của tiến trình. Cần đảm bảo các frame vật lý
    /// trong `LoadedImage` không bị chia sẻ writable trái phép với tiến trình khác.
    pub unsafe fn load_image(&mut self, image: LoadedImage) -> Result<(), MemoryError> {
        let hhdm = hhdm_offset()?;

        for segment in &image.segments {
            for (i, &phys_frame) in segment.phys_frames.iter().enumerate() {
                let page_vaddr = segment.virt_start + (i as u64 * 4096);

                // Mọi trang trong userspace phải bật cờ USER
                let mut flags = FLAG_USER;
                // Nếu segment có quyền ghi (PF_W = 2), bật cờ WRITABLE
                if (segment.flags & 2) != 0 {
                    flags |= FLAG_WRITABLE;
                }

                // Thực hiện ánh xạ ảo-vật lý với cờ is_user = true
                map_user_page(
                    self.l4_table_phys,
                    page_vaddr,
                    phys_frame,
                    flags,
                    hhdm,
                    true,
                )?;
            }
        }

        self.loaded_image = Some(image);
        Ok(())
    }
}

impl Drop for UserAddressSpace {
    fn drop(&mut self) {
        // Giải phóng các frames chứa ảnh chương trình ELF đã nạp
        if let Some(ref image) = self.loaded_image {
            image.deallocate();
        }

        // Giải phóng L4 table frame vật lý
        if let Ok(frame) = crate::memory::frame::PhysFrame::from_start_address(self.l4_table_phys) {
            let _ = crate::memory::frame::deallocate_frame(frame);
        }
    }
}

/// Chạy chẩn đoán (diagnostics) kiểm thử Không gian địa chỉ người dùng
pub fn run_userspace_as_diagnostics() {
    crate::serial_println!("[AXIOMOS MEMORY] Chạy chẩn đoán Không gian địa chỉ người dùng...");

    // 1. Khởi tạo một Address Space mới
    let mut user_as = UserAddressSpace::new().expect("Lỗi: Không tạo được UserAddressSpace");
    assert!(user_as.l4_table_phys() > 0);

    // 2. Tạo một LoadedImage giả lập
    use crate::process::elf::{LoadedImage, LoadedSegment};
    let frame1 = crate::memory::frame::allocate_frame().expect("Lỗi: Không cấp phát được frame");
    let frame2 = crate::memory::frame::allocate_frame().expect("Lỗi: Không cấp phát được frame");

    let mock_image = LoadedImage {
        entry_point: 0x401000,
        segments: alloc::vec![
            LoadedSegment {
                virt_start: 0x400000,
                mem_size: 4096,
                flags: 5, // RX (Read + Execute)
                phys_frames: alloc::vec![frame1.start_address()],
            },
            LoadedSegment {
                virt_start: 0x800000,
                mem_size: 4096,
                flags: 6, // RW (Read + Write)
                phys_frames: alloc::vec![frame2.start_address()],
            }
        ],
    };

    // 3. Ánh xạ LoadedImage giả lập vào Address Space
    // SAFETY: Các frame mock_image vừa được cấp phát độc quyền cho chẩn đoán
    unsafe {
        user_as
            .load_image(mock_image)
            .expect("Lỗi: Ánh xạ LoadedImage thất bại");
    }

    // 4. Kiểm chứng các entries bảng trang ảo của userspace thông qua HHDM
    let hhdm = crate::memory::frame::hhdm_offset().expect("Lỗi: Không lấy được HHDM");

    // SAFETY: Chúng ta truy cập bảng trang ảo vừa tạo của userspace qua HHDM
    unsafe {
        let l4_ptr = (user_as.l4_table_phys() + hhdm) as *const u64;

        // Địa chỉ ảo 0x400000 có:
        // l4_idx = (0x400000 >> 39) & 0x1FF = 0
        let l4_entry = l4_ptr.read();
        assert!(l4_entry & 1 != 0, "Lỗi: L4 entry 0 phải có cờ PRESENT");
        assert!(l4_entry & 4 != 0, "Lỗi: L4 entry 0 phải có cờ USER");

        // Sao chép vùng nhớ kernel (entry 256..512) phải hoạt động
        let kernel_l4_entry = l4_ptr.add(256).read();
        // Kiểm tra entry 256 (HHDM offset)
        assert!(
            kernel_l4_entry & 1 != 0,
            "Lỗi: Entry kernel 256 của bảng userspace phải PRESENT"
        );
    }

    // 5. Drop UserAddressSpace giải phóng an toàn bộ nhớ
    drop(user_as);

    crate::serial_println!(
        "[AXIOMOS MEMORY] Chạy chẩn đoán Không gian địa chỉ người dùng: THÀNH CÔNG"
    );
}
