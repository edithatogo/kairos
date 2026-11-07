---
title: Architecture
description: The Rust simulation core, optional execution backends, native adapters and downstream application boundary.
---

KairoECS keeps simulation semantics in Rust. An application supplies its model
and consumes the engine as a dependency. Language bindings convert arguments,
manage native lifetimes and return results; they do not provide another scheduler,
random generator or simulation implementation.

## Runtime boundaries

| Layer | Responsibility | Key Rust crates |
| --- | --- | --- |
| Application | Domain rules, inputs, calibration and decisions; CareOps is a downstream consumer | Application-owned code outside Kairos |
| Native adapters | C ABI, Rust-compiled Wasm and host object/lifetime conversion | `kairo-ecs-ffi`, `kairo-ecs-wasm`, `kairo-ecs-uniffi`, `kairo-ecs-diplomat` |
| Model primitives | Resources, processes, behaviours, spatial state and model-specific method interfaces | `kairo-ecs-des`, `kairo-ecs-abm`, optional game-theory crates |
| Execution and state | Scheduling, entity/component storage, random streams, replay and supported parallel/distributed modes | `kairo-ecs-core`, `kairo-ecs-state`, `kairo-ecs-rng`, `kairo-ecs-pdes`, transport crates |
| Data and observation | Optional native data exchange, traces, debugging and visualization | `kairo-ecs-arrow`, `kairo-ecs-debug`, `kairo-ecs-viz`, `kairo-ecs-streaming` |

The [workspace manifest](https://github.com/edithatogo/kairos/blob/main/Cargo.toml)
and [crate sources](https://github.com/edithatogo/kairos/tree/main/crates)
are authoritative for inventory and feature flags. A compiled module is not
necessarily a qualified production backend.

## Time, state and randomness

The core uses fixed integer ticks (`SimTime` stores `u128`) and a binary-heap
scheduler ordered by time, priority and insertion sequence. The basic scheduler
is single-threaded. Parallel runtimes own their logical-process execution and
must demonstrate equivalent permitted behaviour against an appropriate serial
reference.

Entity identifiers include generations. State storage offers different layouts;
cache alignment is a trade-off to measure, not a universal performance guarantee.
The current native random streams use ChaCha8, with entity/purpose isolation and
versioned checkpoint positions. Earlier SplitMix64 receipts remain historical;
upgrading an RNG requires explicit replay and consumer qualification.

Sources: [types](https://github.com/edithatogo/kairos/blob/main/crates/kairo-ecs-types/src/lib.rs),
[core](https://github.com/edithatogo/kairos/blob/main/crates/kairo-ecs-core/src/lib.rs),
[state](https://github.com/edithatogo/kairos/blob/main/crates/kairo-ecs-state/src/lib.rs),
[RNG](https://github.com/edithatogo/kairos/blob/main/crates/kairo-ecs-rng/src/lib.rs).

## Optional execution and data paths

- Conservative PDES and optimistic rollback have native implementations and
  bounded validation. MPI and gRPC qualification is separate from cluster scaling.
- GPU backends execute particular kernels. Current particle parity uses a
  numerical tolerance; it does not establish arbitrary GPU model execution or
  bitwise equality across devices.
- Arrow/Polars features provide actual native interchange paths. Buffer lifetime,
  schema compatibility, zero-copy claims and installed-consumer acceptance are
  checked at each interface. HDF5/ADIOS2-labelled contract bytes are not proof of
  physical HDF5/ADIOS2 output or resumable simulation checkpoints.
- Inference, vectorized RL and scientific-method surfaces have incomplete or
  demonstration implementations. Their names must not be read as qualification.

Consult [current capabilities](https://github.com/edithatogo/kairos/blob/main/docs/current-state.md)
and the [capability register](https://github.com/edithatogo/kairos/blob/main/conductor/capabilities.json)
for evidence and unresolved gates.

## Downstream dependency contract

CareOps owns healthcare entities, pathways, operational inputs and model
validation. Kairos owns generic execution semantics and exposes versioned Rust
APIs and optional interchange adapters. Changes cross that boundary through a
pinned dependency, compatibility fixtures and installed-consumer tests. Domain
code and duplicate kernels should not be copied into Kairos.

The [dependency and consumer plan](https://github.com/edithatogo/kairos/blob/main/conductor/tracks/114-dependency-and-consumer-contracts/spec.md)
records supported features, versioning, ownership and acceptance. The
[native-only decision](https://github.com/edithatogo/kairos/blob/main/docs/adr/0010-rust-only-execution.md)
applies to every language surface.
