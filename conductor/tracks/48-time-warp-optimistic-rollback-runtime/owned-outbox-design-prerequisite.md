# Owned-process and accounted-outbox prerequisite

Status: architecture preparation; no accepted signatures or production dispatch. The independent architecture role audited the actual runtime; coordinator verified the same functions after the additive bridge integration at c19fbf4. Source SHA256: `cb1254e5cc60bb1d856a051efe312d7025b56131233d4680a6054c16dee3d49f`. Track48 remains In Progress.

## Verified gap

`OptimisticRuntime::new` requires processes for every partition LP. Global topology is constrained to those local processes. `validate_event` requires both source and destination locally present. Execution and rollback deliver positives and antis directly into destination queues, then `published_messages` copies already-delivered envelopes. `minimum_pending_tick` sees local queues only. Consequently publishing those copies over a network is not remote delivery or retained send accounting; a local observer stream cannot substitute for a remote outbox.

## Contract candidate to review jointly

Preserve the existing all-local constructor. A separate owned constructor distinguishes global partition/topology from the nonempty owned process subset, using the emission-authority contract decided separately. Native receive must accept globally authorized sources but require an owned destination. Only owned emitters schedule initial work and allocate handler outputs. Process execution, model snapshots, replay and validity tokens operate only on owned LPs.

Use one routing/preflight operation for execution outputs and rollback antis. Local destination routes once to its native queue; remote destination routes once to an exact retained outbox obligation. History retains the original envelope for future rollback even after positive admission acknowledgment. Outbox send identity includes delivery identity AND message kind: an anti is a separate obligation from its positive. Inspecting/retrying retains identity/metadata and never removes accounting. Timeout/socket write/cancel request cannot discharge a send.

Candidate API shapes for later review are `new_owned(...)`, `outbound_pending()`, `acknowledge_accounted(send_id, receipt)` and `accounting_snapshot()`. Names/types, authority configuration, admission status and snapshot/revision format are not frozen by this note. A native caller assertion is not proof of durable receipt. Track49 must persist and prove exact accounted queue/history/tombstone admission before ACK, including duplicate delivery readback; DuplicatePositive alone is insufficient.

Validate routes, full identity/metadata and both local and outbox capacities before publishing any mixed batch. Rollback capacity rejection must preserve healthy model/membership state; invalid post-handler outputs retain the existing poison boundary. Preserve the observation-only meaning of published_messages.

Unresolved outbound positives and antis constrain fossil/GVT floors. Independently sampled sender and receiver minima can miss a message during ACK handoff: Track49 needs a consistent participant/channel accounting cut, revision/round binding and durable unresolved-send recovery. This local seam does not prove distributed GVT or crash safety.

## Behavioral proof required before transport binding

- Two native runtimes own disjoint LP subsets, with no process/token for remote LPs. Remote-source delivery executes only at its owned destination.
- Local outputs enqueue once; remote outputs exist only in retained outbox. Repeated polling neither executes nor removes remote obligations.
- Delayed remote input triggers real suffix replay and downstream remote antis. Changed-metadata replacement survives its old anti, preserving model/RNG and committed trace parity with an independent serial oracle.
- Positive receipt does not discharge its anti. Unknown/wrong identity, kind or metadata receipts reject unchanged; lost ACK, duplicate retry and anti-before-positive preserve accounting.
- Remote positives and antis each prevent GVT beyond their tick; equality remains reversible.
- Mixed local/remote execution and rollback hit bounded capacities without partial publication.

Native local seam evidence must then be followed by actual OS-process/rank crash, restart, fencing, migration, durable receipt recovery and global GVT tests. Do not replace these with mirrored all-LP engines or observer message copies.

## Required order

Finish and independently accept the codec bridge. Jointly freeze authority identity, owned-process routing and receipt/accounting contracts. Implement their native leaves with explicit joins and existing regressions. Bind the actual versioned codec to those APIs, then implement persistent admission/recovery and real gRPC/MPI execution. Retain Track29 conditional gates, other track dependencies and unchanged raw validator failures. No new security exception, status advancement or release claim follows from this preparation.
