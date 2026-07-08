#![cfg_attr(not(test), no_std)]

pub const STDOUT: u64 = 1;
pub const STDERR: u64 = 2;

const SYSCALL_EXIT: u64 = 1;
const SYSCALL_WRITE: u64 = 2;
const SYSCALL_YIELD: u64 = 3;
const SYSCALL_LIST_DIR: u64 = 4;
const SYSCALL_READ_FILE: u64 = 5;
const SYSCALL_ERROR: u64 = u64::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallError {
    KernelRejected,
}

pub fn write(fd: u64, bytes: &[u8]) -> Result<usize, SyscallError> {
    // SAFETY: Userspace truyền con trỏ và độ dài của slice hợp lệ cho kernel syscall.
    let ret = unsafe { syscall3(SYSCALL_WRITE, fd, bytes.as_ptr() as u64, bytes.len() as u64) };
    syscall_result(ret)
}

pub fn exit(code: u64) -> ! {
    // SAFETY: `sys_exit` không quay lại userspace theo ABI của AxiomOS.
    unsafe { syscall_exit(code) }
}

pub fn yield_now() -> Result<(), SyscallError> {
    // SAFETY: `sys_yield` không nhận con trỏ userspace.
    let ret = unsafe { syscall0(SYSCALL_YIELD) };
    if ret == SYSCALL_ERROR {
        Err(SyscallError::KernelRejected)
    } else {
        Ok(())
    }
}

pub fn list_dir(path: &str, output: &mut [u8]) -> Result<usize, SyscallError> {
    // SAFETY: Path và output đều là slice hợp lệ trong address space userspace.
    let ret = unsafe {
        syscall4(
            SYSCALL_LIST_DIR,
            path.as_ptr() as u64,
            path.len() as u64,
            output.as_mut_ptr() as u64,
            output.len() as u64,
        )
    };
    syscall_result(ret)
}

pub fn read_file(path: &str, output: &mut [u8]) -> Result<usize, SyscallError> {
    // SAFETY: Path và output đều là slice hợp lệ trong address space userspace.
    let ret = unsafe {
        syscall4(
            SYSCALL_READ_FILE,
            path.as_ptr() as u64,
            path.len() as u64,
            output.as_mut_ptr() as u64,
            output.len() as u64,
        )
    };
    syscall_result(ret)
}

fn syscall_result(ret: u64) -> Result<usize, SyscallError> {
    if ret == SYSCALL_ERROR {
        return Err(SyscallError::KernelRejected);
    }

    usize::try_from(ret).map_err(|_| SyscallError::KernelRejected)
}

/// Gọi syscall không tham số.
///
/// # Safety
/// Preconditions: CPU đang ở Ring 3 và kernel đã cấu hình MSR syscall.
/// Postconditions: Trả về giá trị trong `rax` theo ABI AxiomOS.
/// Memory safety assumptions: Không truyền con trỏ nên không dereference memory.
/// CPU state assumptions: `syscall` clobber `rcx` và `r11`.
unsafe fn syscall0(id: u64) -> u64 {
    let ret: u64;
    core::arch::asm!(
        "syscall",
        inlateout("rax") id => ret,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack),
    );
    ret
}

/// Gọi syscall có ba tham số thanh ghi.
///
/// # Safety
/// Preconditions: CPU đang ở Ring 3 và mọi con trỏ trong tham số phải hợp lệ theo syscall đích.
/// Postconditions: Trả về giá trị trong `rax` theo ABI AxiomOS.
/// Memory safety assumptions: Kernel chịu trách nhiệm validate con trỏ userspace trước khi dùng.
/// CPU state assumptions: `syscall` clobber `rcx` và `r11`.
unsafe fn syscall3(id: u64, arg1: u64, arg2: u64, arg3: u64) -> u64 {
    let ret: u64;
    core::arch::asm!(
        "syscall",
        inlateout("rax") id => ret,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack),
    );
    ret
}

/// Gọi syscall có bốn tham số thanh ghi.
///
/// # Safety
/// Preconditions: CPU đang ở Ring 3 và mọi con trỏ trong tham số phải hợp lệ theo syscall đích.
/// Postconditions: Trả về giá trị trong `rax` theo ABI AxiomOS.
/// Memory safety assumptions: Kernel chịu trách nhiệm validate con trỏ userspace trước khi dùng.
/// CPU state assumptions: Tham số thứ tư nằm trong `r10`; `syscall` clobber `rcx` và `r11`.
unsafe fn syscall4(id: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64) -> u64 {
    let ret: u64;
    core::arch::asm!(
        "syscall",
        inlateout("rax") id => ret,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        in("r10") arg4,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack),
    );
    ret
}

/// Gọi syscall kết thúc tiến trình.
///
/// # Safety
/// Preconditions: CPU đang ở Ring 3 và kernel đã cấu hình `sys_exit`.
/// Postconditions: Hàm không quay lại caller.
/// Memory safety assumptions: Không truyền con trỏ userspace.
/// CPU state assumptions: Kernel xử lý transition sang halt/exit path.
unsafe fn syscall_exit(code: u64) -> ! {
    core::arch::asm!(
        "syscall",
        in("rax") SYSCALL_EXIT,
        in("rdi") code,
        options(noreturn),
    );
}
