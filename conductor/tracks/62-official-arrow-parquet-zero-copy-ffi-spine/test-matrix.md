# Track 62 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| `official-arrow-crate-parity` | `cargo test --workspace` | Review and Done |
| `parquet-zstd-roundtrip` | `cargo test --workspace` | Review and Done |
| `arrow-c-data-interface-ffi` | `cargo test --workspace` | Review and Done |
| `pycapsule-duckdb-polars-smoke` | `cargo test --workspace` | Review and Done |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Every phase closeout |
| DAG validation | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` | Every phase closeout |
| Artifact validation | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_artifacts.ps1` | Every phase closeout |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | After commit/push |
