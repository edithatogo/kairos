# Handoff

## Matched ring and worker reuse — 23 November 2026

Chat `01a1fd18-51c7-7963-a3cc-d262d54582ba` owns the Kairos closeout worktree
on `codex/pdes-crossover-20261123`. The measured source is `a0a9ae53e3a536309abe67269167ad730f7f44d2`.
The scoped and pooled binaries used that same tree and Rust 1.99.1.
[Execution receipt](evidence/receipt.json).

The eight historical strong/weak cases reproduced on `ba58873` with ten
samples per case. The four-LP strong ring processed 4,096 events in both
executors. Its medians were 0.871 ms for the serial scheduler and 78.699 ms
for scoped PDES. A separate two-second macOS stack sample observed thread
creation and join calls inside the parallel round.
[Historical samples](evidence/historical-ring.json).

The matched four-LP sweep varies dependent SplitMix64 mixes per dispatch
over the same 2,048-input tape. Each source emitted one successor event at
the next tick. Both builds matched the per-LP counts, ordered state checksums
and runtime counters across all measured pairs. The independent reviewer
reconstructed the tape hash and ordered checksums from the seed.

| Mixes per event | Serial median ms | Scoped median ms | Pool median ms | Scoped/pool ratio with 95% interval |
| ---: | ---: | ---: | ---: | ---: |
| 0 | 0.860 | 75.412 | 18.063 | 4.071 [3.903, 4.281] |
| 512 | 9.513 | 76.636 | 28.031 | 2.757 [2.611, 2.901] |
| 2,048 | 35.792 | 83.064 | 37.458 | 2.200 [2.154, 2.313] |
| 8,192 | 139.958 | 109.454 | 75.185 | 1.439 [1.395, 1.483] |
| 32,768 | 556.682 | 227.400 | 188.136 | 1.206 [1.126, 1.235] |

The serial-to-pool crossover fell between the 2,048 and 8,192 samples on
this Apple M1 Max. At 8,192 the paired serial/pool ratio was 1.852
[1.747, 1.885]. The pool exceeded the predeclared 20% cheap-work time
reduction and improved the median in all eight historical workload shapes.
This supports the opt-in `pdes-worker-pool` feature for the measured regime.

Ten fresh process pairs per cost alternated which binary ran first. Each
process ran one warmup and two measured repetitions with opposite executor
orders. Each pair contributes the geometric mean of its two ratios. The
interval resamples the ten pairs 10,000 times with seed 111 and takes the
2.5th and 97.5th empirical percentiles of the median. The RSS fields measure
peak memory across both passes within each process. Full timings include result
extraction. Pool retirement can complete afterward. Setup and dispatch
times are retained separately, with first pool startup included in dispatch.
The one-minute host load readings before and after were 21.25 and 23.35.
[Raw paired samples](evidence/paired-runs.jsonl) · [Reproduction runner](evidence/compare.py).

The worker reuse regression observed 64 distinct callback threads before the
change and one thread across both horizons in the pooled build. Review
identified an escaping panic during payload destruction. The failing fixture
reproduced it, and the shared handler now poisons the runtime before any
outbound commit. The final tests passed 210 cases with all features and 19
scoped runtime cases. The existing collector accepted the historical format.

The current slice completes the historical reproduction and the bounded
worker-pool comparison. Allocation and steady-state profiling, safe event
batching, additional model classes and dedicated-runner qualification remain
open in the plan. The next integration operation is exact-head hosted CI.

## Registration — 7 November 2026

The initial registration was Spec Approved with implementation evidence pending.

## Summary

Remove demonstrated overhead, establish a measured serial/parallel/GPU crossover and publish reproducible matched-workload results.

## Files changed

This track's specification, plan, metadata, contract, risk register, test matrix and index; coordinated programme records and registries. Runtime changes and hosted outcomes are recorded separately in the integration receipt.

## Contracts consumed

AGENTS.md, existing core/FFI/conformance/versioning contracts, the approved [programme](../../frontier-programme-20261107.md), Tracks 18, 31, 47, 48, 78, 101.

## Contracts changed

The programme establishes immediate Rust-only support; this track qualifies its own specific outcomes. No release/publication or downstream-pin acceptance follows from track registration.

## Tests added

Each implementation phase must add or identify the distinguishing oracles in test-matrix.md. Structural manifest/track checks are not native execution or scientific validation.

## Known risks

Batching or work stealing can alter causality and stochastic order. Correctness gates precede speed gates, and serial remains available where it wins.

## Follow-up issues

Execute the unchecked tasks in plan.md with original owner handoffs. Keep unsupported capability claims and release gates blocked until their evidence exists.

## Integration notes

Use isolated bounded worktrees. The root integration packet combines disjoint changes after review; actual commit/PR/CI outcomes go in the programme integration receipt. Preserve old immutable receipts.

## Phase closeout evidence

No phase is closed. Record $conductor-review, accepted fixes, commit SHA, pushed ref, scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree and the next-phase decision only when they occur. Current planning status is not execution evidence.

