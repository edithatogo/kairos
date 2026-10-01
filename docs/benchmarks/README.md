# Benchmarks

Maturity: preview metadata gate. Native performance comparisons are not yet
published.

This directory is the benchmark and reproducibility entry point for KairoECS.
It is intentionally source-backed and offline-first.

Benchmark readers should start here before reading the policy or reproduction
pages.

## What to read first

- [Benchmark policy](benchmark-policy.md)
- [Reproduce comparison](reproduce-comparison.md)
- Raw-results policy manifest: `benches/raw-results-policy.json`

## What the benchmark surface proves

- The benchmark plan names the canonical scenario set.
- The smoke metadata stays aligned with the conformance fixture manifest.
- Track 18 reproducibility checks keep the ready fixture IDs and source files
  tied to real committed inputs.
- The raw-results policy blocks public performance claims until command,
  environment, seed, fixture, baseline, and raw output artifacts are archived.
- GPU performance claims additionally require Track 52 live wgpu or CUDA
  hardware evidence. The current Track 52 local CPU parity manifest is scaffold
  evidence only and does not publish a speedup result.

## Local validation

```bash
python benches/benchmark_smoke.py
python benches/benchmark_reproducibility.py
```

Expected output: both commands emit JSON with `"status": "ok"`. These checks
confirm benchmark metadata and reproducibility evidence alignment only; they do
not publish or validate native timing results.

## Related files

- `benches/benchmark-plan.md`
- `benches/benchmark-smoke.json`
- `benches/raw-results-policy.json`
- `benches/benchmark_smoke.py`
- `benches/benchmark_reproducibility.py`
- `conductor/hpc-evidence/manifests/track52-local-cpu-parity-scaffold.json`
- `conductor/hpc-evidence/manifests/track52-live-gpu-hardware-template.json`
