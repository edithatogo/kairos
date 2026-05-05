# Tutorials

This directory is the source-backed tutorial layer for KairoECS. It is designed
for new users who want a practical path through the checked-in code and example
docs before package registries, notebooks, images, or hosted demos are treated
as release evidence.

## Learning paths

| Goal | Start here | What it proves |
|---|---|---|
| Learn the scheduler contract in Rust | [Rust getting started](rust-getting-started.md) | The core scheduler API, cancellation behavior, and deterministic event order are discoverable from checked-in source. |
| Try the Python facade | [Python getting started](python-getting-started.md) | A Python user can schedule, cancel, step, and inspect scheduler statistics with the local facade. |
| Understand the Wasm/TypeScript boundary | [Wasm and TypeScript getting started](wasm-getting-started.md) | The TypeScript facade and native-Wasm `not-configured` contract are documented without claiming a generated Wasm artifact. |
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

## Claim boundary

These tutorials are offline learning paths. They do not claim that published
packages, hosted notebooks, generated image assets, native Wasm binaries, GPU
execution, or production support are available. Those claims remain gated by
the packaging, documentation-site, playground, release, and runtime tracks.

Run `powershell -NoProfile -ExecutionPolicy Bypass -File docs/tutorials/validate-tutorials.ps1`
from the repository root after changing tutorial links or learning-path wording.
