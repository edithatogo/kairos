# Track 56 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| Ontology subrepo boundary | `pwsh -NoProfile -File conductor/tracks/56-open-game-theory-ontology-multigame-framework/validate-track56.ps1 -CheckSubrepo` | In Progress |
| Turtle/JSON-LD ingestion | `cargo test -p kairo-ecs-game-theory ontology_ingestion --features ontology-ingest` | In Review |
| Codegen determinism | `cargo test -p kairo-ecs-game-theory ontology_codegen --features ontology-ingest` | In Review |
| Feature isolation | `cargo check -p kairo-ecs-core --no-default-features` and `cargo check -p kairo-ecs-core --features graph-relations` | In Review |
| EntityId graph topology | `cargo test -p kairo-ecs-state graph_relations --features graph-relations` | In Review |
| Pointer-free topology scan | `pwsh -NoProfile -File conductor/tracks/56-open-game-theory-ontology-multigame-framework/validate-track56.ps1 -CheckNoPointerTopology` | In Review |
| Normal-form solver parity | `cargo test -p kairo-ecs-game-theory normal_form_solver --features ontology-ingest` | In Review |
| Extensive-form traversal | `cargo test -p kairo-ecs-game-theory extensive_form_solver --features graph-relations,ontology-ingest` | Done |
| Full workspace | `cargo test --workspace --all-features` | Phase closeout |
| Clippy workspace | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Phase closeout |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Phase movement |
| DAG gates | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` | Phase movement |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Closeout |

Strict closeout requires `RequireCleanWorkingTree` after each task commit.
Commands that reference future crates or validators are blocking gates once the
track moves from artifact creation into implementation.
