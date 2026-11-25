set -euo pipefail
stage=$1
cd "$stage/frontier-source"
export CARGO_TARGET_DIR="$stage/frontier-target"
export CARGO_PROFILE_RELEASE_DEBUG=1
export CARGO_PROFILE_RELEASE_SPLIT_DEBUGINFO=unpacked
build() {
    name=$1
    rustup run 1.99.1 cargo bench --locked --offline -p kairo-ecs-pdes --features pdes-worker-pool --bench production --no-run --message-format=json > "$stage/$name-build.jsonl" 2> "$stage/$name-build.log"
    binary=$(jq -er 'select(.target.name=="production" and .executable!=null)|.executable' "$stage/$name-build.jsonl")
    cp "$binary" "$stage/$name"
}
build frontier-base
git apply --check "$stage/one-pass-frontiers.patch"
git apply "$stage/one-pass-frontiers.patch"
build frontier-candidate
shasum -a 256 "$stage/frontier-base" "$stage/frontier-candidate" "$stage/"*.patch > "$stage/frontier-checksums.txt"
