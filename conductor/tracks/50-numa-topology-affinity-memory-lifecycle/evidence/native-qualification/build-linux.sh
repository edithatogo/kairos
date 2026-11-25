#!/usr/bin/env bash
set -euo pipefail
cd /session/linux-source
export PATH=/usr/local/rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu/bin:$PATH
export CARGO_TARGET_DIR=/session/linux-target
export RUSTFLAGS='-C target-feature=+crt-static'
export CARGO_PROFILE_RELEASE_DEBUG=0
uname -sm
rustc -Vv
cargo -V
test "$(uname -sm)" = "Linux x86_64"
rustc -V | grep -q '^rustc 1\.99\.1 '
cargo build --locked --offline -j 3 --release -p kairo-ecs-core --features numa
release="$CARGO_TARGET_DIR/release"
rustc --edition 2021 -D warnings -C opt-level=3 -C target-feature=+crt-static \
    -L "dependency=$release/deps" --extern "kairo_ecs_core=$release/libkairo_ecs_core.rlib" \
    -o /session/hwloc-readback-linux /session/hwloc-readback.rs
types=$(find "$release/deps" -type f -name 'libkairo_ecs_types-*.rlib')
test -f "$types"
rustc --edition 2021 -D warnings -C opt-level=3 -C target-feature=+crt-static \
    -L "dependency=$release/deps" --extern "kairo_ecs_core=$release/libkairo_ecs_core.rlib" \
    --extern "kairo_ecs_types=$types" -o /session/scheduler-contention-linux /session/scheduler-contention.rs
cargo test --locked --offline --target x86_64-unknown-linux-gnu -j 3 -p kairo-ecs-core --features numa --test linux_numa --no-run --message-format=json > /session/linux-numa-build.jsonl
executable=$(sed -n '/"name":"linux_numa"/s/.*"executable":"\([^"]*\)".*/\1/p' /session/linux-numa-build.jsonl)
test -f "$executable"
cp "$executable" /session/linux-numa-tests
cd /session
sha256sum hwloc-readback-linux scheduler-contention-linux linux-numa-tests > linux-executables.sha256
