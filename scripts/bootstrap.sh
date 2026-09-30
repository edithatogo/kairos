#!/usr/bin/env bash
set -euo pipefail
rustup component add rustfmt clippy || true
cargo install just --version "1.50.0" --locked || true
for spec in \
  "cargo-nextest@0.9.146" \
  "cargo-deny@0.20.2" \
  "cargo-audit@0.22.2" \
  "cargo-llvm-cov@0.9.1"; do
  cargo install "${spec%@*}" --version "${spec#*@}" --locked || true
done
python -m pip install --require-hashes -r scripts/bootstrap-python-tools.lock || true
python3 scripts/bootstrap-node-tools/prepare_npm_cli.py --check
npm ci --ignore-scripts --prefix scripts/bootstrap-node-tools
node scripts/bootstrap-node-tools/node_modules/npm/bin/npm-cli.js --prefix website ci || true
printf '\nKairoECS bootstrap complete. Run: just dev-validate\n'
