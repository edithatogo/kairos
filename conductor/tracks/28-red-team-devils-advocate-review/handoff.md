# Handoff: Track 28 Red Team & Devil's Advocate Review

Last updated: 2026-05-08

## Summary

Captured the adversarial release-risk ledger so release planning can distinguish genuine blockers from general concerns.
This pass tightened the ledger into claim-versus-capability rows with evidence paths, owners, stage impact, blocker rubric, freshness rules, and validation commands.

## Files changed

`reviews/red-team-report.md`
`conductor/red-team-review.md`
`conductor/devils-advocate-review.md`
`conductor/delivery-readiness-checklist.md`
`conductor/tracks/28-red-team-devils-advocate-review/handoff.md`
`conductor/tracks/28-red-team-devils-advocate-review/claim-capability-ledger.json`
`conductor/tracks/28-red-team-devils-advocate-review/risk-register.md`
`conductor/tracks/28-red-team-devils-advocate-review/test-matrix.md`

## Contracts consumed

`conductor/delivery-readiness-checklist.md`, `conductor/quality-gates.md`, `conductor/release-engineering.md`
`conductor/contracts/core-contract.md`
`conductor/contracts/ffi-contract.md`
`conductor/contracts/arrow-schema-contract.md`
`conductor/contracts/conformance-contract.md`
`conductor/contracts/versioning-compatibility.md`
`packaging/release-package-manifest.json`
`conformance/fixtures/manifest.json`

## Release gates affected

Red-team signoff, blocker closure, and claim-versus-capability review now sit on the release path before beta, RC, or 1.0.
RC and 1.0 release-artifact claims are blocked unless the artifact manifest, checksums, SBOM, and provenance evidence exist for the target release train.

## Validation evidence

| Command | Result |
|---|---|
| `Test-Path -LiteralPath 'benches/benchmark-plan.md'; Test-Path -LiteralPath 'conformance/fixtures/manifest.json'; Test-Path -LiteralPath 'docs/release/release-checklist.md'; Test-Path -LiteralPath 'docs/release/compatibility.md'; Test-Path -LiteralPath 'packaging/release-package-manifest.json'; Test-Path -LiteralPath 'dist/release-artifact-manifest.json'; Test-Path -LiteralPath 'dist/SHA256SUMS'; Test-Path -LiteralPath 'dist/sbom.spdx.json'` | `True True True True True True True False` on 2026-05-08; SBOM/provenance absence remains an RC/1.0 blocker for artifact trust claims |
| `Get-Content -LiteralPath 'conformance/fixtures/manifest.json'` | Ready fixture IDs are scheduler ordering, scheduler cancellation, RNG reproducibility, zero-delay guard, and VVUQ replay; DES/ABM/hybrid/Arrow/FFI fixture IDs remain planned |
| `Get-Content -LiteralPath 'packaging/release-package-manifest.json'` | Package release stage is `r2-dry-run`; `production_publish_enabled` is `false` |
| `$ledger = Get-Content -Raw -LiteralPath 'conductor/tracks/28-red-team-devils-advocate-review/claim-capability-ledger.json' \| ConvertFrom-Json; ...; $ledger.entries.Count` | Parsed successfully, freshness date exists, all blocker/warning rows have owners, and entry count is `10` |

## Risks and unresolved questions

The main risk is stale red-team findings being treated as current during release planning.
The concrete unresolved blocker is now SBOM/provenance evidence: `dist/release-artifact-manifest.json` and `dist/SHA256SUMS` exist after the 2026-05-08 local dry-run, but `dist/sbom.spdx.json` is absent and hosted attestation workflows are failing before job steps start. Release notes must not claim SBOM, provenance, or attestation evidence until those outputs exist for the target release train.

## Contracts changed

The claim-capability ledger now carries release-impact rows with evidence paths, owner fields, stage impact, blocker rubric, and freshness expectations.

## Tests added

The current checks cover evidence-path existence, fixture manifest readiness, dry-run package-manifest state, and JSON parsing of the claim-capability ledger.

## Known risks

Red-team evidence can become stale before release planning, and SBOM/provenance evidence is still absent for artifact trust claims.

## Follow-up issues

Run the release dry-run plus SBOM/provenance workflow or local SBOM tool before RC or 1.0 artifact trust claims are allowed.

## Integration notes

Release notes must keep artifact, checksum, SBOM, provenance, and production-readiness claims blocked until the evidence paths in this track exist for the target release train.
## Phase closeout evidence

`$conductor-review` was run as an implementation review for Track 28 on 2026-05-08. Accepted fixes were applied inside Track 28-owned paths: the ledger freshness date was refreshed, the local R2 artifact/checksum evidence was distinguished from missing SBOM/provenance evidence, a dedicated `validate-track28-redteam.ps1` no-critical-release-blockers gate was added, and Track 28 status/phase-closeout entries were synchronized.

Validation commands recorded for this pass:

- `Test-Path -LiteralPath 'benches/benchmark-plan.md'; Test-Path -LiteralPath 'conformance/fixtures/manifest.json'; Test-Path -LiteralPath 'docs/release/release-checklist.md'; Test-Path -LiteralPath 'docs/release/compatibility.md'; Test-Path -LiteralPath 'packaging/release-package-manifest.json'; Test-Path -LiteralPath 'dist/release-artifact-manifest.json'; Test-Path -LiteralPath 'dist/SHA256SUMS'; Test-Path -LiteralPath 'dist/sbom.spdx.json'`
- `pwsh -NoProfile -File conductor/tracks/28-red-team-devils-advocate-review/validate-track28-redteam.ps1` passed with one recorded warning for missing `dist/sbom.spdx.json`.
- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` was rerun. Latest result is blocked outside Track 28 because Track 15 and Track 27 are `In Review` without phase-closeout ledger entries in the current shared worktree.
- `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` passed.

Current cleanup state: commit SHA and pushed ref are blocked because this shared worktree contains Track 28 edits plus possible unrelated multi-worker state; strict `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` is not run until the slice can be committed/pushed or the worktree is otherwise clean. Next-phase decision: Track 28 is In Review with no unresolved Critical release blockers; stage-scoped RC/1.0 blockers remain recorded for SBOM/provenance and unsupported overclaims.
