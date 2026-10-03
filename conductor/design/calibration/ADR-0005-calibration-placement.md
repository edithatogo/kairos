# ADR-0005 — Reusable calibration placement

Date: 2026-10-01. Status: accepted local architecture decision for C0.1;
implementation, dependency selection and remote integration remain gated.
Primary owner: existing Track 21 (VVUQ), with 04/03/22 and 01/12/25/30 review
boundaries. Base source: a71adfd48f42d7c4d04bcb034aad09295c004f40.
This decision adds documents only; it does not add a crate or a public Rust API.

## Context and observed baseline

The workspace contains core/types/RNG, DES/ABM, Arrow and CLI crates, but no
calibration crate. [Arrow](../../../crates/kairo-ecs-arrow/Cargo.toml) depends
only on types and has no Arrow IPC/Parquet features. Its
[event log](../../../crates/kairo-ecs-arrow/src/lib.rs) exposes versioned fields
and custom smoke serialization, not real Arrow IPC interoperability.
The [CLI](../../../crates/kairo-ecs-cli/src/main.rs) handles legacy scenario/seed
validation and scheduler-ordering replay. Its
[parser](../../../crates/kairo-ecs-cli/src/scenario.rs) flattens key/value lines
and ignores section headers; it is not a complete TOML or calibration parser.
`resume-plan`, checkpoint and resume-request output are not calibration state
restoration. [Track 22](../../tracks/22-experiment-runner-scenario-management/spec.md)
references a VVUQ contract that is absent at the base revision.

## Decision and responsibility map

| Owner | Responsibility / future path | Boundary |
| --- | --- | --- |
| 21 | Future `crates/kairo-ecs-calibration` | Rust normalized trace semantics, adapter protocol, replay/probes, pure metrics and bounded search/evaluation; no ED policy or CLI file parsing |
| 04 | Existing `crates/kairo-ecs-arrow` | Physical Arrow schemas and optional real IPC/Parquet readers/writers, batch handling and conformance fixtures |
| 03 | Existing DES/ABM | Resource/fidelity/transit execution and checkpoint hooks through their accepted contracts |
| 22 | Existing `crates/kairo-ecs-cli` and experiment configuration | Config resolution, study orchestration, artifact routing, worker scheduling and resume coordination |
| 01 | Existing types/core/RNG | Exact tick/order/identity and versioned seed-purpose derivation; calibration cannot redefine them |
| 12 | Conformance assets | Portable synthetic fixtures and reproducibility oracles |
| 25/30 | API/dependency policy | Public signatures, MSRV, feature matrix and exact dependency review before implementation |
| CareOps | Domain adapter/profile | ED policies, input applicability, observed event mapping and operational acceptance |

The new library is one package in the existing repository, not a submodule,
experiment framework or domain model. Default pure numerical/semantic code must
remain usable without Arrow, filesystem, CLI, Python, GPU or network facilities.
Declare generic model-adapter contracts in calibration; concrete engine/domain
adapters implement them without forcing the core to depend on calibration.

Keep the dependency graph acyclic: CLI -> calibration and Arrow adapters;
calibration -> shared types (and approved RNG integration as needed);
Arrow -> types. Core/types must not depend on calibration or Arrow. Avoid a
calibration -> Arrow -> calibration cycle: calibration exposes ordinary Rust
records/iterators or sink traits, and an outer integration layer converts them
to Arrow-owned physical columns. Exact normalized/residual/metric wire fields
and stream versions belong to C0.2 with 04, not this placement decision.

Proposed IO feature names are `ipc` and `parquet` on `kairo-ecs-arrow`; default
features preserve today's smoke API. Neither feature exists yet. Gate new
commands on requested capability and fail explicitly if unavailable. Optional
IO dependencies must not leak into a minimal core/calibration build. Exact
versions/features/MSRV remain C0.3 and 25/30 decisions; this ADR approves no
package upgrade and does not claim an Arrow 60 build at the workspace floor.
Preserve declared default-feature Rust 1.76 compatibility unless a reviewed
separate package/feature boundary explicitly changes that promise.

## Configuration and API strategy

Preserve `kairoecs.scenario.v1`, `kairoecs.seed.v1` and `kairo_ecs.event_log.v1`.
A separately versioned explicit calibration config/reference extends the runner;
legacy files without that opt-in retain legacy parsing/behavior. Unknown config
versions or incompatible adapter/trace/checkpoint versions must fail in the new
path rather than silently falling back. C0.1's interface proposal defines that
handoff; C0.2 freezes serialized fields, and C0.3 supplies test oracles.
No scalar metric/objective calculation changes scheduler time or event ordering.

## Alternatives and consequences

Embedding reusable algorithms in the CLI is smaller initially but prevents
library reuse and mixes study validity with file parsing. Adding them to core
would impose statistics/IO dependencies on the scheduler. Extending Arrow into
an execution/calibration owner conflates physical encoding and model semantics.
A second experiment crate is unnecessary for this scope; Track 22 can extract a
library later under its own reviewed need. Choose the one reusable Track 21
library with thin Track 22 orchestration and Track 04 IO adapters.

This creates an adapter/version boundary to test, rather than pretending the
current smoke runner is a calibration system. D2 and C0 closeout precede C1;
C2 also needs P3/Q0, interruption C3 needs Q4, and C6 needs Q5/C5/P5/E2.
CPU correctness and deterministic reductions precede accelerated extensions.

## Required implementation gates

Future tests must cover minimal/default builds, optional IO feature isolation,
legacy manifests/event log, unknown versions, deterministic adapter failures,
checkpoint provenance and worker-count/resume invariance. Real independent IPC/
Parquet round trips are C1 acceptance; synthetic recovery and held-out validity
are C5/C6 acceptance. A documentation check or current smoke test is neither.

C0.1 acceptance requires this ADR, the separate adapter/runner proposal and a
readable Track 21 VVUQ contract at Track 22's referenced path, with source-bound
review and unchanged production code. Full C0 closes only at C0.4.
