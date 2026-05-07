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
- Keep the ready fixture IDs stable: `scheduler_ordering_v1`, `scheduler_cancellation_v1`, `rng_reproducibility_v1`, and `vvuq_scenario_replay_v1`.
- Keep the canonical benchmark scenario names stable: `schedule_1m_events`, `pop_1m_events`, `schedule_cancel_1m_mixed`, `create_1m_entities`, `component_insert_1m`, and `hybrid_des_abm_smoke_100k`.
- Treat `conformance/fixtures/manifest.json` as the source of truth for downstream track consumers and for `benches/benchmark-smoke.json`.
- Use the local runner and smoke commands as the baseline validation path:
  - `node tests/conformance/runner.mjs`
  - `node tests/conformance/runner.mjs --list`
  - `node tests/conformance/runner-self-test.mjs`
  - `python benches/benchmark_smoke.py`
