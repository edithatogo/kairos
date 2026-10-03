# Track 48/49 draft wire requirements and fixture profile

Status: locally reviewable preparation only. This document grants no Track 49 production dispatch, dependency satisfaction, security exception, public schema acceptance or distributed pass. Track 48 remains In Progress. The sibling reference model is a delivery-membership oracle, not a transport, decoder, scheduler or rollback implementation.

## Identity and lossless data

A positive/anti envelope preserves actual emitting source LP, destination LP, unsigned tick and full payload bytes. The fixture profile uses canonical decimal strings for u64 values, integer u32 LPs/ordinals, and lowercase even-length hex for bytes; future protobuf/binary encoding is Track 49's decision. Decimal strings prevent JSON double-precision loss. No payload hash or byte count substitutes for payload bytes.

Logical identity is Root(actual source LP, stable u64 sequence) or Output(full parent ordering key, u32 output ordinal). The complete parent key contains tick, actual source and recursive logical identity. Root sorts before Output; output ordering compares complete parent keys then ordinal. Execution ordering is (tick, actual source, logical identity). Arrival order, incarnation and authority epoch never influence that ordering. Depth zero denotes a root; depth 128 is valid and 129 is rejected. Output ticks must be strictly after their parent tick. The fixture cannot establish authenticated emitter ownership from an ancestry key alone.

Exact cancellation identity is (actual emitting source LP, proposed durable source-authority epoch, full logical ID, incarnation). Epoch is a proposed transport requirement, not an existing local runtime field. Replay retains logical ID and uses a fresh nonrollback incarnation; its destination, tick and payload can legitimately change. Matching anti retains the original delivery metadata. Cross-destination metadata conflicts for the same exact identity must fail before mutation. Root-origin LP does not replace actual emitting LP.

Restart/migration must durably reserve a never-reused authority epoch or preserve equivalent globally non-reused cancellation authority, including counters, outstanding deliveries, tombstones and acknowledgements. Epoch increment in memory is insufficient. Old-epoch positives and antis keep their exact namespace and cannot cancel a new-epoch incarnation. Counter/epoch exhaustion fails closed; no wrap. Persistence, fencing and migration atomicity are unimplemented and need owner review and live crash tests.

## Validation and receipt boundary

The draft fixture profile rejects unknown/missing fields, booleans as integers, noncanonical decimal strings, u64/u32 overflow, malformed payload hex and excess ancestry before membership mutation. Limits for this profile only: 65,536 canonical JSON bytes, 4,096 payload bytes, 129 ancestry nodes and depth 128. These are concrete fixture limits, not negotiated production limits or an allocation/decoder safety guarantee. A real decoder needs byte preflight before parse, duplicate-key rejection and its own fuzz/adversarial evidence. Topology and authorization validation remain transport obligations.

Duplicate pending/executed positives are rejected, consistent with the local runtime. Repeated matching antis and positives matching an exact tombstone are idempotent. Conflicting metadata always rejects. Rejection leaves all ledger state unchanged. The reference does not execute handlers, roll back model/RNG state, represent post-GVT delivery forgetting, or implement replay.

The sender retains a send in GVT accounting until the receiver acknowledges durable admission into the receiver's accounted queue or tombstone state. A socket write, RPC response without admission, or cancellation request is insufficient. Retry uses the same exact identity and metadata; pending/executed-positive duplicate rejection must be interpreted as an already-accounted receipt only when the receiver can prove that admission, not as a fresh delivery. Durable acknowledgement/failure behavior remains proposed. Shutdown drains or reports unresolved sends; lost acknowledgement and sender crash retain unresolved accounting. No timeout silently clears an in-flight send.

A proven global floor is monotonic and no greater than every queued positive, queued anti, replay input, staged output and in-flight positive/anti tick. Equality remains reversible; fossil collection is strictly below the floor. All participants and channel snapshots must contribute to a real distributed GVT protocol; a list minimum in this fixture is only a necessary local bound, not proof of global quiescence.

## Existing wire gaps and ownership

Current simulation.proto RemoteEvent preserves payload bytes but lacks structural logical identity, exact incarnation and durable authority epoch. AntiMessage.sequence cannot identify replay incarnations or full ancestry; migration PendingEvent.sequence/payload_bytes cannot preserve the logical tree or actual payload. GvtProposal.local_min_tick alone does not prove in-flight channel accounting. These are reviewed gaps, not edits or claims about a working transport. Track 49 owns codec/protobuf/transport changes; Track 48 owns runtime semantics. Both must review an additive versioned envelope and migration/receipt protocol before implementation.

## Required future evidence

Run these fixture vectors through the actual codec and receiver after owner acceptance, adding oversized raw input, duplicate fields, authorization/topology and persistence/crash cases. Then record real 2/4-rank MPI and two-process gRPC straggler replay, downstream cancellation, replacement ordering, duplicate retries, migration/failure recovery and GVT/fossil behavior. Compare complete model/RNG state and committed logical trace to the independent sequential oracle; preserve toolchain, topology, source, seed/input hashes, commands, raw logs, exit statuses and independent review. Local reference tests cannot close those gates.

Run local draft checks with: `python3 -B -m unittest discover -s conductor/tracks/48-time-warp-optimistic-rollback-runtime/wire-draft -p test_reference.py -v`.
