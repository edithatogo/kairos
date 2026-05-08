# Handoff — 16 Release Governance & Maintenance

Last updated: 2026-05-08

## Summary

Implemented the minimal R2 release-governance slice: changelog policy/check,
compatibility and deprecation release rules, release evidence requirements, and
maintenance handoff. The slice stays aligned to Track 15's dry-run packaging
posture and Track 25's compatibility boundary.

## Files changed

`conductor/tracks/16-release-governance-maintenance/plan.md`
`conductor/tracks/16-release-governance-maintenance/test-matrix.md`
`conductor/tracks/16-release-governance-maintenance/handoff.md`
`conductor/maintenance-governance.md`
`CHANGELOG.md`
`.github/workflows/changelog-policy.yml`
`docs/release/release-governance.md`
`docs/release/changelog-policy.md`
`docs/release/compatibility.md`
`docs/release/maintenance-handoff.md`
`docs/release/maintainer-rotation.md`
`docs/release/release-checklist.md`
`docs/release/release-notes.md`

## Contracts consumed

`conductor/workflow.md`
`conductor/release-engineering.md`
`conductor/delivery-readiness-checklist.md`
`conductor/quality-gates.md`
`conductor/contracts/versioning-compatibility.md`
`conductor/tracks/15-packaging-publishing-delivery/handoff.md`

## Contracts changed

None.

## Tests added

The track now uses explicit file-existence checks, required-text checks for the
R2 governance docs, the changelog policy static check definition, and the
conductor setup validator as its baseline gate.

Focused offline validator:

```text
powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/16-release-governance-maintenance/validate-release-governance.ps1
```

## Known risks

The changelog policy is now enforced by `.github/workflows/changelog-policy.yml`,
but the PR diff matcher should be expanded if new public release surfaces are
added.
Release process changes still depend on the GitHub workflow files and registry
conventions staying consistent with the release-engineering notes.
Package publication remains blocked until Track 15 clears naming, registry, and
dry-run evidence.

## Integration notes

Track 15 should continue to own package paths and production publish enablement.
Track 16 owns the release-governance evidence and should be consulted before a
publish job is enabled. Tracks 13, 20, 25, and 28 should treat this track as the
source of release-governance expectations.

## Review-hardening update

Added a track-local release-governance validator that checks the changelog,
compatibility, deprecation, release-note, maintenance handoff, maintainer
rotation, `compatibility-policy`, and `changelog-check` claims against
checked-in docs and the central gate registry.

## Follow-up issues

Expand the changelog-policy workflow matcher when new public release surfaces are added, and keep release workflow dry-run posture aligned with Track 15 packaging gates.
## Phase closeout evidence

Implementation/review pass on 2026-05-08:

- `$conductor-review` findings: no blocking Track 16 findings after the maintainer-rotation output and named gate assertions were added.
- Accepted fixes: added `docs/release/maintainer-rotation.md` with a preview maturity label, wired it into release governance/checklist/handoff docs, and hardened `validate-release-governance.ps1` to prove `compatibility-policy` and `changelog-check` are present in Track 16's `conductor/tracks.yaml` gate block and `conductor/quality-gates.md`.
- Deferred or blocked fixes: no Track 16 blocker remains for `compatibility-policy` or `changelog-check`. The shared phase-closeout validator is blocked by Track 19 handoff evidence outside Track 16 ownership.
- Validation commands:
  - `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\16-release-governance-maintenance\validate-release-governance.ps1`
  - `node tests\conformance\track12_20_evidence_check.mjs`
  - `pwsh -NoProfile -File scripts\validate_conductor_phase_gates.ps1` (blocked by Track 19 handoff evidence, not Track 16)
- Commit SHA: blocked; no commit created in the shared worktree.
- Pushed ref: blocked; no push performed.
- `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: not run because shared phase-closeout is already blocked and the worktree is not clean.
- Next-phase decision: keep Track 16 `In Progress` until the unrelated Track 19 phase-closeout evidence blocker is resolved or waived; Track 16's compatibility and changelog gates are locally satisfied.
