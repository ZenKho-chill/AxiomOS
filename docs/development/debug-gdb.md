# Hướng dẫn Debug GDB (Debugging Guide)

Tài liệu này hướng dẫn cách kết nối trình debug GDB vào máy ảo QEMU để debug kernel AxiomOS từng dòng lệnh.

## Các bước Debug

1. **Khởi động QEMU ở chế độ debug**:
   Chạy lệnh sau trên terminal thứ nhất:
   ```bash
   make debug
   ```
   Lệnh này sẽ khởi chạy QEMU kèm theo tham số `-s -S`:
   - `-S`: Tạm dừng CPU ngay tại lệnh đầu tiên khi boot, chờ debugger kết nối.
   - `-s`: Mở một gdbserver trên cổng TCP 1234.

2. **Kết nối GDB**:
   Mở terminal thứ hai, di chuyển vào thư mục dự án và khởi chạy GDB:
   ```bash
   gdb-multiarch target/x86_64-unknown-none/debug/kernel
   ```
   Hoặc:
   ```bash
   rust-gdb target/x86_64-unknown-none/debug/kernel
   ```

3. **Thiết lập kết nối trong GDB**:
   ```gdb
   (gdb) target remote :1234
   (gdb) symbol-file target/x86_64-unknown-none/debug/kernel
   (gdb) break _start
   (gdb) continue
   ```
   Bây giờ bạn có thể debug từng dòng code Rust trong kernel.
