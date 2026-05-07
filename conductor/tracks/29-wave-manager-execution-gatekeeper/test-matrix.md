# Test Matrix: Track 29 Wave Manager & Execution Gatekeeper

Current validation date: 2026-05-07.

| Check | Local command | Expected result | Current evidence |
|---|---|---|---|
| Track inventory parses and dependency targets exist | `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate_conductor_dag.ps1` | Pass | Pass: parsed 41 tracks, parsed 47 agents, 0 errors, 0 warnings. |
| Required track artifacts exist | `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate_conductor_artifacts.ps1` | Pass | Pass: found 41 track directories, 0 errors, 0 warnings, 0 info. |
| Gate files and wave policy are readable | `powershell -NoProfile -ExecutionPolicy Bypass -Command "Get-Content -LiteralPath 'conductor\gates\wave-progression-check.yml','conductor\gates\dependency-closure-check.yml' | Out-Null; Get-Content -LiteralPath 'conductor\wave-policy.md' | Out-Null; 'docs-readable'"` | Pass | Pass: `docs-readable`. |
| `wave-progression-check` gate exists and is documented in `conductor/quality-gates.md` | `rg -n "wave-progression-check" conductor/gates/wave-progression-check.yml conductor/quality-gates.md` | Pass | Pass: gate definition and quality-gate documentation are present. |
| `dependency-closure-check` gate exists and is documented in `conductor/quality-gates.md` | `rg -n "dependency-closure-check" conductor/gates/dependency-closure-check.yml conductor/quality-gates.md` | Pass | Pass: gate definition and quality-gate documentation are present. |
| Wave assignment is derivable from the dependency graph | `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\29-wave-manager-execution-gatekeeper\validate-wave-gates.ps1 -ReportOnly` | Pass | Pass: wave assignment is computed from `depends_on` closure, not hand-coded track lists. |
| Wave membership is derived from `conductor/tracks.yaml` | `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\29-wave-manager-execution-gatekeeper\validate-wave-gates.ps1 -ReportOnly` | Pass | Pass: 41 tracks assigned to Waves 0-6. Wave 6 contains Track 39 because it depends on Wave-5 Track 15. |
| Critical-path heatmap is generated from dependency closure | `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\29-wave-manager-execution-gatekeeper\validate-wave-gates.ps1 -ReportOnly` | Pass | Pass: top heatmap entries are Track 00 (40 transitive dependents), Track 01 (28), Track 12 (21), Track 26 (20), Track 04 (14). |
| `wave-progression-check` blocks direct dependency violations | `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\29-wave-manager-execution-gatekeeper\validate-wave-gates.ps1` | Fail while dependencies are unresolved | Fail as designed: 89 `wave-progression-check/direct-dependency-not-done` blockers. |
| `dependency-closure-check` blocks transitive dependency violations | `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\29-wave-manager-execution-gatekeeper\validate-wave-gates.ps1` | Fail while dependencies are unresolved | Fail as designed: 181 `dependency-closure-check/transitive-dependency-not-done` blockers. |
| Missing owner, missing required gate, unknown dependency, missing artifact, and cycle checks are wired | `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\29-wave-manager-execution-gatekeeper\validate-wave-gates.ps1 -ReportOnly` | No structural blockers in current snapshot | Pass for structural checks: no missing-owner, missing-required-gates, unknown-dependency, missing-required-artifact, or dependency-cycle issues were reported. |

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

## Current Critical-Path Heatmap

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

## Release Stages

| Gate | Alpha | Beta | RC | 1.0 |
|---|---:|---:|---:|---:|
| Structural inventory checks pass | yes | yes | yes | yes |
| Required Track 29 artifacts exist | yes | yes | yes | yes |
| Wave membership derives from `conductor/tracks.yaml` | yes | yes | yes | yes |
| Direct dependency blockers are reported with track and dependency IDs | yes | yes | yes | yes |
| Transitive dependency blockers are reported with track and dependency IDs | yes | yes | yes | yes |
| Dependency cycles are blocking errors | yes | yes | yes | yes |
| Unknown dependencies are blocking errors | yes | yes | yes | yes |
| Gate passes only when advancing tracks have all direct and transitive dependencies `Done` | yes | yes | yes | yes |
| Maintainer override requires an ADR | yes | yes | yes | yes |
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.