#!/usr/bin/env sh
set -eu
# Requires the toolchain declared in rust-toolchain.toml. No network or dependencies.
# Usa cargo-nextest cuando esta instalado (mismos tests y resultados que `cargo test`,
# ejecutados en paralelo entre binarios); si no, cae a `cargo test`. El repositorio no
# tiene doctests, asi que nextest no omite nada. `NEXO_VALIDATE_RUNNER=cargo-test` fuerza
# el runner clasico.
cargo fmt --all -- --check
cargo clippy --locked --offline --all-targets -- -D warnings
if cargo nextest --version >/dev/null 2>&1; then
    python -B scripts/check-nextest-selection.py
else
    python -B scripts/check-nextest-selection.py --cargo-test
fi
if [ "${NEXO_VALIDATE_RUNNER:-auto}" != "cargo-test" ] && cargo nextest --version >/dev/null 2>&1; then
    cargo nextest run --locked --offline --profile default --retries 0
    cargo nextest run --locked --offline --profile default --retries 0 --release
else
    cargo test --locked --offline
    cargo test --locked --offline --release
fi
python -B scripts/f5-stack65.py
