# Track 55 Handoff

## Two physical hosts and a matched ring

The M1 Max Mac and the Intel i3-N300 Linux NAS dispatched the same Rust
model through Kairos framed TCP over SSH. All 44 observations matched the
native serial scheduler. The independent Python replay matched both seed
tapes and final states. Ten alternating pairs per scaling mode were timed.

With 4,096 total events, the one-host median was 144.214 ms and the two-host
median was 156.026 ms. The median paired speed ratio was 0.929×. With 2,048
events per host, the medians were 72.988 and 158.758 ms and paired weak
efficiency was 46.2%. Two hosts took longer in both cases on this
heterogeneous pair. Startup was outside the timed interval. The run
recorded local queue work, wall time, application frames and per-process RSS.

Independent review verified the hashes, event parity and measured ratios.
The broader certification needs integrated failure and restart, live
cluster storage and scheduler evidence on the intended allocation.
[Raw observations and reproduction](evidence/two-host-ring/README.md).

Last updated: 2026-10-23

## Summary

Track 55 owns final weak/strong scaling certification for HPC parity. It is
now implemented with the scaling benchmark runner (`benches/hpc/run_scaling.py`),
local execution harness, scenario aggregation, and release certification report
(`docs/benchmarks/scaling-certification-report.md`). Live HPC cluster evidence
remains cleanly bounded and templated per policy.

## Files changed

- `benches/hpc/run_scaling.py`
- `docs/benchmarks/hpc-scaling-results.json`
- `docs/benchmarks/hpc-scaling-summary.json`
- `docs/benchmarks/scaling-certification-report.md`
- `conductor/tracks/55-end-to-end-weak-strong-scaling-certification/plan.md`
- `conductor/tracks/55-end-to-end-weak-strong-scaling-certification/handoff.md`

## Contracts consumed

- Tracks 47-54 production runtime and evidence contracts.
- Track 46 evidence manifest.
- Tracks 18 and 31 benchmark policy.
- Tracks 42-44 publication and health gates.

## Contracts changed

- Added `benches/hpc/run_scaling.py` wrapping the local runner and validating against evidence manifests.
- Emitted full scaling evaluation results (`hpc-scaling-results.json`) and aggregated summary (`hpc-scaling-summary.json`).
- Authored release certification report (`docs/benchmarks/scaling-certification-report.md`).

## Tests added

- Verified with:
  - `python3 benches/hpc/run_scaling.py --mode weak --manifest conductor/tracks/55-end-to-end-weak-strong-scaling-certification/evidence.json`
  - `python3 benches/hpc/run_scaling.py --mode strong --manifest conductor/tracks/55-end-to-end-weak-strong-scaling-certification/evidence.json`
  - `node scripts/validation/validate-hpc-scaling-certification.mjs --self-test`
  - `node scripts/validation/validate-hpc-parity-evidence.mjs`
  - `node scripts/validation/validate-free-compute-routes.mjs`
  - `python3 benches/benchmark_smoke.py`
  - `python3 benches/regression/compare.py`

## Known risks

- Bare-metal cluster deployments require remote cluster runtime execution before replacing live templates with cluster telemetry.

## Follow-up issues

- Replace draft blockers with live raw-result manifests after cluster infrastructure is provisioned.
- Run multi-node cluster benchmarks on production Slurm partitions when hardware is available.

## Integration notes

- Release and publication workflows consume `docs/benchmarks/scaling-certification-report.md` and `docs/benchmarks/hpc-scaling-summary.json`.

## Phase closeout evidence

- Benchmark runner: `python3 benches/hpc/run_scaling.py --mode both` passed.
- Benchmark smoke: `python3 benches/benchmark_smoke.py` passed.
- Regression thresholds: `python3 benches/regression/compare.py` passed.
- Scaling validator: `node scripts/validation/validate-hpc-scaling-certification.mjs --self-test` passed.
- $conductor-review: review performed with zero findings.
- accepted fixes: none required.
- commit SHA: pending.
- pushed ref: origin/main.
- validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree: pending clean working tree commit.
- next-phase decision: Track 55 implementation slice complete.
