# Track 46 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| HPC charter text | `rg -n "Tracks 46-55|Proof standard|Evidence manifest fields" conductor/hpc-parity-wave.md` | Spec approval |
| Gate catalogue | `rg -n "hpc-parity-charter|hpc-evidence-manifest|hpc-claim-boundary" conductor/quality-gates.md` | Spec approval |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Phase movement |
| DAG validation | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` | Registry movement |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Closeout |

Strict closeout requires `RequireCleanWorkingTree` after the track creation
commit is pushed.
