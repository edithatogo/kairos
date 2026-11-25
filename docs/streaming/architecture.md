# Streaming Architecture

Track 36 provides an in-process stream contract without broker clients:

- `EventSource` consumes stream messages into a running simulation.
- `EventSink` publishes telemetry or deltas from a running simulation.
- `SnapshotProvider` exposes point-in-time snapshots for query-style transports
  such as Arrow Flight.
- `StreamMessage::validate_event_log_contract` enforces the dependency-free
  message contract before the in-memory adapter accepts a message.
- `InMemoryStream` rejects per-run sequence and time regressions so ordering
  checks can run without Kafka, NATS, WebSocket, or Arrow Flight services.
- `WallClockPacer` records the real-time pacing contract without changing the
  virtual-time scheduler owned by Track 01.

Stream messages mirror `kairo_ecs.event_log.v1` from
`conductor/contracts/arrow-schema-contract.md` and the Track 04 crate-level
schema version. Broker-specific implementations must preserve the same field
names and statuses so Arrow, WebSocket, Kafka, and NATS consumers see the same
logical payload.

## Evidence Boundary

Network adapters are unavailable. The crate implements neither broker
acknowledgements nor Arrow IPC or Arrow Flight RPC. The retired transport
aliases never connected to external services. See the
[migration decision](../adr/0027-retire-streaming-aliases.md).
