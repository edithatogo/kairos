set -euo pipefail
cd /session
uname -sm > linux-toolchain.txt
rustc -Vv >> linux-toolchain.txt
cargo -Vv >> linux-toolchain.txt
grep -q '^Linux x86_64$' linux-toolchain.txt
grep -q '^rustc 1\.99\.1 ' linux-toolchain.txt
export RUSTC="$(command -v rustc)"
export RUSTDOC="$(command -v rustdoc)"
export CARGO_TARGET_DIR=/session/linux-target
export RUSTFLAGS='-C target-feature=+crt-static'
export CARGO_PROFILE_RELEASE_DEBUG=0
for arm in base published candidate; do
    (cd "/session/$arm" && cargo clean --release -p kairo-ecs-pdes && cargo bench --locked --offline -j 3 -p kairo-ecs-pdes --features pdes-worker-pool --bench production --no-run --message-format=json) > "/session/$arm-build.jsonl" 2> "/session/$arm-build.log"
    binary="$(sed -n '/"name":"production"/s/.*"executable":"\([^"]*\)".*/\1/p' "/session/$arm-build.jsonl" | tail -1)"
    test -x "$binary"
    cp "$binary" "/session/frontier-$arm"
done
sha256sum frontier-{base,published,candidate} > linux-binaries.sha256
