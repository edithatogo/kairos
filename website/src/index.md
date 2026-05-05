# KairoECS Documentation

KairoECS is a Rust-first simulation engine for deterministic event scheduling, ECS-style state, Arrow telemetry, and polyglot bindings.

## Start Here

- [Documentation overview](../../docs/README.md)
- [Implemented crate inventory](../../crates/README.md)
- [Language binding inventory](../../bindings/README.md)
- [Documentation examples](../../examples/docs/README.md)
- [Community adoption](../../docs/community/adoption.md)
- [Model zoo](../../docs/community/model-zoo.md)
- [Model zoo inventory](../../docs/model-zoo/inventory.md)
- [Starter kits](../../docs/starter-kits/README.md)
- [Scenario run and replay](../../docs/scenarios/factory-bottleneck-run-replay.md)
- [Playground](../../docs/community/playground.md)
- [Headless snapshot playground](../../docs/playground/headless-snapshot.md)
- [Citation and archival](../../docs/research/citation.md)
- [Developer workflow](../../docs/developer-experience/docs-workflow.md)
- [Trustworthy simulation](../../docs/trustworthy-simulation/verification-validation-uncertainty.md)
- [Factory bottleneck VVUQ note](../../docs/validation/factory-bottleneck-v1-vvuq-note.md)

## Tutorials and Examples

- Factory bottleneck tutorial: [scenario run and replay](../../docs/scenarios/factory-bottleneck-run-replay.md) plus the [VVUQ note](../../docs/validation/factory-bottleneck-v1-vvuq-note.md).
- Playground tutorial: [headless snapshot walkthrough](../../docs/playground/headless-snapshot.md) and [interactive playground notes](../../docs/community/playground.md).
- Starter examples: [starter kits](../../docs/starter-kits/README.md), [model zoo inventory](../../docs/model-zoo/inventory.md), and [documentation examples](../../examples/docs/README.md).
- Jupyter notebooks and figures: notebook/image assets are tracked by the examples and education tracks; this site manifest keeps those entry points discoverable without vendoring large generated media into the static shell.

## R2 Implementation Map

- Core and shared contracts: `kairo-ecs-types`, `kairo-ecs-core`, `kairo-ecs-state`, `kairo-ecs-rng`, `kairo-ecs-des`, `kairo-ecs-abm`, `kairo-ecs-arrow`, and `kairo-ecs-ffi`.
- Binding bridges and facades: `kairo-ecs-uniffi`, `kairo-ecs-diplomat`, `kairo-ecs-wasm`, plus preview language packages for Python, R, Julia, TypeScript/Wasm, C#, and Go.
- Optional and advanced tracks now represented in docs: visualization, GPU/WebGPU, PDES, distributed MPI/gRPC, streaming, ML inference, FMI digital twin, cloud/HPC, and time-travel debugging.
- Native binding status remains explicit: preview packages may expose pure-language scheduler facades while reporting native FFI as not configured until Track 02 publishes stable local artifacts.
- Maturity language is staged: alpha means evidence-backed but limited, beta means wider release gates are active, and stable remains blocked until compatibility and release-candidate gates pass.

## Binding Quick Links

These language quickstart entry points document the current binding maturity and the available pure-language or native bridge paths.

- [Python binding](../../bindings/python/README.md)
- [R binding](../../bindings/r/README.md)
- [Julia binding](../../bindings/julia/README.md)
- [TypeScript/Wasm binding](../../bindings/typescript/README.md)
- [C# binding](../../bindings/csharp/README.md)
- [Go binding](../../bindings/go/README.md)

## Language Quickstarts

- Rust starts from the crate inventory and release compatibility notes.
- Python, R, Julia, TypeScript/Wasm, C#, and Go start from the binding README pages above.
- Each quickstart must keep native FFI support clearly bounded until the release and compatibility gates prove otherwise.

## Local workflow

- `npm --prefix website run check:all`
- `just docs-build`
- `npm --prefix website run check:links`
- `npm --prefix website run check:quality`
- `just docs-dev`
- `just check-docs`
- `just validate-conductor`
- `just validate-tracks`
- `just validate-track-docs`
- `just validate-conformance`
- `just dev-validate`

## Current docs tree

- `docs/adr/` for architecture decisions, naming, and release staging.
- `docs/api/api-review-template.md` for API review intake.
- `docs/benchmarks/benchmark-policy.md` for benchmark policy and comparability.
- `docs/benchmarks/reproduce-comparison.md` for the committed fixture and smoke-metadata replay path.
- `docs/cloud-hpc/` for AWS Batch, Azure Batch, GCP Batch, and Slurm runner notes.
- `docs/community/` for contributor onboarding, governance, adoption, model-zoo guidance, roadmap notes, and the playground.
- `docs/community/adoption.md` for the adoption path.
- `docs/community/model-zoo.md` for the example inventory.
- `docs/community/playground.md` for the interactive demo surface.
- `docs/debugging/` for trace format and interactive debugging CLI references.
- `docs/design/api-review.md` for design review guidance.
- `docs/distributed/` for deployment, transport, telemetry aggregation, and entity migration notes.
- `docs/fmi-digital-twin/` for FMI import/export, AAS mapping, and deployment model notes.
- `docs/gpu-compute/` for GPU/WebGPU architecture, hardware requirements, benchmark results, and maintainer notes.
- `docs/interoperability/standards-review.md` for interoperability standards review.
- `docs/ml/` for ML integration architecture, model versioning, and surrogate authoring.
- `docs/pdes/` for PDES protocols, logical-process traits, GVT, and benchmark results.
- `docs/release/` for release and supply-chain checklists.
- `docs/research/citation.md` for citation and archival guidance.
- `docs/scenarios/` for committed run/replay scenario notes.
- `docs/starter-kits/` for starter-kit discovery and maturity guidance.
- `docs/streaming/` for real-time processing architecture, stream schema, and broker setup.
- `docs/trustworthy-simulation/` for replay, seeds, and uncertainty notes.
- `docs/validation/` for fixture-backed VVUQ notes.

## Documentation Quality Gate

- `website/docs-link-manifest.json` lists the source docs and binding pages the site must keep reachable.
- `npm --prefix website run check:links` verifies required paths, navigation targets, and local Markdown links without writing generated output.
- `npm --prefix website run build` renders `website/build/index.html`, source-backed HTML pages for each Markdown navigation target, `website/build/docs-index.json`, `website/build/sitemap.xml`, and `website/build/robots.txt`; the build output is ignored by git.
- `npm --prefix website run check:quality` verifies build artifacts, docs concepts, navigation coverage, generated source pages, and the static output size budget.
- The generated docs index is intentionally offline-first and dependency-light, so CI and local contributors do not need network access after checkout.

## Contributor commands

- `cd website && npm ci && npm run build`
- `cd website && npm run check:links`
- `cd website && npm run check:quality`
- `cd website && npm start`
- `just docs-dev`

## Site owner

Track 14 owns the public docs surface, while Track 27 owns the contributor workflow commands that keep the site buildable.
