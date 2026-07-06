#!/bin/bash
set -e

# Đảm bảo file đĩa ảo tồn tại
if [ ! -f target/axiomOS.img ]; then
    echo "[AXIOMOS ERROR] Không tìm thấy file target/axiomOS.img. Hãy chạy 'make image' trước."
    exit 1
fi

echo "[AXIOMOS] Khởi động hệ điều hành trên QEMU..."

# Chạy QEMU với tùy chọn định tuyến Serial ra stdout
qemu-system-x86_64 \
    -drive format=raw,file=target/axiomOS.img \
    -serial stdio \
    -m 256M \
    -no-reboot \
    -no-shutdown
