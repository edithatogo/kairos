# Track 69 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| `competing-hazards-sampling-invariance` | `cargo test -p kairo-ecs-rng` | Review and Done |
| `non-markovian-survival-clock-oracle` | `cargo test -p kairo-ecs-des` | Review and Done |
| `independent-stream-rng-reproducibility` | `cargo test -p kairo-ecs-rng` | Review and Done |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Every phase closeout |
| DAG validation | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` | Every phase closeout |
| Artifact validation | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_artifacts.ps1` | Every phase closeout |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | After commit/push |
