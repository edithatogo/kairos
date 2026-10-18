# Track 64 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| `soa-column-query-simd-parity` | `cargo test --workspace` | Review and Done |
| `des-resource-preemption-reneging-simpy-parity` | `cargo test --workspace` | Review and Done |
| `abm-spatial-grid-continuous-network-parity` | `cargo test --workspace` | Review and Done |
| `hybrid-des-abm-100k-benchmark` | `cargo test --workspace` | Review and Done |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Every phase closeout |
| DAG validation | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` | Every phase closeout |
| Artifact validation | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_artifacts.ps1` | Every phase closeout |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | After commit/push |
