//! Trình đọc FAT32 read-only tối giản.

use super::vfs;
use crate::drivers::block::{BlockDevice, BlockError, SECTOR_SIZE};

const FAT32_END_OF_CHAIN: u32 = 0x0FFF_FFF8;
const FAT32_BAD_CLUSTER: u32 = 0x0FFF_FFF7;
const DIRECTORY_ENTRY_SIZE: usize = 32;
const ATTR_LONG_FILE_NAME: u8 = 0x0F;
const ATTR_DIRECTORY: u8 = 0x10;
const ATTR_VOLUME_ID: u8 = 0x08;

/// Lỗi khi mount hoặc đọc hệ thống tệp tin FAT32.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsError {
    Block(BlockError),
    InvalidBootSector,
    UnsupportedSectorSize,
    UnsupportedFatLayout,
    InvalidCluster,
    CorruptFat,
    NotFound,
    BufferTooSmall,
    InvalidPath,
    NotAFile,
    NotADirectory,
    SinkFull,
}

impl From<BlockError> for FsError {
    fn from(error: BlockError) -> Self {
        Self::Block(error)
    }
}

/// Loại entry thư mục được FAT32 parser trả về.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    RegularFile,
    Directory,
}

/// Entry thư mục FAT32 đã được parse từ định dạng 8.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectoryEntry {
    pub name: [u8; 11],
    pub file_type: FileType,
    pub first_cluster: u32,
    pub size: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Fat32Node {
    file_type: FileType,
    first_cluster: u32,
    size: u32,
}

/// Sink nhận entry khi liệt kê thư mục mà không cần cấp phát động.
pub trait DirEntrySink {
    fn push(&mut self, entry: DirectoryEntry) -> Result<(), FsError>;
}

/// Volume FAT32 read-only gắn với một thiết bị khối.
pub struct Fat32Volume<'a, D: BlockDevice + ?Sized> {
    device: &'a D,
    fat_start_lba: u64,
    first_data_lba: u64,
    sectors_per_cluster: u8,
    fat_size_sectors: u32,
    root_cluster: u32,
    total_clusters: u32,
}

/// Mount một FAT32 volume read-only từ thiết bị khối đã có.
pub fn mount_fat32<D: BlockDevice + ?Sized>(device: &D) -> Result<Fat32Volume<'_, D>, FsError> {
    Fat32Volume::mount(device)
}

impl<'a, D: BlockDevice + ?Sized> Fat32Volume<'a, D> {
    /// Mount volume và kiểm tra các metadata FAT32 tối thiểu.
    pub fn mount(device: &'a D) -> Result<Self, FsError> {
        let mut sector = [0u8; SECTOR_SIZE];
        device.read_sector(0, &mut sector)?;

        if sector[510] != 0x55 || sector[511] != 0xAA {
            return Err(FsError::InvalidBootSector);
        }

        let bytes_per_sector = le_u16(&sector[11..13]);
        if bytes_per_sector as usize != SECTOR_SIZE {
            return Err(FsError::UnsupportedSectorSize);
        }

        let sectors_per_cluster = sector[13];
        let reserved_sectors = le_u16(&sector[14..16]);
        let fat_count = sector[16];
        let root_entry_count = le_u16(&sector[17..19]);
        let total_sectors_16 = le_u16(&sector[19..21]);
        let fat_size_16 = le_u16(&sector[22..24]);
        let total_sectors_32 = le_u32(&sector[32..36]);
        let fat_size_sectors = le_u32(&sector[36..40]);
        let root_cluster = le_u32(&sector[44..48]);

        if sectors_per_cluster == 0
            || reserved_sectors == 0
            || fat_count == 0
            || root_entry_count != 0
            || fat_size_16 != 0
            || fat_size_sectors == 0
            || root_cluster < 2
        {
            return Err(FsError::UnsupportedFatLayout);
        }

        let total_sectors = if total_sectors_16 != 0 {
            u32::from(total_sectors_16)
        } else {
            total_sectors_32
        };
        if total_sectors == 0 || u64::from(total_sectors) > device.total_sectors() {
            return Err(FsError::InvalidBootSector);
        }

        let fat_span = u64::from(fat_count)
            .checked_mul(u64::from(fat_size_sectors))
            .ok_or(FsError::InvalidBootSector)?;
        let first_data_lba = u64::from(reserved_sectors)
            .checked_add(fat_span)
            .ok_or(FsError::InvalidBootSector)?;
        if first_data_lba >= u64::from(total_sectors) {
            return Err(FsError::InvalidBootSector);
        }

        let data_sectors = u64::from(total_sectors) - first_data_lba;
        let total_clusters = data_sectors / u64::from(sectors_per_cluster);
        if total_clusters == 0 || total_clusters > u64::from(u32::MAX - 2) {
            return Err(FsError::UnsupportedFatLayout);
        }

        let volume = Self {
            device,
            fat_start_lba: u64::from(reserved_sectors),
            first_data_lba,
            sectors_per_cluster,
            fat_size_sectors,
            root_cluster,
            total_clusters: total_clusters as u32,
        };
        volume.validate_cluster(root_cluster)?;
        Ok(volume)
    }

    /// Đọc file 8.3 ở thư mục root vào buffer caller cung cấp.
    pub fn read_file(&self, path: &str, buffer: &mut [u8]) -> Result<usize, FsError> {
        let node = self.open(path)?;
        let file_size = node.size as usize;
        if buffer.len() < file_size {
            return Err(FsError::BufferTooSmall);
        }

        self.read_node_at(&node, 0, &mut buffer[..file_size])
    }

    fn open(&self, path: &str) -> Result<Fat32Node, FsError> {
        let entry = self.find_root_entry(path)?;
        Ok(Fat32Node {
            file_type: entry.file_type,
            first_cluster: entry.first_cluster,
            size: entry.size,
        })
    }

    fn read_node_at(
        &self,
        node: &Fat32Node,
        offset: u64,
        buffer: &mut [u8],
    ) -> Result<usize, FsError> {
        if node.file_type != FileType::RegularFile {
            return Err(FsError::NotAFile);
        }
        if buffer.is_empty() || offset >= u64::from(node.size) {
            return Ok(0);
        }
        if node.size == 0 {
            return Ok(0);
        }

        self.validate_cluster(node.first_cluster)?;

        let file_remaining = u64::from(node.size) - offset;
        let target_len = file_remaining.min(buffer.len() as u64) as usize;
        let cluster_size = usize::from(self.sectors_per_cluster) * SECTOR_SIZE;
        let mut skip_in_cluster = offset as usize;
        let mut current_cluster = node.first_cluster;
        let mut clusters_seen = 0u32;
        let mut bytes_read = 0usize;

        while skip_in_cluster >= cluster_size {
            self.validate_cluster(current_cluster)?;
            clusters_seen = clusters_seen.checked_add(1).ok_or(FsError::CorruptFat)?;
            if clusters_seen > self.total_clusters {
                return Err(FsError::CorruptFat);
            }

            let next = self.read_next_data_cluster(current_cluster)?;
            current_cluster = next;
            skip_in_cluster -= cluster_size;
        }

        while bytes_read < target_len {
            self.validate_cluster(current_cluster)?;
            clusters_seen = clusters_seen.checked_add(1).ok_or(FsError::CorruptFat)?;
            if clusters_seen > self.total_clusters {
                return Err(FsError::CorruptFat);
            }

            let cluster_lba = self.cluster_to_lba(current_cluster)?;
            for sector_index in 0..self.sectors_per_cluster {
                if bytes_read == target_len {
                    break;
                }
                if skip_in_cluster >= SECTOR_SIZE {
                    skip_in_cluster -= SECTOR_SIZE;
                    continue;
                }

                let sector_offset = skip_in_cluster;
                skip_in_cluster = 0;
                if sector_offset >= SECTOR_SIZE {
                    break;
                }

                let mut sector = [0u8; SECTOR_SIZE];
                self.device
                    .read_sector(cluster_lba + u64::from(sector_index), &mut sector)?;

                let copy_len = (target_len - bytes_read).min(SECTOR_SIZE - sector_offset);
                buffer[bytes_read..bytes_read + copy_len]
                    .copy_from_slice(&sector[sector_offset..sector_offset + copy_len]);
                bytes_read += copy_len;
            }

            if bytes_read < target_len {
                current_cluster = self.read_next_data_cluster(current_cluster)?;
            }
        }

        Ok(bytes_read)
    }

    /// Liệt kê thư mục root FAT32 theo thứ tự entry trên disk.
    pub fn list_dir(&self, path: &str, sink: &mut dyn DirEntrySink) -> Result<(), FsError> {
        if path != "/" {
            return Err(FsError::InvalidPath);
        }

        self.visit_root_dir(|entry| {
            sink.push(entry)?;
            Ok(true)
        })
    }

    fn find_root_entry(&self, path: &str) -> Result<DirectoryEntry, FsError> {
        let target_name = format_short_name(path)?;
        let mut found = None;
        self.visit_root_dir(|entry| {
            if entry.name == target_name {
                found = Some(entry);
                Ok(false)
            } else {
                Ok(true)
            }
        })?;

        found.ok_or(FsError::NotFound)
    }

    fn visit_root_dir<F>(&self, mut visit: F) -> Result<(), FsError>
    where
        F: FnMut(DirectoryEntry) -> Result<bool, FsError>,
    {
        let mut current_cluster = self.root_cluster;
        let mut clusters_seen = 0u32;

        loop {
            self.validate_cluster(current_cluster)?;
            clusters_seen = clusters_seen.checked_add(1).ok_or(FsError::CorruptFat)?;
            if clusters_seen > self.total_clusters {
                return Err(FsError::CorruptFat);
            }

            let cluster_lba = self.cluster_to_lba(current_cluster)?;
            for sector_index in 0..self.sectors_per_cluster {
                let mut sector = [0u8; SECTOR_SIZE];
                self.device
                    .read_sector(cluster_lba + u64::from(sector_index), &mut sector)?;

                for raw_entry in sector.chunks_exact(DIRECTORY_ENTRY_SIZE) {
                    match raw_entry[0] {
                        0x00 => return Ok(()),
                        0xE5 => continue,
                        _ => {}
                    }

                    let attributes = raw_entry[11];
                    if attributes == ATTR_LONG_FILE_NAME || attributes & ATTR_VOLUME_ID != 0 {
                        continue;
                    }

                    let first_cluster = (u32::from(le_u16(&raw_entry[20..22])) << 16)
                        | u32::from(le_u16(&raw_entry[26..28]));
                    let file_type = if attributes & ATTR_DIRECTORY != 0 {
                        FileType::Directory
                    } else {
                        FileType::RegularFile
                    };

                    let mut name = [0u8; 11];
                    name.copy_from_slice(&raw_entry[..11]);
                    let entry = DirectoryEntry {
                        name,
                        file_type,
                        first_cluster,
                        size: le_u32(&raw_entry[28..32]),
                    };

                    if !visit(entry)? {
                        return Ok(());
                    }
                }
            }

            let next = self.read_fat_entry(current_cluster)?;
            if is_end_of_chain(next) {
                return Ok(());
            }
            if next == FAT32_BAD_CLUSTER {
                return Err(FsError::CorruptFat);
            }
            current_cluster = next;
        }
    }

    fn read_fat_entry(&self, cluster: u32) -> Result<u32, FsError> {
        self.validate_cluster(cluster)?;
        let fat_offset = u64::from(cluster)
            .checked_mul(4)
            .ok_or(FsError::CorruptFat)?;
        let fat_sector = fat_offset / SECTOR_SIZE as u64;
        if fat_sector >= u64::from(self.fat_size_sectors) {
            return Err(FsError::CorruptFat);
        }

        let mut sector = [0u8; SECTOR_SIZE];
        self.device
            .read_sector(self.fat_start_lba + fat_sector, &mut sector)?;

        let offset = (fat_offset % SECTOR_SIZE as u64) as usize;
        Ok(le_u32(&sector[offset..offset + 4]) & 0x0FFF_FFFF)
    }

    fn read_next_data_cluster(&self, cluster: u32) -> Result<u32, FsError> {
        let next = self.read_fat_entry(cluster)?;
        if is_end_of_chain(next) || next == FAT32_BAD_CLUSTER {
            return Err(FsError::CorruptFat);
        }

        Ok(next)
    }

    fn cluster_to_lba(&self, cluster: u32) -> Result<u64, FsError> {
        self.validate_cluster(cluster)?;
        let cluster_index = u64::from(cluster - 2);
        self.first_data_lba
            .checked_add(cluster_index * u64::from(self.sectors_per_cluster))
            .ok_or(FsError::CorruptFat)
    }

    fn validate_cluster(&self, cluster: u32) -> Result<(), FsError> {
        if cluster < 2 || cluster - 2 >= self.total_clusters {
            return Err(FsError::InvalidCluster);
        }

        Ok(())
    }
}

/// Adapter để dùng `Fat32Volume` như một backend VFS read-only.
pub struct Fat32FileSystem<'a, D: BlockDevice + ?Sized> {
    volume: Fat32Volume<'a, D>,
}

impl<'a, D: BlockDevice + ?Sized> Fat32FileSystem<'a, D> {
    pub const fn new(volume: Fat32Volume<'a, D>) -> Self {
        Self { volume }
    }
}

impl<D> vfs::FileSystem for Fat32FileSystem<'_, D>
where
    D: BlockDevice + Sync + ?Sized,
{
    fn open(&self, path: &str) -> Result<vfs::FileNode, vfs::VfsError> {
        let node = self.volume.open(path).map_err(map_fat32_error)?;
        Ok(vfs::FileNode::new(
            map_file_type(node.file_type),
            u64::from(node.size),
            0,
            u64::from(node.first_cluster),
        ))
    }

    fn read_at(
        &self,
        node: &vfs::FileNode,
        offset: u64,
        buffer: &mut [u8],
    ) -> Result<usize, vfs::VfsError> {
        if node.backend_id() != 0 {
            return Err(vfs::VfsError::BackendError);
        }

        let first_cluster =
            u32::try_from(node.backend_node()).map_err(|_| vfs::VfsError::BackendError)?;
        let size = u32::try_from(node.size()).map_err(|_| vfs::VfsError::BackendError)?;
        let fat_node = Fat32Node {
            file_type: map_vfs_file_type(node.file_type()),
            first_cluster,
            size,
        };

        self.volume
            .read_node_at(&fat_node, offset, buffer)
            .map_err(map_fat32_error)
    }

    fn list_dir(&self, path: &str, sink: &mut dyn vfs::DirEntrySink) -> Result<(), vfs::VfsError> {
        let mut adapter = VfsDirSink { sink };
        self.volume
            .list_dir(path, &mut adapter)
            .map_err(map_fat32_error)
    }
}

struct VfsDirSink<'a> {
    sink: &'a mut dyn vfs::DirEntrySink,
}

impl DirEntrySink for VfsDirSink<'_> {
    fn push(&mut self, entry: DirectoryEntry) -> Result<(), FsError> {
        let (name, name_len) =
            format_display_name(&entry.name).map_err(|_| FsError::InvalidPath)?;
        let vfs_entry = vfs::DirEntry::from_raw_name(
            &name[..name_len as usize],
            map_file_type(entry.file_type),
            u64::from(entry.size),
        )
        .map_err(|_| FsError::SinkFull)?;
        self.sink.push(vfs_entry).map_err(|_| FsError::SinkFull)
    }
}

fn map_fat32_error(error: FsError) -> vfs::VfsError {
    match error {
        FsError::InvalidPath => vfs::VfsError::InvalidPath,
        FsError::NotFound => vfs::VfsError::NotFound,
        FsError::NotAFile => vfs::VfsError::IsDirectory,
        FsError::NotADirectory => vfs::VfsError::NotDirectory,
        FsError::BufferTooSmall => vfs::VfsError::BufferTooSmall,
        FsError::SinkFull => vfs::VfsError::SinkFull,
        FsError::Block(_)
        | FsError::InvalidBootSector
        | FsError::UnsupportedSectorSize
        | FsError::UnsupportedFatLayout
        | FsError::InvalidCluster
        | FsError::CorruptFat => vfs::VfsError::BackendError,
    }
}

fn map_file_type(file_type: FileType) -> vfs::FileType {
    match file_type {
        FileType::RegularFile => vfs::FileType::RegularFile,
        FileType::Directory => vfs::FileType::Directory,
    }
}

fn map_vfs_file_type(file_type: vfs::FileType) -> FileType {
    match file_type {
        vfs::FileType::RegularFile => FileType::RegularFile,
        vfs::FileType::Directory => FileType::Directory,
    }
}

fn format_display_name(raw_name: &[u8; 11]) -> Result<([u8; vfs::MAX_NAME_LEN], u8), FsError> {
    let mut output = [0u8; vfs::MAX_NAME_LEN];
    let base_len = trim_spaces_len(&raw_name[..8]);
    let extension_len = trim_spaces_len(&raw_name[8..11]);
    if base_len == 0 {
        return Err(FsError::InvalidPath);
    }

    output[..base_len].copy_from_slice(&raw_name[..base_len]);
    let mut output_len = base_len;
    if extension_len > 0 {
        output[output_len] = b'.';
        output_len += 1;
        output[output_len..output_len + extension_len]
            .copy_from_slice(&raw_name[8..8 + extension_len]);
        output_len += extension_len;
    }

    Ok((output, output_len as u8))
}

fn trim_spaces_len(bytes: &[u8]) -> usize {
    let mut len = bytes.len();
    while len > 0 && bytes[len - 1] == b' ' {
        len -= 1;
    }
    len
}

fn is_end_of_chain(entry: u32) -> bool {
    entry >= FAT32_END_OF_CHAIN
}

fn le_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

fn le_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn format_short_name(path: &str) -> Result<[u8; 11], FsError> {
    let trimmed = match path.strip_prefix('/') {
        Some(stripped) => stripped,
        None => path,
    };
    if trimmed.is_empty() || trimmed.contains('/') {
        return Err(FsError::InvalidPath);
    }

    let bytes = trimmed.as_bytes();
    let mut dot_index = None;
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'.' {
            if dot_index.is_some() {
                return Err(FsError::InvalidPath);
            }
            dot_index = Some(index);
        }
    }

    let (name_part, extension_part) = if let Some(index) = dot_index {
        (&bytes[..index], &bytes[index + 1..])
    } else {
        (bytes, &[][..])
    };

    if name_part.is_empty()
        || name_part.len() > 8
        || extension_part.len() > 3
        || (dot_index.is_some() && extension_part.is_empty())
    {
        return Err(FsError::InvalidPath);
    }

    let mut name = [b' '; 11];
    for (index, byte) in name_part.iter().enumerate() {
        name[index] = normalize_short_name_byte(*byte)?;
    }
    for (index, byte) in extension_part.iter().enumerate() {
        name[8 + index] = normalize_short_name_byte(*byte)?;
    }

    Ok(name)
}

fn normalize_short_name_byte(byte: u8) -> Result<u8, FsError> {
    match byte {
        b'a'..=b'z' => Ok(byte - 32),
        b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-' | b'$' | b'~' => Ok(byte),
        _ => Err(FsError::InvalidPath),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drivers::block::RamDisk;
    use crate::fs::vfs::VfsRoot;
    use alloc::boxed::Box;
    use alloc::vec;

    const TEST_SECTORS: usize = 8;
    const MARKER_CONTENT: &[u8] = b"AXIOMOS FAT32 MARKER";
    const README_CONTENT: &[u8] = b"readme";

    struct TestSink {
        entries: [Option<DirectoryEntry>; 4],
        len: usize,
    }

    impl TestSink {
        fn new() -> Self {
            Self {
                entries: [None; 4],
                len: 0,
            }
        }
    }

    impl DirEntrySink for TestSink {
        fn push(&mut self, entry: DirectoryEntry) -> Result<(), FsError> {
            if self.len == self.entries.len() {
                return Err(FsError::SinkFull);
            }

            self.entries[self.len] = Some(entry);
            self.len += 1;
            Ok(())
        }
    }

    #[test]
    fn reads_marker_file_from_fat32_image() {
        let disk = RamDisk::new(build_test_image());
        let volume = match mount_fat32(&disk) {
            Ok(volume) => volume,
            Err(error) => panic!("mount failed: {:?}", error),
        };

        let mut buffer = [0u8; 64];
        assert_eq!(
            volume.read_file("/AXIOMOS.TXT", &mut buffer),
            Ok(MARKER_CONTENT.len())
        );
        assert_eq!(&buffer[..MARKER_CONTENT.len()], MARKER_CONTENT);
    }

    #[test]
    fn rejects_corrupt_boot_sector_without_panic() {
        let mut image = vec![0u8; SECTOR_SIZE * TEST_SECTORS];
        fill_test_image(&mut image);
        image[510] = 0;
        let disk = RamDisk::new(Box::leak(image.into_boxed_slice()));

        assert!(matches!(
            mount_fat32(&disk),
            Err(FsError::InvalidBootSector)
        ));
    }

    #[test]
    fn lists_root_entries_in_disk_order() {
        let disk = RamDisk::new(build_test_image());
        let volume = match mount_fat32(&disk) {
            Ok(volume) => volume,
            Err(error) => panic!("mount failed: {:?}", error),
        };

        let mut sink = TestSink::new();
        assert_eq!(volume.list_dir("/", &mut sink), Ok(()));
        assert_eq!(sink.len, 2);

        let first = match sink.entries[0] {
            Some(entry) => entry,
            None => panic!("missing first entry"),
        };
        let second = match sink.entries[1] {
            Some(entry) => entry,
            None => panic!("missing second entry"),
        };

        assert_eq!(first.name, *b"AXIOMOS TXT");
        assert_eq!(first.file_type, FileType::RegularFile);
        assert_eq!(second.name, *b"README  MD ");
        assert_eq!(second.file_type, FileType::RegularFile);
    }

    #[test]
    fn rejects_small_read_buffer() {
        let disk = RamDisk::new(build_test_image());
        let volume = match mount_fat32(&disk) {
            Ok(volume) => volume,
            Err(error) => panic!("mount failed: {:?}", error),
        };

        let mut buffer = [0u8; 4];
        assert_eq!(
            volume.read_file("AXIOMOS.TXT", &mut buffer),
            Err(FsError::BufferTooSmall)
        );
    }

    #[test]
    fn reads_marker_file_through_vfs_adapter() {
        let disk = Box::leak(Box::new(RamDisk::new(build_test_image())));
        let volume = match mount_fat32(disk) {
            Ok(volume) => volume,
            Err(error) => panic!("mount failed: {:?}", error),
        };
        let filesystem = Box::leak(Box::new(Fat32FileSystem::new(volume)));
        let mut root = VfsRoot::new();
        assert_eq!(root.mount_root(filesystem), Ok(()));

        let mut handle = match root.open("/AXIOMOS.TXT") {
            Ok(handle) => handle,
            Err(error) => panic!("open failed: {:?}", error),
        };

        let mut first = [0u8; 7];
        assert_eq!(root.read(&mut handle, &mut first), Ok(7));
        assert_eq!(&first, b"AXIOMOS");

        let mut second = [0u8; 64];
        let remaining = MARKER_CONTENT.len() - first.len();
        assert_eq!(root.read(&mut handle, &mut second), Ok(remaining));
        assert_eq!(&second[..remaining], &MARKER_CONTENT[first.len()..]);
    }

    fn build_test_image() -> &'static [u8] {
        let mut image = vec![0u8; SECTOR_SIZE * TEST_SECTORS];
        fill_test_image(&mut image);
        Box::leak(image.into_boxed_slice())
    }

    fn fill_test_image(image: &mut [u8]) {
        fill_boot_sector(&mut image[..SECTOR_SIZE]);
        set_fat_entry(image, 0, 0x0FFF_FFF8);
        set_fat_entry(image, 1, 0x0FFF_FFFF);
        set_fat_entry(image, 2, 0x0FFF_FFFF);
        set_fat_entry(image, 3, 0x0FFF_FFFF);
        set_fat_entry(image, 4, 0x0FFF_FFFF);

        let root_lba = 2;
        write_directory_entry(
            image,
            root_lba,
            0,
            b"AXIOMOS TXT",
            0x20,
            3,
            MARKER_CONTENT.len() as u32,
        );
        write_directory_entry(
            image,
            root_lba,
            1,
            b"README  MD ",
            0x20,
            4,
            README_CONTENT.len() as u32,
        );

        write_cluster(image, 3, MARKER_CONTENT);
        write_cluster(image, 4, README_CONTENT);
    }

    fn fill_boot_sector(boot_sector: &mut [u8]) {
        boot_sector[0] = 0xEB;
        boot_sector[1] = 0x58;
        boot_sector[2] = 0x90;
        boot_sector[3..11].copy_from_slice(b"AXIOMOS ");
        write_le_u16(boot_sector, 11, SECTOR_SIZE as u16);
        boot_sector[13] = 1;
        write_le_u16(boot_sector, 14, 1);
        boot_sector[16] = 1;
        write_le_u16(boot_sector, 17, 0);
        write_le_u16(boot_sector, 19, 0);
        boot_sector[21] = 0xF8;
        write_le_u16(boot_sector, 22, 0);
        write_le_u32(boot_sector, 32, TEST_SECTORS as u32);
        write_le_u32(boot_sector, 36, 1);
        write_le_u32(boot_sector, 44, 2);
        boot_sector[510] = 0x55;
        boot_sector[511] = 0xAA;
    }

    fn set_fat_entry(image: &mut [u8], cluster: u32, value: u32) {
        let offset = SECTOR_SIZE + cluster as usize * 4;
        image[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_directory_entry(
        image: &mut [u8],
        root_lba: usize,
        index: usize,
        name: &[u8; 11],
        attributes: u8,
        first_cluster: u32,
        size: u32,
    ) {
        let offset = root_lba * SECTOR_SIZE + index * DIRECTORY_ENTRY_SIZE;
        image[offset..offset + 11].copy_from_slice(name);
        image[offset + 11] = attributes;
        write_le_u16(image, offset + 20, (first_cluster >> 16) as u16);
        write_le_u16(image, offset + 26, first_cluster as u16);
        write_le_u32(image, offset + 28, size);
    }

    fn write_cluster(image: &mut [u8], cluster: u32, content: &[u8]) {
        let lba = 2 + (cluster as usize - 2);
        let offset = lba * SECTOR_SIZE;
        image[offset..offset + content.len()].copy_from_slice(content);
    }

    fn write_le_u16(image: &mut [u8], offset: usize, value: u16) {
        image[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn write_le_u32(image: &mut [u8], offset: usize, value: u32) {
        image[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
}
