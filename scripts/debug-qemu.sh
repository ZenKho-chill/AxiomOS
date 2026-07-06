#!/bin/bash
set -e

if [ ! -f target/axiomOS.img ]; then
    echo "[AXIOMOS ERROR] Không tìm thấy file target/axiomOS.img. Hãy chạy 'make image' trước."
    exit 1
fi

echo "[AXIOMOS] Khởi động QEMU ở chế độ debug GDB (chờ kết nối trên port :1234)..."

qemu-system-x86_64 \
    -drive format=raw,file=target/axiomOS.img \
    -serial stdio \
    -m 256M \
    -s -S \
    -no-reboot \
    -no-shutdown
