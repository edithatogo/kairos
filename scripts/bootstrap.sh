#!/usr/bin/env bash
set -euo pipefail
rustup component add rustfmt clippy || true
for spec in \
  "just@1.42.4" \
  "cargo-nextest@0.9.133" \
  "cargo-deny@0.18.9" \
  "cargo-audit@0.22.1" \
  "cargo-llvm-cov@0.6.18"; do
  cargo install "${spec%@*}" --version "${spec#*@}" --locked || true
done
python -m pip install -U pip==25.0.1 || true
python -m pip install -U maturin==1.9.6 pytest==8.3.5 hypothesis==6.131.0 ruff==0.11.13 pyarrow==24.0.0 || true
npm install -g npm@11.6.2 || true
npm --prefix website ci || true
printf '\nKairoECS bootstrap complete. Run: just dev-validate\n'
