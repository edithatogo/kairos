# ADR: Tokio Bounded Transport Runtime Architecture & Red-Team Assessment

**Track:** 82 (`82-tokio-robustness`)  
**Status:** Approved  
**Date:** 2026-10-25  

---

## 1. Context & Architecture Decision

`crates/kairo-ecs-grpc` requires robust asynchronous socket I/O for multi-process distributed PDES without converting Kairos's deterministic synchronous engine to async.

### Decision
Embed a dedicated, isolated single-threaded Tokio runtime (`tokio::runtime::Builder::new_current_thread().enable_io().enable_time().build()`) inside an internal worker thread in `GrpcSocketTransport`.

Key Design Choices:
1. **Isolated Reactor Thread:** The runtime runs on a dedicated background thread that manages `tokio::net::TcpListener` and connection futures.
2. **Synchronous Trait API:** The outer `GrpcSocketTransport` preserves the exact synchronous `PdesTransport` trait signature (`send`, `poll`, `poll_optimistic`), bridging via thread-safe shared queues (`Arc<Mutex<VecDeque<...>>>>`).
3. **Bounded Concurrency via Semaphore:** An explicit `Arc<tokio::sync::Semaphore>` caps simultaneous active TCP connections to 256. If a flood occurs, excess connections are immediately dropped or rejected without exhausting file descriptors or memory.
4. **Cooperative Cancellation:** Shutdown is signalled via `tokio::sync::watch::Sender<bool>`, aborting pending connection tasks and joining the worker thread within 2 seconds.

---

## 2. Red-Team Threat Modeling & Security Review

| Threat Vector | Mechanism / Attack | Mitigation in Track 82 |
|---|---|---|
| **Slowloris / Slow Readers** | Attacker opens connections and sends 1 byte per minute to hold file descriptors open | Bounded connection timeout and idle read deadline via `tokio::time::timeout`; uncooperative connections are aborted. |
| **Connection Flooding / FD Exhaustion** | Attacker attempts thousands of connections simultaneously | Inbound connections acquire a permit from `Arc<tokio::sync::Semaphore>` (cap 256). Excess connections are rejected immediately without worker thread lock contention. |
| **Oversized Message Frame Flooding** | Attacker streams gigantic byte frames to induce memory OOM | Frame length prefix is checked before buffering; any frame exceeding `MAX_FRAME_BYTES` (128 KiB) aborts the connection immediately. |
| **Virtual Time Corruption** | Asynchronous timers drift relative to discrete simulation ticks | Strict architectural invariant: Tokio timers are banned from advancing or checking simulation time. All events use discrete logical `Tick`. |
| **Hanging Shutdown / Zombie Threads** | Background runtime thread hangs on blocked socket reads during process teardown | Cancellation token triggers shutdown; `TcpListener` loop and connection tasks select on cancellation channel; runtime explicitly shuts down with timeout before thread join. |

---

## 3. Compatibility & Downstream Impact

- **Public API:** Zero breaking changes. `GrpcSocketTransport::bind`, `send`, `poll`, `poll_optimistic` retain exact interfaces and error types (`TransportError`).
- **Binary & Memory Footprint:** Net binary delta is minimal (~300 KB for stripped Tokio current-thread runtime). CPU usage drops significantly by eliminating the 1ms busy-sleep polling loop.
- **Platform Portability:** Runs identically across Linux (`epoll`), macOS (`kqueue`), and Windows (`IOCP`).
