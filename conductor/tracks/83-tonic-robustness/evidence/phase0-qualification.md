# Phase 0 Qualification: Tonic Interoperable RPC Transport

**Track:** 83 (`83-tonic-robustness`)  
**Date:** 2026-10-25  
**Owner:** `distributed-agent`  
**Status:** Accepted  

---

## 1. Candidate Library Qualification

- **Crate:** `tonic`
- **Version Qualified:** `0.14.6`
- **Repository:** `https://github.com/hyperium/tonic`
- **License:** MIT (Permitted under workspace license policy)
- **MSRV:** Rust 1.88 (Fully compatible with Kairos MSRV 1.99.1)
- **Selected Features:**
  - `channel` (client HTTP/2 connection channel)
  - `server` (server HTTP/2 gRPC listener)
  - `router` (axum-based gRPC routing)
  - `transport` (standard Tonic transport abstractions)
  - `codegen` (type generation and codec stubs)
- **Excluded Features:** `tls-*` (managed externally / zero OpenSSL ambient dependencies), compression algorithms (`zstd`, `gzip` optional).

---

## 2. Supply Chain & Security Audits

- **`cargo deny check advisories sources licenses bans`**:
  - Result: **PASSED** (0 bans, 0 license violations, 0 source mismatches).
- **`cargo audit`**:
  - Result: **PASSED** (0 security vulnerabilities across 430 crate dependencies).

---

## 3. Mandatory Value Gate Assessment

1. **Replaced Production Code:**
   - Custom length-prefixed TCP framing in `crates/kairo-ecs-grpc/src/socket.rs` is proprietary and cannot interoperate with external Python/TypeScript/C# gRPC peers or standard distributed tracing proxies.
   - Tonic provides true standard gRPC over HTTP/2, enabling cross-language PDES simulation nodes to exchange optimistic envelopes and Mattern GVT proposals with standard protobuf schemas.
2. **Authoritative Boundaries Preserved:**
   - Simulation virtual time (`Tick`, `SimTime`) remains strictly synchronous and authoritative.
   - PDES causality rules, Time Warp rollback, Jefferson antimessages, and entity migration snapshots retain 100% engine control.
   - Deadlines and request timeouts are configured via `GrpcTransportConfig`.
3. **Adoption Decision:** **ADOPT**. Proceed to Phase 1 implementation.
