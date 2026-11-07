# HPC scaling evidence status and correction

**Status: not certified.** Updated 7 November 2026.

The earlier version of this page presented generated local-harness figures as
an evaluation of engine scaling. That presentation was incorrect. The figures
were produced by fixed formulas and hard-coded baseline throughputs in
[`local-runner.mjs`](../../benches/hpc-scaling-certification/local-runner.mjs),
not by measuring the simulation engines or a cluster.

## What the retained output means

The eight scenario families and their weak/strong profiles exercise schema and
aggregation handling. The runner labels them
`measurement_kind: deterministic-local-placeholder` and
`certification_status: not-certified-live-hpc-required`. Its parity fields are
assigned fixture values, not results of executing independent models.

The earlier efficiency, speedup and parity figures must not be cited as measured
performance, model agreement or certified capability. The retained
[raw fixture output](hpc-scaling-results.json) and
[aggregate summary](hpc-scaling-summary.json) remain useful for testing the
reporting pipeline. The summary correctly reports
`certification_status: blocked-live-hpc-required`.

## Evidence that does exist

The [conservative-PDES benchmark](../pdes/benchmark-results.md) records actual
single-host timings, five samples per case and final-state checks against the
sequential Rust scheduler. Its small ring workloads were substantially slower
under PDES. It establishes the execution and parity of that workload at its
recorded revision; it does not establish general scaling or performance parity
with other simulators.

[Native MPI/gRPC qualification](../../conductor/tracks/49-distributed-mpi-grpc-state-synchronization/native-runtime-qualification-20261103.md)
covers bounded synchronization, rollback, transfer and crash/restart cases.
Those acceptance results are valuable but are separate from throughput or
scaling measurements. Historical receipts retain their original source and
capture-time status.

## Requirements for a measured scaling report

A replacement report must identify:

1. The exact engine, model and comparator revisions; enabled features and build
   profile; reproducible inputs, seeds and work definition.
2. Actual raw elapsed times, repetitions, warm-up policy, timing boundaries,
   memory use and uncertainty summaries, including unsuccessful runs.
3. CPU/GPU models, core/rank counts, host topology, interconnect, drivers,
   operating system and scheduler/job metadata.
4. Independent model-equivalence checks, with exact, tolerance-based or
   statistical parity specified before running the comparison.
5. Strong-scaling and weak-scaling measurements at justified workload sizes;
   independent ensemble parallelism reported separately from parallel execution
   of one simulation.
6. Immutable raw-output references, checksums, review and consumer acceptance
   for the claimed scope.

Placeholders, reference fixtures, missing hardware and failed runs must never
be converted into measured success. No throughput, efficiency or speedup is
certified by this page. See the [certification contract](hpc-scaling-certification.md),
[benchmark policy](benchmark-policy.md) and
[measured-performance plan](../../conductor/tracks/111-measured-performance/spec.md).
