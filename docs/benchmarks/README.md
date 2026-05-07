# Benchmarks

This directory is the benchmark and reproducibility entry point for KairoECS.
It is intentionally source-backed and offline-first.

Benchmark readers should start here before reading the policy or reproduction
pages.

## What to read first

- [Benchmark policy](benchmark-policy.md)
- [Reproduce comparison](reproduce-comparison.md)

## What the benchmark surface proves

- The benchmark plan names the canonical scenario set.
- The smoke metadata stays aligned with the conformance fixture manifest.
- Track 18 reproducibility checks keep the ready fixture IDs and source files
  tied to real committed inputs.

## Local validation

```bash
python benches/benchmark_smoke.py
python benches/benchmark_reproducibility.py
```

## Related files

- `benches/benchmark-plan.md`
- `benches/benchmark-smoke.json`
- `benches/benchmark_smoke.py`
- `benches/benchmark_reproducibility.py`
