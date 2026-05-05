#!/usr/bin/env bash
set -euo pipefail
rustup component add rustfmt clippy || true
cargo install cargo-nextest cargo-deny cargo-audit cargo-llvm-cov --locked || true
python -m pip install -U pip || true
python -m pip install -U maturin pytest hypothesis ruff pyarrow || true
npm install -g npm@latest || true
printf '\nKairoECS bootstrap complete. Run: just test\n'
