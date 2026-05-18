# KairoECS Documentation

KairoECS is a Rust-first simulation engine for deterministic event scheduling,
ECS-style state, DES, ABM, Arrow telemetry, and polyglot bindings.

Public project links:

- Repository: [github.com/edithatogo/kairos](https://github.com/edithatogo/kairos)
- CI status: [GitHub Actions](https://github.com/edithatogo/kairos/actions)
- Conductor status: [track index](../../conductor/tracks.md) and [narrative status](../../conductor/status.md)

Current maturity: pre-release. Core scheduler/state/conformance foundations are
implemented; DES/ABM flow APIs and the FFI bridge are in review; registry
publication and stable API promises remain gated by release, compatibility,
security, and packaging tracks.

## Start Here

- [Documentation overview](../../docs/README.md)
- [Install and local workflow](../../docs/install.md)
- [Conductor track status](../../conductor/tracks.md)
- [Current narrative status](../../conductor/status.md)
- [Benchmark overview](../../docs/benchmarks/README.md)
- [Implemented crate inventory](../../crates/README.md)
- [Language binding inventory](../../bindings/README.md)
- [Documentation examples](../../examples/docs/README.md)
- [Community adoption](../../docs/community/adoption.md)
- [Community playground](../../docs/community/playground.md)
- Docs workflow and coverage: [Developer workflow](../../docs/developer-experience/docs-workflow.md), [Docs platform status](../../docs/developer-experience/docs-platform.md), and [Learning Coverage Matrix](../../docs/tutorials/coverage-matrix.md).
- [Model zoo inventory](../../docs/model-zoo/inventory.md)
- [Starter kits](../../docs/starter-kits/README.md)
- [Scenario run and replay](../../docs/scenarios/factory-bottleneck-run-replay.md)
- [Experiment runner CLI](../../docs/cli/kairo-ecs-cli.md)
- [Citation and archival](../../docs/research/citation.md)
- [Trustworthy simulation](../../docs/trustworthy-simulation/verification-validation-uncertainty.md)
- [Factory bottleneck VVUQ note](../../docs/validation/factory-bottleneck-v1-vvuq-note.md)
- [Visualization guide](../docs/visualization/README.md)

## Tutorials and Examples

- Tutorial index: [learning paths](../../docs/tutorials/index.md), [Rust getting started](../../docs/tutorials/rust-getting-started.md), [Python getting started](../../docs/tutorials/python-getting-started.md), [Wasm and TypeScript getting started](../../docs/tutorials/wasm-getting-started.md), and [model-building](../../docs/tutorials/model-building.md).
- Binding quick lessons: [R getting started](../../docs/tutorials/r-getting-started.md), [Julia getting started](../../docs/tutorials/julia-getting-started.md), [C# getting started](../../docs/tutorials/csharp-getting-started.md), and [Go getting started](../../docs/tutorials/go-getting-started.md).
- Example surfaces: [scenario run and replay](../../docs/scenarios/factory-bottleneck-run-replay.md), [headless snapshot playground](../../docs/playground/headless-snapshot.md), and [documentation examples](../../examples/docs/README.md).
- Jupyter notebooks and figures: notebook and image assets are tracked by the examples and education tracks; the learning coverage matrix keeps those entry points discoverable without vendoring large generated media into the static shell.

## Language Quickstarts

- [Python binding](../../bindings/python/README.md)
- [R binding](../../bindings/r/README.md)
- [Julia binding](../../bindings/julia/README.md)
- [TypeScript/Wasm binding](../../bindings/typescript/README.md)
- [C# binding](../../bindings/csharp/README.md)
- [Go binding](../../bindings/go/README.md)

## Governance and Release

- [API review template](../../docs/api/api-review-template.md)
- [API review](../../docs/design/api-review.md)
- [Compatibility governance](../../docs/design/compatibility-governance.md)
- [Interoperability standards](../../docs/interoperability/standards-review.md)
- [Release governance](../../docs/release/release-governance.md)
- [Release checklist](../../docs/release/release-checklist.md)
- [Release compatibility](../../docs/release/compatibility.md)
- [Supply-chain verification](../../docs/release/supply-chain-verification.md)
- [Changelog policy](../../docs/release/changelog-policy.md)
- [Maintenance handoff](../../docs/release/maintenance-handoff.md)

## Specialized Systems

- [Cloud HPC policy](../../docs/cloud-hpc/checkpoint-spot-policy.md)
- [AWS Batch](../../docs/cloud-hpc/aws-batch.md)
- [Azure Batch](../../docs/cloud-hpc/azure-batch.md)
- [GCP Batch](../../docs/cloud-hpc/gcp-batch.md)
- [Slurm](../../docs/cloud-hpc/slurm.md)
- [Distributed deployment](../../docs/distributed/deployment-guide.md)
- [Transport trait](../../docs/distributed/transport-trait.md)
- [Telemetry aggregation](../../docs/distributed/telemetry-aggregation.md)
- [Entity migration protocol](../../docs/distributed/entity-migration-protocol.md)
- [FMI import guide](../../docs/fmi-digital-twin/import-guide.md)
- [FMI export guide](../../docs/fmi-digital-twin/export-guide.md)
- [GPU compute architecture](../../docs/gpu-compute/architecture.md)
- [GPU benchmark results](../../docs/gpu-compute/benchmark-results.md)
- [ML architecture](../../docs/ml/architecture.md)
- [PDES logical-process trait](../../docs/pdes/logical-process-trait.md)
- [Streaming architecture](../../docs/streaming/architecture.md)

## Trust and Quality

- [Benchmark policy](../../docs/benchmarks/benchmark-policy.md)
- [Reproduce comparison](../../docs/benchmarks/reproduce-comparison.md)
- [Research reproducibility](../../docs/research/reproducibility.md)
- [VVUQ note](../../docs/validation/factory-bottleneck-v1-vvuq-note.md)
- [Replay and seeds](../../docs/trustworthy-simulation/replay-and-seeds.md)
- [Scenario evidence](../../docs/trustworthy-simulation/scenario-evidence.md)
- [Trace format](../../docs/debugging/trace-format.md)
- [CLI reference](../../docs/debugging/cli-reference.md)
- [Community governance](../../docs/community/governance.md)
- [Community roadmap](../../docs/community/roadmap.md)

## R2 Implementation Map

- Core and shared contracts: `kairo-ecs-types`, `kairo-ecs-core`, `kairo-ecs-state`, `kairo-ecs-rng`, `kairo-ecs-des`, `kairo-ecs-abm`, `kairo-ecs-arrow`, and `kairo-ecs-ffi`.
- Binding bridges and facades: `kairo-ecs-uniffi`, `kairo-ecs-diplomat`, `kairo-ecs-wasm`, plus preview language packages for Python, R, Julia, TypeScript/Wasm, C#, and Go.
- Optional and advanced tracks now represented in docs: visualization, GPU/WebGPU, PDES, distributed MPI/gRPC, streaming, ML inference, FMI digital twin, cloud/HPC, and time-travel debugging.
- Native binding status remains explicit: preview packages may expose pure-language scheduler facades while reporting native FFI as not configured until Track 02 publishes stable local artifacts.
- Maturity language is staged: alpha means evidence-backed but limited, beta means wider release gates are active, and stable remains blocked until compatibility and release-candidate gates pass.

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
- `docs/api/api-review-template.md` and `docs/design/api-review.md` for API review intake.
- `docs/benchmarks/` for benchmark overview, policy, and reproducibility guidance.
- `docs/cloud-hpc/` for AWS Batch, Azure Batch, GCP Batch, Slurm, and spot-checkpoint policy notes.
- `docs/community/` for contributor onboarding, governance, adoption, model-zoo guidance, roadmap notes, and the playground.
- `docs/developer-experience/` for docs-platform status and workflow notes.
- `docs/debugging/` for trace format and interactive debugging CLI references.
- `docs/cli/` for the scenario runner CLI quickstart and command reference.
- `docs/distributed/` for deployment, transport, telemetry aggregation, and entity migration notes.
- `docs/fmi-digital-twin/` for FMI import/export, AAS mapping, and deployment model notes.
- `docs/gpu-compute/` for GPU/WebGPU architecture, hardware requirements, benchmark results, and maintainer notes.
- `docs/interoperability/standards-review.md` for interoperability standards review.
- `docs/ml/` for ML integration architecture, model versioning, and surrogate authoring.
- `docs/pdes/` for PDES protocols, logical-process traits, GVT, and benchmark results.
- `docs/release/` for release, compatibility, maintenance, changelog, and supply-chain checklists.
- `docs/research/` for citation and reproducibility guidance.
- `docs/scenarios/` for committed run/replay scenario notes.
- `docs/starter-kits/` for starter-kit discovery and maturity guidance.
- `docs/streaming/` for real-time processing architecture, stream schema, and broker setup.
- `docs/trustworthy-simulation/` for replay, seeds, and uncertainty notes.
- `docs/tutorials/` for learning paths, notebooks, and language quickstarts.
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
