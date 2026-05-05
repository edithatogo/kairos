# Test Matrix — 13 CI/CD, Code Quality & Supply Chain

## Required tests

- Root workspace gate: `Cargo.toml`, `rust-toolchain.toml`, and `deny.toml` exist and are used.
- Core CI runs `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace`.
- Docs and release workflows fail when `website/`, `conductor/release-engineering.md`, or the release workflow files are missing.
- Conformance validates fixture structure and expected replay data.
- Package dry-runs and binding CI fail when their own manifests are missing instead of skipping quietly.
- TypeScript binding smoke runs its declared scripts instead of treating them as optional.
- Benchmark and fuzzing workflows fail when the harness directories are missing.
- Benchmark artifact upload fails when the artifact tree is empty.

## CI commands

```bash
test -f Cargo.toml
test -f rust-toolchain.toml
test -f deny.toml
cargo metadata --no-deps --format-version 1
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo nextest run --workspace --all-features
cargo doc --workspace --all-features --no-deps
for f in .github/workflows/*.yml; do test -s "$f"; done
rg -n "future surface; skipping" .github/workflows/ci-bindings.yml .github/workflows/package-dry-run.yml && exit 1 || exit 0
rg -n "No benchmarks yet|No fuzz harness yet|\|\| true" .github/workflows/benchmarks.yml .github/workflows/fuzzing.yml && exit 1 || exit 0
rg -n -- "--if-present|if-no-files-found:\s*ignore" .github/workflows/ci-bindings.yml .github/workflows/benchmarks.yml && exit 1 || exit 0
test -f conductor/tracks.yaml
```
