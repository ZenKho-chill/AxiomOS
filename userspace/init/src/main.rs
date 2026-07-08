#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
use core::panic::PanicInfo;

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[cfg(not(test))]
struct InitRuntime;

#[cfg(not(test))]
impl shell::ShellRuntime for InitRuntime {
    fn write(&mut self, bytes: &[u8]) {
        let _ = axiom_libc::write(axiom_libc::STDOUT, bytes);
    }

    fn list_dir(&mut self, path: &str, output: &mut [u8]) -> Result<usize, shell::ShellError> {
        axiom_libc::list_dir(path, output).map_err(|_| shell::ShellError::Io)
    }

    fn read_file(&mut self, path: &str, output: &mut [u8]) -> Result<usize, shell::ShellError> {
        axiom_libc::read_file(path, output).map_err(|_| shell::ShellError::Io)
    }
}

/// Điểm vào của chương trình userspace init
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut runtime = InitRuntime;
    let exit_code = shell::run_minimal_shell(&mut runtime);
    axiom_libc::exit(exit_code as u64);
}
