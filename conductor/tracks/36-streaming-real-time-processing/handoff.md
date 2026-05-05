# Handoff — 36 Streaming & Real-Time Processing

## Summary

Track 36 now has the first contract scaffold for `kairo-ecs-streaming`. The crate defines streaming adapter traits, an Arrow-aligned event-log message, feature-gated adapter aliases, in-memory contract tests, and the real-time wall-clock pacing contract. All broker adapters remain optional and gated behind Cargo feature flags.

## Files changed

- `crates/kairo-ecs-streaming/Cargo.toml`
- `crates/kairo-ecs-streaming/src/lib.rs`
- `crates/kairo-ecs-streaming/tests/feature_matrix.rs`
- `docs/streaming/architecture.md`
- `docs/streaming/broker-setup.md`
- `docs/streaming/stream-schema.md`

## Contracts consumed

`conductor/workflow.md`, `conductor/contracts/arrow-schema-contract.md`, Track 04 handoff notes, Track 22 experiment runner CLI interface.

## Contracts changed

No shared contracts were changed. The crate mirrors the existing `conductor/contracts/arrow-schema-contract.md` event-log field names.

## Tests added

- `crates/kairo-ecs-streaming/tests/feature_matrix.rs`
- Unit tests in `crates/kairo-ecs-streaming/src/lib.rs`

Validated:

- `cargo check --manifest-path crates/kairo-ecs-streaming/Cargo.toml --no-default-features`
- `cargo check --manifest-path crates/kairo-ecs-streaming/Cargo.toml --all-features`
- `cargo fmt --manifest-path crates/kairo-ecs-streaming/Cargo.toml --all --check`

Blocked validation:

- `cargo test --manifest-path crates/kairo-ecs-streaming/Cargo.toml --no-default-features`
- `cargo test --manifest-path crates/kairo-ecs-streaming/Cargo.toml --all-features`

Both test commands compiled the crate but failed at Windows link time because this shell resolves `link.exe` to Git's `usr\bin\link.exe`; rerunning with `rust-lld` then failed because Windows SDK libraries such as `kernel32.lib` were not visible.

## Known risks

The streaming adapters must remain strictly additive and feature-gated. Any leakage of streaming dependencies into the core compilation path would violate the non-blocking release contract. Broker availability in CI must be solved with testcontainers or embedded mode before integration tests can gate PRs.

## Integration notes

`crates/kairo-ecs-streaming/` is now included in the root workspace. The next implementation step is to replace the in-memory feature aliases with real Kafka, NATS, WebSocket, and Arrow Flight adapters while keeping those dependencies feature-gated.
