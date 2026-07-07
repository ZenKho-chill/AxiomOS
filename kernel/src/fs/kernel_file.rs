//! API đọc tệp tin kernel-internal qua VFS.

use super::vfs::{self, DirEntrySink, FileHandle, VfsError};

pub fn kernel_open_file(path: &str) -> Result<FileHandle, VfsError> {
    vfs::open(path)
}

pub fn kernel_read(handle: &mut FileHandle, buffer: &mut [u8]) -> Result<usize, VfsError> {
    vfs::read(handle, buffer)
}

pub fn kernel_read_file(path: &str, buffer: &mut [u8]) -> Result<usize, VfsError> {
    let mut handle = kernel_open_file(path)?;
    kernel_read(&mut handle, buffer)
}

pub fn kernel_list_dir(path: &str, sink: &mut dyn DirEntrySink) -> Result<(), VfsError> {
    vfs::list_dir(path, sink)
}
