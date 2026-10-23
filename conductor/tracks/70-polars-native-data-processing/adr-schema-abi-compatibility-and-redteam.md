# ADR: Formal Polars / Arrow-rs Migration, Schema/ABI Compatibility & Red-Team Assessment

## Context
KairoECS discrete-event simulation generates voluminous event traces, state snapshots, and telemetry logs. Historically, `kairo-ecs-arrow` implemented bespoke byte serializers and pseudo-Parquet markers (`"PAR1"` byte headers with custom FNV-1a digests) to avoid heavyweight native dependencies. Track 70 formally migrates this layer to native Polars and Apache Arrow formats.

## 1. Schema & ABI Compatibility Assessment

### Schema Evolution: v1 to v2
- **Entity ID**:
  - `v1`: `FixedSizeBinary(12)`
  - `v2`: Generational ID limbs `entity_id_index: UInt64` and `entity_id_gen: UInt32`. Both fields must be null together (enforced by `oneOf` in `event_log_v2.schema.json`).
- **Simulation Ticks**:
  - `v1`: `FixedSizeBinary(16)` (little-endian bytes)
  - `v2`: Limb-based representation `time_ticks_hi: UInt64` and `time_ticks_lo: UInt64`, bounded to exact `u64::MAX`. Supports lexicographical sorting and borrow-preserving 128-bit limb arithmetic without floating point conversion.
- **Compatibility Boundary**:
  - Clean major version bump: `kairo_ecs.event_log.v1` &rarr; `kairo_ecs.event_log.v2`.
  - Upstream readers require explicit v2 parsing; no silent bitcast or hidden fallbacks.
  - Conformance fixture `conformance/fixtures/event_log_v2_conformance.json` registered in `manifest.json`.

---

## 2. Pinned Dependencies & Hashes

- **Polars Tabular Engine**:
  - Crate: `polars = "0.45"` (resolved to `0.45.1` in `Cargo.lock`).
  - Feature selection: `features = ["lazy", "parquet", "ipc", "fmt"]`.
  - Feature flag in `kairo-ecs-arrow`: `polars-native` (default-off).
- **Arrow Logical Schema**:
  - Crate: `arrow-schema = "60.0.0"`.
  - Feature flag: `arrow-native` (default-off).

---

## 3. Red-Team Review Objections & Responses

| Objection | Risk | Mitigation / Architectural Decision |
| :--- | :--- | :--- |
| **Transitive Dependency Bloat** | Polars pulls dozens of transitive crates into the build graph. | Isolated strictly behind the optional `polars-native` feature in `kairo-ecs-arrow`. Minimal core (`kairo-ecs-core`, `kairo-ecs-types`) and WebAssembly (`kairo-ecs-wasm`) remain 100% free of Polars and heavy native dependencies. Verified by `tests/polars_isolation_proof.rs`. |
| **Float Conversion for 128-bit Ticks** | Converting 128-bit ticks to `f64` causes loss of precision for high simulation ticks. | Rejected float conversion. Ticks are serialized as two 64-bit unsigned limbs (`time_ticks_hi`, `time_ticks_lo`). Verified by limb subtraction test `schema_v2_limb_subtraction_preserves_numerical_ordering`. |
| **Non-Interoperable Mock Formats** | Custom pseudo-Parquet writers fool internal tests but fail external analytics engines (DuckDB, Pandas, Polars CLI). | Deprecated and marked for replacement in Phase 1. Added failing test in `track70_phase0_baseline.rs` explicitly asserting that mock bytes fail real Parquet footers. |
| **Unbounded Memory During Trace Flushes** | Large simulations accumulating millions of records could cause Out-Of-Memory aborts. | Phase 3 specifies bounded batch flushing with manifest-last publication. |
| **Entity ID Corruption via Partial Nullability** | Allowing index to be present while generation is null or vice versa. | Enforced lockstep nullability via JSON Schema `oneOf` constraint in `schemas/arrow/event_log_v2.schema.json`. |
