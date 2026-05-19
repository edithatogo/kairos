# 43 Cloud/HPC Registry Publication & Runtime Acceptance - test-matrix.md

| Gate | Command | Required for |
|---|---|---|
| HPC registry manifest | `node scripts/validation/validate-hpc-registry-readiness.mjs` | Review and publish |
| Offline cloud/HPC validation | `python cloud/validate_cloud_hpc.py` | Review |
| Code health | `node scripts/validation/validate-code-health.mjs` | Production publish |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Phase movement |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Closeout |

Strict closeout requires `RequireCleanWorkingTree` after workflow and manifest changes are committed.
