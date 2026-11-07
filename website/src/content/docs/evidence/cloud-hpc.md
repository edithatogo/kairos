---
title: Cloud/HPC Runtime Boundary
description: Correct track ownership and explicit distinction between native acceptance, deployment templates and cluster scaling.
---

Cloud/HPC work spans native runtimes, storage, deployment and measurement.
Locally executed Rust/MPI/GPU tests are real evidence within their scope;
templates, contract bytes and synthetic scaling fixtures prove different things.

## Programme ownership

| Track | Scope | Evidence boundary |
| --- | --- | --- |
| 46 | HPC programme charter and evidence governance | Defines acceptance; does not itself certify hardware performance. |
| 47 | Conservative PDES runtime | Measured single-host execution and parity; recorded small-workload slowdown. |
| 48 | Optimistic Time Warp | Bounded rollback, GVT, durable recovery and native execution qualification. |
| 49 | MPI/gRPC state synchronization | Real bounded native transport cases, including MPI two/four-rank execution; broader cross-host scaling remains separate. |
| 50 | NUMA, affinity and memory lifecycle | Topology and lifecycle implementation; benchmark the actual target topology before claiming an affinity benefit. |
| 51 | Parallel I/O and checkpoint export | Optional real Arrow/Parquet paths exist. HDF5/ADIOS2-shaped contracts are not physical format interoperability or complete engine restart. |
| 52 | Native GPU and persistent memory | Kernel-specific native evidence; no universal model speedup. |
| 53 | FMI co-simulation master | Native lifecycle and contract tests; third-party FMU and multi-rate numerical validation require explicit results. |
| 54 | Slurm/container/cloud acceptance | Deployment artefacts and bounded receipts; provider/platform execution must be demonstrated separately. |
| 55 | Weak/strong scaling certification | Synthetic local output remains blocked from certification; real multi-node measurement is required. |

## Use the right evidence

- [Current capabilities](https://github.com/edithatogo/kairos/blob/main/docs/current-state.md) and [capability register](https://github.com/edithatogo/kairos/blob/main/conductor/capabilities.json).
- [Measured PDES results](https://github.com/edithatogo/kairos/blob/main/docs/pdes/benchmark-results.md) and [corrected scaling report](https://github.com/edithatogo/kairos/blob/main/docs/benchmarks/scaling-certification-report.md).
- [Runtime evidence boundary](https://github.com/edithatogo/kairos/blob/main/docs/cloud-hpc/runtime-evidence-boundary.md) and [free-compute blocker plan](https://github.com/edithatogo/kairos/blob/main/docs/cloud-hpc/free-compute-blocker-plan.md).
- [AWS Batch](https://github.com/edithatogo/kairos/blob/main/docs/cloud-hpc/aws-batch.md), [Azure Batch](https://github.com/edithatogo/kairos/blob/main/docs/cloud-hpc/azure-batch.md), [GCP Batch](https://github.com/edithatogo/kairos/blob/main/docs/cloud-hpc/gcp-batch.md) and [Slurm](https://github.com/edithatogo/kairos/blob/main/docs/cloud-hpc/slurm.md) deployment guidance.

Independent simulations in an ensemble can scale differently from one tightly
coupled PDES model. Measure and report these workload classes separately with
raw timings, hardware, input hashes and correctness checks.
