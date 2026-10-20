#!/usr/bin/env sh
set -eu
# Requires the toolchain declared in rust-toolchain.toml. No network or dependencies.
cargo fmt --all -- --check
cargo clippy --locked --offline --all-targets -- -D warnings
cargo test --locked --offline
cargo test --locked --offline --release
