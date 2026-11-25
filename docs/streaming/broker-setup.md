# Streaming Without a Broker

`kairo-ecs-streaming` provides an in-process event stream and wall-clock
pacing logic. It does not connect to Kafka, NATS, WebSocket clients, or Arrow
Flight servers. The [migration decision](../adr/0027-retire-streaming-aliases.md)
explains how to replace the retired aliases in local tests.

## Tutorial: local contract smoke

```bash
cargo test --locked -p kairo-ecs-streaming
```

The tests exercise the same `InMemoryStream` available to callers. No broker,
open port, or network access is required.

## Evidence boundary

The tests cover event-log validation, per-run ordering, snapshots and pacing
decisions. A real adapter needs independent evidence for delivery semantics,
authentication, backpressure, ordering, resynchronization and the actual
broker or client protocol. In-memory success does not qualify a live service.
