# Agent Contract — 40 Time-Travel Debugging & Interactive Stepping

## Owner

```text
timetravel-agent
```

## Owned paths

```text
crates/kairo-ecs-debug/, docs/debugging/, website/time-travel-demo/
```

## Handoff rules

- Do not change public contracts without ADR.
- Do not modify other track paths without noting the dependency in `handoff.md`.
- The event trace recorder must not alter simulation behavior — it observes only.
- Trace file format must be versioned; any format change requires a version bump and migration path.
- Add tests before requesting integration.
- Update docs for user-visible behavior.
- Browser demo must remain a non-core dependency; workspace must build without it.
