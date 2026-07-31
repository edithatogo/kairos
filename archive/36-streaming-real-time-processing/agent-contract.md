# Agent Contract — 36 Streaming & Real-Time Processing

## Owner

```text
streaming-agent
```

## Owned paths

```text
crates/kairo-ecs-streaming/, docs/streaming/
```

## Handoff rules

- Do not change public contracts without ADR.
- Do not modify other track paths without noting the dependency in `handoff.md`.
- All streaming adapters must be feature-gated; core workspace must compile without any streaming feature enabled.
- Add tests before requesting integration.
- Document broker configuration and stream schemas for every adapter.
- Real-time mode must not alter virtual-time semantics or event ordering.
