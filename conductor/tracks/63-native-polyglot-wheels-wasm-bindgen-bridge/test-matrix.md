# Track 63 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| `pyo3-maturin-native-wheel` | `cargo test --workspace` | Review and Done |
| `wasm-bindgen-browser-node-parity` | `cargo test --workspace` | Review and Done |
| `c-abi-dyld-binding-conformance` | `cargo test --workspace` | Review and Done |
| `no-silent-fallback-scheduler` | `cargo test --workspace` | Review and Done |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Every phase closeout |
| DAG validation | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` | Every phase closeout |
| Artifact validation | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_artifacts.ps1` | Every phase closeout |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | After commit/push |
