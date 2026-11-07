# Measured performance programme

Owner: [Track 111](tracks/111-measured-performance/index.md), with existing Tracks 18/31/47/48/50/51/52/55/78/101. This is an implementation plan, not a speedup claim.

## Observed starting point

The retained [3 October conservative-PDES capture](../docs/pdes/benchmark-results.md) measures approximately 4.4–5.0 million serial events/s and 6.7–23 thousand PDES events/s on small ring workloads. Construction, OS-worker creation/join and synchronization are included. It is historical bounded evidence, not an external simulator comparison or a measurement of this new candidate.

The [local scaling runner](../benches/hpc-scaling-certification/local-runner.mjs) generates placeholders. Its 92%/84% efficiencies are aggregation fixtures, never performance evidence. Keep historical raw records, label them and generate new results from real executions.

## Work packets and decision gates

### P1: profile the same computation

Freeze a serial Rust reference and model semantics. Capture event counts, exact integer-state parity, explicit floating-point tolerances, seeded stream identity, queue depth, allocations, peak RSS, thread creation, barrier/null-message cost, serialization, host/device transfer and end-to-end wall time. Report kernel-only and full-run timings separately. Record cold/warm runs, compiler/profile, source/dependency hashes, seed, input size, CPU topology and noise controls.

Reproduce the small ring baseline before optimization. Use multiple run orders and at least ten independent timed repetitions after predeclared warm-up; publish raw samples, median and uncertainty, not only the best run. Change repetition count only with a recorded precision rationale.

### P2: amortize parallel overhead

Reuse the already-adopted Rayon dependency for a bounded persistent pool if it preserves the PDES ownership model; compare a fixed std worker pool if deterministic communication/control is clearer. Do not create and join workers for every cheap event. Batch multiple eligible events within the proven conservative safe-time window. Keep lookahead, equal-time order, cancellation, anti-message and GVT semantics unchanged; never add scheduling jitter as simulated time.

A parallel strategy is eligible only after it matches the serial reference under adversarial inter-LP messages, tiny lookahead, cancellation, uneven workloads and injected delays. Thread-pool selection is not itself proof of speed.

### P3: storage and queue choices

Profile heap work, tombstones, allocations and component access. Compare existing binary heap against a calendar/ladder queue only on distributions where its assumptions hold; keep the heap fallback. Compare tight SoA columns with the existing aligned layout; measure RSS/cache misses rather than assuming padding helps. Reuse scratch buffers and reserve capacity where evidence shows allocation cost. Qualify generational handles and serialization before any ECS replacement.

### P4: ensembles before intra-model acceleration

Independent scenario replications and uncertainty draws already map naturally to Rayon. Preserve seed/entity/purpose isolation and deterministic result merge. Never confuse faster independent ensembles with strong scaling of one simulation. Evaluate common-random-number variance reduction independently of throughput.

### Provider and consumer hot paths identified by the boundary audit

Two source-derived hypotheses need separate measurements before adding parallelism:

- Kairos game `graph_relations::children_of` scans the component store for each
  parent; DFS/outgoing-edge queries repeat that scan. The optional Paignia adapter
  should build one validated adjacency snapshot with explicit game/ECS identity
  mapping and topology revision invalidation. Compare build-plus-solve time,
  repeated-solve amortization, allocations and memory against the existing route
  on sparse/dense/wide/deep finite games. Reject altered chance, tie or payoff
  semantics. Snapshot traversal can avoid repeated whole-store scans; no speedup
  has been measured by this audit. T17 owns the static solver representation;
  Kairos keeps only demonstrated generic storage mechanisms.
- CareOps `ConfigurablePathwayAdapter::run_simulation` scans incomplete patients
  across `0..=horizon_ticks`. Its consumer-owned follow-up compares sparse scheduled
  transitions with that baseline under long idle horizons and congested service
  cases. Freeze horizon/tie/censoring/RNG semantics first, report event counts and
  full-run timing, and distinguish algorithmic scheduling changes from a change
  in clinical hazard model. This is a candidate bottleneck, not a profile result.

The [existing composition handoff](design/careops-composition-plan-20261020.md#paignia-and-careops-boundary-review)
also routes experiment-lifecycle reuse. Its ED/logistics worker-count fixtures
test deterministic replication scheduling; they do not demonstrate intra-model
PDES speed or qualify an untested backend.

### P5: native GPU and distributed crossover

GPU evaluation includes compilation/setup, upload/download, batching, dispatch, readback and total model runtime. Demonstrate a real supported model beyond particle-addition kernels, including mixed branch behaviour. Test CUDA and Metal/WGPU separately; name exact numerical guarantees per backend.

Distributed evaluation measures serialization, communication density, rollback rate, GVT lag, checkpoints, crash recovery and whole-job completion. Cross-host MPI, multi-node schedulers and parallel filesystems need real hardware/run records. Avoid cluster claims from loopback or scalar transport tests.

## Crossover matrix

Vary: events (10^3–10^7 where feasible), per-event computation (cheap/moderate/expensive), LPs/workers, inter-partition message fraction, lookahead, contention, rollback frequency, state size and independent-world count. Use at least a queueing model, PHOLD-style model, spatial ABM and a held-out hybrid consumer scenario. Record regimes where serial wins.

Any automatic executor choice must select before a run from a recorded configuration and benchmarked decision rule. It cannot change replay semantics or switch mid-run without a separately qualified migration protocol. Provide a serial override and report the actual executor.

## Acceptance

Pass correctness first. Publish a speedup claim only for named workload/hardware/input ranges with comparable baseline semantics and confidence intervals. A suggested optimization acceptance target is >=20% improvement in its declared regime without >5% regression in the agreed serial/common workload panel; treat these as proposed gates to freeze before measurement, not observed results. If it fails, keep the simpler implementation and retain the negative result. Performance CI uses dedicated comparable runners; shared-runner smoke tests only detect gross failures.

