# Current capabilities and evidence

Reviewed 7 November 2026 against
[`d56863505d938feb069af88a737eb7668113d14b`](https://github.com/edithatogo/kairos/commit/d56863505d938feb069af88a737eb7668113d14b).
This is a dated evidence baseline, not a claim that later changes have passed the
same checks. The [machine-readable capability register](../conductor/capabilities.json)
and the [delivery programme](../conductor/frontier-programme-20261107.md) record
current completion work.

## Release status

[v0.4.0-alpha.1](https://github.com/edithatogo/kairos/releases/tag/v0.4.0-alpha.1)
was published on 18 October 2026. It is a prerelease with source/release artefacts;
its existence does not establish publication of every language package or a
stable compatibility promise. Default-branch work after that tag is experimental.

The [beta and stable gates](../conductor/release-gates.md) require actual native
consumer execution, independent method checks, measured performance and coherent
release artefacts. Track completion counts are not a release-readiness metric.

## Execution policy

Effective 7 November 2026, **Rust is the only supported simulation core**.
Language bindings are adapters to Rust native code or Rust-compiled WebAssembly.
Host languages may orchestrate experiments, prepare input and analyse results;
they may not silently replace event scheduling, random streams or model execution.
Unavailable native operations fail explicitly. The earlier pure-language preview
cores are retired, without a parallel-support or gradual-migration period.

See the [decision and compatibility impact](adr/0010-rust-only-execution.md),
[installation guide](install.md) and [binding inventory](../bindings/README.md).
The policy does not imply that every adapter has already completed native
qualification. A metadata probe, library-version query or DTO roundtrip is a
separate result from running a model through the installed Rust engine.

## Evidence by capability

| Capability | Source or evidence | Supported conclusion and remaining work |
| --- | --- | --- |
| Scheduler, state and RNG | [Core](../crates/kairo-ecs-core/src/lib.rs), [state](../crates/kairo-ecs-state/src/lib.rs), [RNG](../crates/kairo-ecs-rng/src/lib.rs) | Rust implementations with ordered dispatch, generational identities and ChaCha8 stream/checkpoint contracts. Full application replay still requires model, RNG-version and numerical agreement. |
| DES and ABM | [DES](../crates/kairo-ecs-des/src/lib.rs), [staged allocation](../crates/kairo-ecs-des/src/staged_runtime.rs), [ABM](../crates/kairo-ecs-abm/src/lib.rs) | Implemented modelling primitives. They do not establish empirical validity of a hospital, factory or population model. |
| Arrow and Polars | [Manifest](../crates/kairo-ecs-arrow/Cargo.toml), [consumer qualification](../conductor/tracks/108-scientific-ecosystem-alignment/handoff.md) | Real optional table/interchange paths and ownership tests. Minimal-core/Wasm feature closure and each installed consumer remain explicit boundaries; registry release of the Git dependency closure is separately gated. |
| Conservative PDES | [Measured benchmark](pdes/benchmark-results.md) | Actual single-host execution and state parity. The recorded small workload is slower than sequential execution; no universal speedup follows. |
| Optimistic PDES and native transport | [Merged PR #366](https://github.com/edithatogo/kairos/pull/366), [native qualification](../conductor/tracks/49-distributed-mpi-grpc-state-synchronization/native-runtime-qualification-20261103.md) | Bounded MPI two/four-rank and Mac/Colab gRPC cases, rollback, entity transfer and crash/restart evidence. Cross-host MPI, cluster scheduling and broad scaling are not established. Historical draft/unmerged statements in retained receipts refer to their capture time. |
| GPU | [Metal tests](../crates/kairo-ecs-gpu/tests/metal_hardware_qualification.rs), [GPU evidence](gpu-compute/benchmark-results.md) | Device execution and persistent buffers for particular kernels. The floating-point parity test uses an absolute tolerance of `5e-5`; it is not a bitwise-equality guarantee. |
| Scientific methods | [Numerical audit](../conductor/tracks/102-ode-solvers-qualification/handoff.md), [game-theory source](../crates/kairo-ecs-game-theory/src/normal_form.rs) | The method named RK4 currently performs an Euler-style update; the named Lemke-Howson path is a limited 2×2 routine. These remain correctness/scope issues until independent method oracles pass. |
| ML and vectorized RL | [ML source](../crates/kairo-ecs-ml/src/lib.rs) | ONNX-like and vector-environment demonstrations do not establish inference or simulation-backed RL. Unavailable backends must be explicit errors in supported APIs. |
| FMI and parallel I/O | [FMI tests](../crates/kairo-ecs-fmi/tests/), [I/O qualification boundary](../conductor/tracks/51-parallel-io-arrow-checkpoint-export/handoff.md) | Native lifecycle loading and contract checks have bounded evidence. Third-party FMU interoperability, physical HDF5/ADIOS2 output and complete engine restart need their own acceptance results. |
| HPC scaling | [Corrected report](benchmarks/scaling-certification-report.md), [machine summary](benchmarks/hpc-scaling-summary.json) | Local placeholder output validates aggregation/schema handling only. It certifies no throughput, speedup, parity or scaling efficiency. |

## Hosted software verification baseline

The [3 November Rust verification job](https://github.com/edithatogo/kairos/actions/runs/41658006983/job/126030245516)
at `d568635` records 994 tests passed, zero skipped in that run, 1,161/1,239 core
lines covered (93.70%) and real native MPI smoke tests at two and four ranks.
The [workspace coverage job](https://github.com/edithatogo/kairos/actions/runs/41658006983/job/126039311480)
records 33,065/38,374 lines covered (86.17%) and a passing zero-regression decision.

Compile-time platform exclusions and feature-specific jobs still define what
executed. These metrics are software-verification evidence, not mathematical
proof, a benchmark ranking, or validation against real systems.

## Choosing a supported claim

For each result, name the exact source revision, operation, model, input hashes,
seed policy, backend, hardware and comparison rule. Distinguish exact discrete
replay, floating-point tolerance and statistical agreement. Preserve failed
attempts and historical receipts; do not reuse an old result for changed inputs.

Use [validation datasets and analytical oracles](../conductor/validation-datasets.md)
for new evaluations. The programme separates implementation, execution,
scientific validation, performance measurement, installed-consumer acceptance
and release acceptance.
