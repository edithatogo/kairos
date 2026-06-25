# Track 58 Test Matrix

| Gate | Command | Required before |
|---|---|---|
| Codegen unit tests | `cargo test -p kairo-ecs-game-ontology --features codegen` | Generator tasks |
| Generated compile | `cargo test -p kairo-ecs-game-theory` | Phase 1 |
| Determinism | `node scripts/validation/validate-game-theory-codegen.mjs` | In Review |
| Conductor phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Every phase |
| DAG | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` | Every phase |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | After commit/push |
| GitHub Actions | `gh pr checks --watch` | After pushed track phase |
