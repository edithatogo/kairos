# Handoff

Updated 23 November 2026. The installed-artifact gate is accepted. Phase 0
remains open for the remaining alpha-to-beta evidence.

## Native core v1 candidate — 23 November 2026

Use the one `native-core-v1` candidate at main
`3b3b8da0a7cdbd414e12a18722f189996a52c7ab`. GitHub run
`45031314631` passed clean Rust SDK, C ABI v1 and CPython 3.15.2 installation
and execution on Linux x86-64 GNU and macOS ARM64. Both ran the installed
CareOps staged allocation case from
`6810aaaa38229b725a9fe0f99a0a6c1d8693ebd9`. The candidate is bound to the API
and targets in the [beta scope](../../release/beta-scope.md).
[Hosted run and exact archives](https://github.com/edithatogo/kairos/actions/runs/45031314631).

The shared Rust SDK SHA-256 is
`a8981c2bd9537dba20d6027d15c75e5deb329a9f0ee6912dab2d9609874006c9`.
Each target's C archive and Python wheel matched its own source-bound receipt.

| Target | Hosted installed receipt SHA-256 |
|---|---|
| Linux x86-64 GNU | `08608f185dcd5443a940262429213d407a8c3ba3ae20ed2dce4e1bf506c558fd` |
| macOS ARM64 | `cf5c1cf3f5ce009ab083d145cc5ae57c5bc9a66f113ff6e6a1fa694d04a2cc4f` |

An independent macOS install used the downloaded bytes, validated their
checksums and source fixture digests, and reran the C lifecycle, installed
Python lifecycle, Rust service-station and replay, and installed CareOps
allocation cases. Every invocation exited zero. The independent receipt
SHA-256 is `f71b182d0e78256da2c10cd029809872dffb0b789abce234ce38da5787504f2c`.

Milestone 113.0.3 is complete. Finish the remaining
[alpha-to-beta gates](../../release-gates.md#alpha-to-beta-all-required),
including the candidate performance evidence and maintainer review, before
the publication packet.

## Summary

Release a precisely scoped beta, then an independently reproducible and compatible stable core, without implying all experimental modules are complete.

## Files changed

This track's specification, plan, metadata, contract, risk register, test matrix and index; coordinated programme records and registries. Runtime changes and hosted outcomes are recorded separately in the integration receipt.

## Contracts consumed

AGENTS.md, existing core/FFI/conformance/versioning contracts, the approved [programme](../../frontier-programme-20261107.md), Tracks 15, 16, 20, 25, 109, 110, 111, 112, 114.

## Contracts changed

The programme establishes immediate Rust-only support; this track qualifies its own specific outcomes. No release/publication or downstream-pin acceptance follows from track registration.

## Tests added

Each implementation phase must add or identify the distinguishing oracles in test-matrix.md. Structural manifest/track checks are not native execution or scientific validation.

## Known risks

Stable branding can overgeneralize experimental scope. Version the supported manifest and keep experimental crates/features separately labelled; do not infer registry publication from packaging scripts.

## Follow-up issues

Execute the unchecked tasks in plan.md with original owner handoffs. Keep unsupported capability claims and release gates blocked until their evidence exists.

## Integration notes

Use isolated bounded worktrees. The root integration packet combines disjoint changes after review; actual commit/PR/CI outcomes go in the programme integration receipt. Preserve old immutable receipts.

## Phase closeout evidence

No phase is closed. Record $conductor-review, accepted fixes, commit SHA, pushed ref, scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree and the next-phase decision only when they occur. Current planning status is not execution evidence.

