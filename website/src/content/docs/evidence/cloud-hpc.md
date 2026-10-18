---
title: Cloud/HPC Runtime Boundary
description: Cloud batch templates, Slurm/Kubernetes manifests, columnar checkpoint I/O, and hardware evidence boundaries.
---

KairoECS separates locally reproducible HPC software engineering from claims that require external multi-node supercomputing hardware.

## HPC Subsystem & Evidence Matrix (`Tracks 48–55`)

| Track | Subsystem | Local & Single-Host Verification (`Done`) | External Cluster Hardware Gate (`In Progress`) |
| --- | --- | --- | --- |
| `48` | HPC Programme Charter & Blocker Governance | `validate-hpc-parity-evidence.mjs` & `validate-free-compute-routes.mjs` | Final closure of Tracks 49–53, 55 on multi-node cluster hardware |
| `49` | Optimistic PDES & Generational Bitset State | `CompactCheckpoint`, `GenerationalBitset`, `MatternGvtTracker`, straggler rollback | 64+ rank distributed Time Warp scaling benchmark |
| `50` | Native MPI & gRPC Distributed Transports | `MpiProcessTransport`, `GrpcSocketTransport`, `KairoOptimisticEnvelopeV1`, OpenMPI 2-rank tests | Multi-node InfiniBand/RDMA fabric latency benchmark |
| `51` | Cloud/HPC Live Execution Templates | Docker, Kubernetes Job, Slurm (`sbatch`), AWS/Azure/GCP Batch templates & structural validators | Live multi-node Slurm / managed cloud batch cluster execution |
| `52` | Native GPU & Persistent Device Memory | `PersistentGpuSession`, Apple Silicon `MetalBackend` (`metal-backend`), `WgpuBackend`, `CudaBackend` | Dedicated multi-GPU NVIDIA CUDA H100/A100 cluster run |
| `53` | Official Apache Arrow 53 & Parquet Zstd | `ArrowTableBatch` (`arrow-array`/`arrow-schema` v53), `ParquetColumnarFile`, Arrow C Data FFI | Multi-terabyte distributed lakehouse ingestion benchmark |
| `54` | Native FMI 2.0/3.0 Dynamic Library Loading | `.fmu` ZIP extraction, `modelDescription.xml` parser, dynamic symbol loading, AAS digital twin | `Done` (fully verified in CI across Linux/macOS/Windows) |
| `55` | Parallel HDF5/ADIOS2 & NUMA Pinning | `Hdf5CheckpointFile`, `Adios2CheckpointEnvelope`, `discover_numa_topology`, `ParallelEnsembleRunner` | Multi-socket Lustre/GPFS parallel filesystem I/O benchmark |

## Canonical Sources

- `docs/cloud-hpc/runtime-evidence-boundary.md`
- `docs/cloud-hpc/free-compute-blocker-plan.md`
- `docs/cloud-hpc/aws-batch.md`
- `docs/cloud-hpc/azure-batch.md`
- `docs/cloud-hpc/gcp-batch.md`
- `docs/cloud-hpc/slurm.md`
