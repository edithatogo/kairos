# KairoECS Inputs, Parameters, and Assumptions Investigation

Maturity: investigation note, not release evidence.

This note records code-level evidence for KairoECS inputs, pseudo-random
parameters, structural assumptions, calibration readiness, provenance, execution
data gaps, and model-boundary claims. It is a bounded investigation artifact; it
does not promote any simulation, calibration, cloud/HPC, or production-readiness
claim.

## 1. Inputs and Source Data

KairoECS currently uses structured TOML manifests to ingest scenario metadata,
execution bounds, and random seed streams.

- **Scenario manifests (`kairoecs.scenario.v1`)**: parsed by `load_scenario` in
  [`crates/kairo-ecs-cli/src/scenario.rs`](../../../crates/kairo-ecs-cli/src/scenario.rs).
  The manifest binds scenario configurations to conformance fixtures and records:
  - `scenario_id`: unique identifier for the experiment setup.
  - `model_id`: target simulation logic, for example `kairoecs.scheduler.ordering`.
  - `fixture_id` and `fixture_path`: local JSON fixture used for validation,
    for example
    [`conformance/fixtures/deterministic_ordering.json`](../../../conformance/fixtures/deterministic_ordering.json).
  - `base_seed`: starting entropy source for the PRNG.
  - `replications`: number of stochastic runs.
  - `max_events`: hard stop event limit.
  - `expected_kind_order`: expected dispatch order for the fixture.

- **Seed manifests (`kairoecs.seed.v1`)**: parsed by `load_seed_manifest` in
  [`crates/kairo-ecs-cli/src/scenario.rs`](../../../crates/kairo-ecs-cli/src/scenario.rs).
  The `[streams]` table maps process-specific stream names, such as
  `arrival_process`, `service_process`, or `resource_allocator`, to stream seed
  offsets.

Current source-data boundary:

- No database, CSV sensor-log, live message-stream, or other real-world data
  source is connected in this slice.
- Current source input is synthetic and fixture-backed.

## 2. Parameters and Randomization

The current RNG crate uses SplitMix64-style mixing for reproducible pseudo-random
streams. The constants are declared in
[`crates/kairo-ecs-rng/src/lib.rs`](../../../crates/kairo-ecs-rng/src/lib.rs).

Recorded parameter surfaces include:

- `RUN_SEED_DOMAIN`
- `ENTITY_INDEX_DOMAIN`
- `ENTITY_GENERATION_DOMAIN`
- `ENTITY_INDEX_MIX`
- `ENTITY_GENERATION_MIX`
- SplitMix64 gamma, shift, and multiplier constants

Entity-specific seeds are derived by `derive_entity_seed`, which combines entity
index and generation fields with the domain constants to reduce direct
correlation between entity streams.

The CLI computes stable run summaries in
[`crates/kairo-ecs-cli/src/main.rs`](../../../crates/kairo-ecs-cli/src/main.rs)
by hashing the scenario id, base seed, and observed event-kind order. This is a
reproducibility fingerprint for fixture checks, not a statistical validation
metric.

## 3. Structural Assumptions

KairoECS currently relies on deterministic event ordering in
[`crates/kairo-ecs-core/src/lib.rs`](../../../crates/kairo-ecs-core/src/lib.rs).
`QueueEntry` ordering resolves simultaneous event conflicts by:

1. Event time.
2. Priority.
3. Monotonic insertion sequence.

This supports deterministic fixture execution under the current queue model. It
does not, by itself, prove correctness for parallel execution, distributed
scheduling, or real-world queueing behavior.

SplitMix64 use assumes the selected stream-splitting scheme is sufficient for
the fixture and smoke-test scope. Broader statistical suitability requires
separate validation evidence.

## 4. Calibration Readiness and Provenance

Calibration status: not ready for real-world calibration claims.

Current gaps:

- Fitting algorithms and a parameter-estimation workflow are not yet connected.
- Reference data, acceptance thresholds, and comparison statistics remain
  unattached.
- Real-world queueing, throughput, and bottleneck claims have not been
  validated.

The VVUQ boundary is documented in
[`docs/validation/factory-bottleneck-v1-vvuq-note.md`](../../validation/factory-bottleneck-v1-vvuq-note.md).

Provenance and supply-chain status:

- Repository provenance is recorded in Git metadata and release documentation.
- Dry-run package manifests and checksums are described in
  [`docs/release/supply-chain-verification.md`](../../release/supply-chain-verification.md).
- Live publication remains disabled unless the relevant registry, provenance,
  and release-manager gates pass.

## 5. Execution Data Gaps and Model Boundaries

Live execution gaps are tracked in
[`docs/cloud-hpc/runtime-evidence-boundary.md`](../../cloud-hpc/runtime-evidence-boundary.md).
As of this note, missing or partial evidence includes:

- Docker container build/run and SIGTERM resume behavior.
- Kubernetes operator smoke test.
- Slurm single-job and array submission.
- AWS Batch and GCP Batch canaries.
- Azure Batch KairoECS container/scenario execution and checksum evidence.
- GPU/HPC hardware proof.

The current model boundary is fixture-conformance deterministic event ordering,
including the four-event sequence in
[`conformance/fixtures/deterministic_ordering.json`](../../../conformance/fixtures/deterministic_ordering.json).
It does not model physical queue delays, backpressure, resource contention,
network latency, state-machine failure, or calibrated operational behavior.

## 6. Follow-Up Candidates

These are candidate work items, not release commitments:

1. Add tutorial notebooks demonstrating PRNG stream splitting, event-queue step
   execution, and sensitivity analysis.
2. Add a web timeline inspector for event queue ordering conflicts and seed
   stream branching.
3. Add canonical queueing examples with analytical references before making any
   calibration or validation claim.
