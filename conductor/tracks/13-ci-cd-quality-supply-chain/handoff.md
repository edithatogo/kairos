# Handoff — 13 CI/CD, Code Quality & Supply Chain

## Summary

CI and supply-chain gates now cover `cargo fmt`, `cargo clippy`, `cargo test`, docs builds, release attestations, and the package dry-run workflows that feed Tracks 12, 14, 15, 20, 25, and 28. The binding, package, benchmark, and fuzz workflows now fail when their owned manifests or harness directories are missing instead of quietly skipping the job.

## Files changed

Current R2 slice:

`.github/dependabot.yml`
`.github/workflows/actions-security.yml`
`.github/workflows/ci-policy.yml`
`.github/workflows/ci-skip-guard.yml`
`.github/workflows/workflow-security.yml`
`deny.toml`
`rust-toolchain.toml`
`conductor/tracks/13-ci-cd-quality-supply-chain/test-matrix.md`
`conductor/tracks/13-ci-cd-quality-supply-chain/handoff.md`

Earlier Track 13 pass:

`.github/workflows/ci-core.yml`
`.github/workflows/conformance.yml`
`.github/workflows/docs.yml`
`.github/workflows/ci-bindings.yml`
`.github/workflows/package-dry-run.yml`
`.github/workflows/release.yml`
`.github/workflows/release-attestations.yml`
`.github/workflows/benchmark-smoke.yml`
`.github/workflows/benchmarks.yml`
`.github/workflows/fuzzing.yml`
`.github/workflows/docs-quality.yml`
`.github/workflows/nightly.yml`
`scripts/validate_conductor_setup.ps1`
`conductor/tracks/13-ci-cd-quality-supply-chain/test-matrix.md`

## Contracts consumed

`conductor/workflow.md`
`conductor/contracts/conformance-contract.md`
`conductor/contracts/package-release-contract.md`
`conductor/contracts/supply-chain-contract.md`

## Contracts changed

None.

## Tests added

Workflow existence checks, Rust metadata checks, cargo-deny/audit gates, Dependabot coverage, and CI skip guard checks are documented in `test-matrix.md`.

## Known risks

Binding, package, benchmark, and fuzz jobs require checked-in manifests or harness directories before their workflow jobs can run successfully.

## Integration notes

Tracks 12, 14, 15, 20, 25, and 28 consume these gates directly. Keep future workflow changes aligned with `conductor/quality-gates.md` and `conductor/delivery-readiness-checklist.md`.

