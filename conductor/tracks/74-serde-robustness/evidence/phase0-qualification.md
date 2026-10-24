# Phase 0 Evidence & Value Gate Qualification: Serde Typed Control Contracts

**Track:** 74-serde-robustness  
**Date:** 2026-10-24  
**Agent:** contracts-agent  
**Status:** Phase 0 Complete — Evaluation Decision: **ADOPT**

---

## 1. Inventory & Target Scope (Task 0.1)

- **Lead Agent:** `contracts-agent`
- **Owned Paths:**
  - `crates/kairo-ecs-cli/`
  - `crates/kairo-ecs-debug/`
  - `conductor/tracks/74-serde-robustness/`
- **Prerequisites Status:**
  - Local Prerequisite: Track 70 (`polars-native-data-processing`) is completed and accepted in Kairos (`main` commit `5692e6a` via PR #304).
  - External Dependency: CareOps milestone `polars_data_processing_20261022:R14` remains in proposed status in `careops-sim`. Per the 24 October 2026 policy refinement in `conductor/dependency-value-policy.md`:
    > *"Real prerequisites versus preferred order: the documented serial order remains the delivery preference. A blocking edge must name the contract/consumer, exact task/receipt and oracle it enables. Record preferred ordering as scheduling guidance, not an artificial technical gate."*
    CareOps Polars R14 produces no symbols or artifacts consumed by Kairos `kairo-ecs-cli` or `kairo-ecs-debug`. Kairos produces generic DTOs that CareOps downstream may subsequently consume. Local qualification and implementation proceed under Kairos ownership.
- **Source Target Audit:**
  - `crates/kairo-ecs-cli/src/main.rs`: Hand-assembled string templates for `manifest.json` (`kairoecs.run-manifest.v1`), `summary.json`, `replay-comparison.json`, `resumability-plan.json` (`kairoecs.resumability-plan.v1`), `checkpoint-manifest.json` (`kairoecs.checkpoint-manifest.v1`), and `resume-request.json` (`kairoecs.resume-request.v1`).
  - `crates/kairo-ecs-cli/src/scenario.rs`: Handwritten key-value line parser for `ScenarioManifest` and `SeedManifest`.
  - `crates/kairo-ecs-debug/src/lib.rs`: Handwritten key-value escaping (`encode_map`, `escape`) and line-based trace reconstruction (`encode_lines`).

---

## 2. Dependency Qualification (Task 0.2)

- **Candidate Crate:** `serde`
- **Candidate Derive Crate:** `serde_derive`
- **Version Pinned in Workspace:** `1.0.229` (locked in `Cargo.lock`)
- **Features Required:** `features = ["derive"]`
- **License:** MIT OR Apache-2.0 (fully compatible with workspace Apache-2.0)
- **MSRV:** Rust 1.31+ (Kairos workspace requires Rust 1.99.1; toolchain is 1.99.1)
- **Portability & Targets:** Full support across Tier 1, embedded, WASM (`wasm32-unknown-unknown`), and non-POSIX platforms (`no_std` compatible with `alloc` / `std`).
- **Maintenance & Security:** Maintained by the Serde project; zero open security advisories; de facto standard serialization framework in Rust.
- **Transitive Footprint & Compilation Cost:**
  `serde v1.0.229` and `serde_derive v1.0.229` are ALREADY compiled in the workspace dependency graph because `kairo-ecs-core` and `kairo-ecs-bench` specify them in `[dev-dependencies]`. Promoting `serde` to `[dependencies]` in `crates/kairo-ecs-cli` and `crates/kairo-ecs-debug` introduces zero new crate downloads, zero lockfile diffs, and negligible compile-time delta.

---

## 3. Value Gate Evaluation (Task 0.3)

| Criterion | Retain Bespoke Hand-Assembled Formatting | Adopt `serde` Typed Contracts | Decision Impact |
|---|---|---|---|
| **Type Safety & Range Preservation** | **FAIL** (Integer truncation risks, string escaping bugs, unescaped quotes in paths/strings) | **PASS** (Compile-time verified DTOs for `u128`, `u64`, `u32`, `PathBuf`, enums) | Eliminates formatting defects and silent data corruption |
| **Schema Invariant Enforcement** | **CONCERN** (Ad hoc string matching for `schema_version`) | **PASS** (`#[serde(deny_unknown_fields)]`, custom validation, explicit schema tags) | Enforces strict contract boundaries |
| **Transitive Footprint** | **PASS** (Zero external dependencies) | **PASS** (`serde` already resolved to 1.0.229 in `Cargo.lock`; 0 new crates) | Neutral footprint impact |
| **Code Maintainability** | **FAIL** (Scattered string concatenation in `main.rs` and `lib.rs`) | **PASS** (Declarative, centralized DTO contracts in `contracts.rs`) | Net maintainability improvement |
| **Downstream Interoperability** | **POOR** (Fragile text outputs) | **PASS** (Typed contracts ready for standard JSON serialization in Track 75) | Clean bridge to Track 75 |

### Decision: **ADOPT**
Pursuant to [dependency-value-policy.md](../../dependency-value-policy.md):
Adopting `serde` with derive macros satisfies the value gate:
1. Replaces manual control DTO conversion and escaping in scenario and debug paths.
2. Derives typed serialization contracts while retaining semantic validation and deterministic field ordering.
3. Does not move tabular data from Polars into JSON.
4. Introduces zero new transitive dependencies to `Cargo.lock`.

### Numerical & Performance Budgets:
- Binary size growth budget: < 150 KB in release profile.
- Memory overhead: Zero heap allocation beyond DTO data structures.
- Throughput budget: Roundtrip serialization/deserialization latency < 10 µs per control manifest on standard test fixtures.

---

## 4. Phase 0 Verification & Phase 1 Authorization (Task 0.4)

- Phase 0 evaluation accepted.
- Phase 1 will implement typed DTO contracts in:
  - `crates/kairo-ecs-cli/src/contracts.rs`
  - `crates/kairo-ecs-debug/src/contracts.rs`
- Add unit tests verifying `dependency-74-behaviour`:
  - Full-range IDs/ticks roundtrip (`0`, `1`, `u64::MAX`, `u128::MAX`).
  - Unknown fields rejection (`deny_unknown_fields`).
  - Strict schema version validation and invalid version rejection.
  - Deterministic field ordering with `BTreeMap`.
