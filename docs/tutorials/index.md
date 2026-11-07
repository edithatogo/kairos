# Tutorials

This directory is the source-backed tutorial layer for KairoECS. It is designed
for new users who want a practical path through the checked-in code and example
docs before package registries, notebooks, images, or hosted demos are treated
as release evidence.

## Learning paths

| Goal | Start here | What it proves |
|---|---|---|
| Learn the scheduler contract in Rust | [Rust getting started](rust-getting-started.md) | The core scheduler API, cancellation behavior, and deterministic event order are discoverable from checked-in source. |
| Execute Rust from Python | [Python getting started](python-getting-started.md) | A configured `NativeEngine` loads the actual Rust library and exercises its narrow scheduling and statistics surface. |
| Execute Rust/Wasm from TypeScript | [Wasm and TypeScript getting started](wasm-getting-started.md) | The loader requires a generated Rust/Wasm module; `not-configured` never enables a host-only core. |
| Build a first model | [Model-building tutorial](model-building.md) | A user can choose an example, identify maturity, define events/entities/outputs, and map the work to validation docs. |

## Source-backed examples

- [Documentation example index](../../examples/docs/README.md)
- [Community adoption path](../community/adoption.md)
- [Community model zoo](../community/model-zoo.md)
- [Model zoo inventory](../../examples/model-zoo/README.md)
- [Factory bottleneck example](../../examples/des/factory_bottleneck/README.md)
- [M/M/1 queue example](../../examples/des/mm1_queue/README.md)
- [Flocking example](../../examples/abm/flocking/README.md)
- [Emergency department flow example](../../examples/hybrid/emergency_department_flow/README.md)

## Coverage matrix

- [Learning Coverage Matrix](coverage-matrix.md) records which languages use a
  tutorial, example, or notebook artifact and which languages intentionally do
  not use notebooks.

## Claim boundary

These tutorials describe source-backed learning paths. Rust is the only supported simulation core; retired host-only previews are not a supported way to run a model. They do not claim that published
packages, hosted notebooks, generated image assets, native Wasm binaries, GPU
execution, or production support are available. Those claims remain gated by
the packaging, documentation-site, playground, release, and runtime tracks.

Run `powershell -NoProfile -ExecutionPolicy Bypass -File docs/tutorials/validate-tutorials.ps1`
from the repository root after changing tutorial links or learning-path wording.
