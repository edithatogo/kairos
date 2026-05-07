# Handoff — 16 Release Governance & Maintenance

Last updated: 2026-05-07

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
compatibility, deprecation, release-note, and maintenance handoff claims
against checked-in docs.

## Follow-up issues

Expand the changelog-policy workflow matcher when new public release surfaces are added, and keep release workflow dry-run posture aligned with Track 15 packaging gates.
## Phase closeout evidence

Pending for the next actual phase closeout. Before this track advances, record `$conductor-review` findings, accepted fixes, deferred or blocked fixes, validation commands, cleanup state, commit SHA or explicit push blocker, pushed ref, strict `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` result, and next-phase decision here.