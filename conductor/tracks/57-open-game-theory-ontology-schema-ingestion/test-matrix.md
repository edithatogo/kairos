# Track 57 Test Matrix

| Gate | Command | Required before |
|---|---|---|
| Parser tests | `cargo test -p kairo-ecs-game-ontology` | Every parser task |
| Fixture validation | `cargo test -p kairo-ecs-game-ontology --test ontology_fixtures` | Phase 1 closeout |
| Conductor phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Every phase |
| DAG | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` | Every phase |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | After commit/push |
| GitHub Actions | `gh pr checks --watch` | After pushed track phase |
