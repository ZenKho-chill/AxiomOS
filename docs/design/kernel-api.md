# Design: Kernel API

Tài liệu này đặc tả giao diện lập trình ứng dụng (API) và cuộc gọi hệ thống (Syscalls) của Kernel.

*(Skeleton)*

## Trạng thái ABI

AxiomOS hiện chưa công bố kernel ABI ổn định cho userspace. Các interface trong
`kernel/src/memory` như `init_memory`, `allocate_frame`, `deallocate_frame`,
`memory_stats` và `hhdm_offset` là internal kernel API phục vụ Milestone 3.

Mọi thay đổi ABI userspace sau này phải cập nhật tài liệu này, spec liên quan,
ADR liên quan và `CHANGELOG.md`.

## Filesystem internal API

Spec 007 hiện cung cấp API kernel-internal cho FAT32 read-only qua module
`kernel/src/fs/fat32.rs`:

```rust
pub fn mount_fat32<D: BlockDevice + ?Sized>(device: &D) -> Result<Fat32Volume<'_, D>, FsError>;

impl<D: BlockDevice + ?Sized> Fat32Volume<'_, D> {
    pub fn read_file(&self, path: &str, buffer: &mut [u8]) -> Result<usize, FsError>;
    pub fn list_dir(&self, path: &str, sink: &mut dyn DirEntrySink) -> Result<(), FsError>;
}
```

API này chưa phải userspace ABI hoặc syscall ABI. Phạm vi hiện tại chỉ hỗ trợ
đường dẫn root 8.3 read-only trên một `BlockDevice`; long filename, thư mục
lồng nhau, quyền truy cập file descriptor và ghi file được hoãn sang spec sau.
