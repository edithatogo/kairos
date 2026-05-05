# Handoff: Track 28 Red Team & Devil's Advocate Review

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
| `Test-Path -LiteralPath 'benches/benchmark-plan.md'; Test-Path -LiteralPath 'conformance/fixtures/manifest.json'; Test-Path -LiteralPath 'docs/release/release-checklist.md'; Test-Path -LiteralPath 'docs/release/compatibility.md'; Test-Path -LiteralPath 'packaging/release-package-manifest.json'; Test-Path -LiteralPath 'dist/release-artifact-manifest.json'` | `True True True True True False`; artifact manifest absence is an RC/1.0 blocker for artifact claims |
| `Get-Content -LiteralPath 'conformance/fixtures/manifest.json'` | Ready fixture IDs are limited to scheduler ordering, scheduler cancellation, RNG reproducibility, and VVUQ replay; DES/ABM/hybrid/Arrow/FFI fixture IDs remain planned |
| `Get-Content -LiteralPath 'packaging/release-package-manifest.json'` | Package release stage is `r2-dry-run`; `production_publish_enabled` is `false` |
| `$ledger = Get-Content -Raw -LiteralPath 'conductor/tracks/28-red-team-devils-advocate-review/claim-capability-ledger.json' \| ConvertFrom-Json; ...; $ledger.entries.Count` | Parsed successfully, freshness date exists, all blocker/warning rows have owners, and entry count is `10` |

## Risks and unresolved questions

The main risk is stale red-team findings being treated as current during release planning.
The concrete unresolved blocker is release-artifact evidence: `dist/release-artifact-manifest.json` was absent in the focused check, so release notes must not claim attached artifacts, checksums, SBOM, or provenance until the release dry-run produces them.
