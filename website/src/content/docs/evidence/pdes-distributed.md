---
title: PDES and Distributed
description: Conservative Chandy-Misra-Bryant, Jefferson Time Warp optimistic rollback, MPI collectives, and gRPC distributed transports.
---

KairoECS implements both Conservative and Optimistic Parallel Discrete-Event Simulation (PDES) in `kairo-ecs-pdes`, backed by multi-process MPI (`kairo-ecs-mpi`) and TCP/gRPC socket (`kairo-ecs-grpc`) transports.

## Distributed Synchronization Mechanisms

| Mechanism | Crate & Type | Algorithmic Guarantees | Verification Status |
| --- | --- | --- | --- |
| Conservative Lookahead | `kairo-ecs-pdes::PartitionCoordinator` | Chandy-Misra-Bryant null-message lookahead windows (`safe_window_end = min_lvt + lookahead`) across 4/8/16/32 Logical Processes (LPs) | Verified bit-for-bit against single-threaded `Scheduler` |
| Jefferson Time Warp | `kairo-ecs-pdes::TimeWarpPartition` | Straggler detection (`timestamp < local_virtual_time`), generational bitset state restoration (`CompactCheckpoint`), and cascading `AntiMessage` cancellation | Verified in multi-rank straggler injection tests |
| Mattern GVT | `kairo-ecs-pdes::MatternGvtTracker` | Asynchronous Global Virtual Time lower bound across LPs and in-flight transient messages, driving fossil collection | Verified with in-flight message accounting |
| Multi-Rank MPI | `kairo-ecs-mpi::MpiProcessTransport` | Multi-process rank coordination, point-to-point LP messaging, and `all_reduce_min_gvt` collective synchronization | Local `mpirun` 2-rank & multi-process socket proof complete |
| Distributed TCP/gRPC | `kairo-ecs-grpc::GrpcSocketTransport` | Length-prefixed `KairoOptimisticEnvelopeV1` binary framing over non-blocking TCP sockets for events, antimessages, and GVT | Loopback multi-endpoint integration tests complete |

## Optimistic Time Warp Rollback Lifecycle

```text
Straggler Event (t = 12) arrives at LP 1 (current LVT = 20)
  │
  ├─► 1. Restore latest CompactCheckpoint with timestamp <= 12 (GenerationalBitset + component delta)
  ├─► 2. Emit AntiMessage for every output message sent by LP 1 in window (12, 20]
  ├─► 3. Re-enqueue straggler + rolled-back input events in deterministic (timestamp, priority, seq) order
  └─► 4. Advance MatternGvtTracker and fossil-collect checkpoints older than GVT
```

## Canonical Sources

- `docs/pdes/benchmark-results.md`
- `docs/pdes/time-warp-spike.md`
- `docs/distributed/deployment-guide.md`
- `docs/distributed/transport-trait.md`
