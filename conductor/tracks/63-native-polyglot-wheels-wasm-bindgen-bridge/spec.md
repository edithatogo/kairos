# Track 63: Native Polyglot Bindings (PyO3/Maturin, Wasm-Bindgen, Extendr & C ABI)

## Objective

Replace pure-host-language fallback schedulers in Python (`heapq`), TypeScript, Go (`container/heap`), R, Julia, and C# with real compiled Rust native extensions (`PyO3`/`maturin` wheels, `wasm-bindgen` `.wasm` modules, and C ABI dynamic library bindings) so every language surface executes the single authoritative Rust simulation kernel.

## Inputs

- Existing contracts and scaffolds from Tracks 02, 06, 07, 08, 09, 10, 11, 62.
- Upstream ecosystem crate policies in `deny.toml` and `conductor/tech-stack.md`.
- Protected surface inventory in `docs/design/protected-surface-inventory.json`.

## Outputs

- `bindings/python/`
- `bindings/typescript/`
- `bindings/r/`
- `bindings/julia/`
- `bindings/csharp/`
- `bindings/go/`
- `crates/kairo-ecs-wasm/`
- `crates/kairo-ecs-ffi/`
- `conductor/tracks/63-native-polyglot-wheels-wasm-bindgen-bridge/`

## Owned paths

- `bindings/python/`
- `bindings/typescript/`
- `bindings/r/`
- `bindings/julia/`
- `bindings/csharp/`
- `bindings/go/`
- `crates/kairo-ecs-wasm/`
- `crates/kairo-ecs-ffi/`
- `conductor/tracks/63-native-polyglot-wheels-wasm-bindgen-bridge/`

## Blocked paths

- Do not modify unrelated track directories or bypass `deny.toml` license and advisory checks when adding upstream dependencies.
- Do not claim production HPC cluster scaling from single-host or free-tier smoke runs.
- Do not break existing deterministic conformance fixtures (`scheduler_ordering_v1`, `scheduler_cancellation_v1`, `rng_reproducibility_v1`, `vvuq_scenario_replay_v1`).

## Dependencies

- Depends on Tracks: 02, 06, 07, 08, 09, 10, 11, 62

## Parallel-safe tracks

- Can execute in parallel with tracks outside `bindings/python/, bindings/typescript/` provided shared contracts in `conductor/contracts/` remain unchanged or are updated via ADR.

## Acceptance criteria

- Gate `phase-closeout-check` passes with recorded command receipts.
- Gate `pyo3-maturin-native-wheel` passes with recorded command receipts.
- Gate `wasm-bindgen-browser-node-parity` passes with recorded command receipts.
- Gate `c-abi-dyld-binding-conformance` passes with recorded command receipts.
- Gate `no-silent-fallback-scheduler` passes with recorded command receipts.
- All unit, integration, and conformance tests pass under `just ci` and Conductor validators.

## Quality gates

- `phase-closeout-check`
- `pyo3-maturin-native-wheel`
- `wasm-bindgen-browser-node-parity`
- `c-abi-dyld-binding-conformance`
- `no-silent-fallback-scheduler`

## Release implications

- Promotes KairoECS from dependency-free R2 contract scaffolds to R3/R4 ecosystem-integrated runtime execution for Native Polyglot Bindings (PyO3/Maturin, Wasm-Bindgen, Extendr & C ABI).
- Any new external Cargo or language dependencies introduced in this track must pass `cargo deny`, `dependency-review.yml`, and SBOM generation before RC/1.0 release signoff.
