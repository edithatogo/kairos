---
title: PDES and Distributed
description: Implemented parallel and distributed Rust runtimes, bounded native acceptance, and measured performance limits.
---

KairoECS has implemented conservative and optimistic parallel discrete-event
simulation in Rust, with MPI and typed gRPC transport paths. Their evidence is
stronger than a scaffold-only description, but it has explicit boundaries.

## Current evidence

| Area | What the evidence supports | What remains separate |
| --- | --- | --- |
| Conservative PDES | Single-host event processing, final-state parity and repeated wall-clock measurements for the recorded ring workload. | General speedup, multi-host behaviour and useful scaling on other model families. |
| Optimistic Time Warp | Bounded rollback, antimessage, GVT, durable recovery and entity-transfer qualification. | A proof for all models, all failure schedules or arbitrary deployments. |
| Native MPI | Real two/four-rank smoke and bounded distributed execution cases. | Cross-host MPI, network-fabric behaviour, large-rank and cluster-scheduler acceptance. |
| gRPC | Native Mac/Colab and local transport cases in retained qualification receipts. | General WAN reliability, production security posture and end-to-end throughput. |

[PR #366](https://github.com/edithatogo/kairos/pull/366) was merged on 3 November
2026. Its [native-runtime qualification](https://github.com/edithatogo/kairos/blob/main/conductor/tracks/49-distributed-mpi-grpc-state-synchronization/native-runtime-qualification-20261103.md)
and associated receipts describe exact execution scope. Draft/unmerged wording
in historical captures refers to their capture time.

## Performance is an open engineering task

The [measured PDES benchmark](https://github.com/edithatogo/kairos/blob/main/docs/pdes/benchmark-results.md)
retains five samples per configuration on an Apple M1 Max at source
`45f01c49be1d15cefb64ad48e59ee0a7e4146b3e`. For those small ring workloads,
sequential throughput was about 4.4–5.0 million events/s while PDES achieved
about 6.7–23 thousand events/s. Each round created and joined OS workers, and
the 16/32-LP cases exceeded the host CPU count.

These are historical measurements, not a current-revision benchmark or a
comparison with another simulator. Use sequential execution as a measured
baseline, profile coordination cost, and evaluate persistent workers and larger
useful work per synchronization round before recommending parallelism.
See [Track 111](https://github.com/edithatogo/kairos/blob/main/conductor/tracks/111-measured-performance/plan.md).

Generated [scaling fixtures](https://github.com/edithatogo/kairos/blob/main/docs/benchmarks/scaling-certification-report.md)
provide no independent scaling or parity evidence. Preserve exact discrete
replay, floating-point tolerance and statistical-equivalence claims separately.
