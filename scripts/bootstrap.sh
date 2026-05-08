#!/usr/bin/env bash
set -euo pipefail
rustup component add rustfmt clippy || true
for tool in just cargo-nextest cargo-deny cargo-audit cargo-llvm-cov; do
  cargo install "$tool" --locked || true
done
python -m pip install -U pip || true
python -m pip install -U maturin pytest hypothesis ruff pyarrow || true
npm install -g npm@latest || true
npm --prefix website ci || true
printf '\nKairoECS bootstrap complete. Run: just dev-validate\n'
