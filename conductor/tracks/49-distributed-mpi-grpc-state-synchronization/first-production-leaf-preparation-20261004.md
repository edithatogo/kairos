# Track49 first production leaf preparation — 4 October 2026

Status: preparation candidate blocked on a reviewed Track48 codec bridge; no production dispatch or native test execution. Read-only distributed-role inspection bound source08eae25 and verified selected inputs unchanged atdd645cd. Coordinator checked the relevant actual APIs. Human conditional scheduling authority is recorded separately in the Track29 entry ADR; all conjunctive conditions remain mandatory.

## Verified interface gaps

`LogicalEventId` publicly creates roots and appends a checked child to an existing complete `OptimisticEventOrderKey`, but exposes only depth/root fields. Its Output parent key/ordinal are private. `OptimisticEventOrderKey` has getters but private fields/constructor. `OptimisticMessage` exposes immutable event/ID/incarnation/kind/order key and anti conversion; its constructor is private. A transport cannot losslessly inspect a full Output identity or reconstruct the message accepted by `OptimisticRuntime::receive`. Do not use seeding or simplified `RemoteEvent`/sequence fields to fake decoded Output delivery.

The wire draft proposes a durable source-authority epoch, fencing/restart rules and acknowledgement after durable accounted admission. They are not existing runtime fields or persistence guarantees. Decide their ownership in a versioned transport contract before implementing receipt behavior. Rank, LP ID or protocol version cannot substitute for an authority epoch. Codec roundtrip alone will not prove durable admission, rollback, global GVT or live transport.

## Recommended packet order

1. A separate Track48-owner packet freezes and adds a narrow validated codec bridge: full structural-ID inspection, checked ordering-key/inbound-message reconstruction, preservation of payload and incarnation, bounded ancestry, and validation before membership mutation. PDES and distributed roles review the interface; include the existing public child constructor in the design. No model execution/order or RNG change is implied. Keep this prerequisite separate from Track49 blocked paths.
2. A Track49-owner contract packet defines the additive versioned envelope, authority-epoch ownership and durable-admission boundary. Freeze limits and rejection/receipt oracles; retain all old generic protobuf contracts. No durable success claim until persistence/crash/accounting evidence exists.
3. A bounded Track49 codec/conversion leaf can then bind actual new APIs and meaningful roundtrip/negative commands. Production protobuf/tonic/prost selection needs fresh version/MSRV review and exclusive manifest/lockfile ownership; current `grpc=[]`/`mpi=[]` features and source are dependency-free placeholders, with no tonic/prost/rsmpi lock entries. Split shared lock work rather than silently widening a five-file packet.
4. Real gRPC service/client/process delivery and MPI rank/GVT exchange follow separate leaves and live evidence. The existing matrix names real integration targets that do not yet exist; resolve commands at dispatch and never record those planned commands as executed.

## Codec and acceptance oracle

Preserve every event field/payload byte, complete parent ordering tree and ordinal, message kind, full u64 incarnation and source authority namespace. Include roots and outputs, boundary values, exact anti/replay identities, conflicting metadata rejected before admission and duplicate receipt semantics. Match membership to the independently reviewed wire fixtures; retain their stated limits. Receiver acknowledgements require proof of accounted durable queue/tombstone admission, not a socket write or RPC response alone. Preserve unresolved sends in GVT accounting on failure/timeout.

No packet is ready from this preparation note. At dispatch, rebind exact HEAD and selected source hashes, reserve isolated paths, obtain both PDES/distributed review, and record normal PR199 merge/exact-head gates plus the remaining Track29 ADR conditions. Track48 remains In Progress and the raw wave/dependency validator failure remains unchanged. Website/vendor/audit migration, Arrow internals and Slurm infrastructure are outside this candidate.
