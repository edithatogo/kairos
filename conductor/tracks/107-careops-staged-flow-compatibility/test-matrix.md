# Track 107 test matrix

| Gate | Command | Status |
|---|---|---|
| Format | `cargo fmt --all -- --check` | Pass |
| DES focused | `cargo test --locked -p kairo-ecs-des --test careops_staged_allocation_compatibility` | Pass |
| DES package | `cargo test --locked -p kairo-ecs-des` | Pass |
| Core regression | `cargo test --locked -p kairo-ecs-core` | Pass |
| Workspace check/test | `cargo check --workspace --locked`; `cargo test --workspace --locked` | Pass |
| Rustdoc | `cargo doc --locked -p kairo-ecs-des --no-deps` | Pass |
| Fixture validation | `pwsh -NoProfile -File scripts/validate_conformance_fixtures.ps1` | Pass |
| API compatibility pack | `pwsh -NoProfile -File docs/design/validate-compatibility-pack.ps1` | Pass |
| Conductor artifacts/DAG | `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1`; `pwsh -NoProfile -File scripts/validate_conductor_dag.ps1` | Pass with repository-wide informational warnings; zero DAG issues |
| Phase gate | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Pass |
| Full repository CI | `just ci` | Partial: formatting, clippy, 899 tests, 93.39% core coverage, rustdoc and cargo-deny pass; final `cargo audit` finds three baseline zstd advisories with unchanged Cargo.lock |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Pending: candidate has not been committed/pushed |

Record executed commands and exit codes in `evidence/`; a pending command is not a pass. Strict Git closeout remains pending until commit/push and cleanup.
