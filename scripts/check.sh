#!/bin/bash
set -e
cargo clippy --workspace --exclude kernel
RUSTFLAGS="-C relocation-model=static -C link-arg=-no-pie -C link-arg=-Tlinker.ld" cargo build --manifest-path userspace/init/Cargo.toml --target x86_64-unknown-none
cargo build --manifest-path kernel/Cargo.toml --target x86_64-unknown-none
