# Phase 0 Qualification: Tokio Bounded Transport Runtime

**Track:** 82 (`82-tokio-robustness`)  
**Dependency Qualified:** `tokio = { version = "1.54.1", default-features = false, features = ["rt", "net", "io-util", "sync", "macros", "time"] }`  
**Decision:** **ADOPT (GO)**  
**Date:** 2026-10-25  

---

## 1. Context and Problem Statement

`crates/kairo-ecs-grpc/src/socket.rs` provides real multi-process and networked PDES distributed simulation transport (`GrpcSocketTransport`) across distinct OS processes.

Prior to Track 82:
1. `GrpcSocketTransport` launched an OS thread running an ad-hoc, busy-polling loop over non-blocking `TcpListener` and `TcpStream`s with `thread::sleep(Duration::from_millis(1))`.
2. Connection handling scaled linearly $O(N)$ with active inbound connections, requiring periodic polling passes over a `Vec<InboundConnection>`.
3. Worker shutdown relied on relaxed atomic polling, which could introduce up to 1ms latency per loop iteration or delay process joins during high load.
4. Concurrency was unbounded: connection flooding or memory saturation from slow/unbounded consumers could degrade the listening OS process.

Adopting Tokio provides an asynchronous event-driven reactor (via epoll/kqueue) that eliminates busy-waiting, enables bounded connection pools via semaphores, and ensures clean, deterministic cooperative cancellation.

---

## 2. Supply Chain and Maintenance Audit

| Attribute | Assessment | Result |
|---|---|---|
| **Repository** | https://github.com/tokio-rs/tokio | Tier-1 foundational Rust async runtime |
| **Version** | `1.54.1` (pinned) | Passed |
| **License** | MIT | Clean under workspace `deny.toml` |
| **Transitive Dependencies** | `mio` 1.2.4, `socket2` 0.6.5, `pin-project-lite` 0.2.17, `tokio-macros` 2.7.2 | Clean |
| **Security Advisories** | `cargo audit` scanned 403 dependencies: 0 vulnerabilities; `cargo deny` passed: 0 advisories/bans | Clean |
| **MSRV** | 1.70 (workspace uses 1.86/1.99.1) | Compatible |
| **Features Selected** | Minimal subset: `rt`, `net`, `io-util`, `sync`, `macros`, `time` (no full/default bloat) | Passed |

---

## 3. Strict Boundary and Architectural Invariants

As specified by `spec.md` and the Kairos discrete-event contract:
1. **Confined Adapter Runtime:** Tokio is strictly confined inside `crates/kairo-ecs-grpc/src/socket.rs` as a transport I/O adapter. No Tokio types leak into `kairo-ecs-pdes`, `kairo-ecs-core`, or public FFI boundaries.
2. **Virtual Time Independence:** Simulation time (`Tick`, `SimTime`) and discrete-event ordering remain 100% authoritative in Kairos. Tokio timers (`tokio::time`) are strictly restricted to I/O transport timeouts and connection deadlines; they NEVER serve as virtual-time authorities.
3. **Bounded Resources:**
   - Inbound connections bounded via `tokio::sync::Semaphore` (`MAX_CONCURRENT_INBOUND_CONNECTIONS = 256`).
   - Frame sizes capped at `MAX_FRAME_BYTES = 128 * 1024` (128 KiB).
   - Inbound queues and decode failure diagnostic buffers capped at `MAX_RETAINED_DECODE_FAILURES = 64`.
4. **Deterministic Cancellation & Shutdown:**
   - Immediate cancellation via `tokio_util` / `tokio::sync::watch` or cancellation token.
   - Dedicated thread joins gracefully on `Drop` without thread leaks.
5. **No Global Async Conversion:** Core simulation loops, stepping, and scheduling remain 100% synchronous.

---

## 4. Frozen Numerical & Performance Budgets

- Maximum concurrent connections allowed: 256
- Max frame size: 131,072 bytes (128 KiB)
- Connect timeout: 500ms
- Graceful shutdown timeout: 2,000ms
- Memory overhead: < 4 MB resident set size for transport adapter runtime
