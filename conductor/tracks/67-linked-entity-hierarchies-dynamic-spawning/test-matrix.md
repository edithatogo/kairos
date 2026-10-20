# Track 67 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| `dynamic-entity-spawning-ecs-integrity` | `cargo test -p kairo-ecs-core -p kairo-ecs-state` | Review and Done |
| `linked-pair-lifecycle-cascading-parity` | `cargo test -p kairo-ecs-core` | Review and Done |
| `composite-entity-telemetry-arrow-export` | `cargo test -p kairo-ecs-arrow` | Review and Done |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Every phase closeout |
| DAG validation | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` | Every phase closeout |
| Artifact validation | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_artifacts.ps1` | Every phase closeout |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | After commit/push |
