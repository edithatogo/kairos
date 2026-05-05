# Streaming Architecture

Track 36 starts with a contract-first crate that does not pull broker clients into
the default build. The public surface is intentionally small:

- `EventSource` consumes stream messages into a running simulation.
- `EventSink` publishes telemetry or deltas from a running simulation.
- `SnapshotProvider` exposes point-in-time snapshots for query-style transports
  such as Arrow Flight.
- `WallClockPacer` records the real-time pacing contract without changing the
  virtual-time scheduler owned by Track 01.

Stream messages mirror `kairo_ecs.event_log.v1` from
`conductor/contracts/arrow-schema-contract.md`. Broker-specific implementations
must preserve the same field names and statuses so Arrow, WebSocket, Kafka, and
NATS consumers see the same logical payload.

## Feature Flags

| Feature | Purpose | Default |
|---|---|---:|
| `kafka` | Kafka producer and consumer adapter | off |
| `nats` | NATS pub/sub adapter | off |
| `websocket` | Browser dashboard bridge | off |
| `arrow-flight` | Snapshot query endpoint | off |

The current scaffold exposes in-memory adapter aliases for each feature so CI can
prove feature isolation before broker dependencies are introduced.
