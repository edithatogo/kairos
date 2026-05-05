# Benchmarks

Benchmark harnesses belong here until they move into dedicated crates.

Track 12 owns the shared benchmark plan. Track 01 provides the core Rust kernels, Track 02 measures facade overhead, and Tracks 06-11 measure binding overhead against the same scenario names.

The canonical scenario names are listed in `benches/benchmark-plan.md`.
`benches/benchmark-smoke.json` and `benches/benchmark_smoke.py` provide a
metadata-only smoke harness that verifies those names against the conformance
manifest without requiring native benchmark link tests.

Track 18 adds `benches/benchmark_reproducibility.py` as a lightweight
reproducibility evidence check. It verifies that the ready fixture IDs,
fixture source files, canonical benchmark scenarios, and smoke metadata remain
aligned with `conformance/fixtures/manifest.json`.
