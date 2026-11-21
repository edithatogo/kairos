# 44 Code and Repository Health >= 9.5 - test-matrix.md

| Gate | Command | Required for |
|---|---|---|
| Health score | `node scripts/validation/validate-code-health.mjs` | Review and publish |
| Repository lint | `alint check --config .github/alint.yml .` | Code Health workflow |
| RepoHealth report | `repohealth . --format markdown` | Advisory Code Health job summary |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Phase movement |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Closeout |

Strict closeout requires `RequireCleanWorkingTree` after health workflow and scorecard changes are committed.
