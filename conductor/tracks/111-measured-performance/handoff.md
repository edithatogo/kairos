# Handoff

## Matched workload panel

The conservative runtime now folds every advertised bound into its
destination frontier once. The pinned Rust 1.99.1 repeat matched the tape,
ordered model state and counters across all 20 fresh process pairs. The
32-LP cheap PHOLD case took 23.4% less time end to end by the paired
median. The candidate-to-baseline ratio was 0.766 [0.733, 0.835].
The largest median increase on the other pinned profiles was 4.7%.
[Rust 1.99.1 receipt and paired samples](evidence/frontier-fold/README.md#pinned-toolchain-repeat).

The two-station comparison matched 3,435 ordered transitions for 1,024
arrivals across Kairos Scheduler, conservative PDES and SimPy. The
four-region spatial consumer matched 6,097 events, including 3,025 delayed
cross-region signals. Serial led on both of these shared-host workloads.
The original event-cost sweep includes regimes where the pool wins.
[SimPy comparison](evidence/queueing/README.md) ·
[Spatial/DES comparison](evidence/spatial/README.md) ·
[PHOLD cost sweep](evidence/phold/README.md).

The profile distinguishes thread creation and synchronization, allocator
traffic and retained heap capacity, tight versus aligned scans, snapshots,
JSON encoding and local transfer. Native device results include actual
Metal and CUDA model execution. The transport record measures a physical
two-host ring residual and crash/restart intervals with explicitly bounded
timers. The integrated distributed scale and restart matrix remains under
Track 55. [CPU and storage profiles](evidence/profiling/README.md) ·
[Device and transport receipts](evidence/backends/README.md).

## Remaining acceptance

Broader per-LP event batching needs a method that satisfies the tested
ordering and round commit requirements. Draining all currently eligible
events failed on a handler that inserts an earlier local follow-up.
Keep the existing one-event selection per process per round.
[Batching counterexample](evidence/profiling/README.md#event-batch-boundary).

The predeclared comparable dedicated-runner regression run remains open.
The GitHub repository runner inventory reported zero self-hosted runners
and the existing NAS and Windows SSH routes timed out. The shared-host
results support the bounded frontier change. Leave executor choice
explicit until there is an independently validated crossover rule.

The independent review and its fixes cover the integrated evidence.
The owning checkout is
`/Users/doughnut/.codex/worktrees/kairos-scientific-evidence-closeout/kairos` on `codex/track111-measured-performance-20261125`
under chat `01a1fd18-51c7-7963-a3cc-d262d54582ba`. The next empirical step is to
run the frozen matched protocol on an available dedicated runner.
[Review and command results](review-report.md).

## Matched multiround PHOLD

The optional production benchmark mode uses one seeded tape for the serial
scheduler and conservative executor. Across 24 cases with ten timed pairs
each, both executors agreed on 2,048 events, ordered per-LP state and
routing counters. The runs varied event cost, lookahead, message density
and LP count. Both legacy benchmark modes retained their deterministic
outputs. Independent review reproduced the reported medians and pair ratios.

Serial was faster for all cheap cases. At eight LPs, eight-tick lookahead
and the 75% remote target, the pool reached 2.097× serial throughput at
8,192 mixes per event. The paired interval was 1.991–2.148× for this shared
M1 Max run. The receipts retain cold runs, setup, dispatch and total time,
and process RSS. Dedicated-runner regression thresholds, held-out
workloads and crossover selection remain open.
[Protocol, samples and intervals](evidence/phold/README.md).

## Scheduler cancellation compaction

The scheduler retains its binary heap and pending ID set. Successful
cancellations compact the heap when its length exceeds twice the pending count
plus 1,024 entries. The same pass shrinks either allocation when its capacity
exceeds twice that threshold. The scheduler regression exercises this path
with full-width timestamps, equal-time priority and sequence ordering,
checkpoint replay, cancellation counters and the next event ID.

Release builds used Rust 1.99.1 and ten matched pairs per workload on each
host. One warm-up per variant preceded the pairs, which alternated execution
order. Both variants checked dispatch order and counters on every run. The
allocator probe records outstanding Rust allocation-request bytes after
draining the queue. It forwards allocation and reallocation to the system
allocator in both variants. Time is the scheduling and cancellation loop plus
drain. Intervals use 10,000 bootstrap resamples of the pairs with seed 111.

| Host | Workload | Baseline retained bytes | Compacted retained bytes | Paired compacted/baseline time ratio (95% interval) |
| --- | --- | ---: | ---: | ---: |
| macOS ARM64 | 1,000,000 successive cancellations | 83,886,156 | 163,916 | 0.253 [0.252, 0.255] |
| Linux x86-64 | 1,000,000 successive cancellations | 83,886,164 | 163,924 | 0.223 [0.219, 0.227] |
| macOS ARM64 | 250,000 cancellations after scheduling | 29,884,424 | 301,320 | 0.631 [0.612, 0.642] |
| Linux x86-64 | 250,000 cancellations after scheduling | 29,884,432 | 301,328 | 0.979 [0.953, 1.040] |
| macOS ARM64 | 1,000,000 schedules, 6.25% cancelled | 37,896 | 37,896 | 0.964 [0.960, 0.970] |
| Linux x86-64 | 1,000,000 schedules, 6.25% cancelled | 37,904 | 37,904 | 0.980 [0.959, 1.025] |

Both hosts met the predeclared limits of 1 MiB after successive cancellations,
2 MiB after the burst and 5% maximum median slowdown for mixed work. The
intervals include equal elapsed time for the Linux burst and mixed workloads.
The runs used shared hosts and the Linux container had a two-CPU quota.
[Receipt](evidence/scheduler-compaction/receipt.json) ·
[macOS samples](evidence/scheduler-compaction/macos.csv) ·
[Linux samples](evidence/scheduler-compaction/linux.csv).

With Rust 1.99.1 on the path, run the profile with a gzip Git archive of the
baseline commit from the receipt, the compacted version of
`crates/kairo-ecs-core/src/lib.rs`, and a new scratch directory. It builds two
locked standalone binaries and writes the samples to standard output.
[Profile runner](evidence/scheduler-compaction/profile.sh) ·
[Allocator and order probe](evidence/scheduler-compaction/probe.rs).

## Direct event handoff experiment

The profile used the same 2,048-input tape and 4,096 dispatches on four
logical processes. Separate allocator builds counted successful Rust
allocation and reallocation requests during setup and dispatch. One warm-up
preceded ten measured repetitions per configuration. At both event costs,
the median pooled dispatch count fell from 24,504 to 11,190 allocations.
Median requested bytes fell from 2,728,632 to 2,248,760. Scoped dispatches
fell from 45,912 to 32,598 allocations. These counts measure allocator
traffic across the dispatch phase.

The timing builds used the existing serial scheduler and both conservative
executors on the Apple M1 Max. Ten fresh process pairs per case split the
base-first and candidate-first orders equally. Each process used one
warm-up and two measured repetitions. Final state and reported runtime
counters matched across pairs. Setup, dispatch and total time are retained.
Dispatch includes worker initialization and RSS covers the whole process.

| Executor | Mixes per event | Baseline median ms | Candidate median ms | Paired baseline/candidate ratio with 95% interval |
| --- | ---: | ---: | ---: | ---: |
| Pooled | 0 | 17.566 | 16.318 | 1.093 [0.992, 1.118] |
| Pooled | 8,192 | 73.225 | 73.041 | 0.999 [0.928, 1.031] |
| Scoped | 0 | 73.402 | 73.372 | 0.998 [0.976, 1.018] |
| Scoped | 8,192 | 111.156 | 112.452 | 1.000 [0.943, 1.038] |

The 8.5% pooled zero-work point estimate fell below the predeclared
20% time reduction for adoption. The candidate is retained as an
experimental patch. Its 15 scoped and 16 pooled parity and protocol tests
passed. The intervals describe this shared host and workload.
[Receipt](evidence/round-dispatch/receipt.json) ·
[Timing samples](evidence/round-dispatch/paired.jsonl) ·
[Allocation samples](evidence/round-dispatch/allocation-counts.csv) ·
[Candidate patch](evidence/round-dispatch/candidate.patch).

Use the source, toolchain and features in the receipt to build the baseline
and candidate benchmark binaries. On a scratch checkout at that exact source,
apply the candidate patch with `git apply --unidiff-zero`. Run the Python
comparison with the directory containing `base-scoped`, `base-pooled`,
`candidate-scoped`, and `candidate-pooled`. The separate allocation patch
instruments the benchmark for the allocator counts.
[Comparison script](evidence/round-dispatch/compare.py) ·
[Allocation patch](evidence/round-dispatch/allocation-profile.patch).

## Matched ring and worker reuse

Chat `01a1fd18-51c7-7963-a3cc-d262d54582ba` owns the Kairos closeout worktree
on `codex/pdes-crossover-20261123`. The measured source is `a0a9ae53e3a536309abe67269167ad730f7f44d2`.
The scoped and pooled binaries used that same tree and Rust 1.99.1.
The runs completed on 23 November 2026. [Execution receipt](evidence/receipt.json).

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

The original slice completed the historical reproduction and the bounded
worker-pool comparison. The direct event handoff experiment adds allocation
evidence. Steady-state profiling, safe event batching, additional model
classes and dedicated-runner qualification remain open in the plan.

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

`$conductor-review` accepted Phase 0. The [review report](review-report.md)
records the accepted fixes, and the [phase ledger](../../phase-closeout.yaml)
records the evidence commit SHA, pushed ref and next-phase decision.
`pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`
verifies the clean publication after integration. The remaining acceptance
gates above keep the track In Progress.

