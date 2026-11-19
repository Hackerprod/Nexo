#!/usr/bin/env sh
set -eu

notice='PUERTA RÁPIDA: NO VÁLIDA PARA PASS'
printf '%s\n' "$notice"
trap 'printf "%s\n" "$notice"' EXIT

if ! cargo nextest --version >/dev/null 2>&1; then
    printf '%s\n' 'Falta cargo-nextest; instálelo antes de usar la puerta rápida.' >&2
    exit 2
fi
if ! command -v python >/dev/null 2>&1; then
    printf '%s\n' 'Falta Python 3.11+; instálelo antes de usar la puerta rápida.' >&2
    exit 2
fi

cargo fmt --all -- --check
cargo clippy --locked --offline --all-targets -- -D warnings
python -B scripts/check-nextest-selection.py
cargo nextest run --locked --offline --profile fast --retries 0
cargo nextest run --locked --offline --profile fast --retries 0 --release
