# 44 Code and Repository Health >9.5 - handoff.md

Last updated: 2026-05-19

## Summary

Track 44 adds the code/repo health scorecard and makes `>= 9.5` a publication gate.

## Files changed

- `conductor/code-health.md`
- `.github/workflows/code-health.yml`
- `scripts/validation/validate-code-health.mjs`
- `conductor/tracks/44-code-repository-health-95/*`

## Contracts consumed

- CI and security gate evidence from Tracks 13 and 20
- Docs and learning coverage from Track 41
- Compatibility and release governance from Tracks 25 and 16

## Contracts changed

Track 42 and Track 43 must pass the health floor before production publication.

## Tests added

- `node scripts/validation/validate-code-health.mjs`

## Known risks

The initial validator is evidence-presence based. Future work should add live GitHub API score ingestion for PR/issue queues and branch hygiene.

## Follow-up issues

- Add live Scorecard score ingestion.
- Add trend storage for health score over releases.
- Add package-specific health sub-scores.

## Integration notes

The score threshold is intentionally high and release-gating.

## Phase closeout evidence

`$conductor-review` must be run before promotion. Record accepted fixes, commit SHA, pushed ref, `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`, and next-phase decision here during closeout.
