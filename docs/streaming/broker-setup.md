# Streaming Broker Setup

Broker clients are not compiled by default. When real adapters are added, each
adapter must remain behind its dedicated feature flag and must accept externally
managed broker configuration.

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
