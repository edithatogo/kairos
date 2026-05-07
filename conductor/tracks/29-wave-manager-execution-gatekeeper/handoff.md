# Handoff: Track 29 Wave Manager & Execution Gatekeeper

Handoff date: 2026-05-07.

## Summary

Implemented a local Wave Manager validator that derives wave membership,
dependency closure, and the critical-path heatmap from the current
`conductor/tracks.yaml` inventory. The gate definitions now name the concrete
PowerShell command for blocking validation and report-only evidence generation.
The wave policy is now graph-derived and enforced through `wave-progression-check`
and `dependency-closure-check`.

The current dependency graph derives Waves 0-6, not the prior fixed 0-5 model.
Track 39 is Wave 6 because it depends on Track 15, which is Wave 5.

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
| `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\29-wave-manager-execution-gatekeeper\validate-wave-gates.ps1` | Fail as designed | Exit 1 with 89 direct dependency blockers and 181 transitive dependency blockers. |

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
| 1 | 00 | 0 | 13 | 40 | Spec Approved |
| 2 | 01 | 1 | 10 | 28 | In Progress |
| 3 | 12 | 1 | 14 | 21 | In Progress |
| 4 | 26 | 1 | 4 | 20 | In Progress |
| 5 | 04 | 2 | 9 | 14 | In Progress |
| 6 | 02 | 2 | 8 | 13 | In Progress |
| 7 | 25 | 3 | 7 | 11 | In Progress |
| 8 | 14 | 1 | 4 | 6 | In Progress |
| 9 | 09 | 4 | 3 | 4 | In Progress |
| 10 | 13 | 1 | 3 | 4 | In Progress |

## Known risks

- The current `tracks.yaml` status snapshot has many tracks marked `In Progress`
  while dependencies are not `Done`. The new gate treats this as a blocker.
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
