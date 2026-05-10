# Handoff: Track 28 Red Team & Devil's Advocate Review

Last updated: 2026-05-11

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
The earlier SBOM/provenance gap has been closed on the current tree: the local release-artifact, checksum, and SBOM paths now exist, and the Track 28 gate passes cleanly.
The remaining closeout blocker is workspace cleanliness outside Track 28-owned files, so strict git closeout still cannot be certified until the shared tree is clean.

## Contracts changed

The claim-capability ledger now carries release-impact rows with evidence paths, owner fields, stage impact, blocker rubric, and freshness expectations.

## Tests added

The current checks cover evidence-path existence, fixture manifest readiness, dry-run package-manifest state, and JSON parsing of the claim-capability ledger.

## Known risks

Red-team evidence can become stale before release planning, and the shared worktree still needs to be cleaned before strict closeout can be claimed.

## Follow-up issues

Re-run the strict closeout gate after unrelated Track 39 edits are isolated or committed, then refresh the release-trust checks if the release train changes.

## Integration notes

Release notes should keep artifact, checksum, SBOM, provenance, and production-readiness claims anchored to the current release-train evidence; Track 28 itself is no longer the blocker.
## Phase closeout evidence

`$conductor-review` was run as an implementation review for Track 28 on 2026-05-08. Accepted fixes were applied inside Track 28-owned paths: the ledger freshness date was refreshed, the local R2 artifact/checksum evidence was distinguished from missing SBOM/provenance evidence, a dedicated `validate-track28-redteam.ps1` no-critical-release-blockers gate was added, and Track 28 status/phase-closeout entries were synchronized.

Validation commands recorded for this pass:

- `Test-Path -LiteralPath 'benches/benchmark-plan.md'; Test-Path -LiteralPath 'conformance/fixtures/manifest.json'; Test-Path -LiteralPath 'docs/release/release-checklist.md'; Test-Path -LiteralPath 'docs/release/compatibility.md'; Test-Path -LiteralPath 'packaging/release-package-manifest.json'; Test-Path -LiteralPath 'dist/release-artifact-manifest.json'; Test-Path -LiteralPath 'dist/SHA256SUMS'; Test-Path -LiteralPath 'dist/sbom.spdx.json'`
- `pwsh -NoProfile -File conductor/tracks/28-red-team-devils-advocate-review/validate-track28-redteam.ps1` passed on 2026-05-11 with 0 errors and 0 warnings; the stage-scoped release blockers remain recorded for planning only.
- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` was last recorded as blocked outside Track 28 because Track 15 and Track 27 were `In Review` without phase-closeout ledger entries in the current shared worktree.
- `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` passed.

Current cleanup state: strict `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` still fails because the shared worktree contains unrelated tracked changes outside Track 28-owned files. Next-phase decision: Track 28 has no unresolved Critical release blockers; the remaining blocker is workspace cleanliness, not Track 28 evidence.
