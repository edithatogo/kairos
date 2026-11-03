# API Review: Track49 native records and versioned synchronization

## Problem

The native MPI preview needs fallible signed-record transport for the external
optimistic engine. Legacy gRPC migration drops generations/pending metadata,
and a queue acknowledgement previously claimed application. Distributed
telemetry needs actual standard IPC and explicit source identity.

## Affected root and surface family

`crates/kairo-ecs-mpi` and `crates/kairo-ecs-grpc`: experimental Rust APIs and
Protobuf wire contracts, alpha stage, PR366. Neither root is a protected root
listed in the current protected-surface inventory. No root is renamed/removed;
no C ABI, host binding or maintained Arrow schema changes. This review follows
the applicable fields of `docs/api/api-review-template.md`.

## Proposed API

The [runtime ADR](../../conductor/tracks/49-distributed-mpi-grpc-state-synchronization/adr-runtime-completion-20261103.md)
binds the accepted scope. [Native MPI API](native-mpi-runtime.md) adds bounded
opaque records with actual source-rank identity and unanimous pure validation.
[gRPC synchronization](track49-grpc-sync.md) specifies full canonical v2
snapshots, typed status-preserving failures, real IPC streaming and deterministic
merge. The existing signed external engine remains the sole durable admission,
rollback, retirement and common-cut authority.

## Compatibility matrix

| Surface | Change | Compatibility and transition |
|---|---|---|
| Native MPI Rust | Add record types/methods; preserve preview methods | Additive experimental API; communicator stays neither Send nor Sync |
| gRPC Rust | Add full snapshot, typed telemetry/failure/merge APIs | Additive; opt-in `telemetry-ipc` for IPC; custom service implementations retain default Unimplemented methods |
| Legacy migration | Generation0 means unspecified; queue ACK applied=false | Experimental behavior correction: consumers requiring real generations/pending work must use full snapshots and signed engine application |
| Protobuf | Add v2 routes/frame; qualify Heartbeat type reference | Canonical versioned full snapshots; unknown/noncanonical versions reject; existing legacy routes remain explicitly limited |
| Arrow event log v2 | Existing fields/order/types retained | No schema change; merged IPC requires its aligned row_sources vector |
| C ABI/host APIs | No changes | No consumer transition |

Compatibility classification is **experimental-breaking** for the legacy
migration/ACK correction, additive for new methods. Source acceptance does not
declare these preview APIs beta/stable or authorize a release. Downstream callers
must stop treating an ACK as application; use `drain_migration_snapshots` instead
of the limited legacy projection and retain `row_sources` with merged IPC.
These are the migration/release notes for this source packet; no package is
published or renamed and no version/catalog promotion occurs.

## Memory ownership and thread safety

MPI lifetime is borrowed from caller-owned Universe; all collectives stay on its
compliant thread. Validators must have no model/store/I/O/MPI side effects.
Portable snapshots contain application data, never native World/Arc/Weak or
accounting handles. Tonic queues are bounded, in-memory and nondurable. The
example's typed listener is loopback-only and reached through owned SSH forwards;
engine traffic separately uses bidirectionally pinned mTLS and signatures.

## Error model and determinism/replay

One attempt preserves actual tonic status/details/metadata. Unavailable is
transient and a deadline is partition suspicion; neither proves peer death or
permits takeover/retry. MPI rank death requires whole-allocation termination and
fresh full-membership joins with every reopened store standby. Scalar minima
never authorize reclamation. Entity transfer is an ordinary reversible event
between fixed LPs. Full LP relocation remains a separate draft.

MPI ordering is source-rank/FIFO. Telemetry ordering is
`(tick, priority, sequence, source LP, event index, generation)` with full u128
ticks. Source-scoped identities detect duplicates/conflicts. Missing sources are
returned explicitly. Independent serial models compare complete entity/RNG/
generation/pending state and full logical ancestry keys.

## Conformance fixtures and evidence

`native_mpi_smoke` covers real two/four-rank collective rejection and subsequent
valid exchange, limits and source binding. `tests/track49_sync.rs` covers actual
Tonic RPCs, independent protoc36.2 golden bytes, full metadata, legacy width,
duplicate/conflicting records, actual closed/blackhole endpoints and IPC merge.
Independent valid nested/wide Arrow schemas reject before recursive decoding.
The shared `docs/distributed/track49-fixtures/model.rs` and native controllers
exercise applied work/ACK descendants, delayed rollback, held replacement,
SQL commit-boundary death/restart and finite common cuts. Exact native/hosted
acceptance remains bound in the Track49 qualification report.

## Alternatives and red-team objections

- A legacy PdesTransport adapter cannot express MPI errors or complete cut proof;
  use the fallible signed external driver instead.
- A sender may claim another issuer: check the actual MPI source slot against
  the immutable signature registry before durable ingress.
- One rank may reject a batch: gather every validation result before delivery.
- Native capability import/direct RPC application would escape accounting:
  snapshots remain data and queue ACKs explicitly unapplied/nondurable.
- Arbitrary Arrow trees can exhaust recursive parsers: preflight only the exact
  maintained flat-schema metadata and bounded uncompressed frames before Polars.
  This is a canonical-writer profile, not all Arrow encodings.
- Timeout or missing telemetry cannot authorize membership/GVT: return suspicion
  and missing sources, retain the full-quorum engine gate.

## Decision

Accepted for the bounded experimental source scope after independent reviewer
checks of source identity, cut authority, metadata preservation, IPC preflight
and truthful ACK/failure behavior. Review findings corrected source-slot binding,
pre-decode Arrow schema bounds, loopback listener enforcement and native-controller
cleanup. Final runtime/hosted acceptance is recorded separately; release and
main integration remain outside this decision.

## Native amendment review — 3 November 2026

The independently reviewed private SQLite read snapshot changes no API/wire,
CAS, schema, locking, bounds or durable commit behavior. Every new guard performs
full retained-history validation; every proof still checks exact linkage and
historical digest. A typed connectivity cause may classify an actual Unknown
Tonic status as transient without rewriting its code/details/metadata. Arbitrary
application Unknown remains permanent. Fresh source-bound MPI2/4 and Mac/Colab
matrices qualify these changes; see the Track49 native acceptance record.
