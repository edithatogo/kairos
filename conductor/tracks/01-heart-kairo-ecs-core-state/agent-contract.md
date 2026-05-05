# Agent Contract — 01 The Heart: kairo-ecs-core & kairo-ecs-state

## Owner

core-scheduler-agent + ecs-agent + contracts-agent

## Owned paths

```text
crates/kairo-ecs-types, crates/kairo-ecs-core, crates/kairo-ecs-state, crates/kairo-ecs-rng
```

## Handoff rules

- Do not change public contracts without ADR.
- Do not modify other track paths without noting the dependency in `handoff.md`.
- Add tests before requesting integration.
- Update docs for user-visible behavior.
