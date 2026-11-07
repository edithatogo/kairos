# KairoECS

[![ci-core](https://github.com/edithatogo/kairos/actions/workflows/ci-core.yml/badge.svg)](https://github.com/edithatogo/kairos/actions/workflows/ci-core.yml)
[![Docs Quality](https://github.com/edithatogo/kairos/actions/workflows/docs-quality.yml/badge.svg)](https://github.com/edithatogo/kairos/actions/workflows/docs-quality.yml)

**A Rust simulation engine for reproducible discrete-event and agent-based models.**

KairoECS combines event scheduling, entity/component state, resource queues,
agent behaviours and deterministic random streams. Optional Rust crates add
columnar data exchange, parallel execution, distributed synchronization and
GPU kernels. Applications such as CareOps own their domain models and consume
Kairos through a versioned dependency boundary.

Kairos owns simulation mechanisms. UOGTO supplies game semantics; [**Paignia**](https://github.com/edithatogo/paignia) is
the separate Rust game library for representations and solvers; CareOps, Innovate and reimbursement applications
own their models. Economic valuation belongs above neutral resource-use records.
See [scope and ecosystem](docs/ecosystem.md) for the accepted boundaries and
the distinction between implemented packages and planned extraction.

[Documentation](https://edithatogo.github.io/kairos/) ·
[Install from source](docs/install.md) ·
[Current capabilities and evidence](docs/current-state.md) ·
[Development programme](conductor/frontier-programme-20261107.md)

## Current release and support

**Alpha.** [v0.4.0-alpha.1](https://github.com/edithatogo/kairos/releases/tag/v0.4.0-alpha.1)
was published on 18 October 2026. The default branch contains later experimental
work; it is not a stable API or production-release promise. Beta and 1.0 have
[explicit acceptance gates](conductor/release-gates.md).

**Rust is the only supported simulation core.** Python, R, Julia, TypeScript,
C# and Go are integration surfaces. They may execute a compiled Rust native
library or Rust-compiled WebAssembly; they must never substitute a scheduler,
random generator or simulation model written in the host language. Missing
native capabilities return an explicit error. The former host-language preview
cores are unsupported immediately; see the [breaking-change notice](docs/adr/0010-rust-only-execution.md).
A successful package import or metadata check is not native-engine qualification.

## What you can build on

| Area | Implemented scope | Qualification boundary |
| --- | --- | --- |
| Core simulation | Ordered event scheduling, cancellation, generational entities, component storage and versioned random streams | Replay depends on the recorded engine, model, RNG and numerical contract. |
| DES and ABM | Resource queues, priority/preemption primitives, staged allocation, behaviours, spatial operations and hazard clocks | Applications must validate their own models and supported compositions. |
| Data exchange | Optional Arrow/Polars tables, IPC/Parquet paths and C Data ownership interfaces | Features and installed consumers need separate qualification; zero-copy applies only to proven boundaries. |
| Parallel and distributed execution | Conservative PDES, optimistic rollback, native MPI and typed gRPC synchronization | Bounded native acceptance exists; general HPC scaling and performance superiority are unproven. |
| GPU execution | Native backends and persistent buffers for specific particle/event kernels | Kernel evidence does not establish arbitrary model offload, universal bitwise parity or end-to-end speedup. |
| Scientific extensions | Game-theory, calibration, inference and FMI surfaces with differing maturity | Known algorithm gaps and demonstration backends are tracked; feature names do not certify scientific correctness. |

The [capability register](conductor/capabilities.json) separates implementation,
execution, validation and release acceptance. A Conductor track marked `Done`
means its recorded scope was completed; it does not certify every feature in
that topic.

## Verification and performance

At the reviewed baseline
[`d568635`](https://github.com/edithatogo/kairos/commit/d56863505d938feb069af88a737eb7668113d14b),
[hosted Rust verification](https://github.com/edithatogo/kairos/actions/runs/41658006983/job/126030245516)
reported 994 passing tests, 93.70% core line coverage and native MPI smoke runs
with two and four ranks. Those results belong to that revision and execution
matrix; each change requires its own relevant checks.

Read the [measured PDES benchmark](docs/pdes/benchmark-results.md) before choosing
parallel execution. Its small ring workloads were slower than the sequential
baseline. The [scaling-report correction](docs/benchmarks/scaling-certification-report.md)
explains why generated placeholder figures are not performance evidence.
General comparative performance and real-system model validity remain evaluation
work, with a [dataset and oracle plan](conductor/validation-datasets.md).

## Start with Rust

Clone the repository and use the toolchain pinned in `rust-toolchain.toml`:

```bash
git clone https://github.com/edithatogo/kairos.git
cd kairos
cargo test --locked -p kairo-ecs-core -p kairo-ecs-state
cargo test --locked -p kairo-ecs-des -p kairo-ecs-abm
```

The [Rust quickstart](docs/tutorials/rust-getting-started.md) introduces the
scheduler. The [installation guide](docs/install.md) explains the native-library
requirement for language wrappers. The [binding inventory](bindings/README.md)
records which operations each adapter can actually perform.

## Learn, evaluate and contribute

- [Documentation overview](docs/README.md), [tutorials](docs/tutorials/index.md) and [model building](docs/tutorials/model-building.md).
- [Rust crates](crates/README.md), [conformance](conformance/README.md) and [benchmark policy](docs/benchmarks/benchmark-policy.md).
- [Scenario replay](docs/scenarios/factory-bottleneck-run-replay.md), [model zoo](docs/model-zoo/inventory.md) and [verification, validation and uncertainty](docs/trustworthy-simulation/verification-validation-uncertainty.md).
- [Contributing](CONTRIBUTING.md), [security](SECURITY.md), [citation](docs/research/citation.md) and [release checklist](docs/release/release-checklist.md).
- [Conductor index](conductor/index.md), [track registry](conductor/tracks.yaml), [workflow](conductor/workflow.md) and [phase evidence](conductor/phase-closeout.yaml).
- [Consumer inventory](conductor/ecosystem-consumers.md), [movement/resource contract](conductor/contracts/movement-resource-usage.md) and [bounded maturity tasks](conductor/execution/README.md).

For a coherent Rust change, use `just ci`. Documentation changes use:

```bash
npm --prefix website ci
npm --prefix website run check:all
```

Prefer qualified maintained dependencies; see the
[dependency policy](conductor/dependency-value-policy.md). Python 3.15.2 remains
the selected tooling default. Necessary compatible profiles need a recorded
consumer and qualification; follow [AGENTS.md](AGENTS.md).

## License

Apache-2.0 OR MIT. See [LICENSE](LICENSE), [LICENSE-APACHE](LICENSE-APACHE)
and [LICENSE-MIT](LICENSE-MIT).
