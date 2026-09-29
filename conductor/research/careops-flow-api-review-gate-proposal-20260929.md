# Proposed Track 25 review gate for CareOps Flow API

Status: review-ready proposal; not an API approval or compatibility-policy change.
Date: 2026-09-29
Related CareOps decision record: `conductor/design/queue/ADR-0003-flow-runtime-contract-proposed.md`

## Review outcome today

**Decision: release hold.** The queue/Flow design can be reviewed, but the Track 25 API review template cannot accept it yet: its required exact affected root, `crates/kairo-ecs-des`, is absent from `docs/design/protected-surface-inventory.json`. The API review template requires an exact inventory match. This record does not edit that inventory or policy and does not treat the proposal as a public API.

The proposed Rust contract is additive and experimental in intent. It preserves existing `DESContext`, FIFO `Resource`, `ABMContext`, and `BehaviorSimulation` behavior; keeps scheduler order and RNG derivation; and does not change C ABI, Arrow schemas, or host bindings. These are design claims to verify again against implementation, not release evidence. The proposals specify one private shared `FlowRuntime`, opaque handles, checked commands, facade-scoped lifetime caps and cleanup, and typed in-memory continuation context. Portable checkpoint encoding remains with Track 22 and its upstream owners.

## Proposed Track 25 intake

| Template field | Proposed value | Current evidence/state |
|---|---|---|
| Review title | Additive experimental FlowRuntime and queue API | CareOps ADR-0003 |
| Proposed by | CareOps Sim coordinator | Recorded in ADR-0003 |
| Affected root | `crates/kairo-ecs-des` | Missing from protected-surface inventory; blocking rule fails |
| Surface family | `rust_api` | Proposed; no C ABI, Arrow, or host API change in this design |
| Proposed first release stage | `alpha` after root/policy review | Not authorized today; the API remains on release hold |
| Compatibility level | `release-hold` pending exact-root registration and owner decisions | Track 25 template cannot accept while root is absent |
| Decision | `release hold` | Required until Track 25 review is complete |

### Proposed inventory treatment for Track 25 to review

If the maintainers accept this boundary, add one exact experimental Rust root for `crates/kairo-ecs-des` and align the versioning policy, compatibility matrix, API review record, release note, and compatibility validator. The inventory example should distinguish the legacy DES helpers from the new Flow surface. Track 25 owns and approves these edits; this proposal grants no write or acceptance authority over those files.

If Track 25 rejects that root or classification, it must identify an alternative exact protected boundary that satisfies its template before Q0.1 can close. Until then, implementation remains behind Q0.1 and D2.

## Required owner decisions

| Owner | Decision needed | Proposed direction | Gate if unresolved |
|---|---|---|---|
| Track 01 — core/state | Are a fresh private runtime and facade-only scheduling/create/despawn with cumulative `u32::MAX` caps acceptable, alongside typed component-removal hooks? | Accept the documented supported-use bound; make cleanup guarantee conditional on successful in-memory cleanup and `World::despawn`; do not claim global core/state overflow safety. | No Q1 implementation |
| Track 03 — Flow/DES/ABM | Where does the adapter live, what dependency edge does it add, and how are event kinds routed to handlers at deterministic dispatch boundaries? | Add a Flow-specific adapter with read-only shared queries and buffered checked commands; preserve the separate legacy `BehaviorSimulation`. | No Q1 implementation |
| Track 25 — API governance | Register/classify the exact DES crate root and complete the API review and compatibility pack. | Add an experimental Rust root and keep release hold until the inventory, policy, matrix, API record, and release note agree. | No Q0.1 close or release claim |
| Track 22 with Tracks 01/03/04/25 | Which complete state is needed for portable checkpoint/resume? | Keep queue continuation typed and in-memory now; coordinate a later versioned checkpoint contract; do not add a second snapshot standard. | No portable checkpoint claim |

Track-level owners may delegate evidence gathering, but only recorded owner dispositions count as approval. The prior source reviews are technical reviews, not owner signoffs.

## Required evidence before the hold can be lifted

1. Track 01 disposition on the runtime ownership, operation limits, cleanup hooks, and conditional failure semantics.
2. Track 03 disposition on adapter crate/dependency direction, dispatch route, handler/context identity, and buffered command boundary.
3. Track 25 protected-root inventory and aligned compatibility artifacts, plus a completed API review record with reviewer signoff.
4. Consolidated ADR-0003 updated with those dispositions and source hashes, then reviewed at the pinned Kairos commit.
5. Q0.2 semantic fixtures only after Q0.1 is accepted; Q1 implementation only after Q0 and D2 gates.

## Evidence references

- Parent proposal: `conductor/design/queue/ADR-0003-flow-runtime-contract-proposed.md`
- Kairos runtime proposal: `conductor/research/careops-flow-runtime-contract-proposal-20260929.md`
- Kairos continuation proposal: `conductor/research/careops-flow-context-codec-proposal-20260929.md`
- Track 25 review form: `docs/design/api-review-template.md`
- Exact-root inventory: `docs/design/protected-surface-inventory.json`
- Track 25 source of truth: `conductor/contracts/versioning-compatibility.md`

This record makes the release hold explicit and reviewable. It is not the required Track 25 acceptance and does not mark Q0.1 complete.
