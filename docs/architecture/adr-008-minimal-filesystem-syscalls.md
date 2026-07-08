# ADR-008: Syscall filesystem read-only tối thiểu cho Milestone 7

- **Trạng thái**: APPROVED
- **Ngày**: 2026-07-08
- **Liên quan**: Spec 018, Spec 010, Spec 016, Spec 017, ADR-007

## Bối cảnh

Milestone 7 cần shell userspace tối thiểu có thể chạy `ls` và `cat`. Kernel đã có VFS read-only và Kernel File API, nhưng userspace mới chỉ có `exit`, `write` và `yield`. Nếu thêm file descriptor đầy đủ ngay bây giờ, phạm vi sẽ kéo theo bảng descriptor theo process, lifecycle handle, quyền truy cập và nhiều lỗi cleanup chưa cần thiết cho milestone này.

## Quyết định

Thêm hai syscall read-only dạng buffer caller-provided:

- `sys_list_dir` (`id = 4`): nhận path userspace và buffer output, ghi danh sách tên entry dạng newline-delimited.
- `sys_read_file` (`id = 5`): nhận path userspace và buffer output, đọc nội dung file vào buffer.

Hai syscall này chỉ truy cập VFS root mount hiện có, không tạo file descriptor, không giữ state theo process và không allocation trong handler. Shell Milestone 7 chạy theo dạng scripted shell được `init` host sau khi vào Ring 3. Shell ELF riêng và `exec` được hoãn sang spec sau.

Vì kernel heap hiện nằm ở lower-half virtual address, page table userspace phải copy mapping L4 của kernel heap dưới dạng supervisor-only. Mapping này cho phép syscall handler truy cập VFS và các object cấp phát trên heap khi CR3 đang là page table của process, nhưng không bật cờ `USER` nên Ring 3 không được phép đọc hoặc ghi vùng heap này.

## Lý do

- Giữ Milestone 7 nhỏ và kiểm chứng được trong QEMU.
- Tận dụng Spec 016/017 thay vì tạo filesystem API song song.
- Tránh mở rộng process model trước khi syscall ABI và scheduler ổn định hơn.
- Vẫn cung cấp behavior user-visible đủ rõ: `ls /` và `cat /HELLO.TXT`.

## Hệ quả

- ABI userspace có thêm syscall IDs 4 và 5, phải ghi trong `docs/design/kernel-api.md`.
- Shell hiện chưa interactive và chưa là process ELF riêng.
- Output `ls` là format tạm thời newline-delimited, chưa phải POSIX.
- File reads là whole-file read vào buffer userspace, chưa có offset hay file descriptor.
- Kernel heap được map vào page table userspace với quyền supervisor-only để syscall dùng được VFS trong giai đoạn chưa có CR3 switch riêng cho kernel.

## Phương án bị loại

- **Triển khai `open/read/close` đầy đủ**: bị loại vì cần descriptor table và lifecycle theo process, vượt phạm vi Milestone 7 tối thiểu.
- **Nạp `shell.elf` riêng bằng `exec`**: bị loại vì cần process replacement/spawn policy chưa có spec.
- **In kết quả `ls` từ kernel**: bị loại vì sẽ tạo shell giả trong kernel và không chứng minh userspace nhận dữ liệu từ syscall.
