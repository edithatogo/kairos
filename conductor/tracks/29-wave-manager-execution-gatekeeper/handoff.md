# Handoff: Track 29 Wave Manager & Execution Gatekeeper

Handoff date: 2026-05-08.

## Summary

Implemented a local Wave Manager validator that derives wave membership,
dependency closure, and the critical-path heatmap from the current
`conductor/tracks.yaml` inventory. The gate definitions now name the concrete
PowerShell command for blocking validation and report-only evidence generation.
The wave policy is now graph-derived and enforced through `wave-progression-check`
and `dependency-closure-check`.

The current dependency graph derives Waves 0-6, not the prior fixed 0-5 model.
Track 39 is Wave 6 because it depends on Track 15, which is Wave 5.

Track-scoped gate mode was added for implementation closeout evidence:
`validate-wave-gates.ps1 -TrackId 29` validates Track 29's own direct and
transitive dependencies while preserving the default global release gate.

## Files changed

- `conductor/wave-policy.md`
- `conductor/gates/wave-progression-check.yml`
- `conductor/gates/dependency-closure-check.yml`
- `conductor/tracks/29-wave-manager-execution-gatekeeper/validate-wave-gates.ps1`
- `conductor/tracks/29-wave-manager-execution-gatekeeper/spec.md`
- `conductor/tracks/29-wave-manager-execution-gatekeeper/plan.md`
- `conductor/tracks/29-wave-manager-execution-gatekeeper/test-matrix.md`
- `conductor/tracks/29-wave-manager-execution-gatekeeper/risk-register.md`
- `conductor/tracks/29-wave-manager-execution-gatekeeper/handoff.md`

## Contracts consumed

- `conductor/tracks.yaml`: canonical track IDs, statuses, owners, dependencies, and required gates.
- `conductor/tracks/`: required six-artifact track directory surface.
- `conductor/gates/`: local gate definition surface for Track 29 controls.

## Contracts changed

- `wave-progression-check` now has a concrete local command and report-only command.
- `dependency-closure-check` now has a concrete local command and report-only command.
- `conductor/wave-policy.md` now defines waves as topological dependency depth rather than a fixed 0-5 hand list.
- Track 29 owns `validate-wave-gates.ps1` as the local implementation for both gate definitions.

## Tests added

New local validator:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\29-wave-manager-execution-gatekeeper\validate-wave-gates.ps1
```

Report-only evidence command:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\29-wave-manager-execution-gatekeeper\validate-wave-gates.ps1 -ReportOnly
```

## Validation Evidence

| Command | Result | Evidence |
|---|---|---|
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate_conductor_dag.ps1` | Pass | Parsed 41 tracks and 47 agents; 0 errors, 0 warnings. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate_conductor_artifacts.ps1` | Pass | Found 41 track directories; 0 errors, 0 warnings, 0 info. |
| `powershell -NoProfile -ExecutionPolicy Bypass -Command "Get-Content -LiteralPath 'conductor\gates\wave-progression-check.yml','conductor\gates\dependency-closure-check.yml' | Out-Null; Get-Content -LiteralPath 'conductor\wave-policy.md' | Out-Null; 'docs-readable'"` | Pass | Returned `docs-readable`. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\29-wave-manager-execution-gatekeeper\validate-wave-gates.ps1 -ReportOnly` | Pass | Reported 41 tracks, Waves 0-6, heatmap led by Tracks 00, 01, 12, 26, 04. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\29-wave-manager-execution-gatekeeper\validate-wave-gates.ps1 -TrackId 29` | Pass | Track 29 has direct dependency 00 `Done` and no unresolved transitive dependency blockers. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\29-wave-manager-execution-gatekeeper\validate-wave-gates.ps1` | Fail as designed | Exit 1 with 16 direct dependency blockers and 25 transitive dependency blockers outside Track 29. |

## Current Wave Membership

| Wave | Tracks |
|---:|---|
| 0 | 00 |
| 1 | 01, 12, 13, 14, 19, 26, 27, 28, 29 |
| 2 | 02, 03, 04, 05, 17, 18, 20, 30, 32, 34, 40 |
| 3 | 21, 22, 23, 25, 31, 35 |
| 4 | 06, 07, 08, 09, 10, 11, 16, 36, 37 |
| 5 | 15, 24, 33, 38 |
| 6 | 39 |

## Critical-Path Heatmap

| Rank | Track | Wave | Direct dependents | Transitive dependents | Current status |
|---:|---:|---:|---:|---:|---|
| 1 | 00 | 0 | 13 | 40 | Done |
| 2 | 01 | 1 | 10 | 28 | Done |
| 3 | 12 | 1 | 14 | 21 | Done |
| 4 | 26 | 1 | 4 | 20 | In Review |
| 5 | 04 | 2 | 9 | 14 | Done |
| 6 | 02 | 2 | 8 | 13 | Done |
| 7 | 25 | 3 | 7 | 11 | In Review |
| 8 | 14 | 1 | 4 | 6 | In Review |
| 9 | 09 | 4 | 3 | 4 | Done |
| 10 | 13 | 1 | 3 | 4 | Done |

## Known risks

- The current `tracks.yaml` status snapshot has multiple tracks marked
  `In Progress`, `In Review`, or `Done` while dependencies are not `Done`.
  The global gate treats this as a blocker; Track 29 itself passes with
  `-TrackId 29`.
- Prior documentation that assumes only Waves 0-5 is stale against the current
  dependency graph.
- The validator checks dependency status and conductor artifacts; it does not
  prove each upstream track's implementation quality.

## Follow-up issues

- Track 13 should wire the two gate commands into CI once CI ownership is ready.
- Track 15 should treat blocking output from the validator as release-packaging
  stop evidence.
- Track owners should either move dependencies to `Done` with evidence or record
  ADR-backed overrides before claiming release readiness.

## Integration notes

- The validator intentionally does not modify `conductor/tracks.yaml`.
- The validator derives track directories by ID prefix and does not hard-code a
  track count.
- Use `-ReportOnly` for planning dashboards and handoff evidence; use default
  blocking mode for merge/release gates.
## Phase closeout evidence

`$conductor-review` was run locally against Track 29 and the current diff.
Accepted fixes: added target-track gate mode, refreshed current wave evidence,
and documented that the default global gate still blocks unrelated dependency
status violations. No rejected in-scope fixes.

Commit SHA: pending because this shared worktree may contain unrelated worker
edits before final integration. Pushed ref: pending for the same reason.
`validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` is deferred until
the cleaned slice is committed. Next-phase decision: Track 29 is ready for
review; do not mark Done until global dependency blockers are either resolved
by their owners or waived by ADR.
