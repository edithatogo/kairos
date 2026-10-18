# Track 62: Official Apache Arrow, Parquet & Zero-Copy C Data Interface Spine

## Objective

Upgrade `kairo-ecs-arrow` and `kairo-ecs-ffi` from custom smoke TSV/binary envelopes to the official upstream Apache `arrow-array`, `arrow-schema`, `arrow-ipc`, `arrow-ffi`, and `parquet` crates behind explicit Cargo features, exposing the zero-copy Arrow C Data Interface (`FFI_ArrowArray` / `FFI_ArrowSchema`) across C ABI and Python (`__arrow_c_array__` / `__arrow_c_stream__`) for zero-copy Polars, DuckDB, PyArrow, R nanoarrow, and Julia Arrow.jl interoperability.

## Inputs

- Existing contracts and scaffolds from Tracks 02, 04, 25, 51.
- Upstream ecosystem crate policies in `deny.toml` and `conductor/tech-stack.md`.
- Protected surface inventory in `docs/design/protected-surface-inventory.json`.

## Outputs

- `crates/kairo-ecs-arrow/`
- `crates/kairo-ecs-ffi/`
- `include/kairo_ecs.h`
- `schemas/arrow/`
- `docs/parallel-io/`
- `conductor/tracks/62-official-arrow-parquet-zero-copy-ffi-spine/`

## Owned paths

- `crates/kairo-ecs-arrow/`
- `crates/kairo-ecs-ffi/`
- `include/kairo_ecs.h`
- `schemas/arrow/`
- `docs/parallel-io/`
- `conductor/tracks/62-official-arrow-parquet-zero-copy-ffi-spine/`

## Blocked paths

- Do not modify unrelated track directories or bypass `deny.toml` license and advisory checks when adding upstream dependencies.
- Do not claim production HPC cluster scaling from single-host or free-tier smoke runs.
- Do not break existing deterministic conformance fixtures (`scheduler_ordering_v1`, `scheduler_cancellation_v1`, `rng_reproducibility_v1`, `vvuq_scenario_replay_v1`).

## Dependencies

- Depends on Tracks: 02, 04, 25, 51

## Parallel-safe tracks

- Can execute in parallel with tracks outside `crates/kairo-ecs-arrow/, crates/kairo-ecs-ffi/` provided shared contracts in `conductor/contracts/` remain unchanged or are updated via ADR.

## Acceptance criteria

- Gate `phase-closeout-check` passes with recorded command receipts.
- Gate `official-arrow-crate-parity` passes with recorded command receipts.
- Gate `parquet-zstd-roundtrip` passes with recorded command receipts.
- Gate `arrow-c-data-interface-ffi` passes with recorded command receipts.
- Gate `pycapsule-duckdb-polars-smoke` passes with recorded command receipts.
- All unit, integration, and conformance tests pass under `just ci` and Conductor validators.

## Quality gates

- `phase-closeout-check`
- `official-arrow-crate-parity`
- `parquet-zstd-roundtrip`
- `arrow-c-data-interface-ffi`
- `pycapsule-duckdb-polars-smoke`

## Release implications

- Promotes KairoECS from dependency-free R2 contract scaffolds to R3/R4 ecosystem-integrated runtime execution for Official Apache Arrow, Parquet & Zero-Copy C Data Interface Spine.
- Any new external Cargo or language dependencies introduced in this track must pass `cargo deny`, `dependency-review.yml`, and SBOM generation before RC/1.0 release signoff.
