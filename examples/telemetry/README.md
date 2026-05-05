# Telemetry Examples

Track 04 owns schema-backed telemetry examples.

The first example is `telemetry_event_log_roundtrip`, compiled through the `kairo-ecs-arrow` crate:

```bash
cargo run -p kairo-ecs-arrow --example telemetry_event_log_roundtrip
```

It builds a minimal dispatched-event record, validates the `kairo_ecs.event_log.v1` schema, and round-trips the dependency-light smoke payload used by the crate tests.
