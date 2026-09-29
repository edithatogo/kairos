# Proposed Track 25 review gate for CareOps Flow API

Status: Q0.1 design disposition recorded; exact method signatures and release remain held. This proposal is not a completed Track 25 template review.
Date: 2026-09-29
Related CareOps decision record: `conductor/design/queue/ADR-0003-flow-runtime-contract-proposed.md`

## Review outcome today

**Decision: accept the owner-approved Q0.1 design direction and experimental classification for both exact roots; retain a release hold.** `crates/kairo-ecs-des` and `crates/kairo-ecs-abm` are now in the protected-surface inventory and aligned policy, matrix, and release note. Preimplementation design dispositions are recorded per root. Concrete Flow symbol signatures and implementation-level API governance/release review remain incomplete; this proposal is not that signoff.

The owner-approved Rust design is additive and experimental in intent. It preserves existing `DESContext`, FIFO `Resource`, `ABMContext`, and `BehaviorSimulation` behavior; keeps scheduler order and RNG derivation; and does not change C ABI, Arrow schemas, or host bindings. These remain implementation claims to verify, not release evidence. The design specifies one private shared `FlowRuntime`, opaque handles, checked commands, facade-scoped lifetime caps and cleanup, a DES-owned dispatcher with ABM adapter, and typed in-memory continuation. Both DES and ABM exact roots are registered. Portable checkpoint encoding remains with Track 22. See the preimplementation design disposition at `docs/design/api-reviews/flow-runtime-q0.1.md` and the per-root review records `flow-runtime-des-q0.1.md` and `flow-runtime-abm-q0.1.md`; all retain a release hold.

## Proposed Track 25 intake

| Template field | Proposed value | Current evidence/state |
|---|---|---|
| Review title | Additive experimental FlowRuntime and queue API | CareOps ADR-0003 |
| Proposed by | CareOps Sim coordinator | Recorded in ADR-0003 |
| Affected roots | `crates/kairo-ecs-des`; `crates/kairo-ecs-abm` | Both exact roots are registered as experimental; per-root design dispositions exist, but concrete symbols remain unreviewed |
| Surface family | `rust_api` | Proposed; no C ABI, Arrow, or host API change in this design |
| Proposed first release stage | Alpha candidate after Q0.2/Q0.3 and symbol-level review | Not authorized by this design disposition; release remains held |
| Compatibility level | `experimental-breaking` for proposed Flow symbols | Existing legacy API behavior is preserved; proposed symbols remain held before release |
| Decision | `accepted` for design classification; release hold remains | Concrete symbol-level API and release reviews are pending |

### Root registration disposition

The Kairos owner directed registration of both existing crate roots because the new API is additive across DES and ABM. The inventory, policy, matrix, release note, and validator are aligned. The Q0.1 design records accept this classification only; they do not review concrete method signatures or authorize alpha/release. The full Track 25 track remains open.

## Required owner decisions

| Owner | Decision needed | Proposed direction | Gate if unresolved |
|---|---|---|---|
| Track 01 — core/state | Are a fresh private runtime and facade-only scheduling/create/despawn with cumulative `u32::MAX` caps acceptable, alongside typed component-removal hooks? | Accept the documented supported-use bound; make cleanup guarantee conditional on successful in-memory cleanup and `World::despawn`; do not claim global core/state overflow safety. | No Q1 implementation |
| Track 03 — Flow/DES/ABM | Where does the adapter live, what dependency edge does it add, and how are event kinds routed to handlers at deterministic dispatch boundaries? | Add a Flow-specific adapter with read-only shared queries and buffered checked commands; preserve the separate legacy `BehaviorSimulation`. | No Q1 implementation |
| Track 25 — API governance | After Q0.2/Q0.3 and implementation settle concrete symbols, complete the symbol-level DES and ABM API review and confirm compatibility fixtures and release evidence. | If either root cannot be classified under the existing policy, hold implementation and revise the API boundary through its owner review. | Concrete-symbol approval and release remain held; Q0.1 root registration and design disposition are already recorded |
| Track 22 with Tracks 01/03/04/25 | Which complete state is needed for portable checkpoint/resume? | Keep queue continuation typed and in-memory now; coordinate a later versioned checkpoint contract; do not add a second snapshot standard. | No portable checkpoint claim |

The Kairos owner disposition is recorded in the parent Q0.1 package. Independent read-only technical reviews informed the clarified boundaries, but are not represented as release signoffs or full Track 25 closeout.

## Remaining gates

1. Q0.2 must freeze queue/tie/deadline/completion/preemption semantics as executable fixtures.
2. Q0.3 must freeze Flow event IDs, lifecycle records, and transition joins.
3. Before implementation merge, perform exact-symbol compatibility review for both roots and run the required conformance/MSRV/API tests.
4. Keep the release hold until implementation evidence, release reviewer signoff, and any required red-team review are complete.
5. Q1 implementation remains gated on Q0 closeout and D2 hosted-CI readiness.

## Evidence references

- Parent proposal: `conductor/design/queue/ADR-0003-flow-runtime-contract-proposed.md`
- Kairos runtime proposal: `conductor/research/careops-flow-runtime-contract-proposal-20260929.md`
- Kairos continuation proposal: `conductor/research/careops-flow-context-codec-proposal-20260929.md`
- Track 25 review form: `docs/design/api-review-template.md`
- Exact-root inventory: `docs/design/protected-surface-inventory.json`
- Track 25 source of truth: `conductor/contracts/versioning-compatibility.md`

This proposal records the design-time gate and remaining release hold; it does not close Track 25. Q0.1 phase acceptance is recorded separately in the parent CareOps evidence after its manual traces and local checks pass.
