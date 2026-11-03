# Track 49 gRPC snapshot and telemetry synchronization

Status: bounded implementation/transport qualification; root owns integrated
engine, two-host and final hosted acceptance. Public additions are opt-in through
existing Tonic transport; standard IPC decoding/merge requires `telemetry-ipc`.
No dependency version or native accounting capability is introduced here.

## Entity data and application boundary

`send_migration_snapshot` transports a version-2 `GrpcEntityMigrationSnapshot`.
It preserves entity identity/generation, every component generation/payload, and
pending event source/destination, exact u128 tick, sequence, payload length and
anti metadata. Pending work may be local to an LP; only the ownership transfer
itself must change LP. Pending payload bytes and model RNG belong in the portable
application component data: the metadata structure contains lengths, not payloads.

`encode_migration_snapshot` / `decode_migration_snapshot` provide a canonical
versioned Protobuf data payload for an authenticated ordinary optimistic event.
The decoder rejects unsupported versions, unknown/duplicate/reordered fields,
malformed ticks, duplicate component identities and bounded-size violations.
This data codec cannot construct native `World`, Arc/Weak, accounting, ownership
or verified-journal handles. The signed engine/model path must check route,
current entity ownership/generation and complete pending work before applying it.
Rollback must restore source ownership and undo destination/descendant effects.
Whole-LP relocation and membership changes remain a separate draft contract.

The direct Tonic snapshot RPC validates and places data in an **in-memory queue**.
Its ACK is always `applied=false` and explicitly says neither durable nor model
applied. `drain_migration_snapshots` retains version and all fields. Exact retries
for the same `(source LP, migration ID)` are queue-idempotent; conflicting data
returns `AlreadyExists`. Ledger and queue retention are capped at256 records and
are not crash durable. Receipt-based engine application must use the external
optimistic runtime, not this queue ACK.

The older `send_migration` cannot supply component generations or pending work.
It now writes generation0 as **unspecified legacy metadata**, instead of inventing
actual generation1. The old `drain_migrations` is an explicitly limited projection;
new consumers must use full snapshots. No legacy send can claim model application.

## Standard Arrow IPC and deterministic merge

`StreamTelemetryV2` is a real client-streaming HTTP/2 RPC, with version, run ID,
source-scoped batch ID, exact u128 tick range and standard Arrow IPC stream bytes.
The entire bounded stream validates before any batch becomes visible. Conflicts
return `AlreadyExists`; a malformed/truncated final batch publishes no prefix.
Exact duplicate batches do not add queue entries. All accepted IDs remain in the
bounded transport-lifetime ledger after draining; rebind/restart is a new queue,
not a durable restart protocol.

The declared legacy `StreamTelemetry` route is also implemented. It retains its
u64 wire range and rejects wider local ticks instead of narrowing. A nonempty
standard IPC stream supplies the actual run identity, and SHA-256 of canonical
legacy Protobuf bytes supplies a source-scoped content identity. Identityless
empty legacy streams reject; v2 can represent an empty participant batch with an
explicit run/batch identity. Legacy ACKs also never assert model application.

IPC decoding uses the already qualified Track70 Polars standard-stream reader,
not the older custom `ARROW1` writer. Before any untrusted Arrow schema tree reaches Polars, a framing preflight
requires the exact fixed flat-schema metadata emitted by the qualified writer
for an empty schema-v2 event-log frame. Other schemas and dictionary messages
reject before recursive decoding. The preflight also rejects compressed buffers,
oversized row/node/variadic-buffer declarations, buffer ranges outside the actual
body and incomplete streams before the maintained reader allocates from them. The reader then enforces exact schema-v2 field/order/types,
known schema version, run identity and range for every row.

Bounds:128KiB encoded RPC message;64KiB IPC/component aggregate payload;64 items
per stream and512KiB aggregate stream payload;256 retained batch IDs;4096 rows
per IPC stream and merged result;128 expected participants. IPC frames are
limited to128 and field-node/buffer vectors to256, with nonnegative known node
and buffer bounds. This is an explicit canonical-writer, uncompressed schema-v2 profile, not all
possible Arrow schema encodings or compression modes. Equivalent schemas emitted
with different metadata bytes are outside this qualified profile and reject.

`merge_telemetry_batches` rejects unknown participants, mixed runs, conflicting
batch IDs and conflicting source-scoped event IDs. It deduplicates identical
rows and orders by `(tick, priority, sequence, source LP, event index, generation)`.
It returns actual re-encoded standard IPC plus explicit missing sources. Because
the existing event-log schema has no source-LP column, the aligned `row_sources`
vector is part of the result and must accompany IPC rows; no hidden schema change
or implicit loss of distributed source identity is claimed.

## Transport identity and failures

This legacy `TonicTransport` service is for a trusted controlled allocation; its
source membership checks validate configured identities, not TLS authentication.
Production external accounting uses the separate certificate-pinned mTLS signed
record path. Do not feed unauthenticated queue bytes into native state or mistake
a queue acknowledgement for an external admission/retirement proof.

The additive full snapshot/telemetry calls retain actual `tonic::Status` codes,
metadata and details in `GrpcCallFailure`. Unavailable/resource exhaustion is
transient; explicit connect/RPC deadline expiry is partition **suspicion**; other
statuses are permanent for this caller policy, except Unknown with a concrete
typed Tonic transport source and recognized connectivity I/O, which is transient
while retaining the actual Unknown status. Message text alone never changes kind. No timeout proves peer death or
authorizes GVT, ownership takeover or retry. Calls make one attempt. Legacy APIs
retain their existing coarser `TransportError` compatibility surface.

## Qualification and compatibility

Nine all-feature tests exercise actual loopback gRPC: full snapshot metadata and
canonical codec, independent protoc36.2 wire bytes, exact/conflicting retry,
real blackhole deadline vs closed-peer unavailability, standard IPC streaming and
serial merge order, missing participants, invalid-tail atomicity, legacy stream
and checked width. Independent maintained polars-arrow-format0.2.1/planus1.1.1
fixture generation produces valid64-level-list and128-field schema-only streams;
raw RPC rejects both with an explicit pre-decode schema-profile error and no
queue changes, then accepts the valid fixed-schema stream. Seven existing Tonic oracles also pass after correcting their
old false `applied=true` expectation. All50 ordering assertions/deadlines in the
separate legacy socket fixture remain untouched by this leaf.

The new service trait methods have default `Unimplemented` bodies so existing
custom implementations still compile. New clients/routers and `simulation.proto`
are updated together. The old proto's Heartbeat message reference was qualified
to remove method-name shadowing and make the complete schema compile with protoc.
No generator dependency or unsafe Rust was added. Source/native integration and
phase status remain with the root integrator.
