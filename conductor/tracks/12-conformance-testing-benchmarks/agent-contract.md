# Agent Contract — 12 Conformance, Testing & Benchmarks

## Owner

conformance-agent + performance-agent

## Owned paths

```text
conformance, tests/conformance, benches, crates/kairo-ecs-bench
```

## Handoff rules

- Do not change public contracts without ADR.
- Do not modify other track paths without noting the dependency in `handoff.md`.
- Add tests before requesting integration.
- Update docs for user-visible behavior.
- Keep fixture IDs, fixture semantics, and benchmark scenario names stable once published.
- Treat `conformance/fixtures/manifest.json` as the source of truth for downstream track consumers.
