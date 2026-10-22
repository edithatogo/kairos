# zip FMU archive replacement

Maturity: planned experimental evaluation/adoption; no runtime work started. Priority class: subsystem. Apply [value policy](../../dependency-value-policy.md).

## Inputs

Accepted prerequisites 78; exact live source, relevant contracts and independent synthetic/standard fixtures. External gates: []. Historical status does not prove capability.

## Outputs

A documented adopt/defer/reject decision, then only if justified a library-backed replacement, deleted/retained-code inventory and consumer acceptance receipts.

## Replacement and benefit

Custom stored-entry ZIP extraction and limited FMU packaging support.

## Contract

Use zip with minimal necessary features to replace actual decoding/encoding. Retain containment, file-count/expanded-byte budgets, symlink/overwrite policy and failure cleanup. Qualify Rust compression backend if pure-Rust deployment is required.

## Owned paths

Lead: fmi-agent. Prospective implementation: crates/kairo-ecs-fmi/src/import/fmu_loader.rs; crates/kairo-ecs-fmi/src/export/packager.rs. Planning owns this folder and coordinated registry entries. Shared manifests/lockfiles/CI/FFI/schema changes require current-owner handoff and separate bounded claim. Existing CLI/game-theory reservations must be reconciled before those surfaces can be written.

## Blocked paths

Unrelated or actively claimed paths, other repositories and substantive scheduler/resource/clinical semantics. No whole-repository rewrite or dual-backend framework.

## Dependencies

Registry prerequisites: 78. External gates above require actual receipts. Shared path owners serialize; IDs are not priority order. Read-only review may proceed independently; implementation starts only with the value gate and prerequisites accepted.

## Acceptance criteria

Independent compressed/stored/ZIP64 fixtures where supported, malformed/truncated CRC, traversal, symlink/duplicate entries, expansion limit, partial extraction cleanup and FMU roundtrip.

Stop criterion: Run only for a concrete FMI consumer; if rust-fmi replaces this whole path, use its facilities and avoid duplicate direct dependencies or extracting twice.

## Quality gates

Dependency-specific behavioural gate `dependency-79-behaviour` plus `dependency-value-proof` and `phase-closeout-check`; all pending. Exact pinned features/targets and maintenance qualification, independent negative tests, replay compatibility and baseline-derived cost budgets. Phase 0 must name concrete executable tests before replacement. See [test matrix](test-matrix.md).

## CareOps handoff

CareOps consumes accepted generic APIs/pins only when used by an actual domain/application path. Parent coordinator verifies immutable Kairos commit, versions/features, contract fingerprints and raw behaviour/cost/recovery receipts before a bounded consumer/pin update. Do not install another direct dependency merely because it is present upstream. Any needed parent change is owned by its current domain/application owner, not this upstream writer.

## Release implications

No publication or release authority. Public API/ABI/schema/checkpoint changes require ADR, compatibility assessment, conformance and red-team response. Deferred/rejected evaluation is not implementation acceptance.

## Reference to requalify

https://github.com/zip-rs/zip2

Before Phase 1, record the outcome of Track 91 scope assessment; an accepted whole FMI adapter replacement cancels redundant direct-adoption work. This is a choice gate, not a requirement to adopt rust-fmi.
