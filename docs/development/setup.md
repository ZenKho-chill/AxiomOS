# Thiết lập môi trường phát triển (Development Environment Setup)

Tài liệu này hướng dẫn chi tiết cách cài đặt môi trường để xây dựng và phát triển hệ điều hành AxiomOS trên hệ điều hành Linux (Ubuntu/Debian) hoặc Windows sử dụng WSL2.

## Yêu cầu hệ thống

- Hệ điều hành: Linux (Ubuntu 20.04 LTS trở lên) hoặc Windows 10/11 có bật WSL2 (Ubuntu).
- Dung lượng ổ đĩa: Tối thiểu 5GB trống.
- Kết nối Internet hoạt động.

## Các bước cài đặt chi tiết

### 1. Cài đặt các công cụ biên dịch và giả lập

Chạy lệnh sau trên terminal Linux hoặc WSL2:

```bash
sudo apt update
sudo apt install -y build-essential git qemu-system-x86 llvm clang mtools parted dosfstools curl
```

Trên Windows, mở đúng distro Ubuntu trước khi chạy các lệnh build:

```powershell
wsl -d Ubuntu
```

Nếu repository nằm trên ổ D của Windows, đường dẫn trong WSL thường có dạng:

```bash
cd "/mnt/d/Personal Project/AxiomOS"
```

Các công cụ bao gồm:
- `build-essential`: Makefile, gcc, các thư viện runtime cơ bản.
- `qemu-system-x86`: Trình giả lập PC x86_64 để chạy OS.
- `mtools`, `parted`, `dosfstools`: Dùng để định dạng đĩa ảo FAT32 cho Limine bootloader.
- `llvm`, `clang`: Trình biên dịch hỗ trợ Linker.

### 2. Cài đặt Rust toolchain

Tải và cài đặt Rustup (nếu chưa cài):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Thực hiện theo các hướng dẫn trên màn hình. Sau khi cài đặt, khởi động lại terminal hoặc chạy:

```bash
source $HOME/.cargo/env
```

### 3. Cài đặt component và target cần thiết

Dự án sử dụng target `x86_64-unknown-none` để biên dịch mã nguồn Kernel không cần hệ điều hành nền.

```bash
rustup component add rust-src
rustup component add rustfmt clippy
rustup target add x86_64-unknown-none
```

### 4. Kiểm tra cấu hình

Di chuyển vào thư mục dự án và chạy thử lệnh build:

```bash
make build
```

Sau khi build thành công, tạo image bootable bằng:

```bash
make image
```
