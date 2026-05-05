# Agent Contract — 02 The Bridge: kairo-ecs-ffi, UniFFI & Diplomat

## Owner

ffi-agent + uniffi-agent + diplomat-agent

## Owned paths

```text
crates/kairo-ecs-ffi, crates/kairo-ecs-uniffi, crates/kairo-ecs-diplomat, include/
```

## Handoff rules

- Do not change public contracts without ADR.
- Do not modify other track paths without noting the dependency in `handoff.md`.
- Add tests before requesting integration.
- Update docs for user-visible behavior.
