#![cfg_attr(not(test), no_std)]

const ROOT_PATH: &str = "/";
const HELLO_PATH: &str = "/HELLO.TXT";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellError {
    Io,
}

pub trait ShellRuntime {
    fn write(&mut self, bytes: &[u8]);
    fn list_dir(&mut self, path: &str, output: &mut [u8]) -> Result<usize, ShellError>;
    fn read_file(&mut self, path: &str, output: &mut [u8]) -> Result<usize, ShellError>;
}

pub fn run_minimal_shell<R: ShellRuntime>(runtime: &mut R) -> i32 {
    runtime.write(b"[AXIOMOS USERSPACE] init entered Ring 3\n");
    runtime.write(b"axiomsh> ls /\n");

    let mut list_buffer = [0u8; 256];
    let list_len = match runtime.list_dir(ROOT_PATH, &mut list_buffer) {
        Ok(len) => len,
        Err(_) => {
            runtime.write(b"ls: syscall failed\n");
            return 1;
        }
    };
    runtime.write(&list_buffer[..list_len]);

    runtime.write(b"axiomsh> cat /HELLO.TXT\n");
    let mut file_buffer = [0u8; 256];
    let file_len = match runtime.read_file(HELLO_PATH, &mut file_buffer) {
        Ok(len) => len,
        Err(_) => {
            runtime.write(b"cat: syscall failed\n");
            return 1;
        }
    };
    runtime.write(&file_buffer[..file_len]);

    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    struct MockRuntime {
        output: Vec<u8>,
        fail_list: bool,
    }

    impl MockRuntime {
        fn new() -> Self {
            Self {
                output: Vec::new(),
                fail_list: false,
            }
        }
    }

    impl ShellRuntime for MockRuntime {
        fn write(&mut self, bytes: &[u8]) {
            self.output.extend_from_slice(bytes);
        }

        fn list_dir(&mut self, path: &str, output: &mut [u8]) -> Result<usize, ShellError> {
            if self.fail_list || path != ROOT_PATH {
                return Err(ShellError::Io);
            }

            let listing = b"INIT.ELF\nHELLO.TXT\n";
            output[..listing.len()].copy_from_slice(listing);
            Ok(listing.len())
        }

        fn read_file(&mut self, path: &str, output: &mut [u8]) -> Result<usize, ShellError> {
            if path != HELLO_PATH {
                return Err(ShellError::Io);
            }

            let content = b"Hello from AxiomOS userspace file.\n";
            output[..content.len()].copy_from_slice(content);
            Ok(content.len())
        }
    }

    #[test]
    fn scripted_shell_prints_ls_and_cat_output() {
        let mut runtime = MockRuntime::new();

        assert_eq!(run_minimal_shell(&mut runtime), 0);

        let output = std::str::from_utf8(&runtime.output).expect("output must be utf8");
        assert!(output.contains("axiomsh> ls /"));
        assert!(output.contains("INIT.ELF"));
        assert!(output.contains("HELLO.TXT"));
        assert!(output.contains("axiomsh> cat /HELLO.TXT"));
        assert!(output.contains("Hello from AxiomOS userspace file."));
    }

    #[test]
    fn scripted_shell_returns_error_when_list_dir_fails() {
        let mut runtime = MockRuntime::new();
        runtime.fail_list = true;

        assert_eq!(run_minimal_shell(&mut runtime), 1);

        let output = std::str::from_utf8(&runtime.output).expect("output must be utf8");
        assert!(output.contains("ls: syscall failed"));
    }
}
