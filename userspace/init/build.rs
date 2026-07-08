use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Lấy thư mục đầu ra OUT_DIR của quá trình biên dịch cargo
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Đọc linker script nội bộ và ghi vào OUT_DIR
    let linker_code = include_bytes!("linker.ld");
    fs::write(out_dir.join("linker.ld"), linker_code)
        .expect("Lỗi: Không ghi được linker script vào OUT_DIR");

    // Chỉ thị cho cargo thêm OUT_DIR vào thư mục tìm kiếm của linker (-L)
    println!("cargo:rustc-link-search={}", out_dir.display());

    // Chỉ chạy lại build.rs nếu file linker.ld thay đổi
    println!("cargo:rerun-if-changed=linker.ld");
}
