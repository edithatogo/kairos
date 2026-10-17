# Track 49 transport contract, packet 2: versioned envelope, authority epoch, durable admission

Status: contract definition only, dispatched under
`conductor/tracks/29-wave-manager-execution-gatekeeper/adr-track49-phase-entry-20261017.md`
which authorises packet 2 of the four-packet order in
`first-production-leaf-preparation-20261004.md`. This document adds no
dependency, changes no source file, changes no protobuf file, and records no
gate result. `mpi = []` and `grpc = []` remain empty marker features and the
lockfile still carries no `rsmpi`, `tonic` or `prost` entry. Track 48 and Track 49
both remain In Progress.

Scope boundary: this packet freezes a *wire-level definition and its oracles*.
It implements no codec, no decoder, no journal, no socket, no rank exchange, no
service. Packet 3 (codec/conversion leaf) and packet 4 (live gRPC/MPI delivery)
are not authorised by this document.

Workspace `rust-version` is `1.99` (`Cargo.toml:39`). The historical Track 48
MSRV 1.76 evidence describes wrapper-labelled receipts, not the current
workspace floor, and is superseded for dependency selection.

## 1. Additive versioned envelope

### 1.1 Relationship to the existing contracts

The envelope is **additive**. Every existing generic contract is retained
byte-for-byte and semantically unchanged:

- `GrpcContractEnvelope` and `GrpcContractMessage`
  (`crates/kairo-ecs-grpc/src/lib.rs:30`, `:16`) with
  `GRPC_PROTOCOL_ID`/`GRPC_PROTOCOL_VERSION`/`GRPC_SERVICE_NAME`
  (`:10`, `:11`, `:12`).
- `GrpcPeer` (`:148`), `GrpcTransportConfig` (`:155`), `GrpcRetryPolicy` (`:172`),
  `GrpcMigrationRequest` (`:201`), `GrpcTelemetryBatch` (`:227`),
  `WorkerHeartbeat` (`:254`).
- `MpiContractEnvelope` (`:54`) and `MpiContractMessage` (`:21`) with
  `MPI_PROTOCOL_ID`/`MPI_PROTOCOL_VERSION`
  (`crates/kairo-ecs-mpi/src/lib.rs:9`, `:10`), `MpiRankAssignment` (`:14`),
  `MpiMessageTag` with its pinned discriminants `Event=100, Null=101,
  Migration=102, Telemetry=103, AntiMessage=104` (`:245`, `:254`),
  `MpiMigrationRequest` (`:285`), `MpiTelemetryBatch` (`:311`).
- `crates/kairo-ecs-grpc/proto/simulation.proto` in full, including the existing
  `RemoteEvent`, `AntiMessage`, `PendingEvent`, `GvtProposal` and the
  `SimulationTransport` service. No message is renamed, renumbered or removed.

The optimistic envelope defined here travels as an additional body alongside the
existing per-transport contracts. It does not replace `simulation.proto`'s
`RemoteEvent` and does not redefine `AntiMessage.sequence`, which per
`wire-draft/contract.md:27` still cannot identify replay incarnations or full
ancestry.

### 1.2 `KairoOptimisticEnvelopeV1` field set

Encoding is **not** selected by this packet. Protobuf/`tonic`/`prost` selection
belongs to packet 3 with fresh version and MSRV review. The field set below is
the language-neutral logical contract; a decoder must recover exactly these
values and reject everything else.

Mandatory fields (all present, all validated before any membership mutation):

| # | Field | Type | Native counterpart (grounded) |
|---|---|---|---|
| 1 | `envelope_schema_version` | `u32`, value `1` | new; no native analogue |
| 2 | `protocol_id` | ASCII string | must equal `GRPC_PROTOCOL_ID` (`kairo-ecs-grpc/src/lib.rs:10`) or `MPI_PROTOCOL_ID` (`kairo-ecs-mpi/src/lib.rs:9`) per enclosing transport |
| 3 | `protocol_version` | `u16` | `GRPC_PROTOCOL_VERSION` (`:11`) / `MPI_PROTOCOL_VERSION` (`mpi/src/lib.rs:10`), both `1` |
| 4 | `authority_mode` | enum `LocalPreview = 0`, `Scoped = 1` | `OptimisticAuthority` (`crates/kairo-ecs-pdes/src/optimistic.rs:433`) |
| 5 | `simulation_namespace` | `u128`, full width | `OptimisticAuthority::Scoped { simulation_namespace: u128, .. }` (`optimistic.rs:438`) |
| 6 | `ownership_epoch` | `u64`, full width | `OptimisticAuthority::Scoped { ownership_epoch: u64 }` (`optimistic.rs:439`) |
| 7 | `emission_epoch` | `u64`, full width | `OptimisticOwnedOptions::emission_epochs: BTreeMap<LpId, u64>` (`optimistic/owned.rs:15`) |
| 8 | `source_lp` | `u32` | `LpId(pub u32)` (`kairo-ecs-pdes/src/lib.rs:42`), `RemoteEvent.source_lp` (`:58`) |
| 9 | `dest_lp` | `u32` | `RemoteEvent.dest_lp` (`kairo-ecs-pdes/src/lib.rs:60`) |
| 10 | `tick` | `u128`, unsigned, full width | `SimDuration { ticks: u128 }` (`crates/kairo-ecs-types/src/lib.rs:98`, `.ticks()` `:109`); no float, no truncation |
| 11 | `ancestry_frames` | ordered list of `AncestryFrame` | see 1.3 |
| 12 | `incarnation` | `u64`, full width incl. `0` and `u64::MAX` | `OptimisticMessage::incarnation()` (`optimistic.rs:534`) |
| 13 | `kind` | enum `Positive = 0`, `Anti = 1` | `OptimisticMessageKind` (`optimistic.rs:423`) |
| 14 | `event_payload` | opaque bytes, exact | `RemoteEvent.event_payload: Vec<u8>` (`kairo-ecs-pdes/src/lib.rs:61`) |

A payload length, digest or byte count never substitutes for
`event_payload`. A re-encoded value must compare equal byte-for-byte.

Reserved fields: absent, or explicitly zero, in `V1`. A non-zero or present
reserved field is rejected under `R05`.

| Reserved field | Why reserved |
|---|---|
| `signed_evidence` | Requires the security-reviewed signer, trust store and key-rotation decision from `external-accounting-interface-proposal-20261004.md:35-40`. No key material, algorithm or key id is selected here. |
| `accounting_revision` | Receiver-owned volatile counter (`accounting_revision()`, `optimistic.rs:1156`). Per `adr-owned-authority-outbox.md:19` a receiver must never rewrite observed authority from current emission authority, and a revision must not be carried inbound as authority. |
| `sender_runtime_id` | Allocated from the process-local `NEXT_RUNTIME_ID: AtomicU64` (`optimistic.rs:34`, consumed at `:905`), which restarts with the process and can collide across hosts. |
| `sender_recovery_generation` | Hard-coded initial value `0` at `owned.rs:159`. A zero initial generation cannot distinguish two restarts. |
| `payload_schema_id`, `payload_schema_version` | `RemoteEvent` carries raw bytes only (`kairo-ecs-pdes/src/lib.rs:57-62`); no payload schema concept exists in the PDES crate to validate against. Packet 3 must either add the concept with Track 48 review or leave these reserved. |
| `migration_id` | Belongs to the existing migration contracts (`GrpcMigrationRequest.migration_id`, `GrpcContractEnvelope.migration_id`). Migration is out of scope for optimistic delivery and must not be smuggled through this envelope. |
| `gvt_floor`, `cut_id` | Distributed cut authority is a separate protocol (`external-accounting-interface-proposal-20261004.md:123-146`); a per-message GVT field would falsely imply cut authority. |

### 1.3 Structural ancestry encoding

The wire form is a top-down chain, not a recursive blob, so a decoder can bound
its work before recursion:

```
EnvelopeIdentity = (tick:u128, source_lp:u32, chain)
chain            := RootFrame | OutputFrame+
RootFrame        = { root_source_lp: u32, root_sequence: u64 }
OutputFrame      = { parent_tick: u128, parent_source_lp: u32, ordinal: u32 }
```

Reading order is outermost ancestor first. For a root identity, `chain` is the
single `RootFrame` and `depth == 0`. For an output identity, `chain` ends in a
`RootFrame` and each preceding `OutputFrame` is one complete parent ordering
key `(parent_tick, parent_source_lp)` plus the child's `u32` ordinal.

This shape corresponds exactly to the merged codec bridge:

- `LogicalNode::Output { parent: OptimisticEventOrderKey, ordinal: u32 }`
  (`optimistic.rs:308`),
- `LogicalEventId::output_parts() -> Option<(&OptimisticEventOrderKey, u32)>`
  (`optimistic.rs:366`),
- `OptimisticEventOrderKey::try_from_parts(tick, source_lp, logical_id)`
  (`optimistic.rs:395`),
- `OptimisticMessage::try_from_authority_parts(event, logical_id, authority,
  incarnation, kind)` (`optimistic.rs:508`).

Reconstruction obligation: a conforming decoder walks `chain` from the innermost
`RootFrame` outward, constructing each `LogicalEventId` bottom-up with the
existing checked child constructor `LogicalEventId::child(parent, ordinal)`
(`optimistic.rs:325`), and then MUST call
`OptimisticEventOrderKey::try_from_parts(tick, source_lp, logical_id)` and
`OptimisticMessage::try_from_authority_parts(...)` rather than any private or
unchecked path. Per `validate_complete_ancestry` (`optimistic.rs:561`) the
decoder-side predicate is:

- every `parent_tick` chain is strictly increasing toward the leaf
  (`current_tick <= parent.tick` is rejected at `:593` as
  `OutputNotStrictlyFuture`);
- the innermost `root_source_lp` equals the `source_lp` carried by the frame
  that encloses it (`:577` rejects `EnvelopeSourceMismatch`);
- the number of `OutputFrame` entries never exceeds `MAX_ANCESTRY_DEPTH`
  (`:33`, checked at `:335` and `:587`);
- the emitting `source_lp` of an output need **not** equal its parent's or its
  root's `source_lp`, per `adr-codec-bridge.md`.

Authority metadata, incarnation and message kind MUST NOT influence ordering.
`OptimisticEventOrderKey` is `(tick, source_lp, logical_id)` only
(`optimistic.rs:376-380`).

### 1.4 Version negotiation and rejection rule

1. The **only** accepted `envelope_schema_version` in this packet is `1`. Any
   other value is rejected with `R01` before any other field is interpreted and
   before allocation proportional to the payload.
2. Negotiation is **exact-match, first-match, no downgrade**. There is no
   "highest common version" step, no tolerant default field, no ignored
   unknown mandatory field, and no partial-accept path. Version negotiation is
   therefore a two-valued predicate: the receiver's compiled-in supported
   version equals the envelope's `envelope_schema_version`, or the envelope is
   rejected.
3. Within `V1`, the transport-level `protocol_id` and `protocol_version` must
   also match the enclosing transport's constants exactly, mirroring the
   existing `GrpcContractEnvelope::validate`
   (`kairo-ecs-grpc/src/lib.rs:120-143`) which rejects `ProtocolMismatch`
   (`:122`) and `ProtocolVersionMismatch` (`:125`). These two constants are
   retained unchanged by this packet.
4. Reserved fields are not forward-compatible within `V1`. Because this packet
   adds no protobuf file and selects no encoding, "unknown field" handling is
   **deferred to packet 3**; whatever packet 3 freezes must preserve the
   property that a non-zero reserved field is a rejection (`R05`), not a silent
   drop.
5. A `V2` envelope may be defined later. It may add optional fields and MUST NOT
   reinterpret an existing `V1` field, MUST NOT renumber a `V1` field, and MUST
   NOT weaken a `V1` limit. `MpiMessageTag::validate_stable_values`
   (`kairo-ecs-mpi/src/lib.rs:254`) is the existing precedent for pinning
   wire-stable numeric values.
6. `protocol_version` is a transport compatibility input. Per section 2.4 it is
   **not** an authority epoch and MUST NOT enter any cancellation identity,
   logical identity, ordering key or storage key.

## 2. Authority-epoch ownership

### 2.1 Terminology, and what is not the authority epoch

Three distinct "epoch" values exist in the current source and must not be
conflated in the envelope or in any implementation:

| Value | Source | Meaning |
|---|---|---|
| `ownership_epoch` (wire field 6) | `OptimisticAuthority::Scoped { ownership_epoch }` (`optimistic.rs:439`), supplied through `OptimisticOwnedOptions::current_authorities` (`owned.rs:14`) | The durable source-authority term carried by every envelope |
| `emission_epoch` (wire field 7) | `OptimisticOwnedOptions::emission_epochs` (`owned.rs:15`) | The epoch in force for that source LP when the message was emitted; required equal to `ownership_epoch` at construction (`optimistic.rs:1036-1042`) |
| per-LP validity `epoch` | `LogicalProcessState::epoch`, surfaced as `OptimisticStateToken::epoch` (`optimistic.rs:667`, `next_epoch` `:1811`) | A volatile per-LP state-token counter that fails closed with `EpochExhausted` (`:1817`). It is **not** wire authority and MUST NOT be encoded in field 6 or 7 |

### 2.2 Who allocates an epoch

The allocator is the **durable owner of the LP ownership unit**, that is the
participant session that owns the source LP set, acting under the session
configuration and recovery terms described in
`external-accounting-interface-proposal-20261004.md:26-33`. It is not the
transport, not the receiving participant, and not a rank/socket holder.

The rule this packet freezes:

- **A1 Monotonic per ownership unit.** For a fixed
  `(simulation_namespace, source_lp)`, allocated `ownership_epoch` values are
  strictly increasing and are **never reused**, across process restart,
  re-launch, re-partition and migration.
- **A2 Persist before use.** The newly allocated epoch MUST be durably committed
  before the owner activates and before it admits or emits any traffic bearing
  it. In-memory increment is insufficient (`wire-draft/contract.md:13`).
- **A3 Namespace-scoped.** Epoch uniqueness is scoped to
  `simulation_namespace` (a `u128`, `optimistic.rs:438`). Two namespaces may
  independently use the same numeric epoch; the namespace is therefore a
  mandatory part of every cancellation identity.
- **A4 No wrap, fail closed.** At `u64::MAX` allocation fails closed. It never
  wraps and never reuses a saturated value. The repository's existing
  exhaustion posture is `OptimisticError::EpochExhausted(LpId)`
  (`optimistic.rs:184`), raised by `next_epoch` at `:1817`.
- **A5 Zero is a value.** `0` is a legal epoch. Nothing in the current public API
  distinguishes "unallocated" from "allocated 0": `OptimisticOwnedOptions` has
  no `Default` impl (`owned.rs:11`) and `new_owned` validates only that the map
  key sets and namespace match (`optimistic.rs:1000-1043`). Packet 3 MUST NOT
  therefore treat an absent, zero or omitted epoch field as a wildcard.
- **A6 Exhaustion is a hard stop.** An owner that cannot allocate refuses to
  activate; it does not continue under a reused epoch.

**Implemented status today: none of A1-A6 has an implementation in this
repository.** Epoch values are caller-supplied configuration at
`OptimisticRuntime::new_owned` (`optimistic.rs:956`) via
`OptimisticOwnedOptions.current_authorities` and `.emission_epochs`
(`owned.rs:14`, `:15`). There is no allocator, no journal, no checkpoint and no
persistence anywhere in `crates/kairo-ecs-pdes`, `crates/kairo-ecs-mpi` or
`crates/kairo-ecs-grpc`. A1-A6 are the frozen obligation for a later leaf, not
a description of current behaviour.

### 2.3 Fencing on restart

- **F1.** A restarted owner of an LP unit MUST obtain a strictly greater epoch
  than every epoch it previously activated for that unit, and MUST satisfy A2.
- **F2.** Once the new epoch is active, inbound traffic for the same
  `(simulation_namespace, source_lp)` carrying the fenced older epoch is
  rejected with `R08` before membership mutation. Fencing is by epoch value,
  not by liveness flag.
- **F3.** Old-epoch positives and antis retain their exact namespace and epoch
  and MUST NOT cancel a new-epoch incarnation of the same logical identity
  (`wire-draft/contract.md:11`, `:13`).
- **F4.** Retained historical retries under an older epoch are permitted only
  where they are bound to their original authorized durable send/retirement
  records under the then-current fencing policy (`adr-owned-authority-outbox.md:33`).
  A never-before-seen stale-epoch positive is NOT a historical retry and is
  rejected; blanket rejection of all old epochs is equally forbidden.
- **F5.** Migration fences the old owner durably before the new owner activates
  the transferred ownership unit (`adr-owned-authority-outbox.md:33`,
  `external-accounting-interface-proposal-20261004.md:108`). Fresh handles may
  not revive pre-recovery validity tokens.

What the runtime does and does not enforce today:

- It does reject a Scoped envelope at an all-local runtime with
  `OptimisticError::ScopedAuthorityRequiresOwnedRuntime` (`optimistic.rs:120`,
  raised at `:1211` via `receive`, `:1203`), and rejects `LocalPreview` traffic
  in owned mode with `AuthorityModeMismatch` (`:200`, `:1207`).
- It does compare an inbound native message's authority against the registered
  sender issuer's recorded `current_authorities` for that source LP and rejects
  with `NativeAuthorityMismatch { source_lp, expected, actual }`
  (`optimistic/owned_routing.rs:429-440`). That is the real, existing epoch
  check, and it works only for a peer registered **in this process** via
  `register_native_peer` (`optimistic.rs:1117`).
- It does reject an unregistered or dead issuer with
  `UnregisteredNativeIssuer` (`owned_routing.rs:421`) and
  `StaleNativeAccountingAuthority { runtime_id, recovery_generation }`
  (`owned.rs:208-218`, `with_live_authorities` `:419-454`).
- It does **not** re-check `emission_epoch` on the inbound path.
  `OptimisticError::EmissionEpochMismatch` (`:206`) is produced only at
  construction (`optimistic.rs:1036-1042`). A cross-process receiver therefore
  cannot validate wire field 7 against any current native API, and this packet
  records `R09` as a required oracle whose runtime support is **absent**. Packet
  3 must not present `R09` as enforced until a reviewed mechanism exists.

### 2.4 Why rank id, LP id and protocol version cannot substitute for an authority epoch

**Rank id cannot fence.** `MpiRankAssignment { rank: i32, lp_id: LpId }`
(`kairo-ecs-mpi/src/lib.rs:14`) and `MpiContractEnvelope.source_rank` /
`.destination_rank` (`:58`, `:59`) are launcher and routing metadata inside one
`mpiexec` invocation. A rank is recycled when the process restarts, may be
remapped to a different LP set, and `validate_rank_assignments`
(`:552`) is a shape check only. The external-accounting proposal already states
the rule: "A rank or socket address is routing metadata, never an issuer
witness" (`external-accounting-interface-proposal-20261004.md:33`). The same
argument applies to `GrpcPeer { lp_id, endpoint }`
(`kairo-ecs-grpc/src/lib.rs:148`): an endpoint is re-bindable.

**LP id cannot fence.** `LpId(pub u32)` (`kairo-ecs-pdes/src/lib.rs:42`) is a
topology identity, not a temporal one. It is unchanged by restart and reused
across sessions. Worse for the optimistic path, every candidate temporal value
that hangs off the LP id restarts from zero:

- `LogicalEventId::root(source_lp, sequence)` takes the sequence from the caller
  (`optimistic.rs:317`);
- `new_owned` seeds `next_incarnation` with `Some(0)` for every owned LP
  (`optimistic.rs:1095`), and the incarnation allocator is a checked in-memory
  counter (`observe_incarnation` `:1742`, `IncarnationExhausted` `:1737`);
- `LogicalProcessState.epoch` starts at `0` (`optimistic.rs:934`, `:1073`) and is
  volatile.

So after a restart the same `(source_lp, sequence)`, the same incarnation and
the same per-LP epoch can all legitimately recur. LP id plus any of these
reproduces a previous cancellation identity, which is exactly the collision the
codec bridge ADR flags: "a transport must not merely strip an outer epoch and
admit colliding old/new epochs into this native namespace"
(`adr-codec-bridge.md`).

**Protocol version cannot fence.** `GRPC_PROTOCOL_VERSION` and
`MPI_PROTOCOL_VERSION` are `u16` constants equal to `1`
(`kairo-ecs-grpc/src/lib.rs:11`, `kairo-ecs-mpi/src/lib.rs:10`) and are pinned as
stable by `MpiMessageTag::validate_stable_values`
(`kairo-ecs-mpi/src/lib.rs:254`). They are identical for every session running
the same build, change only when the code changes, and never change on restart.
A constant identical across the sessions it must distinguish fences nothing.

**Incarnation and arrival order cannot fence either.** Incarnation is a
delivery-retry counter, not an ownership term, and "Numerically greatest
epoch/incarnation and arrival order are not winner rules"
(`adr-owned-authority-outbox.md:25`). `runtime_id` cannot substitute either: it is
process-local (`optimistic.rs:34`) and `recovery_generation` is initialised to
`0` (`owned.rs:159`).

## 3. The durable-admission boundary

### 3.1 Definition

A positive or anti message is **accounted as durably admitted at receiver R**
if and only if all of the following hold, evaluated after validation and before
any receipt is produced:

- **D1 Validated.** Every mandatory field of section 1.2 is present, every
  reserved field is absent, `envelope_schema_version == 1`, and no rejection
  oracle of section 5 fired. Rejection leaves queue, tombstone, known-delivery,
  history, outbox, receipt and accounting state unchanged.
- **D2 Identified.** The full structural identity
  `(simulation_namespace, ownership_epoch, source_lp, complete ancestry chain,
  root sequence or ordinal, incarnation, kind)` plus the exact delivery metadata
  `(tick, dest_lp, event_payload)` is bound as one key. This matches the native
  send identity `(source_lp, authority, logical_id, incarnation, kind)`
  (`NativeSendId`, `owned_routing.rs:19-25`, exposed as `OptimisticSendKey`
  getters `:47-65`).
- **D3 Durably committed as accounted membership.** R has durably committed a
  record placing the delivery into its accounted queue or its accounted
  tombstone state, under the receiver's own durable ownership and fencing terms.
  A record only staged in memory, only written to a socket, or only present in a
  WAL entry not yet committed does not satisfy D3.
- **D4 Accounting revision committed.** The accounting revision that includes
  this obligation is durably committed. The revision is receiver-owned
  (`accounting_revision()`, `optimistic.rs:1156`) and MUST NOT be carried inbound
  as authority (reserved field, section 1.2).
- **D5 Receipt only afterwards.** Only after D1-D4 may R emit a receipt binding
  the entire envelope key and digest (`external-accounting-interface-proposal-20261004.md:51`).
  Crash before the receipt must let a retry read back the same committed fact
  (`external-accounting-interface-proposal-20261004.md:115`).

A positive admission receipt discharges only the matching positive send and
retains rollback history; it cannot discharge an anti. An anti retirement
receipt is strictly stronger than a positive admission receipt
(`adr-owned-authority-outbox.md:37`).

### 3.2 What is explicitly NOT admission

The following are transport events, not receipt states, and none of them may be
recorded, logged or reported as accounted durable admission:

| Observation | Why it is not admission |
|---|---|
| `send()`/`write()` returned success; bytes flushed to a socket or MPI buffer | A write completes against a peer socket that may not have a running receiver, and says nothing about any commit on R. `wire-draft/contract.md:21`: "A socket write, RPC response without admission, or cancellation request is insufficient." |
| An RPC returned OK / `GrpcMigrationAck.applied`-style response | The existing acknowledgement shapes (`crates/kairo-ecs-grpc/proto/simulation.proto:39`, `HeartbeatAck` `:74`) carry no admission fact and are not retrofitted by this packet. |
| A heartbeat, a null message, or an `MpiMessageTag::Null` (`kairo-ecs-mpi/src/lib.rs:246`) | Neither carries delivery identity nor admission state. |
| Native `admit_native` returned a `NativeAdmissionCapability` (`optimistic/owned_routing.rs:380`, capability getters `:173-197`) | This is **process-local volatile** accounting evidence. Per `adr-owned-authority-outbox.md:31`, "The native local seam may issue an opaque receiver-bound capability after actual cancellation; it cannot label that capability durable." |
| `acknowledge_native_admission` succeeded (`owned_routing.rs:593`) | That closes a sender obligation using a volatile capability; it is downstream of, not evidence for, durability. |
| A caller-supplied success boolean or an arbitrary verifier callback | Explicitly disallowed by `external-accounting-interface-proposal-20261004.md:63-65`. |
| A gRPC or MPI handshake, peer registration or `mpiexec` launch success | Establishes reachability only. |

Native volatile accounting evidence and durable transport evidence remain two
separate types and two separate claims (`adr-owned-authority-outbox.md:31`).

### 3.3 Ownership of the durable boundary

Per `external-accounting-interface-proposal-20261004.md:52-65`, a fixed audited
`ExternalAccountingDriver` privately issues the verified handles and owns
canonical decoding, authentication, configuration/fence validation and durable
journal access. Transport crates have no native proof-minting access, and a
model codec cannot certify admission, retirement, dependency closure or a
committed cut. That driver does not exist in this repository; this packet names
the boundary and does not implement it.

### 3.4 Unresolved sends and GVT

Every positive, anti, blocked replacement and retirement obligation stays
accounted and constrains the local GVT floor to the minimum relevant tick until
its receipt state is reached. On send failure, timeout or crash the obligation is
retained; no timeout silently clears an in-flight send
(`wire-draft/contract.md:21`). The floor is reversible at equality, and no
local receipt, revision or independently sampled minimum establishes a
consistent distributed cut.

## 4. Frozen limits

Concrete values, and where each comes from.

| Limit | Value | Provenance and rationale |
|---|---|---|
| `MAX_ANCESTRY_DEPTH` | `128` output frames | Non-negotiable. Equals `MAX_CAUSAL_DEPTH` (`optimistic.rs:33`); `validate_limits` rejects any runtime whose `max_causal_depth` exceeds it (`:2326`) and the child constructor caps at it (`:335`). A wire envelope above 128 cannot be represented by the runtime, so a higher transport limit would only create envelopes that must always be rejected. |
| `MAX_ANCESTRY_NODES` | `129` (128 output frames plus 1 root) | Retained from the fixture profile, which uses depth 128 valid / 129 rejected and 129 ancestry nodes (`wire-draft/contract.md:9`, `:17`). |
| `MAX_PAYLOAD_BYTES` | `4_096` | Retained from the fixture profile (`wire-draft/contract.md:17`). It is a fixture bound, not a decoder-safety proof; a real decoder still needs byte preflight before parse plus its own adversarial evidence. Raising it is a reviewed change, not a tuning knob. |
| `MAX_ENCODED_ENVELOPE_BYTES` | `65_536` | Retained from the fixture profile's canonical-body limit (`wire-draft/contract.md:17`). For a binary encoding this bound applies to the complete encoded envelope including the payload. Enforced by length preflight before allocation. |
| `MAX_UNRESOLVED_SENDS_PER_CHANNEL` | `4_096` | **New transport-owned constant introduced by this packet**; no repository value exists. Rationale: it must be less than or equal to the runtime's own outbox budget `OptimisticOwnedOptions::max_outbox_entries` (`owned.rs:18`), which must be nonzero (`optimistic.rs:963-969`) and is enforced natively as `OutboxLimitExceeded` (`:236`). The transport may lower this value per deployment but may never raise it above `max_outbox_entries`. The constant is a placeholder pending the dual PDES/distributed review required by `adr-track49-phase-entry-20261017.md` (condition 4, "Dual review"). |
| `MAX_RETAINED_RECEIPTS` | bounded by `OptimisticOwnedOptions::max_receipt_entries` (`owned.rs:20`), nonzero required (`optimistic.rs:963-969`) | Not a free constant: the transport's retained-receipt table must never exceed the runtime's own receipt budget, which fails closed as `ReceiptLimitExceeded` (`optimistic.rs:239`). |
| `MAX_IDEMPOTENCY_WINDOW_ATTEMPTS` | `8` attempts | **New transport-owned constant introduced by this packet**; no repository value exists. The idempotency window is what bounds replay safety: a retry must be answerable from the receiver's retained receipt for as long as the sender may retry. `8` exceeds any retry count a session would plausibly use and stays inside `GrpcRetryPolicy.max_attempts` (`kairo-ecs-grpc/src/lib.rs:173`, validated nonzero at `:181`). Beyond the window a retry is not guaranteed idempotent and MUST be treated as a new obligation requiring fresh admission, never as a silent success. |

Two limits are deliberately **not** frozen in this packet, and the reasons are
recorded rather than filled with invented numbers:

- **No wall-clock idempotency window (seconds).** Freezing one requires a durable
  monotonic clock and a durable receipt journal. Neither exists in this
  repository. Any seconds-valued window written here would be unverifiable, so
  the window is counted in attempts and bounded by receipt retention.
- **No global send or byte rate limit.** Rate limiting depends on the selected
  transport and on deployment topology, which belong to packets 3 and 4.

A bound exhausted anywhere rejects with the matching code in section 5. It never
wraps, silently drops, or unbinds work
(`adr-owned-authority-outbox.md:39`).

## 5. Rejection oracles

Each oracle is a stable code with a testable predicate. All are evaluated after
byte preflight and **before** any membership mutation. "No mutation" means queue,
tombstone, known-delivery, history, replay, outbox, receipt, retirement,
accounting revision and per-LP validity epoch are all unchanged, and the native
runtime's own report is unchanged. Where a native error already exists, the code
names it; the native error is the in-process outcome, not a substitute for the
transport rejection record.

| Code | Testable predicate | Native counterpart |
|---|---|---|
| `R01 UNSUPPORTED_ENVELOPE_VERSION` | `envelope_schema_version != 1` | none |
| `R02 PROTOCOL_ID_MISMATCH` | `envelope.protocol_id != enclosing transport constant` | `ProtocolValidationError::ProtocolMismatch` (`kairo-ecs-grpc/src/lib.rs:122`) |
| `R03 PROTOCOL_VERSION_MISMATCH` | `envelope.protocol_version != enclosing transport constant` | `ProtocolVersionMismatch` (`:125`), `validate_stable_values` (`mpi/src/lib.rs:254`) |
| `R04 ENVELOPE_TOO_LARGE` | encoded length > `65_536` or `len(event_payload) > 4_096` or `len(chain) > 129`, detected before allocation | none |
| `R05 RESERVED_FIELD_PRESENT` | any reserved field of section 1.2 is present and non-zero | none |
| `R06 NON_MINIMAL_OR_DUPLICATE_FIELD` | a repeated field, or a non-minimal integer encoding, where the packet-3 codec forbids it | none; encoding deferred to packet 3 |
| `R07 AUTHORITY_MODE_MISMATCH` | `authority_mode == LocalPreview` in owned mode, or `Scoped` at an all-local runtime | `AuthorityModeMismatch` (`optimistic.rs:200`), `ScopedAuthorityRequiresOwnedRuntime` (`:120`) |
| `R08 STALE_AUTHORITY_EPOCH` | `simulation_namespace` matches but `ownership_epoch != receiver's current epoch for source_lp` | `NativeAuthorityMismatch { source_lp, expected, actual }` (`owned_routing.rs:435`) |
| `R09 EMISSION_EPOCH_MISMATCH` | `emission_epoch != the emitter's current emission epoch for source_lp` | `EmissionEpochMismatch` (`optimistic.rs:206`) exists but is raised only at construction (`:1036-1042`); **no inbound runtime check exists** |
| `R10 FOREIGN_SIMULATION_NAMESPACE` | `simulation_namespace != receiver-configured namespace` | `AuthorityNamespaceMismatch` (`:201`), enforced at construction (`:1010-1019`) |
| `R11 UNREGISTERED_OR_FENCED_SOURCE` | the source runtime is unregistered, sealed-out, or its gate is inactive | `UnregisteredNativeIssuer` (`owned_routing.rs:421`), `StaleNativeAccountingAuthority` (`owned.rs:211`) |
| `R12 SOURCE_NOT_OWNED_BY_ISSUER` | the registered issuer for `source_lp` does not own `source_lp` | `NativeSendIssuerMismatch` (`owned_routing.rs:427`) |
| `R13 ANCESTRY_DEPTH_EXCEEDED` | number of `OutputFrame` entries > `128` | `CausalDepthExceeded { depth, limit }` (`optimistic.rs:336`, `:588`) |
| `R14 ROOT_SOURCE_MISMATCH` | innermost `root_source_lp != source_lp` of the enclosing frame | `EnvelopeSourceMismatch` (`optimistic.rs:578`, rechecked for natives at `owned_routing.rs:483`) |
| `R15 OUTPUT_TICK_NOT_STRICTLY_FUTURE` | any frame whose `parent_tick` is greater than or equal to its child tick in the chain | `OutputNotStrictlyFuture { input_tick, output_tick }` (`optimistic.rs:594`) |
| `R16 PRE_GVT_DELIVERY` | `tick < receiver's current GVT` | `EventBeforeGvt` (`optimistic.rs:126`) |
| `R17 ROUTE_MISSING` | `source_lp != dest_lp` and `dest_lp` is not in the global topology destinations of `source_lp` | `RouteMissing` (`owned_routing.rs:472`, also `:121`) |
| `R18 UNKNOWN_DESTINATION_LP` | `dest_lp` is not in the global partition | `UnknownLogicalProcess` (`owned_routing.rs:454`) |
| `R19 UNOWNED_DESTINATION_LP` | `dest_lp` is not owned by the receiving issuer | `UnownedLogicalProcess` (`owned_routing.rs:456`) |
| `R20 SEND_BUDGET_EXHAUSTED` | unresolved sends on this channel == `4_096` | `OutboxLimitExceeded { limit }` (`optimistic.rs:236`) |
| `R21 RECEIPT_BUDGET_EXHAUSTED` | retained receipts == `max_receipt_entries` | `ReceiptLimitExceeded { limit }` (`optimistic.rs:239`) |
| `R22 DUPLICATE_POSITIVE` | identity already present as pending or executed positive with identical metadata | `DuplicatePositive { source_lp, incarnation }` (`optimistic.rs:142`, raised `:1244`, `:1261`) |
| `R23 CONFLICTING_METADATA` | same `(source_lp, authority, logical_id, incarnation, kind)` with a different `tick`, `dest_lp` or `event_payload` | `ConflictingDelivery` (`optimistic.rs:146`, raised `:1225`, `:1250`, `:1267`) |
| `R24 ANTI_PREDECESSOR_BINDING_INVALID` | `kind == Anti` and the sender's retirement record's predecessor anti is not byte-identical to this envelope | `ConflictingNativeReceipt` (`owned_routing.rs:388`, `:390`) |
| `R25 NATIVE_ADMISSION_CAPABILITY_REQUIRED` | a Scoped envelope reaches raw `OptimisticRuntime::receive` in owned mode instead of the native admission path | `VerifiedNativeAdmissionRequired` (`optimistic.rs:250`, raised `:1211`) |
| `R26 NATIVE_PEERS_NOT_SEALED` | a delivery channel is opened before `seal_native_peers()` (`optimistic.rs:1128`) | `NativePeersNotSealed` (`optimistic.rs:212`, raised `owned_routing.rs:348`, `:403`) |
| `R27 UNKNOWN_NATIVE_SEND` | the identity does not correspond to a recorded outbound send obligation | `UnknownNativeSend` (`owned_routing.rs:443`) |
| `R28 INCARNATION_EXHAUSTED` | the emitter's incarnation counter is saturated | `IncarnationExhausted(LpId)` (`optimistic.rs:1737`) |
| `R29 AUTHORITY_EPOCH_EXHAUSTED` | the emitter cannot allocate a further epoch | `EpochExhausted(LpId)` (`optimistic.rs:1817`) |

### 5.1 Receipt states

A receipt state is a durable fact at the receiver plus, where stated, a closed
sender obligation. Each is a testable predicate. `SENT_TO_SOCKET` and
`RPC_RESPONSE_RECEIVED` are transport events, deliberately excluded from this
list, and MUST NOT be mapped onto any `RS` state.

| State | Testable predicate |
|---|---|
| `RS_OPEN` | An outbound obligation exists for the exact identity `D2`; no durable admission fact for it exists at the receiver; the obligation constrains the sender's GVT floor to at most its tick. |
| `RS_ADMITTED_PENDING_ACK` | Receiver conditions D1-D4 hold; the admission fact is durable; the sender obligation is still open. Reached from `RS_OPEN` only; never reached from a socket write or RPC response. |
| `RS_ADMISSION_ACK_CONSUMED` | The sender holds a receipt binding the exact envelope key and digest, and closed exactly the matching positive obligation. Rollback history is retained. No anti obligation was discharged. |
| `RS_ADMISSION_REPLAYED_IDEMPOTENT` | A retry of an identity already in `RS_ADMITTED_PENDING_ACK`/`RS_ADMISSION_ACK_CONSUMED` returns the same durable fact, same revision, and allocates no second admission record and no second revision. The pending/executed-positive duplicate rejection of `R22` counts as a receipt only when the receiver can prove that admission. |
| `RS_REJECTED_NO_ADMISSION` | A code in section 5 fired; no membership mutation occurred; the sender obligation remains open and accounted until separately discharged or abandoned by policy. |
| `RS_UNRESOLVED_ON_FAILURE` | Send failed, timed out, or either side restarted before a receipt. The obligation is retained unchanged, identity and metadata preserved, and still constrains the GVT floor. |
| `RS_RETIREMENT_ADMITTED_PENDING` | The anti is admitted under D1-D4 but its cancellation effect is not yet applied. The anti obligation stays accounted. |
| `RS_RETIREMENT_APPLIED` | The exact cancellation-applied effect is durably recorded together with retained tombstone protection and all induced descendant antis accounted. This is stronger than a positive admission ACK and is the only state that permits sender release (`adr-owned-authority-outbox.md:29`, `:37`). |
| `RS_REPLACEMENT_BLOCKED` | A reserved successor remains blocked, accounted and unpublishable until `RS_RETIREMENT_APPLIED` is proven for its exact predecessor. Crash before the sender release commit leaves the successor blocked (`external-accounting-interface-proposal-20261004.md:119`). |

Cancellation-applied means old local effects were removed or never applied, a
delayed old positive cannot reapply, and every induced downstream anti remains
accounted. It does not mean the descendant graph has drained
(`adr-owned-authority-outbox.md:29`).

## 6. Explicit non-claims

This contract does **not** prove, establish, imply or partially establish any of
the following:

1. **Durable restart or recovery.** No journal, WAL, checkpoint, transaction,
   crash-boundary oracle or atomicity guarantee is implemented or tested here.
   Conditions A1-A6, F1-F5 and D3-D5 are frozen obligations with no
   implementation in this repository.
2. **Authenticated external accounting.** No signing key, algorithm, key id,
   trust store, rotation policy, mutual TLS or canonical-record digest is
   selected or implemented. `signed_evidence` is a reserved field. Signature
   verification remains gated on explicit security review
   (`external-accounting-interface-proposal-20261004.md:35-40`).
3. **MPI or gRPC acceptance.** No rank was launched, no socket was bound, no RPC
   was issued and no service was started. `kairo-ecs-mpi` and `kairo-ecs-grpc`
   remain dependency-free placeholders with `mpi = []` and `grpc = []`. The
   `multirank_smoke` and `two_process_smoke` targets named in Track 49's
   `test-matrix.md` do not exist; their planned commands must never be recorded
   as executed.
4. **Codec correctness.** No encoder or decoder exists. `R01`-`R29` are
   predicates for a future implementation. No roundtrip, fuzz or adversarial
   evidence is claimed.
5. **Migration.** No ownership-unit transfer, no component or pending-event
   migration, and no migration atomicity. Migration requests remain in the
   existing Track 35/49 placeholders.
6. **Distributed GVT or global quiescence.** No cut, floor or consistent
   distributed view is claimed. The `GvtProposal`/`GvtDecision` and
   `GvtProposal.local_min_tick` shapes remain insufficient on their own
   (`wire-draft/contract.md:27`); an MPI all-reduce minimum does not authorise
   collection (`external-accounting-interface-proposal-20261004.md:138-139`).
7. **Rolling back or replacing remote effects.** A sender retirement barrier and
   verified cancellation-applied evidence are specified as an oracle, not
   demonstrated. No native or remote replacement path was exercised.
8. **Production protobuf acceptance.** No `.proto` file was added or modified.
   `crates/kairo-ecs-grpc/proto/simulation.proto` is unchanged.
9. **Any gate, review or merge.** No validator was run to a pass by this packet,
   no `$conductor-review` disposition is recorded, no status file was touched,
   and no checkbox, phase, wave, dependency or track status was changed.

Track 48 (`48-time-warp-optimistic-rollback-runtime`) remains **In Progress**
pending live distributed acceptance. Track 49
(`49-distributed-mpi-grpc-state-synchronization`) remains **In Progress**; the
raw `Track49 -> Track48 not Done` dependency-gate failure is unchanged and is
not rewritten as a pass.

## 7. Grounding and verification status

### 7.1 Verified against source at `2e346b54c42f5a80f4f4890075d6ccc180423170`

- `LpId(pub u32)`, `RemoteEvent{source_lp,dest_lp,tick,event_payload}` —
  `crates/kairo-ecs-pdes/src/lib.rs:42`, `:57-62`.
- `SimDuration { ticks: u128 }`, `.ticks()` — `crates/kairo-ecs-types/src/lib.rs:98`, `:109`.
- `MAX_CAUSAL_DEPTH = 128`; `OptimisticLimits.max_causal_depth`; `validate_limits`
  — `crates/kairo-ecs-pdes/src/optimistic.rs:33`, `:85`, `:2318-2331`.
- `OptimisticLimits` defaults (1_024 LPs, 100_000 pending/history/tombstones,
  10_000 output batch) — `optimistic.rs:88-99`.
- `LogicalEventId::root` / `child` / `depth` / `root_parts` / `output_parts` —
  `optimistic.rs:317`, `:325`, `:347`, `:354`, `:366`.
- `LogicalNode::{Root{source_lp,sequence}, Output{parent,ordinal}}` — `:303-312`.
- `OptimisticEventOrderKey::try_from_parts` and its three getters — `:395`, `:408`,
  `:412`, `:416`.
- `OptimisticMessageKind::{Positive,Anti}` — `:423`.
- `OptimisticAuthority::{LocalPreview, Scoped{simulation_namespace: u128,
  ownership_epoch: u64}}` — `:433-441`.
- `OptimisticMessage::try_from_authority_parts`, `event`, `incarnation`,
  `authority`, `kind`, `order_key`, `as_anti` — `:508`, `:526`, `:534`, `:538`,
  `:542`, `:546`, `:551`.
- `validate_complete_ancestry` rejection points — `:561-605`.
- `OptimisticRuntime::receive` authority gate and all listed duplicate/conflict
  outcomes — `:1203`, `:1207`, `:1211`, `:1225`, `:1244`, `:1261`.
- `new_owned` namespace, epoch-set and emission-epoch equality validation —
  `:956`, `:1000-1043`.
- `register_native_peer` / `seal_native_peers` / `accounting_revision` — `:1117`,
  `:1128`, `:1156`.
- `next_epoch` / `EpochExhausted`; `observe_incarnation` / `IncarnationExhausted` —
  `:1811`, `:1817`, `:1737`, `:1742`.
- `NEXT_RUNTIME_ID` process-local allocation — `:34`, `:905`.
- `OptimisticOwnedOptions` fields (no `Default`) — `optimistic/owned.rs:11-21`.
- `NativeAccountingAuthority` getters, `recovery_generation: 0` initial value,
  `validate_live_peers`, `with_live_authorities` — `owned.rs:69-107`, `:159`,
  `:208-218`, `:419-454`.
- `NativeSendId` / `OptimisticSendKey` — `optimistic/owned_routing.rs:19-25`,
  `:42-66`.
- `NativeOutboundSend` / `NativeAdmissionCapability` getters, `ready_native_sends`,
  `admit_native` (anti dispatch to retirement), inbound
  `NativeAuthorityMismatch` / `UnregisteredNativeIssuer` /
  `NativeSendIssuerMismatch` / `UnknownNativeSend` / `RouteMissing` /
  `UnknownLogicalProcess` / `UnownedLogicalProcess` / `EnvelopeSourceMismatch`,
  `acknowledge_native_admission` — `owned_routing.rs:112-135`, `:145-197`, `:339`,
  `:380-393`, `:421-489`, `:593`.
- `native_intents`, `pending_native_retirement_requests`,
  `applied_native_retirements`, `receive_native_retirement`,
  `acknowledge_native_retirement`, `fossil_collect_native_group` — `optimistic/
  owned_execution.rs:564`, `:599`, `:622`, `:644`, `:868`, `:1090`.
- gRPC placeholder constants, envelope, peers, config, retry policy, migration,
  telemetry, heartbeat classification and validation errors —
  `crates/kairo-ecs-grpc/src/lib.rs:10-12`, `:16-39`, `:120-143`, `:148`, `:155`,
  `:172-190`, `:194-223`, `:227-244`, `:247-267`, `:497-515`.
- MPI placeholder constants, envelope, rank assignment, message tags with pinned
  discriminants and their stability check, migration, telemetry, validation
  errors — `crates/kairo-ecs-mpi/src/lib.rs:9-10`, `:14-17`, `:21-50`, `:54-67`,
  `:245-274`, `:285-289`, `:311`, `:532-550`.
- `crates/kairo-ecs-grpc/proto/simulation.proto` message and service inventory —
  `:6-11`, `:14-43`, `:45-85`, `:89-96`.
- Workspace `rust-version = "1.99"`; empty `mpi = []` and `grpc = []` features —
  `Cargo.toml:39`, `crates/kairo-ecs-mpi/Cargo.toml`, `crates/kairo-ecs-grpc/Cargo.toml`.

### 7.2 Explicitly unverified in this packet

These are stated rather than invented:

1. **No wire encoding is selected.** Field numbers, wire types, varint rules,
   duplicate-field tolerance and unknown-field policy are deferred to packet 3.
   `R06` is therefore scoped but not implementable as written.
2. **`R09 EMISSION_EPOCH_MISMATCH` has no runtime support.** `EmissionEpochMismatch`
   is produced only at construction. A cross-process receiver has no current
   public API with which to validate wire field 7.
3. **Epoch allocation, persistence and fencing are unimplemented.** There is no
   allocator, no journal, no durable clock, no fencing store and no restart test
   anywhere in the repository. Rules A1-A6 and F1-F5 are obligations only.
4. **`MAX_UNRESOLVED_SENDS_PER_CHANNEL` and `MAX_IDEMPOTENCY_WINDOW_ATTEMPTS`
   are new constants with no repository precedent.** They require the dual
   PDES/distributed review mandated by `adr-track49-phase-entry-20261017.md:77-78`
   before they are treated as frozen.
5. **No canonical-record digest is defined.** Section 3 requires receipts to bind
   the envelope key and digest, but the digest function, domain separation and
   byte canonicalisation belong to the security-reviewed boundary and are not
   selected here.
6. **No packet 3 or packet 4 work is authorised or performed** by this document,
   and the codec, service and integration prerequisites named in `plan.md`
   remain unmet.
7. **No validator, test, build or review was run** for this packet. Nothing here
   is execution evidence.

## 8. Conditions carried forward

Unchanged and still mandatory for later packets, per
`adr-track49-phase-entry-20261017.md:60-81`: supply-chain and advisory review
(`SECURITY.md`, `cargo-deny`, `cargo-audit`, Track 20 review) before any
dependency or CI enablement; MSRV review against workspace `rust-version` 1.99;
real evidence; dual PDES and distributed review before any interface freeze;
normal exact-head push, hosted checks and merge. EXC-193 is PR #193-only and
EXC-199 is an unapproved proposal; neither extends to this packet or any later
PR.