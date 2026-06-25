# Track 59 Test Matrix

| Gate | Command | Required before |
|---|---|---|
| Default build boundary | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-game-theory --doc --no-default-features` | Phase 0 |
| Feature tests | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-game-theory --test graph_relations --features graph-relations` | Phase 1 |
| Feature check | `rustup run stable-x86_64-pc-windows-gnu cargo check -p kairo-ecs-game-theory --features graph-relations --tests` | Phase 1 |
| Pointer scan | `node scripts/validation/validate-graph-relations-no-pointer-topology.mjs --self-test` | Phase 1 |
| Conductor phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Every phase |
| DAG | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` | Every phase |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | After commit/push |
| GitHub Actions | `gh pr checks --watch` | After pushed track phase |
