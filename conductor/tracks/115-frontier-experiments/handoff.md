# Handoff

## Proposed non-HPC checkpoint comparison — 25 November 2026

The [branching protocol draft](branching-protocol.md) defines a held-out
entity-lifecycle comparison against full replay with conventional CRN, with
negative controls for draw order and cursor restoration. It has not been run
or preregistered. The existing claim-validation result is unchanged. Next:
independently review and freeze the implementation, source, fixture and
oracle before measuring the held-out cases. No new claim, paper, executor
selection or phase advancement follows from the draft.

Independent protocol review separated cursor restoration from dispatch-order
controls and required a pre-fork draw by the retired generation. Those
changes are reflected in the draft; review of the frozen runnable case and
preregistration are still pending.

Chat `01a21056-b5b2-7591-9981-88554b9d38ec` owns the protocol draft in
`/Users/doughnut/.codex/worktrees/kairos-ponytail-simplification/kairos`;
the draft was recovered unchanged from `d528c1e8` on 26 November.
No native operation was run for this draft. The separately qualified
[Track 111 performance scope](../../archive/111-measured-performance/handoff.md)
is closed; its evidence does not accept this protocol or its results.

## Claim validation experiment

The protocol and harness were committed as `d8ba90e` before evaluation.
The macOS ARM64 and Linux x86-64 runs of Node 26.13.1 produced identical
decisions for all 22 cases against the frozen `077721d` validator. Linux
ran in a network-disabled Apple Container with the source mounted read-only.
Independent review accepted the protocol before execution and checked
the resulting case decisions and digests.

Both checks accepted six valid controls and rejected four file-integrity
defects. The typed validator rejected eight additional defects, spanning
installation, native execution and benchmarking. The SHA-256 baseline
accepted those eight internally consistent bundles. Each typed rejection
had its intended diagnostic. The result met the preregistered rule.

Both checks accepted four false assertions when their metadata and file
hashes agreed. Those cases changed a lock digest, relabelled a GPU,
rehashed an empty Arrow output or claimed the legacy Sobol sequence
matched the reference. Execution and scientific claims still require
their source-bound evidence and independent oracles.

The planner needs a frozen decision rule and held-out matched workloads.
Branching needs shared checkpoint evidence across lifecycle and rollback
cases with full-replay and conventional CRN comparators. Hybrid evaluation
needs a coupling contract and an independent reference model. Track 115
remains In Progress.
[Protocol](claim-contract/protocol.md) · [Exact results](claim-contract/results.json) ·
[Execution receipt](claim-contract/execution.json).

Updated 7 November 2026. Status: Spec Approved; no implementation phase is accepted by this registration.

## Summary

Evaluate specific research advances beyond established ingredients with falsifiable hypotheses, held-out workloads and publicly reproducible evidence.

## Files changed

This track's specification, plan, metadata, contract, risk register, test matrix and index; coordinated programme records and registries. Runtime changes and hosted outcomes are recorded separately in the integration receipt.

## Contracts consumed

AGENTS.md, existing core/FFI/conformance/versioning contracts, the approved [programme](../../frontier-programme-20261107.md), Tracks 110, 111, 112, 114.

## Contracts changed

The programme establishes immediate Rust-only support; this track qualifies its own specific outcomes. No release/publication or downstream-pin acceptance follows from track registration.

## Tests added

Each implementation phase must add or identify the distinguishing oracles in test-matrix.md. Structural manifest/track checks are not native execution or scientific validation.

## Known risks

Feature breadth can outrun scientific support. Cap scope to falsifiable experiments, retain negative results and do not promise exceeding every competitor.

## Follow-up issues

Execute the unchecked tasks in plan.md with original owner handoffs. Keep unsupported capability claims and release gates blocked until their evidence exists.

## Integration notes

Use isolated bounded worktrees. The root integration packet combines disjoint changes after review; actual commit/PR/CI outcomes go in the programme integration receipt. Preserve old immutable receipts.

## Phase closeout evidence

No phase is closed. Record $conductor-review, accepted fixes, commit SHA, pushed ref, scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree and the next-phase decision only when they occur. Current planning status is not execution evidence.

