# Handoff

Updated: 2026-10-25. Maturity: completed; implemented and verified.

## Summary

Track 83 qualifies and implements `tonic = "0.14.6"` and `prost = "0.13.5"` to provide standards-compliant gRPC simulation RPC transport for distributed PDES nodes. The implementation is pure-Rust and completely eliminates external host toolchain dependencies (`protoc`, `prost-build`), preserving cross-platform deterministic builds. `TonicTransport` implements the `PdesTransport` trait with full support for remote discrete events, optimistic Jefferson Time Warp envelopes with rollback ancestry chains, dynamic entity migration, and Mattern GVT consensus rounds over HTTP/2.

## Files changed

- `crates/kairo-ecs-grpc/Cargo.toml`: Added `tonic = { version = "0.14.6", default-features = false, features = ["channel", "server", "router", "transport", "codegen"] }` and configured Tokio features.
- `crates/kairo-ecs-grpc/src/envelope.rs`: Added wire message types for gRPC RPCs (`WireRemoteEvent`, `WireMigrationRequest`, `WireMigrationAck`, `WireGvtProposal`, `WireGvtDecision`, `WireHeartbeat`, `WireHeartbeatAck`).
- `crates/kairo-ecs-grpc/src/service.rs`: Pure-Rust tonic service definitions (`SimulationTransport`, `SimulationTransportServer`, `SimulationTransportClient`) and `ProstCodec` bridge.
- `crates/kairo-ecs-grpc/src/tonic_transport.rs`: `TonicTransport` implementation of `PdesTransport`.
- `crates/kairo-ecs-grpc/src/lib.rs`: Exported `service` and `tonic_transport` modules.
- `crates/kairo-ecs-grpc/tests/tonic_transport_oracles.rs`: 6 independent behavioral and negative oracles.
- `conductor/tracks/83-tonic-robustness/`: ADR, qualification evidence, plan, and review report.

## Contracts consumed

- `kairo_ecs_pdes::PdesTransport`, `LogicalEventId`, `LpId`, `OptimisticAuthority`, `OptimisticMessage`, `RemoteEvent`.
- `kairo_ecs_types::EntityId`, `SimTime as Tick`.
- Conductor dependency value policy and security audit gates.

## Contracts changed

- Extended `kairo-ecs-grpc` public API with `TonicTransport`, `SimulationTransport`, `SimulationTransportClient`, `SimulationTransportServer`. No breaking changes to existing contracts.

## Tests added

- 6 independent behavioral and negative oracles in `crates/kairo-ecs-grpc/tests/tonic_transport_oracles.rs`:
  1. `oracle_tonic_transport_clean_lifecycle_and_port_rebinding`
  2. `oracle_tonic_transport_wire_protocol_and_version_rejection`
  3. `oracle_tonic_transport_independent_grpc_peer_rpc_interoperability`
  4. `oracle_tonic_transport_transient_failure_recovery_and_deadline_policy`
  5. `oracle_tonic_transport_migration_and_gvt_consensus_round`
  6. `oracle_tonic_transport_optimistic_envelope_full_roundtrip_and_pdes_ordering`

## Known risks

- macOS single-process ephemeral port/file descriptor limits: default `ulimit -n = 256`; concurrent client-server connections bound to `MAX_CONCURRENT_INBOUND_CONNECTIONS = 64`.

## Follow-up issues

- Advance to Track 84 (`84-hecs-storage-eval`) for ECS storage qualification.

## Integration notes

- Validated via `just ci`, `cargo test -p kairo-ecs-grpc`, `cargo deny check`, and `cargo audit`.

## Phase closeout evidence

- Conductor Review: Approved in `review-report.md`.
- Cargo deny: 0 advisories, 0 bans, clean licenses.
- Cargo audit: 0 vulnerabilities across 430 dependencies.
