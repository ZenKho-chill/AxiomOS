# Khởi chạy trên QEMU (Run QEMU Guide)

Tài liệu này hướng dẫn cách chạy AxiomOS trên trình giả lập QEMU.

## Cấu hình chạy cơ bản

Chúng ta sử dụng QEMU hệ thống x86_64 để giả lập phần cứng PC. 
Lệnh khởi chạy nhanh thông qua Makefile:

```bash
make run
```

Trên Windows, chạy lệnh trong Ubuntu WSL:

```powershell
wsl -d Ubuntu
```

```bash
cd "/mnt/d/Personal Project/AxiomOS"
make image
make run
```

Lệnh QEMU được thực thi bên dưới (trong `scripts/run-qemu.sh`):

```bash
qemu-system-x86_64 \
    -drive format=raw,file=target/axiomOS.img \
    -serial stdio \
    -m 256M \
    -no-reboot \
    -no-shutdown
```

Ý nghĩa các tham số:
- `-drive`: Nạp đĩa ảo định dạng raw vừa build được.
- `-serial stdio`: Định tuyến cổng nối tiếp COM1 của máy ảo ra terminal của host (giúp xem log early).
- `-m 256M`: Cấp 256MB RAM cho máy ảo.
- `-no-reboot`, `-no-shutdown`: Giữ cửa sổ QEMU không tự động tắt khi xảy ra crash/triple fault để hỗ trợ chẩn đoán.

## Serial output kỳ vọng

```text
[AXIOMOS] Bootloader handoff complete
[AXIOMOS] Kernel started
[AXIOMOS] Serial logger initialized
[AXIOMOS] System halted
```

## Framebuffer output kỳ vọng

Khi chạy với cửa sổ QEMU/WSLg, framebuffer console tối thiểu sẽ mirror boot sequence lên màn hình QEMU. Dòng tối thiểu cần thấy là:

```text
[AXIOMOS] Kernel started
```

## Kiểm thử headless trong WSL

Khi cần kiểm tra log không mở cửa sổ QEMU, có thể chạy:

```bash
timeout 18s qemu-system-x86_64 \
    -drive format=raw,file=target/axiomOS.img \
    -serial file:qemu_serial.log \
    -display none \
    -m 256M \
    -no-reboot \
    -no-shutdown

cat qemu_serial.log
```
