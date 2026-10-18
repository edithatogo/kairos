# KairoECS Crates

This directory is the implementation root for the Rust workspace (`23` crates across 5 architectural tiers).

Initial crates are deliberately narrow:

- `kairo-ecs-types`: shared public types and DTOs.
- `kairo-ecs-core`: deterministic event scheduler skeleton.
- `kairo-ecs-state`: entity lifecycle skeleton.
- `kairo-ecs-rng`: deterministic seed/stream skeleton.
- `kairo-ecs-ffi`: stable C ABI bridge skeleton.
- `kairo-ecs-uniffi`: UniFFI wrapper root anchored to the C ABI.
- `kairo-ecs-diplomat`: Diplomat wrapper root anchored to the C ABI.
- `kairo-ecs-bench`: benchmark helper crate for core and bridge scenarios.

Additional crates listed in `conductor/tech-stack.md` should be added only when their owning track reaches implementation.

## Full Workspace Crate Matrix

| Crate | Tier | Summary |
| :--- | :--- | :--- |
| `kairo-ecs-types` | Core Foundation | `SimTime`, `EntityId`, `EventId`, `Priority`, `ScheduleKey` |
| `kairo-ecs-core` | Core Foundation | Deterministic event `Scheduler`, cancellation, `NumaTopology` |
| `kairo-ecs-state` | Core Foundation | `World`, `ArchetypeTable` (dense SoA storage), `EventArena` |
| `kairo-ecs-rng` | Core Foundation | Deterministic `RngStream` and entity substream seeding |
| `kairo-ecs-des` | Domain Paradigms | `SimResource`, `PriorityResource`, `PreemptiveResource`, `Container`, `Store` |
| `kairo-ecs-abm` | Domain Paradigms | `GridSpace2D`, `ContinuousSpace2D`, `NetworkSpace` (CSR graph) |
| `kairo-ecs-game-ontology` | Domain Paradigms | Turtle & JSON-LD ontology parser, IR normalization, deterministic Rust codegen |
| `kairo-ecs-game-theory` | Domain Paradigms | Normal-form (`PureNashSolver`, `LemkeHowsonSolver`) & Extensive-form (`BackwardInductionSolver`, `CfrPlusSolver`, `graph-relations`) |
| `kairo-ecs-pdes` | Parallel & Distributed | Conservative lookahead PDES and Jefferson Time Warp optimistic rollback + GVT |
| `kairo-ecs-mpi` | Parallel & Distributed | `MpiProcessTransport` multi-rank collectives and GVT synchronization |
| `kairo-ecs-grpc` | Parallel & Distributed | `GrpcSocketTransport` and `KairoOptimisticEnvelopeV1` TCP framing |
| `kairo-ecs-gpu` | GPU Acceleration | `PersistentGpuSession`, `MetalBackend` (Apple Silicon unified memory), `WgpuBackend`, `CudaBackend` |
| `kairo-ecs-webgpu` | GPU Acceleration | Browser WGSL compute shader pipeline and staging buffers |
| `kairo-ecs-arrow` | Data Spine & I/O | Apache Arrow 53 `RecordBatch`, Parquet Zstd, C Data Interface FFI, HDF5/ADIOS2 checkpoints |
| `kairo-ecs-ml` | AI & RL | Multi-threaded `ParallelVecEnv`, `OrtSession`, `InferenceTickHook` |
| `kairo-ecs-fmi` | Digital Twin | FMI 2.0/3.0 co-simulation master, `.fmu` extraction, AAS submodels |
| `kairo-ecs-streaming` | Streaming | Real-time wallclock pacing, Kafka & Arrow Flight streaming contracts |
| `kairo-ecs-debug` | Debugging | `CowWorldSnapshot` delta snapshots, time-travel stepping, breakpoints |
| `kairo-ecs-viz` | Visualization | Headless snapshot and trajectory summary exporters |
| `kairo-ecs-cli` | Experimentation | `ParallelEnsembleRunner`, `SobolSensitivityAnalyzer`, `LatinHypercubeSampler`, `AbcSmcCalibrator` |
| `kairo-ecs-bench` | Benchmarking | Reproducible throughput and regression benchmark suites |
| `kairo-ecs-ffi` | Polyglot Bridges | `extern "C"` ABI (`include/kairo_ecs.h`) and Arrow C Data Interface export |
| `kairo-ecs-wasm` | Polyglot Bridges | `WasmSimulation` and `WasmTimeTravelInspector` (`wasm-bindgen`) |
| `kairo-ecs-uniffi` | Polyglot Bridges | UniFFI cross-language binding metadata |
| `kairo-ecs-diplomat` | Polyglot Bridges | Diplomat multi-language FFI definitions |
