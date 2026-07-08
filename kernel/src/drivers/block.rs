//! Lớp trừu tượng hóa thiết bị khối (Block Device Abstraction) và Mock RAM Disk

use crate::utils::sync::SpinlockIrqSave;

/// Kích thước mặc định của một sector trên thiết bị khối (512 bytes)
pub const SECTOR_SIZE: usize = 512;

/// Các lỗi có thể xảy ra khi tương tác với thiết bị khối
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockError {
    OutOfBounds,
    DeviceFault,
}

/// Giao diện trừu tượng hóa thiết bị khối trong AxiomOS
pub trait BlockDevice {
    /// Đọc một sector kích thước 512 bytes từ LBA (Logical Block Address) được chỉ định.
    fn read_sector(&self, lba: u64, buf: &mut [u8; SECTOR_SIZE]) -> Result<(), BlockError>;

    /// Trả về tổng số sectors của thiết bị khối này.
    fn total_sectors(&self) -> u64;
}

/// Trình giả lập RAM Disk lưu dữ liệu thô trong một vùng nhớ byte tĩnh
#[derive(Debug, Clone, Copy)]
pub struct RamDisk {
    data: &'static [u8],
}

impl RamDisk {
    /// Khởi tạo RamDisk từ một tham chiếu byte tĩnh
    pub const fn new(data: &'static [u8]) -> Self {
        Self { data }
    }
}

impl BlockDevice for RamDisk {
    fn read_sector(&self, lba: u64, buf: &mut [u8; SECTOR_SIZE]) -> Result<(), BlockError> {
        let offset = lba as usize * SECTOR_SIZE;
        if offset + SECTOR_SIZE > self.data.len() {
            return Err(BlockError::OutOfBounds);
        }

        // Copy dữ liệu thô vào buffer
        buf.copy_from_slice(&self.data[offset..offset + SECTOR_SIZE]);
        Ok(())
    }

    fn total_sectors(&self) -> u64 {
        (self.data.len() / SECTOR_SIZE) as u64
    }
}

/// Thiết bị khối hệ thống tĩnh (được truy cập thông qua SpinlockIrqSave)
pub static SYSTEM_BLOCK_DEVICE: SpinlockIrqSave<Option<RamDisk>> = SpinlockIrqSave::new(None);

/// Khởi tạo và đăng ký thiết bị khối hệ thống
pub fn register_system_block_device(disk: RamDisk) {
    let mut guard = SYSTEM_BLOCK_DEVICE.lock();
    *guard = Some(disk);
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;
    use alloc::vec;

    #[test]
    fn test_ramdisk_read_success() {
        let mut raw_data = vec![0u8; 2048];
        // Điền dữ liệu giả lập cho sector 1
        for i in 0..512 {
            raw_data[512 + i] = (i % 256) as u8;
        }
        let static_data = Box::leak(raw_data.into_boxed_slice());

        let disk = RamDisk::new(static_data);
        assert_eq!(disk.total_sectors(), 4);

        let mut buf = [0u8; 512];
        let res = disk.read_sector(1, &mut buf);
        assert!(res.is_ok());

        // Kiểm tra xem dữ liệu đọc ra có khớp không
        for i in 0..512 {
            assert_eq!(buf[i], (i % 256) as u8);
        }
    }

    #[test]
    fn test_ramdisk_out_of_bounds() {
        let raw_data = vec![0u8; 1024]; // 2 sectors
        let static_data = Box::leak(raw_data.into_boxed_slice());
        let disk = RamDisk::new(static_data);
        let mut buf = [0u8; 512];

        // Sector 2 vượt ngoài biên (chỉ có sector 0 và 1)
        let res = disk.read_sector(2, &mut buf);
        assert_eq!(res, Err(BlockError::OutOfBounds));
    }
}
