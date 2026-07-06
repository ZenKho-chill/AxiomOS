#!/bin/bash
set -e

# Đảm bảo PATH chứa thư mục cargo của rustup
export PATH="$HOME/.cargo/bin:$PATH"

# 1. Chuyển đổi kết thúc dòng của file cấu hình sang định dạng LF (Unix)
echo "[AXIOMOS] Định dạng file cấu hình limine.cfg sang LF..."
tr -d '\r' < assets/boot/limine.cfg > assets/boot/limine.tmp && mv assets/boot/limine.tmp assets/boot/limine.cfg

echo "[AXIOMOS] Bắt đầu đóng gói đĩa ảo raw IMG..."

# 1. Tạo thư mục target nếu chưa có
mkdir -p target

# 2. Biên dịch Kernel
echo "[AXIOMOS] Biên dịch Kernel..."
KERNEL_FEATURE_ARGS=()
if [ -n "${KERNEL_FEATURES:-}" ]; then
    KERNEL_FEATURE_ARGS=(--features "$KERNEL_FEATURES")
fi

cargo +nightly build --manifest-path kernel/Cargo.toml --target x86_64-unknown-none \
    "${KERNEL_FEATURE_ARGS[@]}" \
    -Zbuild-std=core,compiler_builtins \
    -Zbuild-std-features=compiler-builtins-mem

# 3. Tạo file image trống 64MB
echo "[AXIOMOS] Tạo file đĩa ảo 64MB trống..."
dd if=/dev/zero of=target/axiomOS.img bs=1M count=64

# 4. Tạo bảng phân vùng GPT và phân vùng ESP FAT32 (giới hạn từ 1MB đến 61MB)
echo "[AXIOMOS] Tạo phân vùng EFI System Partition (ESP)..."
parted -s target/axiomOS.img mklabel gpt
parted -s target/axiomOS.img mkpart ESP fat32 2048s 124928s
parted -s target/axiomOS.img set 1 esp on

# 5. Tạo phân vùng FAT32 ảo để copy file (dung lượng 60MB)
dd if=/dev/zero of=target/esp.img bs=1M count=60
mformat -i target/esp.img -F ::

# 6. Sao chép Bootloader, Kernel, và cấu hình vào phân vùng ESP (sử dụng cờ -o để tự động ghi đè)
mmd -i target/esp.img ::/EFI
mmd -i target/esp.img ::/EFI/BOOT
mcopy -o -i target/esp.img assets/limine/BOOTX64.EFI ::/EFI/BOOT/
mmd -i target/esp.img ::/boot
# Sửa OS/ABI byte trong ELF header từ UNIX-GNU (0x03) về ELFOSABI_NONE (0x00)
# Limine yêu cầu kernel ELF phải có OS/ABI = 0x00 để accept entry là hợp lệ.
objcopy --remove-section .comment \
    target/x86_64-unknown-none/debug/kernel \
    target/kernel-stripped.elf
# Ghi byte 0x07 (OS/ABI field offset) thành 0x00
printf '\x00' | dd of=target/kernel-stripped.elf bs=1 seek=7 count=1 conv=notrunc 2>/dev/null
mcopy -o -i target/esp.img target/kernel-stripped.elf ::/boot/kernel.elf
mcopy -o -i target/esp.img target/kernel-stripped.elf ::/kernel.elf
mcopy -o -i target/esp.img assets/boot/limine.cfg ::/boot/limine.cfg
mcopy -o -i target/esp.img assets/limine/limine-bios.sys ::/boot/
mcopy -o -i target/esp.img assets/boot/limine.cfg ::/limine.cfg
mcopy -o -i target/esp.img assets/boot/limine.cfg ::/EFI/BOOT/limine.cfg
mcopy -o -i target/esp.img assets/boot/limine.cfg ::/boot/limine.conf
mcopy -o -i target/esp.img assets/boot/limine.cfg ::/limine.conf
mcopy -o -i target/esp.img assets/boot/limine.cfg ::/EFI/BOOT/limine.conf

# 7. Ghi đè phân vùng FAT32 vào file image chính (ghi từ 1MB đến 61MB)
dd if=target/esp.img of=target/axiomOS.img bs=1M seek=1 conv=notrunc
rm -f target/esp.img

# 8. Cài đặt boot sector của Limine lên image
echo "[AXIOMOS] Cài đặt boot sector Limine..."
assets/limine/limine bios-install target/axiomOS.img

echo "[AXIOMOS] Đóng gói đĩa ảo target/axiomOS.img thành công!"
