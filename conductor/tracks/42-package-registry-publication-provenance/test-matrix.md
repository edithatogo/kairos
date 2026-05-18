# 42 Package Registry Publication & Provenance - test-matrix.md

| Gate | Command | Required for |
|---|---|---|
| Publication manifest | `node scripts/validation/validate-publication-readiness.mjs` | Review and publish |
| Code health | `node scripts/validation/validate-code-health.mjs` | Production publish |
| Package dry run | `python packaging/scripts/build_release_manifest.py --verify-existing` | Review |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Phase movement |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Closeout |

Strict closeout requires `RequireCleanWorkingTree` after publication workflow and manifest changes are committed.
