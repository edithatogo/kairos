# ADR: scoped native authority and owned optimistic delivery

Status: proposed for architecture and independent adversarial review, 4 October 2026. Source baseline: merged codec bridge `1afe522239087d2a7a92caa352ffd19a4fd1b254`. Track48 remains In Progress. This contract describes the required native path toward faithful Track49 delivery; it does not grant production transport, persistence, release or dependency waivers.

## Decision and compatibility

Use native scoped delivery authority and genuinely disjoint owned-process runtimes. A transport epoch stripped before native admission and a published_messages observer copy do not meet this contract.

`OptimisticAuthority` is a public immutable Copy/Debug/Eq/PartialEq enum with `LocalPreview` and `Scoped { simulation_namespace: u128, ownership_epoch: u64 }`. Zero/MAX values are valid representations. Durable namespace/epoch uniqueness and activation fencing are Track49 responsibilities. The actual source LP stays separate. No authority value, incarnation or arrival order may select the executable winner of a logical occurrence or enter logical ancestry/order keys.

Existing native constructors and internally emitted legacy messages retain LocalPreview semantics. Add `OptimisticMessage::authority() -> OptimisticAuthority` and checked `try_from_authority_parts(event, logical_id, authority, incarnation, kind) -> Result<Self, OptimisticError>`. Reconstruction validates complete bounded ancestry and retains exact native u128 tick, source/destination, payload, sequence/ordinal, authority, incarnation and kind. It authenticates nothing. Clone and as_anti preserve authority.

The initial representation leaf rejects every Scoped positive or anti at the existing all-local runtime receive boundary with `OptimisticError::ScopedAuthorityRequiresOwnedRuntime`, before changing queue/history/model/RNG/tokens/tombstones/counters/GVT or allocation. It does not propagate scoped identity into executable indexes or enable scoped admission. There is no claim of ownership, fencing or multi-authority cancellation from that leaf.

## Owned execution and exact identity

A separately reviewed new_owned constructor distinguishes complete global partition/topology from a nonempty exact subset of owned processes. Emission epoch keys equal that subset. Only owned LPs have process state, validity tokens and allocation authority. Global sources may deliver to owned destinations, but local scheduling from an unowned source and receive to an unowned destination reject unchanged. Foreign simulation namespaces and LocalPreview traffic are rejected in scoped owned mode.

A separate owned-options type supplies finite local, global-LP, outbox and receipt limits; do not add public fields to OptimisticLimits or break exhaustive existing literals. Never rewrite receiver-observed authority into the current emission authority. Inbound observations do not advance another authority's emitter counter. Persisted root-sequence ownership remains necessary independently of exact delivery incarnation.

Every delivery index, queue storage key, known-delivery record, history/output record, replay marker, conflict index and tombstone must preserve `(actual source LP, authority, complete logical ID, incarnation)`. These storage dimensions must not resolve competing executable versions. Positive and anti remain separate outbound obligations keyed additionally by kind and bound to exact event metadata. Old recorded antis retain their original authority, incarnation, tick, destination and payload.

## Replacement and retirement barrier

The logical cohort is `(actual source LP, complete logical ID)`, excluding tick/destination because replay can change both. At most one version's effects may be active for a cohort; speculative old execution followed by rollback is allowed. Final committed trace has no duplicate logical occurrence. Numerically greatest epoch/incarnation and arrival order are not winner rules.

Choose a sender retirement barrier for cross-destination replay replacement. The source retains the exact successor as blocked, accounted and unpublishable until verified cancellation-applied evidence for its exact predecessor arrives. No atomic local anti/new-positive pair can cancel effects at another owner. Runtime output history generates predecessor bindings; a model handler cannot invent them. Exact repeated transitions are idempotent; conflicting successors/forks/cycles fail unchanged. Skipped or unknown predecessor chains need bounded staging or explicit rejection/retry; no silent activation.

Retirement evidence binds old source/authority/completeID/incarnation/anti kind/tick/destination/payload, predecessor-successor cohort/transition, receiver ownership/recovery generation, retirement record and accounting revision. Cancellation-applied means old local effects removed or never applied, delayed old positive cannot reapply, and every induced downstream anti remains accounted. It does not mean the entire descendant graph has drained. Ordinary queued anti admission, socket write, timeout or caller boolean does not satisfy it.

Native volatile accounting evidence and durable transport evidence are separate types/claims. The native local seam may issue an opaque receiver-bound capability after actual cancellation; it cannot label that capability durable. Production release requires the separately reviewed Track49 verifier/persistence boundary, durable receiver retirement and durable sender release decision. Public receipt-shaped values or user-implemented callbacks alone are not durability proof. Exact trusted verifier API, recovery activation and ownership-generation binding must be frozen before the enabled owned/retirement writer packet.

Historical retries/antis are authorized from exact retained send/retirement records under current fencing policy. Do not blanket reject all old epochs or treat every never-seen stale positive as historical. Migration transfers model/RNG, pending work, history/outputs, replay markers, tombstones/conflicts, allocation/root-sequence reservations, GVT and unresolved sends/receipts. Fresh handles cannot revive pre-recovery validity tokens.

## Routing, bounded state and GVT

Execution output and rollback anti use one atomic route/preflight operation: local destination enqueues once; remote destination creates one retained send. Poll/retry preserves exact identity and never removes accounting. Positive admission ACK may discharge its positive send only, retains rollback history, and cannot discharge an anti. Anti retirement receipt is stronger than admission ACK. Blocked replacement remains accounted until verified release. Wrong identity/kind/recipient/authority/metadata/receipt rejects unchanged.

Mixed local/remote batch limits and routes preflight before publication. Healthy rollback-capacity rejection leaves model/membership unchanged; invalid post-handler output retains existing explicit poison behavior. Bound blocked/staged transitions, receipts and retirement records; exhaustion never wraps, silently drops or unblocks work.

Unresolved remote positives, antis, blocked replacements and retirement traffic constrain local GVT to the minimum relevant tick. Equality remains reversible. Independently sampled participant minima or a receipt revision alone cannot establish a consistent distributed cut. Track49 proves ACK-handoff accounting, crash-safe recovery and real OS-process/rank fencing/migration.

## Required leaves and joins

1. Authority envelope representation plus fail-closed legacy scoped rejection (the companion leaf packet). Review source/held-outs/MSRV; no scoped executable capability claim.
2. Jointly freeze owned configuration, native accounting capability and production receipt verifier/recovery boundaries, then propagate exact authority identity throughout native state.
3. Owned construction/admission and genuine local-versus-remote retained routing with atomic capacity checks.
4. Blocked replacement, exact cancellation-applied retirement, receipt bookkeeping and release transitions; source/old/new destinations owned by three disjoint runtimes.
5. Native GVT/outbox bounds and independent serial model/RNG/committed-trace parity across reordered positives/antis/receipts and budget-one resume.
6. Track49 durable admission/recovery, consistent cuts and actual process/rank crash/restart/migration/fencing acceptance, with its own mandatory dependency gates.

The full objective remains all implementation tasks. Passing an earlier leaf does not replace later joins or mark Track48/49 Done. No source writer starts an enabled scoped path until its precise contract is independently accepted.
