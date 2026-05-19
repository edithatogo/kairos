# 44 Code and Repository Health >= 9.5 - test-matrix.md

| Gate | Command | Required for |
|---|---|---|
| Health score | `node scripts/validation/validate-code-health.mjs` | Review and publish |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Phase movement |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Closeout |

Strict closeout requires `RequireCleanWorkingTree` after health workflow and scorecard changes are committed.
