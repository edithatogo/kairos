# ADR: Polars & Arrow-rs Dependency Qualification and Architecture Contract

## Context
KairoECS requires native tabular processing, reliable Parquet/Zstd persistence, Arrow IPC streaming, and C Data Interface polyglot interchange. Historical tracks relied on bespoke handwritten encoders/decoders (`SMOKE_HEADER` TSV, custom pseudo-Parquet with `"PAR1"` markers, and mock C capsules), which are generic non-differentiators.

## Decisions

### 1. Library Selection & Version Pin
- **Primary Tabular Engine**: `polars` (v0.45+).
- **Arrow Spine**: Arrow `60.0.0` (as aligned with `arrow-schema = "60.0.0"` in `crates/kairo-ecs-arrow`).
- **Feature Isolation**: Polars is strictly contained in `crates/kairo-ecs-arrow` (behind explicit optional feature flags where needed) and native CLI binaries (`crates/kairo-ecs-cli`). It is strictly forbidden from minimal core (`kairo-ecs-core`, `kairo-ecs-types`) and Wasm/browser runtimes (`kairo-ecs-wasm`).

### 2. Schema v2 Contract
- **Generational Entity IDs**:
  - `entity_id_index`: `UInt64` (required / nullable in lockstep with generation)
  - `entity_id_gen`: `UInt32`
- **Simulation Time (Full 128-bit Range)**:
  - `time_ticks_hi`: `UInt64`
  - `time_ticks_lo`: `UInt64`
  - Lexicographical limb subtraction preserves numerical ordering without float rounding.
- **Run Manifests**:
  - SHA-256 digest of normalized effective inputs and schema fingerprint.
  - Manifest-last atomic publication.

### 3. Durability & Recovery
- Bounded batch flushing.
- Parquet with Zstd compression.
- Arrow IPC streams for inter-process communication.
