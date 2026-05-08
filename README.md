# KairoECS

[![ci-core](https://github.com/edithatogo/kairos/actions/workflows/ci-core.yml/badge.svg)](https://github.com/edithatogo/kairos/actions/workflows/ci-core.yml)
[![ci-bindings](https://github.com/edithatogo/kairos/actions/workflows/ci-bindings.yml/badge.svg)](https://github.com/edithatogo/kairos/actions/workflows/ci-bindings.yml)
[![Validate Conductor](https://github.com/edithatogo/kairos/actions/workflows/validate-conductor.yml/badge.svg)](https://github.com/edithatogo/kairos/actions/workflows/validate-conductor.yml)
[![Docs Quality](https://github.com/edithatogo/kairos/actions/workflows/docs-quality.yml/badge.svg)](https://github.com/edithatogo/kairos/actions/workflows/docs-quality.yml)

KairoECS is a Rust-first simulation engine for deterministic event scheduling,
ECS-style state, Discrete Event Simulation (DES), Agent-Based Modeling (ABM),
Arrow telemetry, and polyglot bindings.

The repository is now public, CI-backed, and organized with a Conductor track
system so implementation, validation, documentation, release, and supply-chain
work stay traceable.

- Repository: <https://github.com/edithatogo/kairos>
- Documentation site: <https://edithatogo.github.io/kairos/>
- Status index: [conductor/tracks.md](conductor/tracks.md)
- Narrative status: [conductor/status.md](conductor/status.md)

## Current Status

This is an active pre-release project. The core scheduler/state/conformance
foundation is in place, with selected language bindings and docs gates already
running in CI. Registry publication and stable API promises remain gated by the
release, compatibility, security, and packaging tracks.

Current Conductor highlights:

- Done: Track 00 foundation, Track 01 core/state, Track 05 headless visualization, Track 10 C# binding, Track 12 conformance/testing/benchmarks.
- In review: Track 02 FFI/UniFFI/Diplomat bridge and Track 03 DES/ABM flow APIs.
- In progress: Python, R, TypeScript/Wasm, Go, docs, CI/supply chain, packaging, release governance, benchmarks, research/citation, security, DX, and wave management.
- Planned or spec-approved: Arrow, Julia, community/model-zoo/playground, API compatibility, interoperability, GPU/WebGPU, PDES, distributed simulation, streaming, ML, FMI/digital twin, cloud/HPC, and time-travel debugging.

For the authoritative machine-readable view, use [conductor/tracks.yaml](conductor/tracks.yaml).

## What Is Implemented

Core Rust crates:

- `kairo-ecs-types`: shared identifiers, event kinds, simulation time, and errors.
- `kairo-ecs-core`: deterministic scheduler and event dispatch.
- `kairo-ecs-state`: ECS-style world state and component storage.
- `kairo-ecs-rng`: deterministic random streams.
- `kairo-ecs-des`: DES trajectory/resource queue API.
- `kairo-ecs-abm`: ABM behavior update API.
- `kairo-ecs-viz`: dependency-light headless visualization frames.
- `kairo-ecs-ffi`, `kairo-ecs-uniffi`, `kairo-ecs-diplomat`: bridge/facade surfaces in review.

Preview binding and package surfaces are tracked under [bindings/](bindings/) and
[packaging/](packaging/). Some bindings deliberately expose pure-language smoke
paths while native FFI artifacts are still being stabilized.

## Start Here

- Install and local workflow: [docs/install.md](docs/install.md)
- Documentation overview: [docs/README.md](docs/README.md)
- Rust crates: [crates/README.md](crates/README.md)
- Bindings inventory: [bindings/README.md](bindings/README.md)
- Conformance fixtures: [conformance/README.md](conformance/README.md)
- Scenario replay: [docs/scenarios/factory-bottleneck-run-replay.md](docs/scenarios/factory-bottleneck-run-replay.md)
- Release checklist: [docs/release/release-checklist.md](docs/release/release-checklist.md)
- Supply-chain verification: [docs/release/supply-chain-verification.md](docs/release/supply-chain-verification.md)

## Quick Validation

```powershell
cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-core -p kairo-ecs-state
cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-des -p kairo-ecs-abm
node tests/conformance/conformance-check.mjs
npm --prefix website run check:all
pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1
```

CI also runs cross-platform Conductor, docs, core, bindings, security, and
supply-chain checks on `main`.

## Documentation Site

The static site is built from Markdown sources in [docs/](docs/),
[bindings/](bindings/), [crates/](crates/), and selected examples. The source
entry point is [website/src/index.md](website/src/index.md), and the rendered
GitHub Pages site is published at <https://edithatogo.github.io/kairos/>.

Local site checks:

```powershell
npm --prefix website run check:links
npm --prefix website run build
npm --prefix website run check:quality
```

## Project Governance

Conductor is the source of truth for status and closeout evidence:

- [conductor/workflow.md](conductor/workflow.md): development and review workflow.
- [conductor/phase-closeout.yaml](conductor/phase-closeout.yaml): closeout ledger.
- [conductor/track-map.md](conductor/track-map.md): roadmap map.
- [conductor/implementation-readiness.md](conductor/implementation-readiness.md): readiness gates.
- [conductor/quality-gates.md](conductor/quality-gates.md): validation catalogue.

When a track changes status, `conductor/tracks.yaml`, `conductor/tracks.md`,
`conductor/phase-closeout.yaml`, and `conductor/status.md` must be synchronized.

## License

Apache-2.0. See [LICENSE](LICENSE).
