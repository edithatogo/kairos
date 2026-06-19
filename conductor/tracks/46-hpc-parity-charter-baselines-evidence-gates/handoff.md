# Track 46 Handoff

Last updated: 2026-06-19

## Summary

Track 46 creates the shared HPC parity charter and evidence contract. It now
includes an executable evidence-manifest and claim-boundary validator. It does
not implement runtime behavior.

## Files changed

- `conductor/hpc-parity-wave.md`
- `conductor/hpc-evidence/schema.json`
- `conductor/hpc-evidence/manifests/track46-local-scaffold.json`
- `conductor/hpc-evidence/manifests/track46-live-hpc-template.json`
- `conductor/quality-gates.md`
- `conductor/sota-scorecard.md`
- `scripts/validation/validate-hpc-parity-evidence.mjs`
- `conductor/tracks/46-hpc-parity-charter-baselines-evidence-gates/*`

## Contracts consumed

- Track 18 benchmark reproducibility policy.
- Track 26 interoperability standards mapping.
- Track 31 performance regression guard.
- Track 44 code-health floor.

## Contracts changed

Tracks 47-55 must use the live evidence manifest fields before claiming `Done`.
Any real `live-hpc` manifest must record a pushed 40-character commit SHA,
immutable raw artifact path, sha256 checksum, hardware/scheduler/toolchain
metadata, reviewer, date, and `waiver.status: none`.

## Tests added

- `node scripts/validation/validate-hpc-parity-evidence.mjs`

## Known risks

Live hardware, scheduler, and provider evidence is intentionally absent for
Track 46. The `track46-live-hpc-template.json` file is a template, not proof.

## Follow-up issues

- Backfill SOTA baseline source citations in the charter.

## Integration notes

Downstream tracks must treat this charter as the release-claim source of truth.

## Phase closeout evidence

Run `$conductor-review`, record accepted fixes, commit SHA, pushed ref,
`validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`, and the
next-phase decision before advancing this track.
