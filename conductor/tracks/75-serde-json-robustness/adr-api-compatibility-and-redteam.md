# ADR: Track 75 — Public API Compatibility Assessment & Red-Team Review

**Track:** 75-serde-json-robustness  
**Date:** 2026-10-24  
**Author:** Principal Software Engineer / Conductor Contracts Agent  
**Status:** Approved & Implemented  

---

## 1. Context & Scope

Track 75 adopts `serde_json` `1.0.154` to replace handwritten string-formatting across execution manifests and CLI stdout in `kairo-ecs-cli` and trace streams in `kairo-ecs-debug`. In conformance with `AGENTS.md` and `CONTRIBUTING.md`, all public API boundaries, error enums, schema definitions, and serialization behaviours undergo formal compatibility assessment and red-team review.

---

## 2. Public API Stability & Error Contracts

### 2.1 `DebugContractError` and Stream Resource Bounds
- **Resource Exhaustion Mitigation**: To prevent memory exhaustion and DoS from oversized trace files or decompression bombs, `validate_jsonl_trace_records` enforces explicit, documented bounds:
  - `MAX_JSONL_TOTAL_BYTES`: 16 MiB (`16 * 1024 * 1024`) total stream capacity.
  - `MAX_JSONL_RECORD_BYTES`: 1 MiB (`1024 * 1024`) single-record line limit.
- **Contract Error Representation**: When a stream or line exceeds these bounds, `DebugContractError::ExceededByteLimit { limit, actual }` is returned.
- **Syntax and Schema Failures**: Syntax or record structure defects deterministically map to `DebugContractError::MissingSchema` (header or syntax defect) or `DebugContractError::EmptyKind` (invalid empty event kind).

### 2.2 Strict Variant-Specific Payloads and Duplicate Map-Key Rejection
- **Variant Separation**: `TraceRecordDto` strictly separates `snapshot` and `delta` payloads. A snapshot containing delta fields (`event_id_index`, `event_id_generation`, `sequence`, `kind`, `changes`) or a delta containing snapshot state (`state`) is strictly rejected.
- **Required Fields**: `state` is strictly required on `snapshot`; `changes`, `event_id_index`, `event_id_generation`, `sequence`, and `kind` are strictly required on `delta`. Missing fields are rejected.
- **Duplicate Key Prevention**: Maps (`state` and `changes`) and envelope payloads enforce duplicate key rejection via `deserialize_unique_map` rather than silently adopting "last value wins".

### 2.3 Structural Prohibition of Floating-Point Types
- **Float Elimination**: All DTO numerical fields are explicitly discrete integers (`u128`, `u64`, `u32`, `i32`). Floating-point fields (`f32`, `f64`) are structurally prohibited across all public DTO contracts.
- **Integer Type Safety**: Deserialization strictly rejects non-integer floats (e.g. `{"tick": 12.345}`) and non-numeric inputs in integer fields. We do not rely on or claim generic float rejection in Serde; precision and RFC 8259 compliance are guaranteed by type structure.

### 2.4 CLI Stdout Serialization
- All CLI subcommands (`validate-scenario`, `replay`, `resume-plan`, `run`, `checkpoint`, `resume`) emit JSON status output serialized via `serde_json` over typed status DTOs (`ValidateScenarioStatusDto`, `ReplayStatusDto`, `ResumePlanStatusDto`, `RunStatusDto`, `RunCapturedStatusDto`, `CheckpointStatusDto`, `ResumeStatusDto`).
- Eliminates handwritten `format!` stdout strings and ensures proper RFC 8259 escaping for exotic paths containing quotes, spaces, backslashes, and control characters. Verified by independent process-level integration tests in `crates/kairo-ecs-cli/tests/cli_process_stdout.rs`.

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
| CLI Stdout | JSON | N/A | Fully serialized typed status DTOs, valid JSON on all paths |
| Debug Trace | JSON Lines (`jsonl`) | `kairoecs.debug-trace.v1` | Monotonically non-decreasing `u128` ticks, byte limits |

---

## 4. Red-Team Objections & Mitigations

| Objection | Risk | Mitigation / Architectural Decision |
|:---|:---|:---|
| **Float Narrowing for 128-bit Ticks** | Standard JSON deserializers parse numbers as `f64`, losing precision past $2^{53}-1$ ($9,007,199,254,740,991$). | `TraceRecordDto` uses dedicated Serde visitor deserializing `tick: u128` directly without floating point intermediate representations. Verified by test asserting exact roundtrip of $3.4028 \times 10^{38}$. |
| **Float Nonfinite Claims** | Serde serializer defaults can emit null on floats; claiming generic float rejection is inaccurate. | All DTO contracts structurally forbid floating-point types entirely. Discrete integers are enforced; floating-point JSON in integer fields fails deserialization closed. |
| **Silent Variant Field Pollution** | Payloads accepting fields from both snapshot and delta variants permit invalid mixed states. | Strict variant validation rejects delta fields on snapshots and snapshot fields on deltas. |
| **Duplicate Map Key Injection** | Repeated keys in JSON maps can cause parser discrepancies ("last value wins" vs "first value wins"). | `deserialize_unique_map` rejects duplicate keys in `state`, `changes`, and envelope payloads. |
| **Unbounded Trace Ingestion** | Untrusted multi-gigabyte lines or streams can exhaust memory before validation. | Enforced `MAX_JSONL_TOTAL_BYTES` (16 MiB) and `MAX_JSONL_RECORD_BYTES` (1 MiB) with dedicated error variant. |
| **Malformed CLI Stdout on Quoted Paths** | Handwritten JSON `println!` statements break when paths contain quotes or escapes. | All CLI command outputs serialized via `serde_json` over typed status DTOs, verified with process tests. |
| **Transitive Footprint Bloat** | Adding `serde_json` adds external dependencies to the runtime. | `serde_json` was already locked at `1.0.154` in workspace lockfile. Zero new dependencies downloaded. |
