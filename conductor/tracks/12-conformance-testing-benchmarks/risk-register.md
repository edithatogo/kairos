# Risk Register — 12 Conformance, Testing & Benchmarks

Severity scale: Likelihood 1-5 x Impact 1-5. Low 1-4, Medium 5-9, High 10-16, Critical 17-25.

| Risk | Likelihood | Impact | Severity | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Contract drift | 3 | 4 | 12 | Use `conductor/contracts` and conformance fixtures | conformance-agent | Conformance fixture fails on any subagent output |
| CI blind spot | 3 | 4 | 12 | Add track-specific workflow/test gate | ci-agent | A merge passes CI but breaks a downstream track |
| Public API churn | 4 | 3 | 12 | Gate public surface changes through ADR | conformance-agent | Public API changes without ADR >2 times |
| Package/version mismatch | 3 | 5 | 15 | Use publishing dry-runs before production | conformance-agent | Any dry-run reveals version mismatch |
| Fixture naming drift | 3 | 3 | 9 | Freeze fixture IDs in `conformance/fixtures/manifest.json`; keep `scheduler_ordering_v1`, `scheduler_cancellation_v1`, `rng_reproducibility_v1`, and `vvuq_scenario_replay_v1` aligned with the runner | conformance-agent | Fixture ID collision, rename, or manifest/source mismatch detected |
| Benchmark naming drift | 3 | 3 | 9 | Keep scenario names stable in `benches/benchmark-plan.md` and `benches/benchmark-smoke.json` | conformance-agent | Benchmark name change not reflected in the plan or smoke metadata |
| Smoke metadata drift | 2 | 4 | 8 | Keep `tests/conformance/runner.mjs` and `benches/benchmark_smoke.py` aligned on ready fixtures and canonical benchmarks | conformance-agent | Runner/list output or smoke metadata diverges from the manifest |
| Planned fixture gap | 2 | 3 | 6 | Keep the remaining planned families documented as future scope only: `des_resource_queue_v1`, `abm_behavior_update_v1`, `hybrid_des_abm_v1`, `arrow_event_log_v1`, and `ffi_lifecycle_v1` | conformance-agent | A doc claims one of the planned families is already ready |
| Chaos overclaiming | 2 | 4 | 8 | Keep `conformance/chaos/manifest.json` explicit that the current slice is metadata-only and has no checked-in runtime chaos runner or native link tests | conformance-agent | Docs or CI claim runtime fault injection before a native harness exists |
