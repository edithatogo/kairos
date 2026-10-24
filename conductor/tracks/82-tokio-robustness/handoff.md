# Handoff: Track 82 (Tokio Bounded Transport Runtime)

Updated: 2026-10-25. Maturity: complete; adoption decision: adopt.

## Summary

Phase 0, Phase 1, and Phase 2 completed. Adopted `tokio = "1.54.1"` (minimal features: `rt`, `net`, `io-util`, `sync`, `macros`, `time`) in `crates/kairo-ecs-grpc`. Replaced the handwritten, busy-polling `TcpListener` worker thread loop and linear connection scanning in `crates/kairo-ecs-grpc/src/socket.rs` with an isolated single-threaded Tokio adapter runtime. Bound inbound connection concurrency via `tokio::sync::Semaphore` (`MAX_CONCURRENT_INBOUND_CONNECTIONS = 256`), instituted immediate cooperative watch cancellation on shutdown without self-connect port hacks, and eliminated busy-sleep CPU consumption. Added 5 comprehensive behavioral and negative oracle tests in `crates/kairo-ecs-grpc/tests/tokio_transport_oracles.rs`.

## Files changed

- `CHANGELOG.md`
- `Cargo.lock`
- `crates/kairo-ecs-grpc/Cargo.toml`
- `crates/kairo-ecs-grpc/src/socket.rs`
- `crates/kairo-ecs-grpc/tests/tokio_transport_oracles.rs`
- `conductor/tracks/82-tokio-robustness/evidence/phase0-qualification.md`
- `conductor/tracks/82-tokio-robustness/adr-api-compatibility-and-redteam.md`
- `conductor/tracks/82-tokio-robustness/review-report.md`
- `conductor/tracks/82-tokio-robustness/plan.md`
- `conductor/tracks/82-tokio-robustness/handoff.md`
- `conductor/tracks/82-tokio-robustness/metadata.json`
- `conductor/tracks.yaml`
- `conductor/tracks.md`
- `conductor/phase-closeout.yaml`

## Contracts consumed

[Value policy](../../dependency-value-policy.md), `distributed-agent` ownership contract, and `spec.md`.

## Contracts changed

Bounded asynchronous transport runtime backing `GrpcSocketTransport`:
- `MAX_CONCURRENT_INBOUND_CONNECTIONS = 256` semaphore-based connection cap.
- `MAX_FRAME_BYTES = 128 * 1024` frame size cap with instant connection termination on violation.
- Cooperative cancellation via `tokio::sync::watch::channel` allowing clean shutdown without dummy self-connections.
- `active_connections(&self) -> usize` and `max_concurrent_connections(&self) -> usize` diagnostic accessors.
- Complete isolation of virtual simulation time (`Tick`, `SimTime`) from async timers.

## Tests added

- `crates/kairo-ecs-grpc/tests/tokio_transport_oracles.rs`:
  - `oracle_tokio_transport_clean_shutdown_and_cancellation`: Verifies immediate and complete worker shutdown (<500ms) and immediate port reuse without `EADDRINUSE`.
  - `oracle_tokio_transport_bounded_inbound_connection_saturation`: Floods listener with concurrent connections, verifying strict 256-connection cap and graceful overflow shedding without process exhaustion.
  - `oracle_tokio_transport_slow_consumer_and_oversized_frame_rejection`: Streams partial chunks and oversized frames (>128KiB), verifying strict rejection, frame isolation, and zero disruption to healthy peers.
  - `oracle_tokio_transport_reconnect_and_transient_failure_recovery`: Proves transport cleanly detects peer unavailability and recovers automatically once peer comes back online.
  - `oracle_tokio_transport_deterministic_handoff_ordering_despite_io_jitter`: Verifies strict FIFO inbox delivery and tick monotonicity across 50 sequenced events under concurrent network scheduling.

## Known risks

None. Tokio is strictly confined inside `crates/kairo-ecs-grpc/src/socket.rs` as an internal reactor adapter; no Tokio types or async signatures leak into PDES or Core domain models.

## Follow-up issues

Track 83 (`Tonic interoperable RPC transport`) builds on this bounded Tokio runtime foundation.

## Integration notes

All 47 tests in `kairo-ecs-grpc` (25 unit, 15 prost oracles, 5 tokio oracles, 2 multi-process socket tests) pass cleanly. `cargo deny` and `cargo audit` passed with 0 vulnerabilities across all 403 dependencies.

## Phase closeout evidence

- `$conductor-review`: Phase 0, Phase 1, and Phase 2 review executed and passed; see [review-report.md](review-report.md).
- accepted fixes: Adopted `tokio 1.54.1` with minimal features; replaced manual busy-sleep listener thread with single-threaded Tokio adapter runtime; added bounded connection semaphore (256); verified cooperative watch channel cancellation; clippy and fmt hygiene fixes; attached source-bound validation receipts.
- rejected findings: None.
- commit SHA: HEAD
- pushed ref: origin/feat/track-82-tokio-transport-runtime
- next-phase decision: Track 82 is Done; advance to Track 83 (Tonic interoperable RPC transport).
- Clean working tree verification: `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`.

### Validation Receipts Table

| Gate / Command | CWD | Commit | Toolchain | Exit Status | Artifact Path |
|---|---|---|---|---|---|
| `cargo test -p kairo-ecs-grpc --all-features` | repo root | HEAD | rustc 1.99.1 | 0 | `evidence/cargo-test-grpc.txt` |
| `cargo clippy -p kairo-ecs-grpc --all-targets --all-features -- -D warnings` | repo root | HEAD | rustc 1.99.1 | 0 | `evidence/cargo-clippy.txt` |
| `cargo fmt --check` | repo root | HEAD | rustc 1.99.1 | 0 | `evidence/cargo-fmt.txt` |
| `cargo deny check advisories sources licenses bans` | repo root | HEAD | cargo-deny 0.18.2 | 0 | `evidence/cargo-deny.txt` |
| `cargo audit` | repo root | HEAD | cargo-audit 0.21.0 | 0 | `evidence/cargo-audit.txt` |
| `pwsh scripts/validate_conductor_phase_gates.ps1 -TrackDir conductor/tracks/82-tokio-robustness` | repo root | HEAD | pwsh 7.5.0 | 0 | `evidence/conductor-phase-gates.txt` |
| `pwsh scripts/validate_conductor_dag.ps1` | repo root | HEAD | pwsh 7.5.0 | 0 | `evidence/conductor-dag.txt` |
| `pwsh scripts/validate_conductor_artifacts.ps1` | repo root | HEAD | pwsh 7.5.0 | 0 | `evidence/conductor-artifacts.txt` |
