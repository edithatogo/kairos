# 36 Streaming & Real-Time Processing — spec.md

## Mission

Enable simulation engines to consume and produce live data streams via Kafka, NATS, WebSocket, and Arrow Flight. Support real-time mode for hardware-in-the-loop (HIL) and digital twin synchronization without sacrificing deterministic replay.

## Primary subagent

```text
streaming-agent
```

## Dependencies

```text
Track 04 (Arrow telemetry), Track 22 (experiment runner CLI).
Can design streaming contract in parallel after Arrow schema contract is stable.
```

## Owned paths

```text
crates/kairo-ecs-streaming/, docs/streaming/
```

## Blocked paths

```text
.github/ — owned by Track 13 (CI/CD)
crates/kairo-ecs-core/ — owned by Track 01
crates/kairo-ecs-arrow/ — owned by Track 04
crates/kairo-ecs-cli/ — owned by Track 22
bindings/ — owned by Tracks 06-11
```

## Parallel-safe with

```text
Most tracks are parallel-safe after their contract inputs are accepted. See conductor/parallel-execution.md for the wave model.
```

## Inputs

- Arrow telemetry schema contract from Track 04.
- Experiment runner CLI and scenario manifest from Track 22.
- Async runtime (Tokio).
- Kafka client libraries (rdkafka).
- NATS client libraries (async-nats).
- Arrow Flight RPC bindings.

## Outputs

- Streaming crate `crates/kairo-ecs-streaming/` with:
  - Kafka producer and consumer (feature-gated).
  - NATS pub/sub adapter (feature-gated).
  - WebSocket bridge for browser dashboards (feature-gated).
  - Arrow Flight DoPut/DoGet endpoints for snapshot queries during run.
  - Real-time wall-clock mode that paces simulation ticks to physical time.
- Documentation in `docs/streaming/` covering configuration, broker setup, and stream schemas.
- Integration examples demonstrating HIL and digital twin synchronization.

## Acceptance criteria

- Simulation can consume events from a Kafka topic during an active run.
- Telemetry publishes to a Kafka topic in real time without buffering delays.
- Arrow Flight endpoint serves snapshot queries during a running simulation.
- WebSocket bridge streams entity/component deltas to connected browser clients.
- Real-time mode drift stays within configurable tolerance (±1 ms per tick).
- All streaming features are gated behind Cargo feature flags; core simulation builds without any streaming dependency.
- Streaming crate passes `cargo test`, `cargo fmt`, and `cargo clippy`.
- `handoff.md` is completed before merge.

## Release implications

Streaming is optional — every adapter is gated behind its own feature flag (`kafka`, `nats`, `websocket`, `arrow-flight`). Non-blocking for headless release. A feature-flag smoke test must verify that the core workspace compiles and passes all tests with no streaming features enabled.

## Non-goals

- This track does NOT implement a custom streaming protocol.
- This track does NOT provide a management GUI for broker administration.
- This track does NOT guarantee exactly-once delivery semantics (at-least-once is acceptable for telemetry).
- This track does NOT replace or modify the core scheduler's virtual time mechanism.

## Quality gates

Use the gates in `conductor/quality-gates.md`. Track-specific gates must be listed in `test-matrix.md`.
