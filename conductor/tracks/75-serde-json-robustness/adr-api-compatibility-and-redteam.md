# ADR: Track 75 — Public API Compatibility Assessment & Red-Team Review

**Track:** 75-serde-json-robustness  
**Date:** 2026-10-24  
**Author:** Principal Software Engineer / Conductor Contracts Agent  
**Status:** Approved & Implemented  

---

## 1. Context & Scope

Track 75 adopts `serde_json` `1.0.154` to replace handwritten string-formatting (`format!`) across execution manifests in `kairo-ecs-cli` and trace streams in `kairo-ecs-debug`. In conformance with `AGENTS.md` and `CONTRIBUTING.md`, all public API boundaries, exhaustive enums, schema definitions, and serialization behaviours undergo formal compatibility assessment and red-team review.

---

## 2. Public API Stability & Enum Exhaustiveness

### 2.1 Preserving `DebugContractError` Enum Stability
- **Issue Evaluated**: Adding new variants (such as `MalformedRecord`) to an exhaustive public enum breaks external `match` expressions across consumers expecting exhaustive patterns.
- **Architectural Decision**: Reverted all additions to `DebugContractError`. The enum retains its exact 4 variants:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub enum DebugContractError {
      MissingSchema,
      UnsupportedSchema(String),
      TicksOutOfOrder { previous: u128, current: u128 },
      EmptyKind,
  }
  ```
- **Error Mapping Policy**: Syntax or deserialization failures in `validate_jsonl_trace_records` map deterministically to `DebugContractError::MissingSchema` (for header/format syntax defects) or `DebugContractError::EmptyKind` (for invalid empty kind payloads). This maintains 100% backward compatibility.

### 2.2 DTO Invariant Enforcement
- `TraceDeltaDto` enforces non-empty `kind` invariants at all entrypoints:
  1. `TraceDeltaDto::new` returns `Err(DebugContractError::EmptyKind)`.
  2. `TraceDeltaDto::validate` returns `Err(DebugContractError::EmptyKind)`.
  3. `TraceRecordDto` custom deserializer inspects `kind` and errors during deserialization if empty.
  4. `validate_jsonl_trace_records` explicitly checks `kind` validity at both headerless first record and subsequent positions.

---

## 3. Schema & Serialization Boundaries

| Surface | Format | Schema Version Tag | Invariant Check |
|---|---|---|---|
| `manifest.json` | JSON | `kairoecs.run-manifest.v1` | `#[serde(deny_unknown_fields)]`, non-zero replications |
| `summary.json` | JSON | N/A | `#[serde(deny_unknown_fields)]`, non-empty summary hash |
| `replay-comparison.json` | JSON | N/A | `#[serde(deny_unknown_fields)]`, matched status check |
| `resumability-plan.json` | JSON | `kairoecs.resumability-plan.v1` | `#[serde(deny_unknown_fields)]`, non-zero checkpoint interval |
| `checkpoint-manifest.json` | JSON | `kairoecs.checkpoint-manifest.v1` | `#[serde(deny_unknown_fields)]`, valid state string |
| `resume-request.json` | JSON | `kairoecs.resume-request.v1` | `#[serde(deny_unknown_fields)]`, valid checkpoint path |
| `run-request.json` | JSON | `kairoecs.run-request.v1` | `#[serde(deny_unknown_fields)]`, captured request metadata |
| Debug Trace | JSON Lines (`jsonl`) | `kairoecs.debug-trace.v1` | Monotonically non-decreasing `u128` ticks |

---

## 4. Red-Team Objections & Mitigations

| Objection | Risk | Mitigation / Architectural Decision |
|:---|:---|:---|
| **Float Narrowing for 128-bit Ticks** | Standard JSON deserializers parse numbers as `f64`, losing precision past $2^{53}-1$ ($9,007,199,254,740,991$). | `TraceRecordDto` uses dedicated Serde visitor deserializing `tick: u128` directly without floating point intermediate representations. Verified by test asserting exact roundtrip of $3.4028 \times 10^{38}$. |
| **Non-finite Values (`NaN`, `Infinity`)** | Non-finite IEEE-754 values violate RFC 8259 JSON specifications and cause parser failures in downstream tooling. | `serde_json` strictly rejects `NaN` and `Infinity` during serialization. Negative tests verify that malformed or non-finite numbers fail closed. |
| **Exhaustive Enum Match Breakage** | Downstream consumers matching on `DebugContractError` fail to compile if new variants are added. | No new variants added to `DebugContractError`. API remains 100% stable. |
| **Bypassing Delta Invariants via Direct Deserialization** | Untrusted JSON with `"kind": ""` could bypass `new()` constructor checks. | Deserializer and validator explicitly reject empty `kind` at all stream positions. |
| **Transitive Footprint Bloat** | Adding `serde_json` adds external dependencies to the runtime. | `serde_json` was already locked at `1.0.154` in workspace lockfile. Promoting to `[dependencies]` in CLI and debug crates adds zero new dependencies. |
| **Tabular Trace Format Leaks** | Replacing control JSON might encourage serializing full simulation matrices to JSON. | Tabular time-series traces remain strictly confined to Apache Arrow and Polars/Parquet formats. |
