# Source authority identity: candidates for joint PDES/distributed review

Status: preparation only, not an accepted interface or dispatch packet. Architecture role inspected native source at a0ca742 (identical implementation to merged PR199). Coordinator records the finding to preserve the blocker rather than treating codec reconstruction as distributed admission. Track48 remains In Progress.

## Verified failure boundary

A new runtime initializes per-source incarnation counters at zero. Inbound observation advances only in-memory counters. DeliveryIdentity is (actual source LP, complete logical ID, incarnation); EventQueue indexes only incarnation beneath each logical ordering key. Output history records native envelopes and rollback generates antis from those records. Runtime identity and LP validity-token epochs are not delivery authority. Thus a transport-only authority epoch can still collide in native delivery indexes or overwrite queued work after stripping the outer envelope.

Native source SHA256: `f728d1338ed6846d3da6f4f86734347e5cd8a041efb2b1b5f0271f98d2ea1b89`. Existing wire epoch is a fixture/proposed transport namespace, not an implemented native contract. Logical root sequences can also be reused across restarts independently of delivery incarnation, so fixing exact cancellation identity alone does not establish logical replay correctness.

## Candidate A: native additive authority identity

Preferred candidate for joint review. Add typed source authority, with persistent simulation namespace and never-reused ownership epoch, to the immutable native message. Add an authority-bearing checked reconstruction constructor and explicit runtime emission configuration while retaining clearly local-preview existing constructors. Exact names/representations and compatibility defaults are not frozen here.

Propagate authority through DeliveryIdentity, queue inner keys, retained send/history envelopes, replay markers, duplicate/conflict indexes and tombstones. Never add it to logical ancestry or ordering. Old recorded antis retain their original source authority/incarnation and original tick/destination/payload; new replay outputs get fresh owned identity. A transport-only field or queue key change alone is insufficient.

Track49 owns durable reservation, namespace/source authorization, fencing, migration protocol and durable accounted admission. The native configuration must distinguish owned emitters from observed inbound sources; an arbitrary epoch integer or observed counter is not ownership proof. Native admission and durable transport authorization need a reviewed boundary, including controlled historical retries/antis after ownership changes.

## Candidate B: durable non-overlapping incarnation ranges

Preserve native identity shape but introduce an owned-source allocator seam with durably reserved disjoint bounded ranges. Persist high-water reservations before use; burn possibly used ranges after ambiguous crashes. Native allocation must enforce range upper bounds and exhaustion, never wrap/roll back, and separate inbound observation from owned allocation. Merely setting a start counter or rewriting incarnations after send-history creation is insufficient.

Both candidates require persistent root-sequence ownership, exact old send records and unresolved receipt accounting. Candidate B is faithful only with global source-scoped non-reuse and restart/migration/split-brain fencing; it is not a smaller shortcut accepted by this note.

## Recovery and acceptance requirements

Migration must transfer model/RNG state, pending positives/antis, reversible history with recorded outputs, replay markers, tombstones, conflict metadata, GVT and unresolved sends/receipts. Retired owners cannot emit fresh work; blanket rejection of old ownership traffic must not strand valid historical cancellation or retry. Fresh runtime/component handles must not revive pre-recovery tokens.

Joint review must settle namespace scope, authoritative persistence owner, reservation/activation atomicity, allowed historical admission, failure/poison boundaries and source API compatibility before a writer packet. Then prove:

1. Identical source/logical ID/incarnation under distinct authorities coexist without queue overwrite; old anti cannot cancel fresh ownership. For ranges prove equivalent disjoint reservations.
2. Crash before/after reservation, allocation, send-log persistence, transmission, durable admission and ACK never reuses exposed identity or loses unresolved accounting.
3. Migration with executed history/downstream sends, then a late straggler, cancels exact old sends and preserves replacements; complete model/RNG state and committed logical trace match the sequential oracle.
4. Split-brain stale producer, delayed historical retry, replacement-before-old-anti, anti-before-positive, duplicate receipt and changed/cross-destination metadata conflicts follow the frozen authorization/accounting rules.
5. Namespace/authority/counter exhaustion, failed reservation and ambiguous recovery fail closed without silently mutating healthy state.
6. In-flight positives and antis constrain GVT through restart/migration, equality remains reversible and fossil collection preserves cancellation obligations.
7. Different authority/range allocations for an identical logical workload leave ordering, payloads, model/RNG state and committed logical trace unchanged.

No code, persistence, transport, native execution, release or production evidence is claimed here. The accepted three-method codec bridge remains a separate prerequisite; this decision must not be smuggled into that two-file implementation packet.
