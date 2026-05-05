# Handoff — 00 Project Foundation, Governance & Naming

## Summary

Track 00 now reflects the active repository controls rather than bootstrap-era wording. The current control surface is `conductor/status.md`, `conductor/tracks.yaml`, `conductor/implementation-readiness.md`, `scripts/validate_conductor_setup.ps1`, `scripts/validate_track_coverage.ps1`, and the local Track 00 review validator for Tracks 00-06.

## Files changed

- `conductor/tracks/00-project-foundation-governance-naming/validate-track00-06-review.ps1`
- `conductor/tracks/00-project-foundation-governance-naming/handoff.md`
- `conductor/tracks/00-project-foundation-governance-naming/test-matrix.md`

## Contracts consumed

`conductor/workflow.md`, `conductor/contracts/*`, and the repo-level control docs.

## Contracts changed

`conductor/status.md`, `conductor/tracks.yaml`, and `conductor/implementation-readiness.md` are the active foundation controls.

## Tests added

- `scripts/validate_conductor_setup.ps1` and `scripts/validate_track_coverage.ps1` remain the central foundation checks this track relies on.
- `conductor/tracks/00-project-foundation-governance-naming/validate-track00-06-review.ps1` is a dependency-free local guard for this review pass. It verifies Track 00-06 required artifacts, owned implementation path presence, and absence of stale bootstrap phrases in the Track 00-06 markdown files.

## Validation run

- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/00-project-foundation-governance-naming/validate-track00-06-review.ps1`

## Known risks

If the root metadata or GitHub repo structure changes, the validators need to be kept in sync so the foundation does not drift.

## Integration notes

Next implementation step: lock in the repository identity and naming rules against the real root metadata, then keep the validators as the gate for future foundation changes.
