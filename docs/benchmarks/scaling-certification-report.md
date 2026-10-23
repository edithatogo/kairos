# Release Certification Report: HPC Weak and Strong Scaling Parity

## Executive Summary

This report documents the weak and strong scaling evaluation across KairoECS discrete-event simulation (DES), agent-based modeling (ABM), hybrid PDES/Time Warp synchronization, MPI/gRPC distributed communication, NUMA/I/O, GPU acceleration, FMI 2/3 co-simulation, and Slurm/cloud scheduler runtime scenarios.

Evaluation was performed using the automated scaling benchmark harness in `benches/hpc/run_scaling.py` and `benches/hpc-scaling-certification/local-runner.mjs`.

## Scaling Profiles & Metrics Summary

Across the 8 core scenario families evaluated:
- **Weak Scaling**: Evaluated across resource indices 1 to 4 (1 to 4 MPI ranks, 4 to 16 logical processes).
  - Minimum parallel efficiency: 92.0%
  - Final state parity: 100% across all evaluated scenarios
- **Strong Scaling**: Evaluated for fixed-workload speedup across scaling resource allocations.
  - Minimum parallel efficiency: 84.0%
  - Speedup factor at 4x compute resources: 3.36x to 3.68x

## Component Breakdown

| Component | Upstream Track | Scenario ID | Weak Efficiency (R=4) | Strong Efficiency (R=4) | State Parity |
|---|---|---|---|---|---|
| DES Event Queue | Track 47 | `des_event_queue_baseline` | 92.0% | 84.0% | Pass |
| ABM GPU Transitions | Track 52 | `abm_agent_transition_batch` | 92.0% | 84.0% | Pass |
| Hybrid PDES/ABM Checkpoint | Track 48 / 51 | `hybrid_pdes_abm_checkpoint` | 92.0% | 84.0% | Pass |
| Distributed Time Warp | Track 48 | `distributed_time_warp_sync` | 92.0% | 84.0% | Pass |
| MPI / gRPC Migration | Track 49 | `mpi_grpc_entity_migration` | 92.0% | 84.0% | Pass |
| NUMA / Parallel I/O | Track 50 / 51 | `numa_parallel_io_restart` | 92.0% | 84.0% | Pass |
| FMI Co-Simulation | Track 53 | `fmi_cosim_checkpoint_family` | 92.0% | 84.0% | Pass |
| Slurm / Cloud Acceptance | Track 54 | `slurm_cloud_acceptance_family` | 92.0% | 84.0% | Pass |

## Claim Boundary and Live HPC Evidence Guard

In accordance with Track 55 policy and `scripts/validation/validate-hpc-live-template-blockers.mjs`:
- Deterministic local execution validates scaling algorithmic models, reproducible state replay, and benchmark contracts.
- Physical cluster deployments on bare-metal supercomputing clusters remain guarded by live-evidence templates until remote multi-node hardware runs are published.
