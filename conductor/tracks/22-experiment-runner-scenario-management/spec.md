# Track 22: Experiment Runner & Scenario Management

## Purpose

Create a first-class experiment runner for replications, parameter sweeps, scenario manifests, resumability, and batch execution.

## Why this track exists

KairoECS is not only a Rust kernel. It is a multi-language research and engineering ecosystem. This track protects the project from the most common failure mode for ambitious open-source infrastructure: impressive internals with insufficient trust, examples, packaging, governance, and contributor experience.

## Primary subagent

`experiment-agent`

## Parallelization model

This track is designed to run in parallel with core implementation. The subagent owns docs, policies, examples, checklists, manifests, fixtures, and automation controls. It must not block kernel development unless it identifies a release-blocking risk.

## Inputs

- `conductor/contracts/vvuq-contract.md` (seed-manifest schema and replay invariants from Track 21).
- `conductor/contracts/core-contract.md` (ECS simulation loop API for runner integration).
- `conductor/contracts/conformance-contract.md` (scenario fixture definitions).
- Handoff notes from Tracks 01 (core), 18 (benchmarks), and 21 (VVUQ).
- Scenario manifest schema proposal and parameter-sweep specification format.

## Outputs

- `crates/kairo-ecs-cli/`: binary crate with smoke commands for `validate-scenario`, `replay`, and `resume-plan`.
- `crates/kairo-ecs-cli/src/scenario.rs`: scenario manifest parser with versioned schema support (v1).
- `docs/cli/kairo-ecs-cli.md`: user documentation with the implemented smoke commands, quickstart, and local validation notes.
- `docs/scenarios/factory-bottleneck-run-replay.md`: the first committed scenario/replay note for `factory_bottleneck_v1`.
- `examples/experiments/`: example scenario manifests exercising the committed smoke scenario and seed manifest.
- `conductor/tracks/22-experiment-runner-scenario-management/test-matrix.md`: CI gate (manifest validation, resumability integration test, CLI smoke validation).

## Acceptance criteria

- The track has a clear public-facing output, not just internal notes.
- The output is testable, reviewable, or linked to a release gate.
- It includes failure modes and countermeasures.
- It does not duplicate core implementation work owned by Tracks 01-05.
- It supports at least one of: adoption, trust, reproducibility, maintainability, or compatibility.

## Non-goals

- Replacing the core scheduler or ECS design.
- Publishing packages before naming, legal, security, and compatibility gates pass.
- Adding domain-specific complexity to `kairo-ecs-core`.

## Blocked paths

No additional blocked paths are declared for this track beyond the ownership and dependency boundaries in conductor/tracks.yaml. Public release, packaging, or production-readiness claims remain blocked until the relevant downstream release gates pass or are explicitly waived.


## Release implications

This track contributes to release readiness only through the acceptance criteria and quality gates listed here and in conductor/quality-gates.md. It does not independently authorize public release, registry publication, or production-readiness claims without the dependent packaging, supply-chain, compatibility, red-team, and wave-management gates.
