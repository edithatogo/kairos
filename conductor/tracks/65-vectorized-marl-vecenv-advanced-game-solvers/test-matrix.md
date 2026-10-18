# Track 65 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| `mixed-nash-lemke-howson-parity` | `cargo test --workspace` | Review and Done |
| `cfr-plus-extensive-form-convergence` | `cargo test --workspace` | Review and Done |
| `rayon-vecenv-1024-worlds-throughput` | `cargo test --workspace` | Review and Done |
| `pettingzoo-gymnasium-api-conformance` | `cargo test --workspace` | Review and Done |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Every phase closeout |
| DAG validation | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` | Every phase closeout |
| Artifact validation | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_artifacts.ps1` | Every phase closeout |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | After commit/push |
