# Streaming Broker Setup

Broker clients are not compiled by default. When real adapters are added, each
adapter must remain behind its dedicated feature flag and must accept externally
managed broker configuration.

The current crate scaffold has no network broker dependencies. The `kafka`,
`nats`, `websocket`, and `arrow-flight` feature flags compile in-memory contract
test doubles only, which means local validation does not require Docker,
Testcontainers, broker binaries, open ports, or network access.

## Kafka

Expected configuration keys:

- `bootstrap_servers`
- `topic`
- `client_id`
- `acks`

## NATS

Expected configuration keys:

- `server_url`
- `subject`
- `queue_group`

## WebSocket

Expected configuration keys:

- `bind`
- `path`
- `max_clients`

## Arrow Flight

Expected configuration keys:

- `bind`
- `stream`
- `snapshot_window_ticks`

All brokers must preserve at-least-once telemetry delivery. Exactly-once
semantics are out of scope for Track 36.

Until real clients land, this page is a configuration contract rather than an
operational setup guide.

## Tutorial: local contract smoke

Use the broker-free path while developing docs, examples, and feature gates:

```bash
cargo check --manifest-path crates/kairo-ecs-streaming/Cargo.toml --all-features --tests
cargo test --manifest-path crates/kairo-ecs-streaming/Cargo.toml --no-default-features
```

The first command proves that all adapter feature modules compile together. The
second command is the runtime unit-test gate once the local linker can execute
Rust tests. Neither command connects to Kafka, NATS, WebSocket clients, or Arrow
Flight servers.

## Evidence boundary

Current evidence proves the message contract, feature isolation, and in-memory
ordering guards. It does not prove broker authentication, broker acknowledgements,
partition ordering, consumer lag handling, TLS, backpressure, or Arrow Flight RPC
compatibility. Record those separately in `handoff.md` when live services are
actually run.
