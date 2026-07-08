# Makefile cho dự án AxiomOS

.PHONY: all build image run debug test fmt lint clean

all: build

build:
	RUSTFLAGS="-C link-arg=-Tlinker.ld" cargo build --manifest-path userspace/init/Cargo.toml --target x86_64-unknown-none
	cargo build --manifest-path kernel/Cargo.toml --target x86_64-unknown-none

image:
	@echo "[AXIOMOS] Xây dựng đĩa ảo raw IMG..."
	bash scripts/build-image.sh

run:
	@echo "[AXIOMOS] Khởi động hệ điều hành trên QEMU..."
	bash scripts/run-qemu.sh

debug:
	@echo "[AXIOMOS] Khởi động chế độ debug GDB trên QEMU..."
	bash scripts/debug-qemu.sh

test:
	@echo "[AXIOMOS] Chạy kiểm thử..."
	cargo test --workspace --exclude kernel

fmt:
	cargo fmt --all

lint:
	cargo clippy --workspace --exclude kernel

clean:
	cargo clean
	rm -f target/*.img
