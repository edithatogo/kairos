# Arrow Schemas

Track 04 owns Arrow schema definitions for event logs, entity snapshots, and run metadata.

The first schema should be a versioned event log compatible with `conformance/fixtures/deterministic_ordering.json`.

## Event Log v1

`event_log_v1.schema.json` defines the initial `kairo_ecs.event_log.v1` table shape. The Rust smoke implementation in `crates/kairo-ecs-arrow` keeps this deliberately dependency-light for the R2 slice: it validates field order/types, encodes `u128` simulation ticks as 16 little-endian bytes, and round-trips a deterministic tabular payload without requiring native Arrow libraries.

The schema includes a `schema_version` field. Additive nullable fields can remain on major version 1; removals or type changes require a new stream name such as `kairo_ecs.event_log.v2`.
