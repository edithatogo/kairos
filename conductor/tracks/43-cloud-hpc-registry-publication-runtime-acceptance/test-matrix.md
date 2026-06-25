# 43 Cloud/HPC Registry Publication & Runtime Acceptance - test-matrix.md

| Gate | Command | Required for |
|---|---|---|
| HPC registry manifest and runtime evidence | `node scripts/validation/validate-hpc-registry-readiness.mjs --check-negative-fixtures` | Review and publish |
| Protected publish blocker | `node scripts/release/publish-hpc.mjs --mode publish --version 0.0.0-test` must fail until live evidence is complete | Review |
| Offline cloud/HPC validation | `python cloud/validate_cloud_hpc.py` | Review |
| Code health | `node scripts/validation/validate-code-health.mjs` | Production publish |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Phase movement |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Closeout |

Strict closeout requires `RequireCleanWorkingTree` after workflow and manifest changes are committed.
