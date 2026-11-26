# Handoff

## Native runtime acceptance — 25 November 2026

Phase 0 is closed. The binding inventory names each retired or unavailable
host entry point. The native-only implementation packet and exact candidate
binding CI exercised retirement controls and actual Rust-backed Python, C#
and Wasm paths. Go, R and Julia retain their documented unavailable or
data-only boundaries. [Runtime inventory](../../../bindings/runtime-policy.json) ·
[Implementation](../../frontier-implementation-20261107.md) ·
[Candidate binding CI](https://github.com/edithatogo/kairos/actions/runs/45051171197).

The frozen `native-core-v1` Rust SDK, C ABI v1 and CPython 3.15.2 adapter
are accepted on Linux x86-64 GNU and macOS ARM64 at
`e2875387ce45c34cd19716ea66f0102251a6aa6f`. Installed checks covered dispatch,
cancellation, wide ticks, foreign and retired handles, reset/free, buffer
ownership, and typed loader failures. The standalone Rust fixture and
installed CareOps executed against the same SDK. The downloaded macOS
library and wheel independently repeated the installed C fixture, six
Python lifecycle tests and four loader rejection controls.
[Support and replay contract](../../release/beta-scope.md) ·
[Installation receipts](../113-beta-and-stable-release/handoff.md#native-core-v1-decision--25-november-2026).

The independent Conductor review checked the frozen header and adapter,
retirement inventory, installed failure controls, and the scope of the
receipts. Phase 1 continues with the remaining adapter and platform profiles.
The [runtime inventory](../../../bindings/runtime-policy.json) states the
current boundary for each. The release candidate decision is recorded in
[Track 113](../113-beta-and-stable-release/handoff.md).

The maintainer accepted this exact two-platform beta and its seeded replay
break on 25 November 2026. This closes the beta-specific 109 support decision,
not the wider binding work. Track 109 does not claim installed support for
the remaining hosts, platforms or adapter operations.

## Registration history — 7 November 2026

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

