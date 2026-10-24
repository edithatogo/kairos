# proptest-state-machine qualification

Planned; no runtime acceptance. Apply [value policy](../../dependency-value-policy.md) and [sequencing](../../dependency-sequencing.md).

## Inputs

Prerequisites: [96]. Read-only cross-track inventory is allowed before prerequisite acceptance; replacement is not.

## Outputs

Explicit adopt/defer/reject decision, qualified bounded replacement and immutable consumer receipts.

## Owned paths

Lead: conformance-agent. Prospective target: Repeated generated operation-sequence and shrinking scaffolding only if plain proptest proves insufficient. Planning owns this track folder. Runtime paths, shared manifests and fixtures require current owner handoff and a narrow claim; no blanket ownership transfer.

## Blocked paths

Other repositories, active reservations and unrelated runtime semantics. No implicit CI/publishing permission.

## Dependencies

Registry dependencies [96]. No dependency on completed Polars for Tracks 93/94/96: this prevents a migration cycle. Owner-coordinated source inventory decides deleted versus retained targets.

## Contract

Optional dev-only extension after Track 96 baseline. Require measured reduction in bespoke transition/shrinking code for actual resource/cancellation/checkpoint sequences. Independent reference model stays project-owned. No dependency from core migration to this optional track.

## Acceptance criteria

Valid transitions/preconditions survive shrinking; injected conservation/replay bug produces reproducible minimal sequence; compare complexity and runtime against plain proptest.

Record exact features/version, maintenance/security/licence/MSRV/target qualification and wrapper-inclusive code/cost budgets before adoption. No net benefit means defer/reject, not implemented.

## CareOps handoff

Accepted immutable Kairos receipt before consuming upstream API changes. Direct CareOps implementations/tests remain parent-owned in the paired sha2/tempfile/thiserror/proptest tracks. Optional libraries need a bounded parent packet and concrete consumer; no duplicate direct dependency solely because upstream uses it.

## Quality gates

`dependency-99-behaviour`, `dependency-value-proof`, `phase-closeout-check`. Tests in [matrix](test-matrix.md) remain future work.

## Release implications

Public contract/digest/schema/FFI changes require normal compatibility review and versioned migration. No release or runtime claim follows from planning.

## CareOps-Sim impact and opportunity review

Closeout includes the [lightweight downstream review](../../dependency-value-policy.md#careops-sim-impact-and-opportunity-review). Kairos supplies its accepted immutable receipt; the CareOps-Sim coordinator and consumer owner record implications, worthwhile opportunities and one of the four outcomes in `conductor/dependency-handoffs.json`, keyed by `owner_track: 99-proptest-state-machine`. Reuse existing consumer tracks; no automatic direct dependency or duplicate implementation track. For already accepted phases this is downstream follow-through, not reopening or changing historical acceptance. Review remains pending until evidenced separately from implementation.
