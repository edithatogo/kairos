# Track 68 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| `boundary-channel-transit-delay-distribution` | `cargo test -p kairo-ecs-des` | Review and Done |
| `egress-sink-handover-backpressure-parity` | `cargo test -p kairo-ecs-des` | Review and Done |
| `inter-unit-patient-conservation-accounting` | `cargo test -p kairo-ecs-des` | Review and Done |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Every phase closeout |
| DAG validation | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` | Every phase closeout |
| Artifact validation | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_artifacts.ps1` | Every phase closeout |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | After commit/push |
