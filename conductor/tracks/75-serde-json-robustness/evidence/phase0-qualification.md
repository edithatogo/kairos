# Phase 0 Evidence & Value Gate Qualification: serde_json Metadata and Control Encoding

**Track:** 75-serde-json-robustness  
**Date:** 2026-10-24  
**Agent:** contracts-agent  
**Status:** Phase 0 Complete — Evaluation Decision: **ADOPT**

---

## 1. Inventory & Target Scope (Task 0.1)

- **Lead Agent:** `contracts-agent`
- **Owned Paths:**
  - `crates/kairo-ecs-cli/`
  - `crates/kairo-ecs-debug/`
  - `conductor/tracks/75-serde-json-robustness/`
- **Prerequisites Status:**
  - Local Prerequisite: Track 74 (`serde-typed-control-contracts`) is completed and accepted in Kairos (`main` commit `48f0270` via PR #309). All DTOs are defined, strongly typed, and derive `Serialize` and `Deserialize`.
  - External Dependency: CareOps downstream integration. Per the 24 October 2026 policy refinement in `conductor/dependency-value-policy.md`:
    > *"Real prerequisites versus preferred order: the documented serial order remains the delivery preference. A blocking edge must name the contract/consumer, exact task/receipt and oracle it enables. Record preferred ordering as scheduling guidance, not an artificial technical gate."*
    Kairos Track 75 produces compliant JSON control contracts and encoders that CareOps or other consumers can reliably parse.
- **Source Target Audit:**
  - `crates/kairo-ecs-cli/src/main.rs`: Hand-assembled string templates (`format!("{...}")`) for writing:
    - `manifest.json` (`kairoecs.run-manifest.v1`)
    - `summary.json`
    - `replay-comparison.json`
    - `resumability-plan.json` (`kairoecs.resumability-plan.v1`)
    - `checkpoint-manifest.json` (`kairoecs.checkpoint-manifest.v1`)
    - `resume-request.json` (`kairoecs.resume-request.v1`)
    - `run-request.json` (`kairoecs.run-request.v1`)
  - `crates/kairo-ecs-debug/src/contracts.rs` / `lib.rs`: Trace metadata and line encoding for:
    - `TraceHeaderDto` (`kairoecs.debug-trace.v1`)
    - `TraceSnapshotDto`
    - `TraceDeltaDto`
    - `TraceRecordDto` (JSON lines representation)

---

## 2. Dependency Qualification (Task 0.2)

- **Candidate Crate:** `serde_json`
- **Version Pinned in Workspace:** `1.0.154` (locked in `Cargo.lock`)
- **Features Required:** `default` (`std`)
- **License:** MIT OR Apache-2.0 (fully compatible with workspace Apache-2.0)
- **MSRV:** Rust 1.56+ (Kairos workspace requires Rust 1.99.1; toolchain is 1.99.1)
- **Portability & Targets:** Full support across Tier 1, embedded, and WASM platforms.
- **Maintenance & Security:** Maintained by the Serde project; zero open security advisories; de facto standard JSON serializer/deserializer in Rust.
- **Transitive Footprint & Compilation Cost:**
  `serde_json v1.0.154` is ALREADY compiled in the workspace dependency graph because `kairo-ecs-cli` and `kairo-ecs-debug` specify it in `[dev-dependencies]`. Promoting `serde_json` to `[dependencies]` in `crates/kairo-ecs-cli` and `crates/kairo-ecs-debug` introduces zero new crate downloads, zero lockfile diffs, and zero transitive dependencies.

---

## 3. Value Gate Evaluation (Task 0.3)

| Criterion | Retain Bespoke `format!` String Formatting | Adopt `serde_json` Control Encoders | Decision Impact |
|---|---|---|---|
| **Escaping & Injection Safety** | **FAIL** (Vulnerable to control characters, quotes, and newlines in file paths or IDs) | **PASS** (RFC 8259 compliant escaping for Unicode, quotes, control characters `\u0000`-`\u001f`) | Eliminates JSON injection and syntax corruption |
| **Integer & Numeric Precision** | **CONCERN** (Formatting logic must ensure exact integers up to `u128::MAX` without narrowing through floating point) | **PASS** (Direct integer formatting preserving full `u128`, `u64`, `u32` ranges) | Prevents tick/ID precision loss |
| **Non-finite Number Rejection** | **FAIL** (Ad hoc or unvalidated formatting) | **PASS** (`serde_json` rejects `NaN` and `Infinity` by default, complying with RFC 8259) | Prevents invalid non-finite floating point emissions |
| **Malformed & Duplicate Key Handling** | **FAIL** (No input validation on serialized hand-crafted buffers) | **PASS** (Strict validation during deserialization rejecting malformed JSON and unknown fields) | Prevents invalid state persistence |
| **Transitive Footprint** | **PASS** (Zero external dependencies) | **PASS** (`serde_json` already resolved to 1.0.154 in `Cargo.lock`; 0 new crates) | Neutral footprint impact |
| **Code Maintainability** | **FAIL** (Fragile multiline string templates in `main.rs`) | **PASS** (Direct `serde_json::to_string_pretty` / `to_string` on typed DTOs) | Major maintainability win |

### Decision: **ADOPT**
Pursuant to [dependency-value-policy.md](../../dependency-value-policy.md):
Adopting `serde_json` satisfies the value gate:
1. Replaces manual `format!` JSON construction and manual escaping across CLI and debug crates.
2. Serializes strictly via the accepted Track 74 typed DTO contracts with schema validation.
3. Does NOT move tabular time-series traces out of Polars/Parquet into JSON; confines JSON to control metadata, manifests, requests, and debug trace records.
4. Introduces zero new transitive dependencies to `Cargo.lock`.

### Numerical & Performance Budgets:
- Binary size growth budget: < 150 KB in release profile.
- Memory overhead: Zero heap allocation beyond output string buffers.
- Throughput budget: Serialization latency < 10 µs per control manifest on standard test fixtures.

---

## 4. Phase 0 Verification & Phase 1 Authorization (Task 0.4)

- Phase 0 evaluation accepted.
- Phase 1 will implement:
  - Promote `serde_json = "1"` to `[dependencies]` in `crates/kairo-ecs-cli/Cargo.toml` and `crates/kairo-ecs-debug/Cargo.toml`.
  - Add `RunRequestDto` in `crates/kairo-ecs-cli/src/contracts.rs`.
  - Replace all `format!` JSON generation in `crates/kairo-ecs-cli/src/main.rs` with `serde_json::to_string_pretty(&dto)` or `serde_json::to_string(&dto)`.
  - Add JSON serialization / deserialization helper functions and tests in `crates/kairo-ecs-debug/src/contracts.rs` (and `lib.rs`) for `TraceRecordDto`, `TraceHeaderDto`, `TraceSnapshotDto`, `TraceDeltaDto`.
  - Add independent test suite covering `dependency-75-behaviour`:
    - Escaped Unicode and control characters roundtrip.
    - Full-range integer preservation (`u128::MAX`, `u64::MAX`).
    - Rejection of malformed JSON, duplicate keys, and unknown fields.
    - Rejection of non-finite numbers (`f64::NAN`, `f64::INFINITY`).
