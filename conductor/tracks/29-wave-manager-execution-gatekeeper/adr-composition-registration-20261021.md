# ADR: pending composition registration and capability dependencies

Date: 2026-10-21. Status: incorporated planning decision; runtime dispatch and
API acceptance pending. Scope: Kairos and its careops-sim consumer.

## Context

The user requested reconciliation and merge preparation of the parallel proposals.
Local registration0157d0c introduced67–69 on mainf0d30e2; corrected composition
d0637aa and capability auditfc971d5 refine the semantics. Track29 requires an ADR
for ownership/dependency changes. Registry Done labels for64/66 are disputed as
full-capability claims and cannot certify an interface by themselves.

## Decision

Retain67 (relationships),68 (boundary transfers) and69 (competing hazards) as
**Planned**. Retain the existing owner assignments and dependency edges to01/03/64
for consumed identity/scheduler/RNG contracts. Remove69's blanket dependency on66:
its smallest analytic sampler/clock uses existing RNG and scheduler mechanics;
ensemble/SBI/CLI tooling is not a prerequisite for that contract. If a later leaf
consumes66 experiment facilities it must bind a separate owner/capability receipt.
This is not a waiver of RNG, statistical, checkpoint or worker-equivalence tests.
Track21 reviews statistical semantics; existing01/25 owners review public contracts.

The broad owned_paths entries are routing envelopes, not exclusive runtime leases.
Before dispatch,67 must obtain exact-file handoffs from core/state/ABM owners;
68 and69 must serialize shared DES changes or prove disjoint files/interfaces.
Never recover another writer's reservation merely because it expired. Track65/66
reserved worktrees and plan inputs remain untouched; pending audit work lives in
their handoffs. A status transition requires the existing coordinator and gates.

All historical track statuses and closeout records remain unchanged. The registry
now has70 tracks:60 recorded Done,7 In Progress,3 Planned. Historical67-track audit
totals remain dated snapshots. No runtime API, dependency package, source pin,
hardware qualification or release status is changed by registration.

## Alternatives and consequences

Keeping all of66 as a prerequisite would impose unrelated ensemble/CLI work on a
small hazard primitive. Removing all upstream gates would permit unsupported
scheduler/RNG assumptions. The selected contract-specific approach retains the
minimal owner graph and adds evidence-bound conditional dependencies at dispatch.
A required capability absent at the bound revision blocks that leaf despite Done.

Small experiment-lifecycle reuse K-C05/CAP-22 is qualified first with ED/logistics
consumers and recovery; resource bundle/transfer extraction follows. Broader
experiment and observation qualification remains a later integration step.

## Verification and rollback

Conductor setup/DAG/artifact/phase/coverage validation and gate-catalog checks must
pass on the integrated head. Behavioural gates remain pending in quality-gates.md
and track test matrices. Rollback restores the prior registry/dependency proposal
and linked planning text together, without changing runtime or historical evidence.
