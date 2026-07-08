# Hướng dẫn Build (Build Guide)

Tài liệu này hướng dẫn cách biên dịch kernel AxiomOS và đóng gói thành đĩa ảo bootable.

## Biên dịch Kernel

Sử dụng Cargo để biên dịch Kernel với target bare-metal `x86_64-unknown-none`:

```bash
cargo build --manifest-path kernel/Cargo.toml --target x86_64-unknown-none
```

Output sẽ được lưu tại `target/x86_64-unknown-none/debug/kernel`.

## Biên dịch Userspace Init

`userspace/init` phải được build thành ELF64 `EXEC` static tại địa chỉ `0x400000`. Dùng cùng flags với `scripts/build-image.sh`:

```bash
RUSTFLAGS="-C relocation-model=static -C link-arg=-no-pie -C link-arg=-Tlinker.ld" \
  cargo build --manifest-path userspace/init/Cargo.toml --target x86_64-unknown-none
```

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

## Cấu hình Limine v7

Limine 7.x yêu cầu entry trong `assets/boot/limine.cfg` dùng cú pháp chữ hoa và nhãn bắt đầu bằng dấu `:`:

```text
TIMEOUT=5

:AxiomOS
    PROTOCOL=limine
    KERNEL_PATH=boot:///boot/kernel.elf
```

Nếu dùng cú pháp cũ như `/AxiomOS`, `protocol:` hoặc `path:`, Limine sẽ báo `[config file contains no valid entries]`.

## Linker script

`kernel/linker.ld` tách `.text`, `.rodata/.requests` và `.data/.bss` thành các `PT_LOAD` segment riêng, căn theo page 4K. Cách này tránh lỗi Limine `Attempted to load ELF file with PHDRs with different permissions sharing the same memory page`.

`userspace/init/linker.ld` cũng căn `.text` và `.rodata` theo page 4K. Kernel ELF loader hiện tại map từng `PT_LOAD` theo trang, vì vậy các segment userspace không được chồng lên cùng một page ảo.
