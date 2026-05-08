# Test Matrix — 13 CI/CD, Code Quality & Supply Chain

## Required tests

- Root workspace gate: `Cargo.toml`, `rust-toolchain.toml`, and `deny.toml` exist and are used.
- Core CI runs `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace`.
- Docs and release workflows fail when `website/`, `conductor/release-engineering.md`, or the release workflow files are missing.
- Conformance validates fixture structure and expected replay data.
- Conformance runs the checked-in Node validators, including the Track 07-13 hardening check, without depending on central script edits.
- Conformance runs the Track 12-20 evidence check so release, citation, benchmark, and supply-chain evidence cannot be skipped silently.
- Track 13 metadata alignment validates `conductor/tracks.yaml` without changing track statuses and maps `workflow-presence`, `cargo-metadata`, and `dependency-policy` to checked-in workflow evidence.
- Track 13 metadata alignment dynamically inventories every checked-in `.github/workflows/*.yml` file, requires an explicit workflow `name`, `on`, and top-level `permissions` block, and verifies both `ci-policy.yml` and `workflow-security.yml` list every workflow.
- The offline supply-chain gate runs `scripts/validate_track13_supply_chain.ps1`, which executes the Track 13 metadata validator, `cargo metadata --no-deps --format-version 1`, cargo-deny advisory/source checks, and `cargo audit` when those scanners are installed locally.
- Validate Conductor runs on both `ubuntu-latest` and `windows-latest` so PowerShell and Node validators are exercised cross-platform.
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
pwsh -NoProfile -File scripts/validate_track13_supply_chain.ps1
for f in .github/workflows/*.yml; do test -s "$f"; done
node tests/conformance/conformance-check.mjs
node tests/conformance/track07_13_hardening_check.mjs
node tests/conformance/track12_20_evidence_check.mjs
node scripts/validation/validate-track13-metadata.mjs
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

## 2026-05-08 validation notes

- Passed: `node scripts/validation/validate-track13-metadata.mjs`.
- Passed: `node tests/conformance/track07_13_hardening_check.mjs`.
- Passed: `node tests/conformance/track12_20_evidence_check.mjs`.
- Passed: `pwsh -NoProfile -File scripts\validate_track13_supply_chain.ps1`.
- Supply-chain scanner note: `cargo-deny` and `cargo-audit` were not installed locally; `scripts\validate_track13_supply_chain.ps1` reported both advisory scanner lanes as skipped rather than failed, while the Track 13 metadata validator and `cargo metadata --no-deps --format-version 1` passed.

## 2026-05-07 validation notes

- Passed: `node scripts/validation/validate-track13-metadata.mjs`.
- Passed: `node tests/conformance/track07_13_hardening_check.mjs`.
- Passed: `node tests/conformance/track12_20_evidence_check.mjs`.
- Passed: `pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo`.
- Passed: `git diff --check -- .github/workflows/ci-policy.yml .github/workflows/workflow-security.yml .github/workflows/codeql.yml scripts/validation/validate-track13-metadata.mjs conductor/tracks/13-ci-cd-quality-supply-chain` with only line-ending normalization warnings.
- Added coverage: `validate-track13-metadata.mjs` now catches missing top-level workflow permissions and workflow inventory drift for every checked-in workflow file.
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.
