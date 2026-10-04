# Full owned execution: implementation review tripwires

Read-only architecture review of accepted baseline e332dc829c57fa5b1c4b0087e25d6ca31fd4c96b, 4 October 2026. This records review targets and a contract-consistent selection interpretation; it is not source acceptance, dispatch expansion or executed test evidence. The [frozen behavioral contract](owned-handler-retirement-leaf.md), SHA c7ca42d6ab4845d5eef4dcf9719b3c917cce8c65d389b57f162f50402a01f7a1, and [API appendix](owned-handler-retirement-api.md), SHA f8c4e76b0c3c78b660ecefa814a627c7938d7562c21a70d4802ebdf043131456, remain immutable.

Baseline files independently read and coordinator hash-verified:

| Path under crates/kairo-ecs-pdes/src | SHA-256 |
| --- | --- |
| optimistic.rs | 036c9432e8c8d3f9b28451a983258d47fdf4f8b49e69978e170239d55873196f |
| optimistic/owned.rs | 957e9fd7b20949d6e1b4ef71442ef57bde103415cde60fb00128649950b01e60 |
| optimistic/owned_routing.rs | 61d1611debe0110c22aa7160e69d1189601c93b5910268b7bf2453dce071426a |
| generation_bitset.rs | eb8a15153ac95666a3962e895ea6bfbbeb738191fc4b7a83c763ed00ae725a90 |

## Selection and budget interpretation

Apply the first eligible anti in ascending LP order as one Anti unit. Its executed-positive suffix rollback stays inside that unit. Otherwise select up to remaining budget, one earliest positive per LP in ascending LP order. Scan that vector before PositiveRound callbacks/dequeue. The first selected LP whose history contains a full key later than its candidate performs one Rollback unit, triggered by that exact positive; do not choose the globally earliest trigger tick. Charge one, discard the candidate vector, and reselect actual post-publication state using remaining budget, with antis first again. Other candidates do not execute, charge or invalidate merely because they were inspected; real rollback queue effects still invalidate affected LPs.

Without a straggler, stage the entire selected vector as one atomic PositiveRound; charge its width at first staging callback. Callback-free anti/control rejection costs zero, successful guarded publication one. A callback-bearing control unit charges once at its first staging callback. Failure inspection contains the one full Anti/Rollback trigger or complete PositiveRound selection. Final model/RNG, active work and committed logical trace must agree after drain/common cut across budget chunks; provisional attempts and revision counts need not. Architecture and both implementation/independent authors received this interpretation; no contract edit is required.

## Concrete source review targets

- Existing legacy execute_round, remove_pending, allocate_incarnation and rollback_suffix mutate queue/epoch/emitter state before publication and assume local destinations. Do not reuse them unchanged for native staged transactions or publish a prefix before a later selected LP fails. Earlier committed units must survive a later error.
- Trace every snapshot/Clone/restore/Drop path, including early returns and detached-history cleanup: no model operation or snapshot destruction under lifecycle guards. Runtime destruction invalidates its actual gate before process destructors.
- Compensation restores every restorable touched LP without foreign gates, preserving logical model/RNG and unpublished protocol facts while advancing nonrollback validity as specified. Fatal restore/clone/cleanup/gate failure preserves the first deterministic fatal cause and original trigger; poison invalidates all runtime tokens.
- Validate actual issuer/witness/configuration before caches. Existing root ACK completion lacks a GVT check and receiver retry recognizes only Pending; the joined implementation must enforce actual envelope event ticks before completion readback and verify current Executed/Tombstoned facts while keeping the original capability immutable. Historical ancestry ticks are not pending work: child40/ancestor10 remains valid at floor26.
- Keep blocked local successors outside EventQueue; its mechanical within-key storage ordering cannot choose competing executable versions. Bound graph traversal and role records, share root reservations and local self-cancellation roles, coalesce repeated absence, and prove request-first/anti-first canonicality and backward P-to-N1-to-N2 completion. Promote only the current ready local successor once.
- Preflight full net deltas for intent/outbox/receipt/pending/history/tombstone demand and checked IDs/epochs/revisions. Count reserved versus retained unique slots, including local bundles. Exact retained retries require no additional accounting capacity; returned observer clones may allocate ordinary heap memory. Applied proof closes lost-ACK obligations without unreserved accounting demand.
- Never-arrived remote P gets its stable Tombstoned capability at anti application. Preserve existing Pending capabilities as historical readback. Local self cancellation exposes its shared anti/applied bundle without manufacturing a positive transport receipt.
- Detailed CommittedCleanupFailed progress includes the current published unit; the three prepublication phases exclude staged messages. Observer copies never authenticate transport or retirement.
- Group cut stages all actual complete healthy/sealed/closed participants before any floor publication. Verify unresolved minima, equality, old30/new25 retention and bounded surviving-parent rewiring. Reject pre-GVT tickets/ACK/proofs before caches even if some records remain retained. Separate prepublication failure from committed postpublication cleanup poisoning.

The exact source plus independent fixtures must pass actual full Rust1.98.1 and1.76.0 crate lanes with no failures/ignored tests, formatting and strict all-target Clippy. Source-only obsolete heldout-guard failures remain provisional and cannot be skipped or recorded as green. Fresh full CI, public API/determinism/Conductor review, hosted checks, normal merge and real distributed acceptance remain separate gates.
