# Track 59 Test Matrix

| Gate | Command | Required before |
|---|---|---|
| Default build boundary | `cargo test -p kairo-ecs-state` | Phase 0 |
| Feature tests | `cargo test -p kairo-ecs-state --features graph-relations` | Phase 1 |
| Pointer scan | `rg -n "\\*const|\\*mut|Box<" crates/kairo-ecs-state crates/kairo-ecs-core` | Phase 1 |
| Conductor phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Every phase |
| DAG | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` | Every phase |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | After commit/push |
| GitHub Actions | `gh pr checks --watch` | After pushed track phase |
