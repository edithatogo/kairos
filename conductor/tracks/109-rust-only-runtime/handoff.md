# Handoff

Updated 7 November 2026. Status: In Progress; no implementation phase is accepted by this registration.

## Summary

Retire every executable non-Rust simulation core immediately and qualify supported native/Wasm adapters without silent fallbacks.

## Files changed

This track's specification, plan, metadata, contract, risk register, test matrix and index; coordinated programme records and registries. Runtime changes and hosted outcomes are recorded separately in the integration receipt.

## Contracts consumed

AGENTS.md, existing core/FFI/conformance/versioning contracts, the approved [programme](../../frontier-programme-20261107.md), Tracks 02, 12, 25, 63.

## Contracts changed

The programme establishes immediate Rust-only support; this track qualifies its own specific outcomes. No release/publication or downstream-pin acceptance follows from track registration.

## Tests added

Each implementation phase must add or identify the distinguishing oracles in test-matrix.md. Structural manifest/track checks are not native execution or scientific validation.

## Known risks

Breaking callers may currently depend on host traces that ABI1 cannot return; expose only real native operations and document the exact replacement. New native loading expands FFI/lifetime review obligations.

## Follow-up issues

Execute the unchecked tasks in plan.md with original owner handoffs. Keep unsupported capability claims and release gates blocked until their evidence exists.

## Integration notes

Use isolated bounded worktrees. The root integration packet combines disjoint changes after review; actual commit/PR/CI outcomes go in the programme integration receipt. Preserve old immutable receipts.

## Phase closeout evidence

No phase is closed. Record $conductor-review, accepted fixes, commit SHA, pushed ref, scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree and the next-phase decision only when they occur. Current planning status is not execution evidence.

