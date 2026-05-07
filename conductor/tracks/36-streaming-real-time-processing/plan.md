# 36 Streaming & Real-Time Processing — plan.md

## Phase 0 — Track startup

- Read `conductor/workflow.md`, `conductor/contracts/arrow-schema-contract.md`, and the Track 04 handoff notes.
- Confirm owned paths: `crates/kairo-ecs-streaming/`, `docs/streaming/`.
- Review the Arrow telemetry schema and experiment runner CLI interface for stream hook points.
- Keep `agent-contract.md`, `risk-register.md`, `test-matrix.md`, and `handoff.md` current as implementation lands.

## Phase 1 — Contract alignment

- Define the streaming trait interfaces: `EventSource`, `EventSink`, `SnapshotProvider`.
- Align stream message schemas with the Arrow telemetry format from Track 04.
- Define the feature-flag matrix: `kafka`, `nats`, `websocket`, `arrow-flight`.
- Define the real-time wall-clock pacing contract and its interaction with the simulation clock.
- Propose contract changes through ADR if required.

## Phase 2 — Scaffold

- Keep the `crates/kairo-ecs-streaming/` crate feature-flag gated.
- Maintain `docs/streaming/` with architecture overview and broker setup guides.
- Add a smoke test that verifies the feature-flag matrix compiles cleanly.
- Keep the crate wired into the workspace `Cargo.toml`.

## Phase 3 — Implementation

- Implement the Kafka producer and consumer behind `kafka` feature flag.
- Implement the NATS pub/sub adapter behind `nats` feature flag.
- Implement the WebSocket bridge behind `websocket` feature flag.
- Implement Arrow Flight DoPut/DoGet endpoints behind `arrow-flight` feature flag.
- Implement the real-time wall-clock pacing mode.
- Add unit tests for each adapter with embedded or containerized brokers.
- Add integration tests for end-to-end stream flows.

## Phase 4 — Cross-track integration

- Integrate streaming hooks into the experiment runner CLI (Track 22).
- Validate Arrow IPC compatibility across stream-serialized payloads (Track 04).
- Run owned tests plus affected shared tests.
- Ensure workspace compiles with no streaming features enabled.
- Update docs and release notes.

## Phase 5 — Closeout

- Complete `handoff.md`.
- Record remaining decisions and follow-up tasks.
- Confirm CI gates including feature-flag matrix build.
- Mark the track ready for the next implementation wave.
