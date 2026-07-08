# Spec: 018-minimal-userspace-shell (Shell userspace tối thiểu)

- **Feature ID**: 018-minimal-userspace-shell
- **Tiêu đề**: Shell userspace tối thiểu
- **Trạng thái**: APPROVED
- **Người phụ trách**: Kỹ sư trưởng AxiomOS
- **Ngày tạo**: 2026-07-08
- **Ngày cập nhật**: 2026-07-08

---

## Vấn đề cần giải quyết

Milestone 6 đã chứng minh kernel có thể nạp `init` ELF và chuyển quyền điều khiển sang Ring 3. Milestone 7 cần chứng minh userspace có thể gọi kernel để liệt kê và đọc file từ root filesystem read-only, đồng thời cung cấp shell tối thiểu làm bề mặt thao tác đầu tiên.

## Mục tiêu

- Thêm shell core tối thiểu trong crate `userspace/shell`.
- Cho `userspace/init` gọi shell core sau khi vào Ring 3.
- Thêm libc userspace tối thiểu cho `write`, `exit`, `yield`, `list_dir` và `read_file`.
- Thêm syscall read-only để liệt kê thư mục root và đọc file qua VFS/kernel file API.
- Đóng gói file mẫu `/HELLO.TXT` vào FAT32 RAM disk để shell có nội dung kiểm chứng cho `cat`.
- QEMU serial output phải thể hiện `axiomsh> ls /` và `axiomsh> cat /HELLO.TXT`.

## Không thuộc phạm vi

- Không thêm `fork`, `exec`, pipe, signal hoặc job control.
- Không tạo shell interactive dựa trên keyboard input trong spec này.
- Không thêm filesystem write path.
- Không thêm quyền truy cập file descriptor theo process.
- Không thêm dynamic linking hoặc loader nhiều chương trình userspace.
- Không tuyên bố tương thích Linux/Windows shell.

## Ràng buộc

- Syscall mới phải validate pointer, length và không tin dữ liệu userspace.
- Syscall filesystem chỉ dùng VFS read-only hiện có.
- Không allocation trong syscall handler.
- Shell Milestone 7 chạy theo dạng scripted shell do `init` host; shell ELF riêng và `exec` được hoãn đến spec sau.
- ABI mới phải được ghi trong `docs/design/kernel-api.md`.
- Thay đổi user-visible phải cập nhật `CHANGELOG.md`.

## Dependencies

- Spec 007: FAT32 read-only.
- Spec 010: userspace init.
- Spec 016: Virtual File System.
- Spec 017: Kernel File API.
- ADR 007: userspace layout và syscall ABI.

## ADR liên quan

- [ADR-007](../architecture/adr-007-userspace-layout-and-syscall-abi.md): Layout userspace và nền syscall ABI.
- [ADR-008](../architecture/adr-008-minimal-filesystem-syscalls.md): Syscall filesystem read-only tối thiểu cho Milestone 7.

## Public interfaces

```rust
pub fn write(fd: u64, bytes: &[u8]) -> Result<usize, SyscallError>;
pub fn exit(code: u64) -> !;
pub fn yield_now() -> Result<(), SyscallError>;
pub fn list_dir(path: &str, output: &mut [u8]) -> Result<usize, SyscallError>;
pub fn read_file(path: &str, output: &mut [u8]) -> Result<usize, SyscallError>;
```

## Internal interfaces

```rust
pub trait ShellRuntime {
    fn write(&mut self, bytes: &[u8]);
    fn list_dir(&mut self, path: &str, output: &mut [u8]) -> Result<usize, ShellError>;
    fn read_file(&mut self, path: &str, output: &mut [u8]) -> Result<usize, ShellError>;
}

pub fn run_minimal_shell<R: ShellRuntime>(runtime: &mut R) -> i32;
```

## Data structures

- `ShellRuntime`: interface để shell core gọi I/O mà không phụ thuộc trực tiếp vào assembly syscall.
- `ShellError`: lỗi shell tối thiểu dùng cho syscall failure.
- `UserDirListSink`: sink kernel ghi danh sách entry vào buffer userspace.
- `SyscallError`: lỗi userspace libc khi kernel từ chối syscall.

## Xử lý lỗi

- Syscall trả `u64::MAX` khi path, pointer, length hoặc VFS operation không hợp lệ.
- Libc map `u64::MAX` thành `SyscallError::KernelRejected`.
- Shell in lỗi ngắn và trả exit code `1` nếu `ls` hoặc `cat` thất bại.
- Kernel không panic khi syscall filesystem nhận input không hợp lệ.

## Hành vi logging

- Shell in prompt scripted qua `sys_write`.
- `ls /` in danh sách tên file, mỗi entry một dòng.
- `cat /HELLO.TXT` in nội dung file mẫu.
- Kernel không log nội dung userspace buffer ngoài output do `sys_write` yêu cầu.

## Security considerations

- Userspace pointer được validate theo vùng canonical thấp trước khi dereference.
- Path được copy vào buffer kernel cố định trước khi parse UTF-8.
- Buffer output được giới hạn bởi length do caller cung cấp.
- Kernel heap mapping trong page table userspace phải là supervisor-only, không bật cờ `USER`.
- Không cấp quyền ghi filesystem cho userspace.
- Không claim sandbox production-grade trong Milestone 7.

## Kế hoạch test

- Unit test shell core bằng runtime giả để kiểm tra thứ tự prompt, `ls` và `cat`.
- Unit test shell core khi `ls` thất bại trả exit code `1`.
- Build userspace init với dependency `libc` và `shell`.
- Build kernel và image.
- Boot QEMU, kiểm tra serial output có `axiomsh> ls /`, `INIT.ELF`, `HELLO.TXT`, `axiomsh> cat /HELLO.TXT` và nội dung file mẫu.

## Acceptance criteria

- **Acceptance Criterion 1**:
  - **Given** AxiomOS boot vào userspace init trong QEMU.
  - **When** `init` chạy shell core Milestone 7.
  - **Then** serial output phải có dòng `axiomsh> ls /`.

- **Acceptance Criterion 2**:
  - **Given** FAT32 RAM disk chứa `/INIT.ELF` và `/HELLO.TXT`.
  - **When** shell gọi `list_dir("/")`.
  - **Then** serial output phải liệt kê `INIT.ELF` và `HELLO.TXT`.

- **Acceptance Criterion 3**:
  - **Given** `/HELLO.TXT` chứa nội dung kiểm chứng của AxiomOS.
  - **When** shell gọi `read_file("/HELLO.TXT")`.
  - **Then** serial output phải in nội dung file sau prompt `axiomsh> cat /HELLO.TXT`.

- **Acceptance Criterion 4**:
  - **Given** userspace truyền path hoặc buffer không hợp lệ vào syscall filesystem.
  - **When** kernel xử lý syscall.
  - **Then** syscall phải trả lỗi mà không panic và không halt kernel ngoài đường exit chuẩn của init.

## Kế hoạch rollback hoặc removal

- Có thể rollback bằng cách bỏ `sys_list_dir`, `sys_read_file`, shell dependency trong init và file `/HELLO.TXT` khỏi RAM disk.
- Syscall IDs 4 và 5 chưa được công bố ổn định, nên có thể đổi trong spec sau nếu kernel ABI được cập nhật đồng bộ.

## Câu hỏi mở

- Shell interactive dùng keyboard syscall hay console line discipline? Chưa giải quyết trong spec này.
- Shell ELF riêng sẽ được nạp qua `exec` hay spawn trực tiếp từ init? Chưa thuộc phạm vi Milestone 7 tối thiểu.
