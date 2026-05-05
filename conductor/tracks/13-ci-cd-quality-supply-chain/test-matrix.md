# Test Matrix — 13 CI/CD, Code Quality & Supply Chain

## Required tests

- Root workspace gate: `Cargo.toml`, `rust-toolchain.toml`, and `deny.toml` exist and are used.
- Core CI runs `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace`.
- Docs and release workflows fail when `website/`, `conductor/release-engineering.md`, or the release workflow files are missing.
- Conformance validates fixture structure and expected replay data.
- Conformance runs the checked-in Node validators, including the Track 07-13 hardening check, without depending on central script edits.
- Package dry-runs and binding CI fail when their own manifests are missing instead of skipping quietly.
- TypeScript binding smoke runs its declared scripts instead of treating them as optional.
- Benchmark smoke runs the offline metadata harness and `kairo-ecs-bench` compile check.
- Fuzzing workflows fail when harness directories are missing.

## CI commands

```bash
test -f Cargo.toml
test -f rust-toolchain.toml
test -f deny.toml
cargo metadata --no-deps --format-version 1
cargo deny check
cargo audit
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo nextest run --workspace --all-features
cargo doc --workspace --all-features --no-deps
for f in .github/workflows/*.yml; do test -s "$f"; done
node tests/conformance/conformance-check.mjs
node tests/conformance/track07_13_hardening_check.mjs
python benches/benchmark_smoke.py
cargo check -p kairo-ecs-bench
test -f .github/dependabot.yml
rg -n 'rust-version = "1\.76"' Cargo.toml
rg -n 'channel = "stable"' rust-toolchain.toml
rg -n 'unknown-registry = "deny"|unknown-git = "deny"' deny.toml
rg -n "future surface; skipping" .github/workflows/ci-bindings.yml .github/workflows/package-dry-run.yml && exit 1 || exit 0
rg -n "No benchmarks yet|No fuzz harness yet|\|\| true" .github/workflows/benchmarks.yml .github/workflows/fuzzing.yml && exit 1 || exit 0
rg -n -- "--if-present|if-no-files-found:\s*ignore" .github/workflows/ci-bindings.yml .github/workflows/benchmarks.yml && exit 1 || exit 0
rg -n "skip ci|ci skip|no ci|skip-checks:\s*true" .github/workflows/ci-skip-guard.yml
test -f conductor/tracks.yaml
```
