# Tổng quan Kiến trúc AxiomOS

Tài liệu này trình bày kiến trúc tổng thể của AxiomOS, một hệ điều hành desktop modular được phát triển từ đầu cho nền tảng `x86_64`.

## Triết lý Thiết kế

- **Tính Modular**: Phân tách rõ ràng giữa các phân hệ (subsystems) trong Kernel. Các Driver và hệ thống dịch vụ tương tác thông qua các Interface hoặc Trait được xác định trước, hạn chế phụ thuộc trực tiếp.
- **An toàn (Safety)**: Tối ưu hóa tính an toàn của Rust bằng cách hạn chế tối đa các khối mã `unsafe`. Mọi đoạn mã `unsafe` phải có giải thích rõ ràng và được bao bọc cẩn thận.
- **Tính Tường minh**: Không sử dụng các cơ chế cấp phát ngầm (hidden allocation) trong Kernel Runtime Path.

## Sơ đồ Tổng quan Hệ thống (Dự kiến)

```text
+--------------------------------------------------+
|                   Userspace                      |
|    +------------------+    +----------------+    |
|    |      shell       |    |  init process  |    |
|    +------------------+    +----------------+    |
|             |                      |             |
|    +----------------------------------------+    |
|    |                 libc                   |    |
+----+----------------------------------------+----+
                      | (Syscalls)
+--------------------------------------------------+
|                    Kernel                        |
|    +----------------------------------------+    |
|    |             Syscall Handler            |    |
|    +----------------------------------------+    |
|    |      Process & Thread Scheduler        |    |
|    +----------------------------------------+    |
|    |  VFS (Virtual File System) & FAT32     |    |
|    +----------------------------------------+    |
|    |  Memory Manager (Paging, Allocator)    |    |
|    +----------------------------------------+    |
|    |  Drivers (Serial, PS/2 Keyboard)       |    |
|    +----------------------------------------+    |
|    |  Arch-Specific (x86_64 GDT/IDT/APIC)   |    |
+----+----------------------------------------+----+
                      |
              +---------------+
              | Limine Boot   |
              +---------------+
```

## Các thành phần chính

1. **Kernel Cực tiểu (Micro-like Design)**: Kernel chịu trách nhiệm chính về quản lý luồng CPU, phân phối bộ nhớ ảo, định tuyến ngắt và cung cấp API cuộc gọi hệ thống.
2. **Device Driver Model**: Các driver (Serial, Keyboard) được viết dưới dạng các thành phần độc lập thực thi bên trong Kernel Space nhưng giao tiếp qua các trait trừu tượng.
3. **Userspace Isolation**: Sử dụng cơ chế phân trang (Paging) và phân cấp đặc quyền CPU (Ring 0 cho Kernel, Ring 3 cho Userspace) để bảo vệ bộ nhớ.
