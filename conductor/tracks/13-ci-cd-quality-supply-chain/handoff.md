# Handoff — 13 CI/CD, Code Quality & Supply Chain

## Summary

CI and supply-chain gates now cover core Rust quality, binding smoke workflows, conformance fixture validation, benchmark smoke checks, dependency policy, and workflow security checks. This pass also wires the Track 07-13 hardening validator into the conformance workflow and keeps release/registry actions out of local validation.

The latest pass tightens Track 13 metadata validation so every checked-in `.github/workflows/*.yml` file must have an explicit workflow name, trigger block, and top-level permissions block. It also checks that both `ci-policy.yml` and `workflow-security.yml` inventory every workflow, closing the gap where newer workflows could exist without being named in the policy gates.

A dedicated offline supply-chain gate now lives at `scripts/validate_track13_supply_chain.ps1`. It runs the existing Track 13 metadata validator, verifies `cargo metadata --no-deps --format-version 1`, and executes cargo-deny advisory/source checks plus `cargo audit` only when those tools are installed locally. Missing advisory tools are reported as skipped instead of failing the local gate.

## Files changed

Current R2 slice:

`.github/workflows/conformance.yml`
`.github/workflows/benchmark-smoke.yml`
`renovate.json`
`.github/workflows/actions-security.yml`
`.github/workflows/codeql.yml`
`.github/workflows/ci-policy.yml`
`.github/workflows/ci-skip-guard.yml`
`.github/workflows/workflow-security.yml`
`deny.toml`
`rust-toolchain.toml`
`scripts/validation/validate-track13-metadata.mjs`
`scripts/validate_track13_supply_chain.ps1`
`conductor/tracks/13-ci-cd-quality-supply-chain/test-matrix.md`
`conductor/tracks/13-ci-cd-quality-supply-chain/handoff.md`

Earlier Track 13 pass:

`.github/workflows/ci-core.yml`
`.github/workflows/docs.yml`
`.github/workflows/ci-bindings.yml`
`.github/workflows/package-dry-run.yml`
`.github/workflows/release.yml`
`.github/workflows/release-attestations.yml`
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

Workflow existence checks, Rust metadata checks, cargo-deny/audit gates, Dependabot coverage, CI skip guard checks, offline conformance checks, Track 07-13 hardening, benchmark smoke checks, explicit workflow permissions checks, and dynamic workflow inventory checks are documented in `test-matrix.md`.

The new offline supply-chain gate adds a local PowerShell entrypoint for the Track 13 metadata validator plus `cargo metadata`, with advisory scanners skipped cleanly when unavailable.

## Validation

- `node scripts/validation/validate-track13-metadata.mjs` passed on 2026-05-08.
- `node tests/conformance/track07_13_hardening_check.mjs` passed on 2026-05-08.
- `node tests/conformance/track12_20_evidence_check.mjs` passed on 2026-05-08.
- `node scripts/validation/validate-track13-metadata.mjs` passed on 2026-05-07.
- `pwsh -NoProfile -File scripts/validate_track13_supply_chain.ps1` passed on 2026-05-08.
- `node tests/conformance/track07_13_hardening_check.mjs` passed on 2026-05-07.
- `node tests/conformance/track12_20_evidence_check.mjs` passed on 2026-05-07.
- `pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo` passed on 2026-05-07.
- `git diff --check -- .github/workflows/ci-policy.yml .github/workflows/workflow-security.yml .github/workflows/codeql.yml scripts/validation/validate-track13-metadata.mjs conductor/tracks/13-ci-cd-quality-supply-chain` passed on 2026-05-07 with only line-ending normalization warnings.

## Known risks

Binding, package, benchmark, and fuzz jobs require checked-in manifests or harness directories before their workflow jobs can run successfully. Workflow inventory drift is now a validator failure rather than a manual review-only risk.

## Integration notes

Tracks 07-13, 14, 15, 20, 25, and 28 consume these gates directly. Keep future workflow changes aligned with `conductor/quality-gates.md` and `conductor/delivery-readiness-checklist.md`.

No release, registry, or remote publication side effects were performed.

## Follow-up issues

No additional follow-up issues were recorded by this Conductor hygiene update.
## Phase closeout evidence

`$conductor-review` completed on 2026-05-08 with no blocking Track 13 findings. Accepted fixes: none required in the workflow surface during this closeout pass. Validation commands passed: `node scripts/validation/validate-track13-metadata.mjs`, `node tests/conformance/track07_13_hardening_check.mjs`, `node tests/conformance/track12_20_evidence_check.mjs`, and `pwsh -NoProfile -File scripts\validate_track13_supply_chain.ps1`. `cargo-deny` and `cargo-audit` were unavailable locally and reported as skipped by the Track 13 supply-chain gate. Git cleanup state: dirty because local Conductor closeout/status edits are pending commit. Commit SHA: `5dd1937566898b2e028ac61dab1e9dd173e6d919`; pushed ref: `origin/main`. Strict cleanup gate `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` remains pending until these local closeout edits are committed. Next-phase decision: Track 13 is Done for the current CI/CD and supply-chain scaffold; future mandatory advisory scanner installation or release hardening belongs in Track 20 or a scoped follow-up.

## D1.2 toolchain policy update — 2026-09-28

The root `rust-toolchain.toml`, mise selector, template, core Rust workflows and
Windows setup helper now align to exact Rust 1.98.1. A separate core workflow
job exercises the advertised Rust 1.76 default-feature workspace floor. Track
30's workflow explicitly selects its stable/beta rustup toolchains for exact
version validation; the beta `1.99.0-beta.8` snapshot is advisory and its
workspace compile job is non-blocking. The Kairos owner approved this policy
direction. No package manifests or runtime dependencies changed. local Conductor review passed with no blocking findings; hosted CI remains
unverified until the branch is pushed under D2; see parent `conductor/evidence/d1.2-
compatibility-assessment-20260928.md` for test logs and hashes.


Track 13 supply-chain validation for D1.2 initially found RUSTSEC-2026-0204 in
the Criterion/Rayon benchmark-only graph. Kairos lock-only pin was advanced from
`crossbeam-epoch 0.9.18` to patched `0.9.20` (declared Rust floor 1.61). The
first rerun skipped audit because the script searched for a standalone
`cargo-audit` executable even though Cargo could invoke the installed plugin.
The validator now probes `cargo audit --version`; cargo-audit 0.22.1 and the
complete Track 13 supply-chain script pass. Captured receipt and lock/test hashes
are in parent `conductor/evidence/d1.2-track13-security-remediation-20260928.md`.
