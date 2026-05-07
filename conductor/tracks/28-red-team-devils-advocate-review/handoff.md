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

Pending for the next actual phase closeout. Before this track advances, record `$conductor-review` findings, accepted fixes, deferred or blocked fixes, validation commands, cleanup state, commit SHA or explicit push blocker, pushed ref, strict `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` result, and next-phase decision here.
