# Hướng dẫn Build (Build Guide)

Tài liệu này hướng dẫn cách biên dịch kernel AxiomOS và đóng gói thành đĩa ảo bootable.

## Biên dịch Kernel

Sử dụng Cargo để biên dịch Kernel với target bare-metal `x86_64-unknown-none`:

```bash
cargo build --manifest-path kernel/Cargo.toml --target x86_64-unknown-none
```

Output sẽ được lưu tại `target/x86_64-unknown-none/debug/kernel`.

## Tạo Đĩa Ảo Bootable (raw IMG)

Quá trình đóng gói đĩa ảo được thực hiện bởi script `scripts/build-image.sh`. Quy trình cơ bản bao gồm:
1. Tạo một tệp ảnh đĩa trống `axiomOS.img`.
2. Định dạng phân vùng bằng chuẩn GPT.
3. Tạo phân vùng EFI System Partition (ESP) định dạng FAT32.
4. Sao chép kernel ELF và file cấu hình `limine.cfg` vào phân vùng.
5. Cài đặt boot sector của Limine vào đĩa.

Lệnh nhanh:
```bash
make image
```
