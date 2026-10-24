# ADR: Tonic Interoperable RPC Transport Architecture & Threat Assessment

**Track:** 83 (`83-tonic-robustness`)  
**Status:** Approved  
**Date:** 2026-10-25  

---

## 1. Context & Architecture Decision

Distributed PDES nodes require standards-compliant gRPC over HTTP/2 to enable heterogeneous multi-agent simulation where foreign processes (e.g. Python Gymnasium environments, external data sinks, browser visualizers) interact with the simulation cluster.

### Key Architecture Decisions:
1. **Standards-Compliant gRPC Service:** Implement `SimulationTransport` matching `proto/simulation.proto` with methods `ExchangeEvents`, `SendOptimisticEnvelope`, `MigrateEntity`, `GvtSync`, `Heartbeat`.
2. **Zero Ambient Build Dependencies:** Use `tonic` and `prost` pure Rust derives with generated stubs, requiring zero host `protoc` binaries at build time for deterministic cross-platform compilation.
3. **Transport Isolation:** Dedicated background Tokio runtime manages the HTTP/2 event loop while exposing synchronous trait boundaries to the Kairos discrete-event engine.
4. **Wire Version & Metadata Validation:** Validate `protocol_id` (`kairo.ecs.distributed.grpc.v1`), `protocol_version`, and `envelope_schema_version` on every RPC, immediately rejecting incompatible wire peers with standard gRPC status codes (`tonic::Status::invalid_argument` / `tonic::Status::failed_precondition`).

---

## 2. Threat Modeling & Failure Recovery

| Threat / Failure Mode | Mechanism | Mitigation in Track 83 |
|---|---|---|
| **Version Skew / Deserialization Exploit** | Outdated or adversarial peer sends malformed envelope | Strict protobuf validation and protocol handshake; reject with `Status::invalid_argument`. |
| **Connection Drop / Peer Partition** | Network glitch severs HTTP/2 stream | Channel backoff retry policy; transport marks peer unavailable and retries without corrupting virtual tick. |
| **Slow Peer / HTTP/2 Window Starvation** | Remote peer stalls consumption | Bounded decoding frame size (`max_decoding_message_size`) and channel timeout; stalled RPCs return `Status::deadline_exceeded`. |
| **Virtual Time Leaks** | Wall-clock async timers race discrete ticks | Architectural invariant: Tokio timers never advance logical simulation time. |
