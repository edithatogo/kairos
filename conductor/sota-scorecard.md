# SOTA Readiness Scorecard

| Dimension | Current target | Release gate |
|---|---|---|
| Deterministic scheduler | Fixed-tick SimTime, stable event order | Property tests + replay fixtures |
| DES/ABM parity | Event-first model, ECS entities | Hybrid example passes |
| Cross-language compatibility | C ABI + generator facades | Two bindings pass conformance before beta |
| Python support | 3.10-3.14, optional 3.14 free-threaded checks | Wheels/smoke tests |
| C# support | .NET 10 stable, .NET 11 preview/GA lane | NuGet dry run and SafeHandle tests |
| Telemetry | Arrow IPC/Parquet outputs | Host languages read event logs |
| Reproducibility | seed manifest + scenario manifest + replay | At least DES and ABM examples emit manifests |
| V&V | statistical checks, sensitivity analysis | One validation tutorial |
| Documentation | GitHub Pages + API docs + tutorials | Docs build and link-check pass |
| Benchmarks | fair benchmark suite | Published reproducibility instructions |
| Supply chain | SBOM, attestations, Scorecard, Best Practices | Release checklist gate |
| Community | model zoo, good-first issues, governance | Contributor onboarding page |
| Publishing | staged registries with Track 42/43 gates | Trusted publishing or protected publication before production write |
| Maintenance | deprecation and compatibility policy | Public compatibility table |
| Interoperability | standards review | mapping docs published |
| HPC parity charter | Tracks 46-55 live-proof wave | Evidence manifests and claim-boundary scans |
| PDES production runtime | Conservative lookahead plus Time Warp | Live parity, deadlock, rollback, and scaling evidence |
| Distributed synchronization | Real MPI and gRPC transports | Multi-rank and multi-process runtime proof |
| NUMA and memory topology | hwloc, affinity, arenas, zero-copy layout | Topology, allocator, and FFI layout evidence |
| Parallel I/O | Arrow record batches, HDF5, ADIOS2 | Checkpoint/restart and filesystem throughput evidence |
| Native GPU acceleration | wgpu/CUDA persistent device memory | Real-device CPU parity and benchmark evidence |
| FMI co-simulation | FMI 2/3 master runtime | FMU archive, dynamic loading, 1,000-step, and OpenModelica proof |
| HPC runtime acceptance | Slurm, containers, Kubernetes, cloud batch | Live scheduler and provider canary evidence |
| Scaling certification | Weak and strong scaling profiles | Raw result manifests and release certification |
| Game theory ontology | Open Turtle/JSON-LD ontology with Rust component generation | Subrepo provenance, ingestion equivalence, and deterministic codegen |
| Graph-relational ECS | EntityId relationship components behind `graph-relations` | Feature isolation and pointer-free topology checks |
| Multi-game execution | Normal-form arrays plus extensive-form graph traversal | Solver parity fixtures and traversal conformance |

## Scoring rubric

```text
0 = not planned
1 = planned but no artifact
2 = artifact exists
3 = artifact tested or reviewed
4 = included in CI/release gates
5 = proven in public release
```
