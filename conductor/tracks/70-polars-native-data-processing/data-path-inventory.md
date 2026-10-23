# Data Path Inventory and Baseline Analysis (Track 70 Phase 0)

## Overview & Scope
This inventory identifies all Rust crates, FFI boundaries, binding layers, and scripts that touch tabular data, serialization, or Arrow/Parquet representations in KairoECS. It establishes explicit decisions (Replace, Retain, Remove) in accordance with the [Track 70 Specification](spec.md) and [Plan](plan.md).

## 1. Rust Crates & In-Repo Representations

| Path / Component | Current Implementation | Action | Rationale / Successor |
| :--- | :--- | :--- | :--- |
| `crates/kairo-ecs-arrow/src/lib.rs` (`ArrowEventLog`, `EventLogRecord`, `EventLogBatch`) | Hand-rolled in-memory batch buffers with custom byte codecs (`SMOKE_HEADER` TSV, custom binary) | **Replace** | Replace in-memory batch storage and tabular transforms with upstream Polars `DataFrame` / `LazyFrame`. |
| `crates/kairo-ecs-arrow/src/lib.rs` (`ArrowNativeSchema`, `ArrowSchemaField`, `ArrowColumnBuffer`) | Custom in-repo enum-based schema and columnar buffers (`ArrowColumnBuffer`) | **Replace** | Direct Polars / Arrow-rs representation. Schema will use Arrow v2 layout with generational IDs (UInt64 index + UInt32 generation) and 128-bit ticks (UInt64 high/low limbs). |
| `crates/kairo-ecs-arrow/src/lib.rs` (`ParquetZstdCheckpointWriter`, `ParquetZstdCheckpointReader`) | Pseudo-Parquet encoder writing raw bytes prefixed with `"PAR1"` and custom checksums | **Replace** | Real Parquet writer/reader with standard Zstd compression and official Parquet metadata. |
| `crates/kairo-ecs-arrow/src/lib.rs` (`ArrowIpcStreamWriter`, `ArrowIpcStreamReader`) | Custom pseudo-IPC codec using `"ARR1"` magic bytes and hand-rolled column serialisation | **Replace** | Real Arrow IPC Stream format for cross-process interchange. |
| `crates/kairo-ecs-arrow/src/lib.rs` (`Hdf5CheckpointAdapter`, `Adios2CheckpointAdapter`) | In-repo adapters for scientific I/O checkpointing | **Retain** | Retain scientific simulation checkpoint boundaries; update internal data marshaling to use native buffers. |
| `crates/kairo-ecs-arrow/src/native_schema.rs` | Schema mapping to `arrow-schema = "60.0.0"` types | **Replace** | Update to match Polars Arrow type definitions and v2 field contracts. |
| `crates/kairo-ecs-ffi/src/lib.rs` (`kairo_ecs_export_event_log_arrow_schema`, `kairo_ecs_export_event_log_arrow_array`) | Mock/descriptor C Data Interface capsules (`KairoArrowSchema`, `KairoArrowArray`) | **Replace** | True populated Arrow C Data Interface arrays and streams with verified single-release lifecycles. |
| `crates/kairo-ecs-state` (`ArchetypeTable`, query systems) | Core ECS state and archetype storage | **Retain** | Retain scheduler, ECS identity/lifecycle, online counters, and numerical solvers; do not replace internal ECS archetype storage with Polars. |

## 2. Polyglot Bindings & Consumers

| Path / Consumer | Current Implementation | Action | Plan |
| :--- | :--- | :--- | :--- |
| `bindings/python/kairo_ecs/_arrow.py` | Dict/row-by-row reconstruction with PyCapsule wrappers (`__arrow_c_schema__`, `__arrow_c_array__`) | **Replace** | Native Python-to-Polars handoff via Arrow C Data Interface / IPC; eliminate row dict reconstruction. |
| `bindings/julia/src/KairoECS.jl` | Dependency-free smoke-byte codec & schema facade | **Replace** | Arrow IPC and Arrow C Data interface consuming v2 schemas. |
| `bindings/r/R/kairoecs.R` | R DataFrame normalization or optional `arrow::Table` roundtrip | **Replace** | Streamlined Arrow Table / IPC roundtrip with v2 schema support. |
| `bindings/typescript/src/index.ts` | JSON roundtrip guard for Arrow event-log boundary | **Replace / Retain** | Maintain clean typed schema JSON contracts; support Arrow IPC stream in browser/Node where available. |

## 3. Toolchain, Packaging & MSRV Constraints

- **Workspace MSRV**: `1.99` (pinned in root `Cargo.toml`). Active compiler is `rustc 1.99.1`.
- **Target Polars Version**: Polars `0.45` - `0.55` (latest crates.io is `0.55.2`).
- **Feature Isolation**: Polars must remain isolated within `crates/kairo-ecs-arrow` and optional native CLI/tooling. It must NOT be added to `kairo-ecs-core`, `kairo-ecs-types`, or minimal browser/Wasm targets (`kairo-ecs-wasm`).
- **Arrow Compatibility**: Align arrow dependency with Polars's bundled `arrow` / `polars-arrow` version to prevent type conflicts and duplicate symbol bloat.

## 4. Next Actions (Phase 0 -> Phase 1)
1. Add failing tests in `crates/kairo-ecs-arrow/tests/` exercising independent interoperability, extreme ticks (u128 full range), join cardinalities, and ABI ownership.
2. Formulate ADR for Polars dependency pin and Arrow v2 schema specification.
