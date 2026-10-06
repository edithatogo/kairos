# Test Matrix — 13 CI/CD, Code Quality & Supply Chain

## Required tests

- Root workspace gate: `Cargo.toml`, `rust-toolchain.toml`, and `deny.toml` exist and are used.
- Core CI installs pinned Rust tool binaries from checksum-verified, SHA-pinned GitHub releases and disables source-build fallbacks.
- Core CI runs `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and one coverage-instrumented nextest pass.
- Required Rust core CI runs stable verification, a Rust 1.76 locked library/binary compile, and a Rust 1.77 wasm-target compile; a stable aggregate check requires all three jobs.
- The Rust 1.76 lane checks `cargo +1.76.0 check --workspace --exclude kairo-ecs-wasm --exclude kairo-ecs-calibration --exclude kairo-ecs-arrow-io --lib --bins --all-features --locked`. The separately declared Wasm binding floor is 1.77 and checks `cargo +1.77.0 check --locked --manifest-path crates/kairo-ecs-wasm/Cargo.toml --features wasm-export --target wasm32-unknown-unknown`.
- Core CI runs `cargo test --doc --workspace --all-features` once as a distinct test class; this complements nextest without repeating unit and integration tests.
- Core CI runs the workspace nextest suite once under coverage instrumentation, then filters the same LCOV report to core scheduler production sources and enforces a 90% line-coverage floor; a minimal-permission job uploads the report with OIDC only on trusted main pushes. PRs use the Rust core check as their merge gate and do not receive OIDC.
- `just test` creates the workspace LCOV report while running tests once. `just check-coverage` reads the last report without rerunning tests. `just ci` is the local equivalent of the core Rust formatting, lint, test/coverage, docs, and dependency-audit lane.
- The weekly/manual mutation lane targets `kairo-ecs-core`, is limited to 25 minutes and two workers, fails on uncaught mutants, and retains `mutants.out` for 30 days.
- Docs and release workflows fail when `website/`, `conductor/release-engineering.md`, or the release workflow files are missing.
- Conformance validates fixture structure and expected replay data.
- Conformance runs the checked-in Node validators, including the Track 07-13 hardening check, without depending on central script edits.
- Conformance runs the Track 12-20 evidence check so release, citation, benchmark, and supply-chain evidence cannot be skipped silently.
- Track 13 metadata alignment validates `conductor/tracks.yaml` without changing track statuses and maps `workflow-presence`, `cargo-metadata`, and `dependency-policy` to checked-in workflow evidence.
- Track 13 metadata alignment dynamically inventories every checked-in `.github/workflows/*.yml` file, requires an explicit workflow `name`, `on`, and top-level `permissions` block, and verifies both `ci-policy.yml` and `workflow-security.yml` list every workflow.
- The offline supply-chain gate runs `scripts/validate_track13_supply_chain.ps1`, which executes the Track 13 metadata validator, `cargo metadata --no-deps --format-version 1`, cargo-deny advisory/source checks, and `cargo audit` when those scanners are installed locally.
- The conformance job runs `tests/conformance/quality-frontier-drift-check.mjs` against pass, pending-integration, drift, and unavailable GitHub evidence fixtures.
- The drift receipt retains all five required ruleset contexts while expecting only the four push-capable checks on a main SHA; it verifies the PR-only skip-guard workflow trigger, job name, and exact main-branch workflow blob.
- Root `codecov.yml` requests a project status for the `rust-core` coverage flag; hosted acceptance still requires an exact-SHA `codecov/project` status after a trusted-main upload.
- The trusted-main Codecov upload job checks out the repository before downloading `lcov.info`, so Codecov can read the root status configuration; the Track 13 metadata validator enforces this ordering.
- `just quality-drift` performs a read-only live settings readback, binds Actions results to the resolved default-branch SHA, and writes a JSON receipt to `artifacts/quality-frontier-drift.json`. Drift, pending integration, or unavailable APIs remain non-pass exit states.
- Validate Conductor runs on both `ubuntu-latest` and `windows-latest` so PowerShell and Node validators are exercised cross-platform.
- Package dry-runs and binding CI fail when their own manifests are missing instead of skipping quietly.
- Package dry-runs validate package artifacts while language tests run only in binding CI; Python package build/twine checks run once on Python 3.14, with the 3.10–3.14 compatibility matrix retained in binding CI.
- npm package validation builds once during `npm ci` preparation and uses `npm pack --dry-run --ignore-scripts` to inspect the resulting package without rerunning prepack; NuGet package validation packs the library without repeating the net10 test project.
- TypeScript binding smoke runs its declared scripts instead of treating them as optional.
- Benchmark smoke runs the offline metadata harness and `kairo-ecs-bench` compile check.
- DST engine tests are excluded because `SimTime` uses logical integer ticks and the engine has no civil-time conversion; any future timestamp adapter must add DST boundary tests at that boundary.
- Multithread stress tests are excluded until the single-thread scheduler and in-memory `&mut self` PDES transport are replaced by a concurrent API; thread-safety guarantees must accompany that API.
- Fuzzing runs the checked-in scheduler request harness for 60 seconds weekly or on demand, enforces a 2 GiB RSS cap, and preserves crash artifacts for 30 days.

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
cargo llvm-cov nextest --workspace --all-features --lcov --output-path lcov.info
node scripts/validation/check-core-coverage.mjs lcov.info
cargo doc --workspace --all-features --no-deps
pwsh -NoProfile -File scripts/validate_track13_supply_chain.ps1
for f in .github/workflows/*.yml; do test -s "$f"; done
node tests/conformance/conformance-check.mjs
node tests/conformance/track07_13_hardening_check.mjs
node tests/conformance/track12_20_evidence_check.mjs
node scripts/validation/validate-track13-metadata.mjs
cargo +nightly fuzz run scheduler_requests -- -max_total_time=60 -rss_limit_mb=2048
python benches/benchmark_smoke.py
cargo check -p kairo-ecs-bench
test -f renovate.json
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
