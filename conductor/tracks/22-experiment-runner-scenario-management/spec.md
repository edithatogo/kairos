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

- `crates/kairo-ecs-cli/`: binary crate with `run`, `collect`, and `analyze` subcommands.
- `crates/kairo-ecs-cli/src/scenario.rs`: scenario manifest parser with versioned schema support (v1).
- `docs/cli/kairo-ecs-cli.md`: user documentation with `--help` output, quickstart, and subcommand examples.
- `fixtures/scenarios/`: example scenario manifests exercising single-run, sweep, and resume modes.
- `conductor/tracks/22-experiment-runner-scenario-management/test-matrix.md`: CI gate (manifest validation, resumability integration test, CLI argument parsing smoke test).

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



