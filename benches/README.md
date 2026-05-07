# Benchmarks

Benchmark harnesses belong here until they move into dedicated crates.

Track 12 owns the shared benchmark plan. Track 01 provides the core Rust kernels, Track 02 measures facade overhead, and Tracks 06-11 measure binding overhead against the same scenario names.

The docs-site entry point for benchmark readers is
[`docs/benchmarks/README.md`](../docs/benchmarks/README.md). It points at the
policy and reproducibility pages that Track 18 keeps aligned.

The canonical scenario names are `schedule_1m_events`, `pop_1m_events`,
`schedule_cancel_1m_mixed`, `create_1m_entities`, `component_insert_1m`, and
`hybrid_des_abm_smoke_100k`; they are listed in `benches/benchmark-plan.md` and
mirrored in `conformance/fixtures/manifest.json`.

`benches/benchmark-smoke.json` and `benches/benchmark_smoke.py` provide a
metadata-only smoke harness that verifies those names against the conformance
manifest without requiring native benchmark link tests.

Track 18 adds `benches/benchmark_reproducibility.py` as a lightweight
reproducibility evidence check. It verifies that the ready fixture IDs,
fixture source files, canonical benchmark scenarios, and smoke metadata remain
aligned with `conformance/fixtures/manifest.json`.

Local validation commands:

```bash
python benches/benchmark_smoke.py
python benches/benchmark_reproducibility.py
```
