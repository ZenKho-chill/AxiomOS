//! VFS tối giản cho một root mount read-only.

use crate::utils::sync::SpinlockIrqSave;

pub const MAX_NAME_LEN: usize = 32;

/// Lỗi của VFS/kernel file API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfsError {
    NoRootMount,
    AlreadyMounted,
    InvalidPath,
    NotFound,
    IsDirectory,
    NotDirectory,
    BufferTooSmall,
    BackendError,
    SinkFull,
}

/// Loại node filesystem backend-neutral.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    RegularFile,
    Directory,
}

/// Entry thư mục backend-neutral với tên byte bounded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirEntry {
    name: [u8; MAX_NAME_LEN],
    name_len: u8,
    file_type: FileType,
    size: u64,
}

impl DirEntry {
    pub fn from_raw_name(name: &[u8], file_type: FileType, size: u64) -> Result<Self, VfsError> {
        if name.is_empty() || name.len() > MAX_NAME_LEN {
            return Err(VfsError::BackendError);
        }

        let mut stored_name = [0u8; MAX_NAME_LEN];
        stored_name[..name.len()].copy_from_slice(name);
        Ok(Self {
            name: stored_name,
            name_len: name.len() as u8,
            file_type,
            size,
        })
    }

    pub fn name_bytes(&self) -> &[u8] {
        &self.name[..self.name_len as usize]
    }

    pub const fn file_type(&self) -> FileType {
        self.file_type
    }

    pub const fn size(&self) -> u64 {
        self.size
    }
}

/// Sink nhận entry thư mục mà không yêu cầu allocation không giới hạn.
pub trait DirEntrySink {
    fn push(&mut self, entry: DirEntry) -> Result<(), VfsError>;
}

/// Metadata node backend-neutral được VFS giữ trong handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileNode {
    file_type: FileType,
    size: u64,
    backend_id: u32,
    backend_node: u64,
}

impl FileNode {
    pub const fn new(file_type: FileType, size: u64, backend_id: u32, backend_node: u64) -> Self {
        Self {
            file_type,
            size,
            backend_id,
            backend_node,
        }
    }

    pub const fn file_type(&self) -> FileType {
        self.file_type
    }

    pub const fn size(&self) -> u64 {
        self.size
    }

    pub(crate) const fn backend_id(&self) -> u32 {
        self.backend_id
    }

    pub(crate) const fn backend_node(&self) -> u64 {
        self.backend_node
    }
}

/// Handle đọc tuần tự kernel-internal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileHandle {
    node: FileNode,
    offset: u64,
}

impl FileHandle {
    pub const fn offset(&self) -> u64 {
        self.offset
    }

    pub const fn size(&self) -> u64 {
        self.node.size()
    }

    pub const fn file_type(&self) -> FileType {
        self.node.file_type()
    }
}

/// Trait backend filesystem read-only.
pub trait FileSystem: Sync {
    fn open(&self, path: &str) -> Result<FileNode, VfsError>;
    fn read_at(&self, node: &FileNode, offset: u64, buffer: &mut [u8]) -> Result<usize, VfsError>;
    fn list_dir(&self, path: &str, sink: &mut dyn DirEntrySink) -> Result<(), VfsError>;
}

/// Root mount registry tối giản cho Milestone 5.
pub struct VfsRoot {
    filesystem: Option<&'static dyn FileSystem>,
}

impl VfsRoot {
    pub const fn new() -> Self {
        Self { filesystem: None }
    }

    pub fn mount_root(&mut self, filesystem: &'static dyn FileSystem) -> Result<(), VfsError> {
        if self.filesystem.is_some() {
            return Err(VfsError::AlreadyMounted);
        }

        self.filesystem = Some(filesystem);
        Ok(())
    }

    pub fn open(&self, path: &str) -> Result<FileHandle, VfsError> {
        validate_path(path)?;
        let filesystem = self.filesystem.ok_or(VfsError::NoRootMount)?;
        open_with(filesystem, path)
    }

    pub fn read(&self, handle: &mut FileHandle, buffer: &mut [u8]) -> Result<usize, VfsError> {
        let filesystem = self.filesystem.ok_or(VfsError::NoRootMount)?;
        read_with(filesystem, handle, buffer)
    }

    pub fn list_dir(&self, path: &str, sink: &mut dyn DirEntrySink) -> Result<(), VfsError> {
        validate_path(path)?;
        let filesystem = self.filesystem.ok_or(VfsError::NoRootMount)?;
        filesystem.list_dir(path, sink)
    }

    fn filesystem(&self) -> Result<&'static dyn FileSystem, VfsError> {
        self.filesystem.ok_or(VfsError::NoRootMount)
    }
}

static ROOT_MOUNT: SpinlockIrqSave<VfsRoot> = SpinlockIrqSave::new(VfsRoot::new());

pub fn mount_root(filesystem: &'static dyn FileSystem) -> Result<(), VfsError> {
    let mut root = ROOT_MOUNT.lock();
    root.mount_root(filesystem)
}

pub fn open(path: &str) -> Result<FileHandle, VfsError> {
    validate_path(path)?;
    let filesystem = {
        let root = ROOT_MOUNT.lock();
        root.filesystem()?
    };
    open_with(filesystem, path)
}

pub fn read(handle: &mut FileHandle, buffer: &mut [u8]) -> Result<usize, VfsError> {
    let filesystem = {
        let root = ROOT_MOUNT.lock();
        root.filesystem()?
    };
    read_with(filesystem, handle, buffer)
}

pub fn list_dir(path: &str, sink: &mut dyn DirEntrySink) -> Result<(), VfsError> {
    validate_path(path)?;
    let filesystem = {
        let root = ROOT_MOUNT.lock();
        root.filesystem()?
    };
    filesystem.list_dir(path, sink)
}

fn open_with(filesystem: &'static dyn FileSystem, path: &str) -> Result<FileHandle, VfsError> {
    let node = filesystem.open(path)?;
    Ok(FileHandle { node, offset: 0 })
}

fn read_with(
    filesystem: &'static dyn FileSystem,
    handle: &mut FileHandle,
    buffer: &mut [u8],
) -> Result<usize, VfsError> {
    if handle.node.file_type() == FileType::Directory {
        return Err(VfsError::IsDirectory);
    }
    if buffer.is_empty() || handle.offset >= handle.node.size() {
        return Ok(0);
    }

    let bytes_read = filesystem.read_at(&handle.node, handle.offset, buffer)?;
    handle.offset = handle
        .offset
        .checked_add(bytes_read as u64)
        .ok_or(VfsError::BackendError)?;
    Ok(bytes_read)
}

fn validate_path(path: &str) -> Result<(), VfsError> {
    if path.is_empty() || !path.starts_with('/') || path.as_bytes().contains(&0) {
        return Err(VfsError::InvalidPath);
    }

    for component in path.split('/') {
        if component == ".." {
            return Err(VfsError::InvalidPath);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    static MOCK_FS: MockFileSystem = MockFileSystem;
    const FILE_CONTENT: &[u8] = b"AXIOMOS";

    struct MockFileSystem;

    impl FileSystem for MockFileSystem {
        fn open(&self, path: &str) -> Result<FileNode, VfsError> {
            match path {
                "/AXIOMOS.TXT" => Ok(FileNode::new(
                    FileType::RegularFile,
                    FILE_CONTENT.len() as u64,
                    0,
                    0,
                )),
                "/" => Ok(FileNode::new(FileType::Directory, 0, 0, 1)),
                _ => Err(VfsError::NotFound),
            }
        }

        fn read_at(
            &self,
            node: &FileNode,
            offset: u64,
            buffer: &mut [u8],
        ) -> Result<usize, VfsError> {
            if node.backend_id() != 0 {
                return Err(VfsError::BackendError);
            }
            if offset >= FILE_CONTENT.len() as u64 {
                return Ok(0);
            }

            let start = offset as usize;
            let copy_len = (FILE_CONTENT.len() - start).min(buffer.len());
            buffer[..copy_len].copy_from_slice(&FILE_CONTENT[start..start + copy_len]);
            Ok(copy_len)
        }

        fn list_dir(&self, path: &str, sink: &mut dyn DirEntrySink) -> Result<(), VfsError> {
            if path != "/" {
                return Err(VfsError::NotDirectory);
            }

            sink.push(DirEntry::from_raw_name(
                b"AXIOMOS.TXT",
                FileType::RegularFile,
                FILE_CONTENT.len() as u64,
            )?)?;
            sink.push(DirEntry::from_raw_name(
                b"README.MD",
                FileType::RegularFile,
                6,
            )?)?;
            Ok(())
        }
    }

    struct TestSink {
        entries: [Option<DirEntry>; 4],
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
        fn push(&mut self, entry: DirEntry) -> Result<(), VfsError> {
            if self.len == self.entries.len() {
                return Err(VfsError::SinkFull);
            }

            self.entries[self.len] = Some(entry);
            self.len += 1;
            Ok(())
        }
    }

    #[test]
    fn open_without_root_mount_returns_error() {
        let root = VfsRoot::new();
        assert_eq!(root.open("/AXIOMOS.TXT"), Err(VfsError::NoRootMount));
    }

    #[test]
    fn relative_path_is_rejected() {
        let root = VfsRoot::new();
        assert_eq!(root.open("AXIOMOS.TXT"), Err(VfsError::InvalidPath));
    }

    #[test]
    fn read_advances_handle_offset() {
        let mut root = VfsRoot::new();
        assert_eq!(root.mount_root(&MOCK_FS), Ok(()));
        let mut handle = match root.open("/AXIOMOS.TXT") {
            Ok(handle) => handle,
            Err(error) => panic!("open failed: {:?}", error),
        };

        let mut first = [0u8; 4];
        assert_eq!(root.read(&mut handle, &mut first), Ok(4));
        assert_eq!(&first, b"AXIO");
        assert_eq!(handle.offset(), 4);

        let mut second = [0u8; 8];
        assert_eq!(root.read(&mut handle, &mut second), Ok(3));
        assert_eq!(&second[..3], b"MOS");
        assert_eq!(handle.offset(), 7);
    }

    #[test]
    fn list_dir_forwards_entries() {
        let mut root = VfsRoot::new();
        assert_eq!(root.mount_root(&MOCK_FS), Ok(()));
        let mut sink = TestSink::new();

        assert_eq!(root.list_dir("/", &mut sink), Ok(()));
        assert_eq!(sink.len, 2);

        let first = match sink.entries[0] {
            Some(entry) => entry,
            None => panic!("missing first entry"),
        };
        let second = match sink.entries[1] {
            Some(entry) => entry,
            None => panic!("missing second entry"),
        };

        assert_eq!(first.name_bytes(), b"AXIOMOS.TXT");
        assert_eq!(second.name_bytes(), b"README.MD");
    }
}
