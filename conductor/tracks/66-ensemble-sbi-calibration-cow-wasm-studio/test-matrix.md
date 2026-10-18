# Track 66 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| `sobol-lhs-crn-ensemble-runner` | `cargo test --workspace` | Review and Done |
| `optuna-abc-smc-calibration-hook` | `cargo test --workspace` | Review and Done |
| `cow-delta-snapshot-memory-budget` | `cargo test --workspace` | Review and Done |
| `wasm-time-travel-inspector-smoke` | `cargo test --workspace` | Review and Done |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Every phase closeout |
| DAG validation | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` | Every phase closeout |
| Artifact validation | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_artifacts.ps1` | Every phase closeout |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | After commit/push |
