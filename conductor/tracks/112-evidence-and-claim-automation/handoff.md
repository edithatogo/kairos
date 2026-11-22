# Handoff

Updated 23 November 2026. Status: In Progress. Phase acceptance is pending.

## Current claims and reused evidence — 23 November 2026

Integration chat `01a1fd18-51c7-7963-a3cc-d262d54582ba` owns the capability
register, its guard tests and the Python candidate qualification row in
the Kairos closeout worktree on `codex/kairos-capability-evidence-20261123`.

The register identifies the installed Rust SDK, C ABI v1, CPython 3.15.2
and CareOps contention cases by source and target. The two receipt files
in this track are the exact bytes from
[run 45051171243](https://github.com/edithatogo/kairos/actions/runs/45051171243).
The existing `packaging/scripts/verify_native_beta_evidence.py` verifier
matched the retained bundles and receipts to the selected candidate and
its two profile targets. Maintainer acceptance is pending.

[Release Attestations run 45053462090](https://github.com/edithatogo/kairos/actions/runs/45053462090)
signed the SPDX 2.3 SBOM for the installed candidate
`e2875387ce45c34cd19716ea66f0102251a6aa6f`. The attestation covers six
named subjects and five distinct archive digests across the two tested
targets. The SPDX SHA-256 is
`34e2bdcc6240d81caff241bb3400387683c8e556b0bf68ea987153b039513ba7`.

Current claims use the merged [scientific evidence](../../scientific-completion.md#qualified-s05s10-scopes)
for Sobol and sensitivity intervals, sequential ABC-SMC, common random
numbers, competing risks, affine inference and the experimental native queue.
The installed CareOps and Innovate records cover the named cases and
artifact revisions. The [matched conservative-PDES ring](../113-beta-and-stable-release/evidence/paired-pdes-e287538.json)
keeps parity over 120 paired samples and shows lower median elapsed time
for serial in all eight cases. The broader Track 111 crossover remains open.

The existing guard suite builds its fixture from the selected claims and
their declared evidence. It still rejects missing native proof, synthetic
performance or native promotion, altered timings, mismatched parity, invalid
track links and reversed dependency direction. A dangling receipt reference
also rejects. Independent review checked the claims and native receipt
bytes, confirmed the guard oracles and added the exact signed SBOM run
and the narrower inference label.

## Summary

Keep public claims traceable to precise evidence and prevent placeholders, historical receipts or interface-shape tests from becoming false acceptance.

## Files changed

This track's specification, plan, metadata, contract, risk register, test matrix and index; coordinated programme records and registries. Runtime changes and hosted outcomes are recorded separately in the integration receipt.

## Contracts consumed

AGENTS.md, existing core/FFI/conformance/versioning contracts, the approved [programme](../../frontier-programme-20261107.md), Tracks 12, 13, 14, 18, 21.

## Contracts changed

The programme establishes immediate Rust-only support; this track qualifies its own specific outcomes. No release/publication or downstream-pin acceptance follows from track registration.

## Tests added

Each implementation phase must add or identify the distinguishing oracles in test-matrix.md. Structural manifest/track checks are not native execution or scientific validation.

## Known risks

A schema validator cannot prove a reported experiment occurred. Require actual raw execution evidence and independent review; never turn metadata success into performance, clinical or release acceptance.

## Follow-up issues

Execute the unchecked tasks in plan.md with original owner handoffs. Keep unsupported capability claims and release gates blocked until their evidence exists.

## Integration notes

Use isolated bounded worktrees. The root integration packet combines disjoint changes after review; actual commit/PR/CI outcomes go in the programme integration receipt. Preserve old immutable receipts.

## Phase closeout evidence

No phase is closed. Record $conductor-review, accepted fixes, commit SHA, pushed ref, scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree and the next-phase decision only when they occur. Current planning status is not execution evidence.

