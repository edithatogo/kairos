---
title: KairoECS Documentation
description: A Rust simulation engine for reproducible discrete-event and agent-based models, with explicit capability and validation evidence.
---

**Build reproducible discrete-event and agent-based models in Rust.**

KairoECS brings event scheduling, entity/component state, resource queues and
agent behaviours into one modular engine. Optional Rust crates provide columnar
data exchange, parallel execution, distributed synchronization and GPU kernels.
Applications own their domain models and depend on Kairos through versioned
interfaces.

## Start here

- [Rust quickstart](polyglot/rust.md) — execute a small scheduler example.
- [Architecture](architecture.md) — understand state, time and adapter boundaries.
- [Installation](https://github.com/edithatogo/kairos/blob/main/docs/install.md) — build the pinned Rust source and select a native adapter.
- [Docs platform](docs-platform.md) and [Learning Coverage Matrix](https://github.com/edithatogo/kairos/blob/main/docs/tutorials/coverage-matrix.md) — documentation workflow and learning material boundaries.
- [Capabilities and evidence](https://github.com/edithatogo/kairos/blob/main/docs/current-state.md) — see what has been implemented, executed and independently checked.

## Alpha, with one simulation core

[v0.4.0-alpha.1](https://github.com/edithatogo/kairos/releases/tag/v0.4.0-alpha.1)
was published on 18 October 2026. Default-branch work is experimental, with
[explicit beta and stable-release gates](https://github.com/edithatogo/kairos/blob/main/conductor/release-gates.md).

**Rust is the only supported simulation core.** Python, R, Julia, TypeScript,
C# and Go are adapters to native Rust or Rust-compiled WebAssembly. A missing
native operation returns an error; a host-language engine is never substituted.
Former pure-language preview cores are unsupported immediately. See the
[compatibility notice](https://github.com/edithatogo/kairos/blob/main/docs/adr/0010-rust-only-execution.md)
before using an earlier quickstart or package.

## Capabilities at a glance

| Area | What to expect |
| --- | --- |
| Core DES and ABM | Ordered scheduling, cancellation, generational state, resource primitives, spatial behaviours and versioned RNG streams. |
| Data exchange | Optional Arrow/Polars and native ownership interfaces; qualify the installed consumer and enabled features. |
| Parallel and distributed | Implemented conservative and optimistic PDES, native MPI and typed gRPC, with bounded execution and recovery evidence. |
| GPU | Specific device kernels and persistent-buffer paths; general model offload and end-to-end speedup are not established. |
| Scientific extensions | Uneven maturity: named numerical methods, inference, RL and third-party FMI workflows have explicit completion gates. |

The [capability register](https://github.com/edithatogo/kairos/blob/main/conductor/capabilities.json)
records the scope of each claim. Track completion is not interchangeable with
scientific validation or release acceptance.

## Evaluate before scaling

The [measured conservative-PDES benchmark](https://github.com/edithatogo/kairos/blob/main/docs/pdes/benchmark-results.md)
reports slower execution than the sequential baseline on its small workloads.
The [corrected scaling report](https://github.com/edithatogo/kairos/blob/main/docs/benchmarks/scaling-certification-report.md)
separates generated fixtures from measurements. No general performance-leadership
or HPC scaling claim is currently supported by those fixtures.

For research and operational models, use the
[validation dataset and oracle plan](https://github.com/edithatogo/kairos/blob/main/conductor/validation-datasets.md),
record uncertainty and check predictions against independent data.

## Continue learning

- [Language adapters](polyglot/python.md), [TypeScript/Wasm](polyglot/typescript-wasm.md) and the [binding inventory](https://github.com/edithatogo/kairos/blob/main/bindings/README.md).
- [PDES and distributed evidence](evidence/pdes-distributed.md), [GPU evidence](evidence/gpu-webgpu.md) and [Conductor status](evidence/conductor-status.md).
- [Tutorials and examples](https://github.com/edithatogo/kairos/blob/main/docs/tutorials/index.md), [community](https://github.com/edithatogo/kairos/blob/main/docs/community/contributor-onboarding.md) and [trustworthy simulation](https://github.com/edithatogo/kairos/blob/main/docs/trustworthy-simulation/verification-validation-uncertainty.md).
- [Citation](https://github.com/edithatogo/kairos/blob/main/docs/research/citation.md), [release checklist](https://github.com/edithatogo/kairos/blob/main/docs/release/release-checklist.md) and [development programme](https://github.com/edithatogo/kairos/blob/main/conductor/frontier-programme-20261107.md).

## Documentation workflow

The active site is Astro/Starlight. Source documentation remains in `docs/`,
`bindings/`, `examples/` and `conductor/`. The
[docs platform](docs-platform.md) explains versioned pages, compatibility routes,
search and `llms.txt`.

Run `just docs-build` to build the site and `just docs-dev` to start the local
documentation server. The [contributor workflow](https://github.com/edithatogo/kairos/blob/main/docs/developer-experience/docs-workflow.md)
explains prerequisites and validation commands.

```bash
npm --prefix website ci
npm --prefix website run check:all
```
