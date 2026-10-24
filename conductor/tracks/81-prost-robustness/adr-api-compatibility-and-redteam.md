# ADR: Prost Distributed Envelope Encoding and Red-Team Security Analysis

**Track:** 81 (`81-prost-robustness`)  
**Status:** Accepted  
**Date:** 2026-10-25  

---

## 1. Context and Problem Statement

The optimistic distributed simulation engine in `crates/kairo-ecs-grpc` requires wire-level serialization for `KairoOptimisticEnvelopeV1`, which carries causal ancestry frames, logical process identity, 128-bit simulation timestamps, 128-bit simulation namespaces, authority epochs, and event payloads across network nodes.

Prior to Track 81:
1. `crates/kairo-ecs-grpc/src/envelope.rs` implemented a custom, ad-hoc, big-endian binary byte-packing codec (~264 lines of manual pointer arithmetic and buffer manipulation).
2. The ad-hoc format lacked language-neutral interoperability with standard Protocol Buffers runtimes (e.g., Python, C++, Go, or standard gRPC endpoints).
3. The custom format could not be easily validated by standard schema linters or deserialized by external distributed participants without maintaining duplicate manual parsers.

Track 81 adopts `prost = "0.13.5"` (minimal features: `std`, `prost-derive`) to replace the hand-rolled binary byte layout with strongly typed Protocol Buffers wire encoding, eliminating manual buffer slicing and guaranteeing schema-level interoperability.

---

## 2. Decision: Adoption of `prost`

`prost = { version = "0.13.5", default-features = false, features = ["std", "prost-derive"] }` is adopted in `crates/kairo-ecs-grpc`:
1. **Zero Native C Runtime Coupling:** Pure Rust proc macros via `prost-derive`; zero requirement for external `protoc`, C compilers, or foreign runtime shared libraries.
2. **Schema Invariant Preservation:** `proto/simulation.proto` is updated with canonical definitions for `OptimisticEnvelope`, `AncestryFrame`, `RootFrame`, `OutputFrame`, `OptimisticAuthorityMode`, and `WireMessageKind`.
3. **Structured Protobuf Serialization:** `WireOptimisticEnvelope`, `WireAncestryFrame`, `WireRootFrame`, and `WireOutputFrame` are typed Rust structs deriving `prost::Message` and `prost::Oneof`.
4. **Deterministic Full-Range Preservation:**
   - 128-bit `tick`: serialized via 16 big-endian raw bytes, preserving full 128-bit discrete simulation time.
   - 128-bit `simulation_namespace`: serialized via 16 big-endian raw bytes, preserving UUID and multi-tenant namespace fidelity.
5. **Pre-flight & Post-flight Bounds Enforced:** Total encoded bytes checked against `MAX_ENCODED_ENVELOPE_BYTES = 65_536`. Payloads capped at `MAX_PAYLOAD_BYTES = 4_096`. Ancestry depth capped at `MAX_ANCESTRY_DEPTH = 128` (129 total nodes).
6. **Code Elimination:** Replaced 264 lines of manual big-endian offset slicing and pushing with declarative, derive-generated Protocol Buffers serialization.

---

## 3. Red-Team Security Analysis

| Threat Vector | Risk Assessment | Mitigation Implemented | Verified Oracle |
|---|---|---|---|
| **Oversized Envelope Buffer (Memory Exhaustion)** | Malicious peer sends an enormous envelope buffer to trigger memory allocation exhaustion or buffer overflow. | Decoding preflights input length: immediately rejects buffers > `MAX_ENCODED_ENVELOPE_BYTES` (65,536 bytes) with `RejectionCode::EnvelopeTooLarge`. | `oracle_prost_envelope_rejects_oversized_total_envelope` |
| **Oversized Event Payload** | Malicious peer embeds multi-megabyte payloads in envelope payload fields. | `event_payload` length validated against `MAX_PAYLOAD_BYTES` (4,096 bytes) during both `to_wire` and `from_wire`. | `oracle_prost_envelope_rejects_oversized_payload` |
| **Deep Ancestry Chain (Causal Tree Bomb)** | Attacker nests hundreds of output frames to blow recursion stacks or exhaust memory during causal rollback. | Ancestry frames count validated against `MAX_ANCESTRY_NODES = 129` and output depth against `MAX_ANCESTRY_DEPTH = 128`. | `oracle_prost_envelope_rejects_ancestry_depth_exceeded` |
| **Malformed / Truncated Wire Stream** | Incomplete, truncated, or corrupted wire frames sent over network socket. | `prost::Message::decode` catches malformed framing and wire errors, mapped to `RejectionCode::MalformedFraming`. | `oracle_prost_envelope_rejects_truncated_or_malformed_protobuf_bytes` |
| **Timestamp / Namespace Truncation** | Protocol implementations that downcast 128-bit simulation time or namespaces to 64-bit lose ordering invariants or cause collision. | Explicit 16-byte fixed payload serialization and validation guarantees full 128-bit range roundtrip. | `oracle_prost_envelope_roundtrip_preserves_full_identity_and_tick_range` |
| **Version & Protocol ID Hijacking** | Mismatched protocol or unsupported schema versions bypass validation. | Strict schema version check (`envelope_schema_version == 1`) and protocol ID validation (`GRPC_PROTOCOL_ID`). | `oracle_prost_envelope_rejects_unsupported_schema_version` |
| **Invalid Enum / Authority Modes** | Undefined authority mode or message kind integer values corrupt internal state. | Checked enum deserialization (`OptimisticAuthorityMode::from_u32`, `WireMessageKind::from_u32`) rejects unknown tags. | `oracle_prost_envelope_rejects_invalid_authority_mode`, `oracle_prost_envelope_rejects_invalid_message_kind` |

---

## 4. API Compatibility & Consumer Invariants

- **Public API:** `KairoOptimisticEnvelopeV1`, `AncestryFrame`, `OptimisticAuthorityMode`, `WireMessageKind`, `RejectionCode`, and `encode()` / `decode()` retain exact public function signatures and return types.
- **Additions:**
  - `pub fn to_wire(&self) -> Result<WireOptimisticEnvelope, RejectionCode>`
  - `pub fn from_wire(wire: WireOptimisticEnvelope) -> Result<Self, RejectionCode>`
  - `WireOptimisticEnvelope`, `WireAncestryFrame`, `WireRootFrame`, `WireOutputFrame`, `WireFrameKind`
- **Zero Breakage:** All 26 existing unit tests, integration socket tests (`two_process_smoke.rs`), and 12 new independent behavioral oracles pass with zero regressions.
