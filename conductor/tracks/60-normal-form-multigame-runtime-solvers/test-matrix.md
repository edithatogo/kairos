# Track 60 Test Matrix

| Gate | Command | Required before |
|---|---|---|
| Component tests | `cargo test -p kairo-ecs-game-theory normal_form_components` | Phase 0 |
| Solver tests | `cargo test -p kairo-ecs-game-theory normal_form` | Phase 1 |
| Bench smoke | `cargo bench -p kairo-ecs-game-theory --bench normal_form -- --quick` | Phase 2 |
| Workspace | `cargo test --workspace --all-features` | Track closeout |
| Conductor phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Every phase |
| DAG | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` | Every phase |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | After commit/push |
| GitHub Actions | `gh pr checks --watch` | After pushed track phase |
