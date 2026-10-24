# toml scenario configuration parsing

Maturity: planned experimental evaluation/adoption; no runtime work started. Priority class: foundation. Apply [value policy](../../dependency-value-policy.md).

## Inputs

Accepted prerequisites 75; exact live source, relevant contracts and independent synthetic/standard fixtures. External gates: []. Historical status does not prove capability.

## Outputs

A documented adopt/defer/reject decision, then only if justified a library-backed replacement, deleted/retained-code inventory and consumer acceptance receipts.

## Replacement and benefit

read_fields currently splits lines on equals and skips section headers; custom quoting/list parsing duplicates format infrastructure.

## Contract

Deserialize typed config using toml. Keep units, ranges, required references and scenario/seed consistency checks in Kairos. Inventory whether existing comma-list syntax is real TOML; version and migrate owned inputs deliberately rather than silently accepting a different format.

## Owned paths

Lead: contracts-agent. Prospective implementation: crates/kairo-ecs-cli/src/scenario.rs. Planning owns this folder and coordinated registry entries. Shared manifests/lockfiles/CI/FFI/schema changes require current-owner handoff and separate bounded claim. Existing CLI/game-theory reservations must be reconciled before those surfaces can be written.

## Blocked paths

Unrelated or actively claimed paths, other repositories and substantive scheduler/resource/clinical semantics. No whole-repository rewrite or dual-backend framework.

## Dependencies

Registry prerequisites: 75. External gates above require actual receipts. Shared path owners serialize; IDs are not priority order. Read-only review may proceed independently; implementation starts only with the value gate and prerequisites accepted.

## Acceptance criteria

Sections, comments, quotes/escapes, arrays, duplicate keys, malformed tokens, missing fields, out-of-range values and existing fixture migration; independent valid TOML fixtures.

Stop criterion: Stop if changed parsing silently changes effective inputs; no compatibility mini-parser without an explicit need.

## Quality gates

Dependency-specific behavioural gate `dependency-76-behaviour` plus `dependency-value-proof` and `phase-closeout-check`; all pending. Exact pinned features/targets and maintenance qualification, independent negative tests, replay compatibility and baseline-derived cost budgets. Phase 0 must name concrete executable tests before replacement. See [test matrix](test-matrix.md).

## CareOps handoff

CareOps consumes accepted generic APIs/pins only when used by an actual domain/application path. Parent coordinator verifies immutable Kairos commit, versions/features, contract fingerprints and raw behaviour/cost/recovery receipts before a bounded consumer/pin update. Do not install another direct dependency merely because it is present upstream. Any needed parent change is owned by its current domain/application owner, not this upstream writer.

## Release implications

No publication or release authority. Public API/ABI/schema/checkpoint changes require ADR, compatibility assessment, conformance and red-team response. Deferred/rejected evaluation is not implementation acceptance.

## Reference to requalify

https://docs.rs/toml/latest/toml/

## CareOps-Sim impact and opportunity review

Closeout includes the [lightweight downstream review](../../dependency-value-policy.md#careops-sim-impact-and-opportunity-review). Kairos supplies its accepted immutable receipt; the CareOps-Sim coordinator and consumer owner record implications, worthwhile opportunities and one of the four outcomes in `conductor/dependency-handoffs.json`, keyed by `owner_track: 76-toml-robustness`. Reuse existing consumer tracks; no automatic direct dependency or duplicate implementation track. For already accepted phases this is downstream follow-through, not reopening or changing historical acceptance. Review remains pending until evidenced separately from implementation.
