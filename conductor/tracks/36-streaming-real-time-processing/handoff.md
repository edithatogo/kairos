# Handoff — 36 Streaming & Real-Time Processing

## Summary

Track 36 now has a broker-free contract scaffold for `kairo-ecs-streaming`. The crate defines streaming adapter traits, an Arrow-aligned event-log message with explicit schema versioning, dependency-free event-log contract validation, feature-gated in-memory adapter aliases, in-memory contract tests, per-run sequence regression checks, unknown-stream snapshot rejection, and the real-time wall-clock pacing contract. All broker adapters remain optional and gated behind Cargo feature flags.

## Files changed

- `crates/kairo-ecs-streaming/src/lib.rs`
- `crates/kairo-ecs-streaming/tests/feature_matrix.rs`
- `conductor/tracks/36-streaming-real-time-processing/test-matrix.md`
- `docs/streaming/architecture.md`
- `docs/streaming/broker-setup.md`
- `docs/streaming/stream-schema.md`

## Sources reviewed for this slice

`conductor/tracks/36-streaming-real-time-processing/spec.md`, `conductor/tracks/36-streaming-real-time-processing/plan.md`, `conductor/tracks/36-streaming-real-time-processing/test-matrix.md`, `conductor/tracks/36-streaming-real-time-processing/handoff.md`, `conductor/contracts/arrow-schema-contract.md`, `crates/kairo-ecs-arrow/src/lib.rs`, `crates/kairo-ecs-arrow/tests/schema_compatibility.rs`, and the current `crates/kairo-ecs-streaming/` and `docs/streaming/` files.

## Contracts changed

No shared contracts were changed. The crate mirrors the existing `conductor/contracts/arrow-schema-contract.md` event-log field names and carries the Track 04 crate-level event-log schema version locally.

## Tests added

- `crates/kairo-ecs-streaming/tests/feature_matrix.rs`
- Unit tests in `crates/kairo-ecs-streaming/src/lib.rs`
- Contract checks now reject wrong schema version, blank `run_id`, missing `event_id`, blank `event_kind`, wrong time scale, wrong stream name, and blank `payload_ref` before the in-memory sink accepts a message.
- In-memory sink checks now reject duplicate or decreasing `sequence` values for the same `run_id`.
- Snapshot checks now reject unknown stream names instead of returning an empty snapshot for typos.

Validated:

- `cargo check -p kairo-ecs-streaming --no-default-features`
- `cargo check -p kairo-ecs-streaming --all-features --tests`
- `cargo clippy -p kairo-ecs-streaming --all-targets --all-features -- -D warnings`
- `rustfmt --check crates/kairo-ecs-streaming/src/lib.rs crates/kairo-ecs-streaming/tests/feature_matrix.rs`

Blocked validation:

- `cargo test -p kairo-ecs-streaming --no-default-features`
- `cargo test -p kairo-ecs-streaming --all-features`

Both test commands compiled the crate but failed at Windows link time because this shell resolves `link.exe` to Git's `usr\bin\link.exe`, which exited with `0xc0000142` and `fatal error - couldn't create signal pipe, Win32 error 5`.

## Known risks

The streaming adapters must remain strictly additive and feature-gated. Any leakage of streaming dependencies into the core compilation path would violate the non-blocking release contract. Broker availability in CI must be solved with Testcontainers or embedded mode before integration tests can gate PRs. Current feature modules are contract test doubles only and must not be described as runtime Kafka, NATS, WebSocket, or Arrow Flight clients.

## Integration notes

`crates/kairo-ecs-streaming/` is included in the root workspace. The current Kafka, NATS, WebSocket, and Arrow Flight feature modules are in-memory contract test doubles only. The next implementation step is to replace those aliases with real adapters while keeping every external dependency feature-gated.
