#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
use core::panic::PanicInfo;

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

/// Helper gọi syscall sys_write (ID = 2)
#[cfg(not(test))]
unsafe fn sys_write(fd: u64, buf: *const u8, len: u64) -> u64 {
    let ret: u64;
    core::arch::asm!(
        "syscall",
        in("rax") 2u64,
        in("rdi") fd,
        in("rsi") buf,
        in("rdx") len,
        out("rcx") _, // CPU ghi đè rip cũ vào rcx
        out("r11") _, // CPU ghi đè rflags cũ vào r11
        lateout("rax") ret,
    );
    ret
}

/// Helper gọi syscall sys_exit (ID = 1)
#[cfg(not(test))]
unsafe fn sys_exit(code: u64) -> ! {
    core::arch::asm!(
        "syscall",
        in("rax") 1u64,
        in("rdi") code,
        options(noreturn),
    );
}

/// Điểm vào của chương trình userspace init
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    let msg = "[AXIOMOS USERSPACE] Hello from init process!\n";
    unsafe {
        sys_write(1, msg.as_ptr(), msg.len() as u64);
        sys_exit(0);
    }
}
