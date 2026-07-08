//! Hệ thống cuộc gọi hệ thống (Syscalls Subsystem)
//!
//! Mô-đun này thiết lập cấu hình MSRs cho lệnh `syscall`/`sysret`,
//! cung cấp handler entry bằng Assembly và bộ điều phối syscall (dispatcher).

// Nhúng mã Assembly xử lý điểm vào syscall của CPU
core::arch::global_asm!(
    r#"
    .global sys_entry
    .extern syscall_dispatch
    .extern USER_RSP
    .extern KERNEL_RSP


    sys_entry:
        # 1. CPU tự động tắt ngắt do FMASK. Vô hiệu hóa ngắt tuyệt đối
        cli

        # 2. Lưu rsp của userspace vào biến tĩnh USER_RSP
        mov qword ptr [rip + USER_RSP], rsp

        # 3. Nạp rsp của kernel từ biến tĩnh KERNEL_RSP
        mov rsp, qword ptr [rip + KERNEL_RSP]

        # 4. Đẩy trạng thái registers của user lên stack để bảo toàn
        push r11 # user rflags (được CPU lưu vào r11)
        push rcx # user rip (được CPU lưu vào rcx)
        push rbp
        push rdi
        push rsi
        push rdx
        push r10 # user r10 (tham số 4)
        push r8  # user r8  (tham số 5)
        push r9  # user r9  (tham số 6)
        push r12
        push r13
        push r14
        push r15
        push rax # syscall id (rax) để giữ chỗ

        # 5. Di chuyển các thanh ghi tham số phù hợp với System V AMD64 ABI của Rust:
        # Thứ tự gọi hàm Rust: rdi, rsi, rdx, rcx, r8, r9
        # Thứ tự register của syscall: rax (id), rdi (arg1), rsi (arg2), rdx (arg3), r10 (arg4), r8 (arg5), r9 (arg6)
        mov r9, r8   # arg5 -> tham số 6 của Rust
        mov r8, r10  # arg4 -> tham số 5 của Rust
        mov rcx, rdx # arg3 -> tham số 4 của Rust
        mov rdx, rsi # arg2 -> tham số 3 của Rust
        mov rsi, rdi # arg1 -> tham số 2 của Rust
        mov rdi, rax # id   -> tham số 1 của Rust

        # 6. Gọi bộ điều phối Rust
        call syscall_dispatch

        # rax chứa giá trị trả về từ syscall_dispatch. 
        # Khôi phục các registers ngoại trừ rax ở đỉnh stack
        add rsp, 8
        pop r15
        pop r14
        pop r13
        pop r12
        pop r9
        pop r8
        pop r10
        pop rdx
        pop rsi
        pop rdi
        pop rbp
        pop rcx # user rip
        pop r11 # user rflags

        # Tắt ngắt trước khi quay lại, sysretq sẽ tự động bật ngắt dựa trên user rflags
        cli
        
        # Khôi phục stack pointer của user
        mov rsp, qword ptr [rip + USER_RSP]

        # Quay lại userspace Ring 3
        sysretq
    "#
);

#[no_mangle]
pub static mut USER_RSP: u64 = 0;
#[no_mangle]
pub static mut KERNEL_RSP: u64 = 0;

const SYSCALL_EXIT: u64 = 1;
const SYSCALL_WRITE: u64 = 2;
const SYSCALL_YIELD: u64 = 3;
const SYSCALL_LIST_DIR: u64 = 4;
const SYSCALL_READ_FILE: u64 = 5;
const SYSCALL_ERROR: u64 = u64::MAX;
const USER_LIMIT: u64 = 0x0000_8000_0000_0000;
const MAX_USER_PATH_LEN: usize = 64;

// Các địa chỉ Model Specific Registers (MSRs) cho syscall x86_64
const MSR_EFER: u32 = 0xC0000080;
const MSR_STAR: u32 = 0xC0000081;
const MSR_LSTAR: u32 = 0xC0000082;
const MSR_FMASK: u32 = 0xC0000084;

extern "C" {
    /// Handler entry bằng Assembly
    fn sys_entry();
}

/// Khởi tạo cấu hình Syscall ABI trên CPU hiện tại
pub fn init() {
    // SAFETY: Các lệnh ghi MSRs yêu cầu đặc quyền Ring 0
    unsafe {
        // 1. Kích hoạt System Call Extension (SCE) trong EFER MSR
        let efer = rdmsr(MSR_EFER);
        wrmsr(MSR_EFER, efer | 1); // Bật bit 0 (SCE)

        // 2. Cấu hình Segment Selectors trong STAR MSR:
        // - STAR[47:32] (Kernel CS/SS): Kernel CS = 0x08, Kernel SS = 0x10.
        // - STAR[63:48] (User CS/SS base): GDT index 3 (offset 0x18).
        //   Khi sysret chạy: CS = 0x18 + 16 (0x28 | 3 = 0x2B), SS = 0x18 + 8 (0x20 | 3 = 0x23).
        let star_val = (0x18u64 << 48) | (0x08u64 << 32);
        wrmsr(MSR_STAR, star_val);

        // 3. Đăng ký địa chỉ của sys_entry vào LSTAR MSR
        let sys_entry_addr = sys_entry as *const () as u64;
        wrmsr(MSR_LSTAR, sys_entry_addr);

        // 4. Cấu hình FMASK MSR để tự động xóa các cờ khi vào syscall:
        // - Tắt ngắt (IF flag, bit 9)
        // - Tắt trap (TF flag, bit 8)
        // - Tắt direction flag (DF flag, bit 10)
        let fmask_val = (1 << 9) | (1 << 8) | (1 << 10);
        wrmsr(MSR_FMASK, fmask_val);
    }
}

/// Ghi giá trị vào Model Specific Register (MSR)
///
/// # Safety
/// Lệnh `wrmsr` yêu cầu CPU ở đặc quyền Ring 0.
#[inline]
unsafe fn wrmsr(msr: u32, value: u64) {
    let low = (value & 0xFFFF_FFFF) as u32;
    let high = (value >> 32) as u32;
    core::arch::asm!(
        "wrmsr",
        in("ecx") msr,
        in("eax") low,
        in("edx") high,
        options(nostack, preserves_flags)
    );
}

/// Đọc giá trị từ Model Specific Register (MSR)
///
/// # Safety
/// Lệnh `rdmsr` yêu cầu CPU ở đặc quyền Ring 0.
#[inline]
unsafe fn rdmsr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    core::arch::asm!(
        "rdmsr",
        in("ecx") msr,
        out("eax") low,
        out("edx") high,
        options(nostack, preserves_flags)
    );
    ((high as u64) << 32) | (low as u64)
}

/// Bộ điều phối syscall nhận giá trị từ handler entry Assembly
#[no_mangle]
pub extern "C" fn syscall_dispatch(
    id: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    _arg5: u64,
) -> u64 {
    match id {
        SYSCALL_EXIT => {
            // sys_exit
            sys_exit(arg1);
        }
        SYSCALL_WRITE => {
            // sys_write
            sys_write(arg1, arg2, arg3)
        }
        SYSCALL_YIELD => {
            // sys_yield
            sys_yield()
        }
        SYSCALL_LIST_DIR => sys_list_dir(arg1, arg2, arg3, arg4),
        SYSCALL_READ_FILE => sys_read_file(arg1, arg2, arg3, arg4),
        _ => {
            crate::serial_println!("[AXIOMOS SYSCALL] Unknown syscall: {}", id);
            SYSCALL_ERROR
        }
    }
}

/// Syscall 1: sys_exit - Kết thúc tiến trình hiện tại
fn sys_exit(code: u64) -> ! {
    crate::serial_println!("[AXIOMOS] init exited with code {}", code);
    crate::console::framebuffer::framebuffer_println(format_args!(
        "[AXIOMOS] init exited with code {}",
        code
    ));

    crate::serial_println!("[AXIOMOS] System halted");
    crate::console::framebuffer::framebuffer_println(format_args!("[AXIOMOS] System halted"));

    // Ở Milestone 6, init kết thúc đồng nghĩa với việc halt CPU an toàn
    loop {
        // SAFETY: cli và hlt dừng CPU an toàn yêu cầu Ring 0
        unsafe {
            core::arch::asm!("cli; hlt");
        }
    }
}

/// Syscall 2: sys_write - Ghi dữ liệu ra console/serial
fn sys_write(fd: u64, buf_ptr: u64, len: u64) -> u64 {
    // Chỉ chấp nhận stdout (1) và stderr (2)
    if fd != 1 && fd != 2 {
        return SYSCALL_ERROR;
    }

    let (addr, len) = match validate_user_range(buf_ptr, len) {
        Ok(range) => range,
        Err(()) => return SYSCALL_ERROR,
    };

    // Đọc an toàn từ buffer của userspace
    // SAFETY: Chúng ta đã kiểm tra con trỏ nằm trong không gian userspace.
    unsafe {
        let slice = core::slice::from_raw_parts(addr as *const u8, len);
        if let Ok(s) = core::str::from_utf8(slice) {
            crate::serial_print!("{}", s);
            crate::console::framebuffer::framebuffer_print(format_args!("{}", s));
            len as u64
        } else {
            SYSCALL_ERROR
        }
    }
}

/// Syscall 3: sys_yield - Nhường CPU cho tiến trình khác
fn sys_yield() -> u64 {
    crate::process::scheduler::yield_now();
    0
}

/// Syscall 4: sys_list_dir - Liệt kê thư mục read-only vào buffer userspace.
fn sys_list_dir(path_ptr: u64, path_len: u64, out_ptr: u64, out_len: u64) -> u64 {
    let mut path_storage = [0u8; MAX_USER_PATH_LEN];
    let path = match copy_user_path(path_ptr, path_len, &mut path_storage) {
        Ok(path) => path,
        Err(()) => return SYSCALL_ERROR,
    };

    let (out_addr, out_len) = match validate_user_range(out_ptr, out_len) {
        Ok(range) => range,
        Err(()) => return SYSCALL_ERROR,
    };

    // SAFETY: Vùng output đã được validate nằm trong canonical userspace range.
    let output = unsafe { core::slice::from_raw_parts_mut(out_addr as *mut u8, out_len) };
    let mut sink = UserDirListSink::new(output);

    match crate::fs::kernel_file::kernel_list_dir(path, &mut sink) {
        Ok(()) => sink.written() as u64,
        Err(_) => SYSCALL_ERROR,
    }
}

/// Syscall 5: sys_read_file - Đọc file read-only vào buffer userspace.
fn sys_read_file(path_ptr: u64, path_len: u64, out_ptr: u64, out_len: u64) -> u64 {
    let mut path_storage = [0u8; MAX_USER_PATH_LEN];
    let path = match copy_user_path(path_ptr, path_len, &mut path_storage) {
        Ok(path) => path,
        Err(()) => return SYSCALL_ERROR,
    };

    let (out_addr, out_len) = match validate_user_range(out_ptr, out_len) {
        Ok(range) => range,
        Err(()) => return SYSCALL_ERROR,
    };

    // SAFETY: Vùng output đã được validate nằm trong canonical userspace range.
    let output = unsafe { core::slice::from_raw_parts_mut(out_addr as *mut u8, out_len) };

    match crate::fs::kernel_file::kernel_read_file(path, output) {
        Ok(bytes_read) => bytes_read as u64,
        Err(_) => SYSCALL_ERROR,
    }
}

fn validate_user_range(ptr: u64, len: u64) -> Result<(usize, usize), ()> {
    if len == 0 || ptr >= USER_LIMIT {
        return Err(());
    }

    let end_addr = ptr.checked_add(len).ok_or(())?;
    if end_addr > USER_LIMIT {
        return Err(());
    }

    let addr = usize::try_from(ptr).map_err(|_| ())?;
    let len = usize::try_from(len).map_err(|_| ())?;
    Ok((addr, len))
}

fn copy_user_path<'a>(
    path_ptr: u64,
    path_len: u64,
    storage: &'a mut [u8; MAX_USER_PATH_LEN],
) -> Result<&'a str, ()> {
    let (addr, len) = validate_user_range(path_ptr, path_len)?;
    if len == 0 || len >= storage.len() {
        return Err(());
    }

    // SAFETY: Vùng path userspace đã được validate trước khi copy vào buffer kernel cố định.
    let user_path = unsafe { core::slice::from_raw_parts(addr as *const u8, len) };
    storage[..len].copy_from_slice(user_path);
    core::str::from_utf8(&storage[..len]).map_err(|_| ())
}

struct UserDirListSink<'a> {
    output: &'a mut [u8],
    written: usize,
}

impl<'a> UserDirListSink<'a> {
    fn new(output: &'a mut [u8]) -> Self {
        Self { output, written: 0 }
    }

    const fn written(&self) -> usize {
        self.written
    }

    fn push_bytes(&mut self, bytes: &[u8]) -> Result<(), crate::fs::vfs::VfsError> {
        let end = self
            .written
            .checked_add(bytes.len())
            .ok_or(crate::fs::vfs::VfsError::SinkFull)?;
        if end > self.output.len() {
            return Err(crate::fs::vfs::VfsError::SinkFull);
        }

        self.output[self.written..end].copy_from_slice(bytes);
        self.written = end;
        Ok(())
    }
}

impl crate::fs::vfs::DirEntrySink for UserDirListSink<'_> {
    fn push(&mut self, entry: crate::fs::vfs::DirEntry) -> Result<(), crate::fs::vfs::VfsError> {
        self.push_bytes(entry.name_bytes())?;
        self.push_bytes(b"\n")
    }
}

/// Chạy chẩn đoán (diagnostics) cấu hình phần cứng Syscall
pub fn run_syscall_diagnostics() {
    crate::serial_println!("[AXIOMOS SYSCALL] Chạy chẩn đoán cấu hình MSRs cho Syscall ABI...");

    // Gọi khởi tạo ghi các MSRs
    init();

    // SAFETY: Đọc lại các MSRs để đối chiếu
    unsafe {
        let efer = rdmsr(MSR_EFER);
        assert_eq!(efer & 1, 1, "Lỗi: Bit SCE trong EFER chưa được kích hoạt!");

        let star = rdmsr(MSR_STAR);
        let expected_star = (0x18u64 << 48) | (0x08u64 << 32);
        assert_eq!(
            star, expected_star,
            "Lỗi: Giá trị STAR MSR không chính xác!"
        );

        let lstar = rdmsr(MSR_LSTAR);
        assert_eq!(
            lstar, sys_entry as *const () as u64,
            "Lỗi: Địa chỉ handler LSTAR không khớp!"
        );

        let fmask = rdmsr(MSR_FMASK);
        let expected_fmask = (1 << 9) | (1 << 8) | (1 << 10);
        assert_eq!(
            fmask, expected_fmask,
            "Lỗi: Giá trị FMASK MSR không chính xác!"
        );
    }

    crate::serial_println!(
        "[AXIOMOS SYSCALL] Chạy chẩn đoán cấu hình MSRs cho Syscall ABI: THÀNH CÔNG"
    );
}
