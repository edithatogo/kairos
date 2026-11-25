#!/usr/bin/env bash
set -euo pipefail
cd /session/source
export PATH=/usr/local/rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu/bin:$PATH
export CARGO_TARGET_DIR=/session/target
export RUSTFLAGS='-C target-feature=+crt-static'
export CARGO_PROFILE_RELEASE_DEBUG=0
test "$(uname -sm)" = "Linux x86_64"
rustc -Vv
cargo -V
cargo build --offline --locked -j 3 --release -p kairo-ecs-core --features numa
release="$CARGO_TARGET_DIR/release"
types=$(find "$release/deps" -type f -name 'libkairo_ecs_types-*.rlib')
test -f "$types"
rustc --edition 2021 -D warnings -C opt-level=3 -C target-feature=+crt-static \
    -L "dependency=$release/deps" --extern "kairo_ecs_core=$release/libkairo_ecs_core.rlib" \
    --extern "kairo_ecs_types=$types" -o /session/scheduler-locality /session/scheduler-locality.rs
sha256sum /session/scheduler-locality /session/scheduler-locality.rs Cargo.lock
