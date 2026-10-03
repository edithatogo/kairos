# VVUQ contract — Track 21 / Track 22

Version: `vvuq_contract.v1`. Date: 2026-10-01. Status: local C0.1 documentation
contract, resolving the previously absent path consumed by
[Track 22](../tracks/22-experiment-runner-scenario-management/spec.md).
Owner: existing Track 21; runner consumer 22; shared seed/order owner 01.
This does not mark either upstream track complete or assert calibration runtime.
See [placement ADR](../design/calibration/ADR-0005-calibration-placement.md) and
[adapter/config proposal](../design/calibration/calibration-adapter-v1.md).

## Implemented legacy seed/scenario contract

Audited base: a71adfd48f42d7c4d04bcb034aad09295c004f40.
[Parser](../../crates/kairo-ecs-cli/src/scenario.rs): seed version is exactly
`kairoecs.seed.v1`; required string fields `scenario_id`, `fixture_id` and required
u64 `base_seed`. They must match the scenario. Scenario version is exactly
`kairoecs.scenario.v1`, with required fields `scenario_id`, `model_id`, `fixture_id`,
`fixture_path`, `base_seed`, `replications`, `max_events`, `artifact_root`,
`resume_checkpoint_every_events` and `expected_kind_order`. Replications and max
events must be nonzero, expected kind order nonempty and fixture path existent.
These are current validation obligations, not a general simulation validity proof.

[Seed example](../../examples/experiments/factory_bottleneck_v1.seeds.toml)
contains `[streams]` arrival/service/resource labels. The loader ignores section
headers and extracts only the declared SeedManifest fields; it does not implement
these purpose streams. Unknown flat fields are currently not strictly rejected,
and duplicate keys overwrite earlier values. This contract documents that
baseline rather than falsely promising strict full-TOML validation. A future
calibration loader must not route nested/versioned inputs through this parser.

## Implemented replay invariants and limitations

[CLI replay](../../crates/kairo-ecs-cli/src/main.rs) accepts only
`scheduler_ordering_v1`, dispatches the controlled three-event scheduler fixture,
checks expected kind order and writes deterministic summary/hash outputs. The
summary hash is the current smoke comparison fingerprint, not an input-integrity
or cryptographic provenance guarantee. The base seed participates in that summary;
this is not proof of clinical stochastic replications or purpose-stream sampling.

Core order is exact `(time_ticks, priority, sequence)` per
[core contract](core-contract.md), and conformance uses
[portable fixtures](conformance-contract.md). Preserve integer ticks and
`kairo_ecs.event_log.v1` field/ID/null semantics. Current Arrow smoke bytes are not
IPC/Parquet interoperability. `resume-plan` describes requirements; checkpoint/
resume-request output does not restore an executed calibration state.

## Proposed additive calibration obligations

The [adapter proposal](../design/calibration/calibration-adapter-v1.md) defines
versioned validation/mapping/context/probe/checkpoint hooks and Track 21/22 split.
Track 21 owns replay validity, residual/metric/search semantics and immutable
observation-versus-prediction separation. Track 22 owns explicit opt-in config,
input hash verification, manifest/artifact handling, evaluation scheduling and
resume reconciliation. Domain models supply rules without changing core order.

Keep legacy scenario/seed versions and no-opt-in commands compatible. Unknown
new config/adapter/schema/checkpoint versions must fail explicitly on the new
path. Logical seed purposes and serialization are C0.2 with 01; no new RNG
algorithm, string hashing or stream interpretation is approved by this document.
Version/hashes/model identity, completed evaluation keys and partial probe/RNG
state must be validated on resume. Canonical reductions and stable candidate ties
must make worker-count/finish-order/crash-resume invariance testable.

## Required evidence before implementation acceptance

C0.2/C0.3 freeze trace/residual/metric schemas, seed map, objectives/splits/budgets,
tolerances and source/dependency applicability; C0.4 closes architecture contracts.
Later gates require real IO round trips, legacy fixtures, unknown-version failure,
no target leakage, typed missing/censoring/failure handling, exact checkpoint
restore and canonical worker/resume results. Synthetic recovery does not establish
clinical validity. All current smoke and proposal results keep those limits.
