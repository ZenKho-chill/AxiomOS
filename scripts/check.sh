#!/bin/bash
set -e
cargo clippy --workspace --exclude kernel
cargo build --manifest-path kernel/Cargo.toml --target x86_64-unknown-none
