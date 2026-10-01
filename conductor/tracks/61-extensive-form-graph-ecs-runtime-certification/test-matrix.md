# Track 61 Test Matrix

| Gate | Command | Required before |
|---|---|---|
| Extensive components | `cargo test -p kairo-ecs-game-theory extensive_form_components --features graph-relations` | Phase 0 |
| Traversal and solver | `cargo test -p kairo-ecs-game-theory extensive_form --features graph-relations` | Phase 1 |
| Certification | `node scripts/validation/validate-multigame-certification.mjs --self-test` | Phase 2 |
| Workspace | `cargo test --workspace --all-features` | Track closeout |
| Conductor phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Every phase |
| DAG | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` | Every phase |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | After commit/push |
| GitHub Actions | `gh pr checks --watch` | After pushed track phase |
