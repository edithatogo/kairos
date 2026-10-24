# Handoff: Track 81 (Prost Distributed Envelope Encoding)

Updated: 2026-10-25. Maturity: complete; adoption decision: adopt.

## Summary

Phase 0, Phase 1, and Phase 2 completed. Adopted `prost = "0.13.5"` (minimal features: `std`, `prost-derive`) in `crates/kairo-ecs-grpc`. Replaced 264 lines of manual big-endian offset slicing and pushing with strongly typed, declarative Protocol Buffers codecs deriving `prost::Message` and `prost::Oneof`. Updated `proto/simulation.proto` with canonical wire definitions (`OptimisticEnvelope`, `AncestryFrame`, `RootFrame`, `OutputFrame`, `OptimisticAuthorityMode`, `WireMessageKind`). Maintained 100% fidelity of 128-bit timestamps and simulation namespaces via explicit 16-byte fixed payload serialization. Maintained existing rejection codes (R01-R15) and size bounds (`MAX_ENCODED_ENVELOPE_BYTES = 65_536`, `MAX_PAYLOAD_BYTES = 4_096`, `MAX_ANCESTRY_DEPTH = 128`). Added 12 independent behavioral and negative oracle tests in `crates/kairo-ecs-grpc/tests/prost_envelope_oracles.rs`.

## Files changed

- `Cargo.lock`
- `crates/kairo-ecs-grpc/Cargo.toml`
- `crates/kairo-ecs-grpc/src/envelope.rs`
- `crates/kairo-ecs-grpc/proto/simulation.proto`
- `crates/kairo-ecs-grpc/tests/prost_envelope_oracles.rs`
- `conductor/tracks/81-prost-robustness/evidence/phase0-qualification.md`
- `conductor/tracks/81-prost-robustness/adr-api-compatibility-and-redteam.md`
- `conductor/tracks/81-prost-robustness/review-report.md`
- `conductor/tracks/81-prost-robustness/plan.md`
- `conductor/tracks/81-prost-robustness/handoff.md`
- `conductor/tracks/81-prost-robustness/metadata.json`
- `conductor/tracks.yaml`
- `conductor/tracks.md`
- `conductor/phase-closeout.yaml`

## Contracts consumed

[Value policy](../../dependency-value-policy.md), `distributed-agent` ownership contract, and `spec.md`.

## Contracts changed

Extended `crates/kairo-ecs-grpc` with typed Protobuf wire codecs:
- `WireOptimisticEnvelope`, `WireAncestryFrame`, `WireRootFrame`, `WireOutputFrame`, `WireFrameKind`
- `KairoOptimisticEnvelopeV1::to_wire(&self) -> Result<WireOptimisticEnvelope, RejectionCode>`
- `KairoOptimisticEnvelopeV1::from_wire(wire: WireOptimisticEnvelope) -> Result<Self, RejectionCode>`
- Wire versioning constants: `ENVELOPE_SCHEMA_VERSION_LEGACY_V1 = 1`, `ENVELOPE_SCHEMA_VERSION_PROTOBUF_V2 = 2`, `ENVELOPE_PROTOCOL_VERSION_V1 = 1`, `ENVELOPE_PROTOCOL_VERSION_V2 = 2`.
- Backward-compatible dual decoding in `KairoOptimisticEnvelopeV1::decode` (detects `[0, 0, 0, 1]` legacy v1 vs protobuf v2).
- Non-silent decode error recording in `GrpcSocketTransport` (`envelope_decode_failure_count()`, `envelope_decode_failures()`).
- Canonical protobuf schema definitions added in `proto/simulation.proto`

## Tests added

- `crates/kairo-ecs-grpc/tests/prost_envelope_oracles.rs`:
  - `oracle_prost_envelope_roundtrip_preserves_full_identity_and_tick_range`
  - `oracle_prost_envelope_roundtrip_with_deep_ancestry_chain`
  - `oracle_prost_envelope_rejects_ancestry_depth_exceeded`
  - `oracle_prost_envelope_rejects_oversized_payload`
  - `oracle_prost_envelope_rejects_oversized_total_envelope`
  - `oracle_prost_envelope_rejects_truncated_or_malformed_protobuf_bytes`
  - `oracle_prost_envelope_rejects_unsupported_schema_version`
  - `oracle_prost_envelope_rejects_invalid_authority_mode`
  - `oracle_prost_envelope_rejects_invalid_message_kind`
  - `oracle_prost_envelope_rejects_invalid_timestamp_byte_length`
  - `oracle_prost_envelope_rejects_missing_oneof_frame`
  - `oracle_prost_optimistic_message_full_roundtrip`
  - `oracle_legacy_v1_and_protobuf_v2_dual_compatibility`
  - `oracle_socket_receiver_records_envelope_decode_failures`

## Known risks

Resource limits protect against memory exhaustion: total envelope bounded to 65,536 bytes, payload bounded to 4,096 bytes, and ancestry chain depth bounded to 128 output frames (129 nodes total). Malformed wire bytes are strictly rejected with `MalformedFraming`.

## Follow-up issues

Advance to next sequenced track: Track 82 (`Tokio bounded transport runtime`).

## Integration notes

All 41 tests in `kairo-ecs-grpc` passed (25 unit tests, 14 oracle tests, 2 multi-process socket tests). Pure-Rust compilation verified with zero C or `libprotoc` dependencies. Passes `cargo deny check advisories sources licenses bans` (0 advisories, 0 bans, clean Apache-2.0 license) and `cargo audit` (402 dependencies scanned, 0 vulnerabilities).

## Phase closeout evidence

- `$conductor-review`: Phase 0, Phase 1, and Phase 2 review executed and passed; see [review-report.md](review-report.md).
- accepted fixes: Adopted `prost 0.13.5` with minimal features; replaced manual big-endian packing with typed protobuf codecs; verified 128-bit timestamp and namespace preservation; clippy and fmt hygiene fixes; attached source-bound validation receipts.
- rejected findings: None.
- Source-bound validation receipts:
  - **Crate Tests:** `cargo test -p kairo-ecs-grpc --all-features` | CWD: `/Volumes/PortableSSD/github/kairos` | Toolchain: `rustc 1.99.1` | Commit: `46a62a92f5e7d3ba1eafa6538f6610a8a761eb0b` | Exit: `0` | Artifact: [`evidence/cargo-test-grpc.txt`](evidence/cargo-test-grpc.txt)
  - **Workspace Tests:** `cargo test --workspace` | CWD: `/Volumes/PortableSSD/github/kairos` | Toolchain: `rustc 1.99.1` | Commit: `46a62a92f5e7d3ba1eafa6538f6610a8a761eb0b` | Exit: `0` | Artifact: [`evidence/cargo-test-workspace.txt`](evidence/cargo-test-workspace.txt)
  - **Clippy Lint:** `cargo clippy -p kairo-ecs-grpc --all-targets --all-features -- -D warnings` | CWD: `/Volumes/PortableSSD/github/kairos` | Toolchain: `rustc 1.99.1` | Commit: `46a62a92f5e7d3ba1eafa6538f6610a8a761eb0b` | Exit: `0` | Artifact: [`evidence/cargo-clippy.txt`](evidence/cargo-clippy.txt)
  - **Format Check:** `cargo fmt --all --check` | CWD: `/Volumes/PortableSSD/github/kairos` | Toolchain: `rustc 1.99.1` | Commit: `46a62a92f5e7d3ba1eafa6538f6610a8a761eb0b` | Exit: `0` | Artifact: [`evidence/cargo-fmt.txt`](evidence/cargo-fmt.txt)
  - **Supply-Chain Deny:** `cargo deny check advisories sources licenses bans` | CWD: `/Volumes/PortableSSD/github/kairos` | Toolchain: `rustc 1.99.1` | Commit: `46a62a92f5e7d3ba1eafa6538f6610a8a761eb0b` | Exit: `0` | Artifact: [`evidence/cargo-deny.txt`](evidence/cargo-deny.txt)
  - **Security Audit:** `cargo audit` | CWD: `/Volumes/PortableSSD/github/kairos` | Toolchain: `rustc 1.99.1` | Commit: `46a62a92f5e7d3ba1eafa6538f6610a8a761eb0b` | Exit: `0` | Artifact: [`evidence/cargo-audit.txt`](evidence/cargo-audit.txt)
  - **Conductor Phase Gates:** `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | CWD: `/Volumes/PortableSSD/github/kairos` | Toolchain: `pwsh 7.5` | Commit: `46a62a92f5e7d3ba1eafa6538f6610a8a761eb0b` | Exit: `0` | Artifact: [`evidence/conductor-phase-gates.txt`](evidence/conductor-phase-gates.txt)
  - **Conductor DAG:** `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` | CWD: `/Volumes/PortableSSD/github/kairos` | Toolchain: `pwsh 7.5` | Commit: `46a62a92f5e7d3ba1eafa6538f6610a8a761eb0b` | Exit: `0` | Artifact: [`evidence/conductor-dag.txt`](evidence/conductor-dag.txt)
  - **Conductor Artifacts:** `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1` | CWD: `/Volumes/PortableSSD/github/kairos` | Toolchain: `pwsh 7.5` | Commit: `46a62a92f5e7d3ba1eafa6538f6610a8a761eb0b` | Exit: `0` | Artifact: [`evidence/conductor-artifacts.txt`](evidence/conductor-artifacts.txt)
- pushed ref: origin/feat/track-81-prost-envelope-encoding
- next-phase decision: Track 81 is Done; advance to Track 82 (Tokio bounded transport runtime).
- Use `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` before advancement.
