# Proposed external accounting interface for Track48/49

4 October 2026. Experimental, proposed joint PDES/distributed/security decision.
This is preparation, not an approved transport dispatch or a completion record.
Native candidate b7bb3e7b6ed7566ef6a7970e3d2ca9f940548687 integrates reviewed
source9e2ca142d48d7f86bcc03bf1239fc2367de47c5c. The current 148-test crate lane
passes on explicitly bound Rust1.98.1 and1.76.0; the independent execution
supplement, full closeout, exact-head hosted checks and merge remain pending.

## Decision proposed

Add a distinct externally authenticated accounting mode. Keep native capability
construction and verification process-local. Native Weak issuer witnesses,
pointer equality, process-local runtime IDs and a zero initial recovery
generation cannot establish another process's authority. Full structural codec
roundtrips authenticate no issuer. The native group cut borrows the actual
complete runtime set and cannot become distributed collection by serializing
its result.

Freeze the external ownership, canonical record, durable transaction and cut
contracts below before dispatching either engine or transport implementation.
Resolve and pin any new cryptography, serialization, TLS and storage dependencies
through their owners; this document selects no unreviewed package version.

## Ownership and authentication

Each session configuration binds a provisioned simulation UUID, immutable
participant UUIDs, fresh session UUIDs, a durable monotonically increasing
recovery generation and the exact LP ownership/fencing terms. Its digest covers
the complete partition/topology, membership, participant public-key map,
protocol/model/payload/RNG versions and limits. Runtime activation persists the
new generation and fencing terms before admitting external traffic. A rank or
socket address is routing metadata, never an issuer witness.

Canonical versioned application records use reviewed participant signing keys.
Key ID, algorithm and version are included in the authenticated record. gRPC
also uses mutual TLS; MPI transports the same authenticated application records.
Provisioning, rotation, trust-store validation and the pinned signature
implementation need explicit security review before interface freeze. No custom
cryptography or caller declaration substitutes for that validation.

## Canonical envelope and proof records

An envelope includes protocol/schema version; simulation/configuration/membership
identity; issuing participant/session/recovery generation; LP fencing term;
authority namespace and emission epoch; source and destination LP; full-width
tick; complete logical ancestry and output ordinals; incarnation; positive/anti
kind; payload schema/version and exact payload bytes. Bound ancestry depth,
payload length and the complete canonical byte length before allocation. The
record digest binds those bytes, with domain separation for each record kind.
Receipts bind the entire envelope key and digest, not a sequence or timestamp.

Portable signed evidence records are data. A fixed audited
`ExternalAccountingDriver` owns canonical decoding, authentication,
configuration/fence validation and durable journal access. Only that reviewed
implementation privately issues local opaque `VerifiedExternalDelivery`,
`VerifiedExternalAdmission`, `VerifiedExternalRetirement` and
`VerifiedExternalCutDecision` handles. Recreating a handle from a portable record
requires the same verifier. The receiver of signed remote evidence authenticates
the issuer's assertion and durably retains that exact evidence locally. An issuer
retry reads back its own original durable fact; where remote readback is needed,
use authenticated protocol readback, not access to another participant's WAL.
No public proof
constructor, arbitrary verifier callback, native-capability deserialization or
caller success boolean is permitted.

The proposed engine adapter operations are:

- `admit_external(VerifiedExternalDelivery)`: stage exact admission; commit the
  admission and accounting fact before producing its signed receipt.
- `acknowledge_external_admission(VerifiedExternalAdmission)`: close only the
  exact corresponding sender obligation; historical authorized retries are
  idempotent and cannot allocate another record or change revision.
- `apply_external_retirement(...)`: apply the actual budgeted cancellation,
  restore the appropriate prior model/RNG state and remove predecessor effects,
  with authoritative descendant antis retained. Commit
  that complete effect/accounting decision before producing retirement proof.
- `release_external_replacement(VerifiedExternalRetirement)`: persist complete
  ancestor-dependency closure before a reserved successor becomes publishable.
- `prepare_external_cut(...)` and
  `apply_external_cut(VerifiedExternalCutDecision)`: implement the complete
  authenticated pause/prepare/commit decision described below.

These names describe the proposed boundary, not implemented public APIs. The
joint review must settle exact typed arguments, failures, bounds and transaction
ownership. Adapter proof constructors and journal coupling stay within the
reviewed engine/accounting boundary. Transport crates have no native proof
minting access. A model codec serializes model data and cannot certify admission,
retirement, dependency closure or a committed cut.

## Durable facts and recovery

An admission proof identifies its durable fact ID, journal position/hash,
receiver owner/session/fence, exact envelope key/digest and accounted state.
ACK loss reads the same committed fact. Applied-retirement proof additionally
establishes removed predecessor effects, retained tombstone protection and all
induced descendant antis durably accounted. Sender release records bind the
complete predecessor/dependency closure and the reserved successor identity.

A versioned checkpoint plus WAL contains every ownership-relevant section:
model and component generations; RNG algorithm/state; pending and replay work;
history and exact outputs; tombstones/conflicts; transition graph/barriers;
lifetime root/incarnation/epoch reservations; ready/blocked outbox obligations;
admissions, retirements and receipts; counters/revisions; and the common
committed floor. Hash every section; missing/incompatible sections fail closed.
Restoration invalidates old local validity handles while preserving historical
identities. Migration fences the old owner durably before the new owner activates
the complete transferred ownership unit.

The transaction contract must give deterministic recovery for these boundaries:

| Crash boundary | Required recovery fact |
| --- | --- |
| Before admission commit | No successful receipt; retry admits once. |
| After admission commit, before ACK | Read back the same admission fact. |
| Before/after model-effect commit | Checkpoint/WAL restores or replays exactly once; no surviving unjournaled effect. |
| Before retirement commit | Reserved successor remains blocked. |
| After retirement commit, before receipt | Read back the exact durable retirement fact. |
| Before sender release commit | Successor remains blocked. |
| After sender release commit | Repeated proof is idempotent; retain the successor until its admission. |
| Restart or migration | Reject stale session/fence traffic; retained historical retries require their original authorized durable records. |

## Complete distributed cuts

Use coordinated pause, prepare and commit. A cut ID binds the previous floor,
exact complete participant/membership/fence set, frozen accounting revisions
and admission boundaries. Pause model execution and freeze all external admission
and accounting revisions; only read-only retries may continue. Freeze outgoing
channel sequence frontiers, then drain/reconcile the bounded captured frontiers
before prepare. Account every in-flight positive,
anti and control message and every retained or blocked obligation.

Prepare persists each participant's checkpoint/accounting summary and channel
reconciliation. Commit one durable common decision only after every participant
prepares. Each participant authenticates and applies that exact decision
idempotently, persists application and then resumes. Recovery queries the
durable common decision. A missing participant or unreachable coordinator does
not permit unilateral collection. MPI all-reduce may calculate a minimum inside
this protocol; its return value alone cannot authorize collection.

Collect strictly below the common floor. Equality remains reversible. A
predecessor tick30/successor tick25 pair contributes25 while retaining tick30
proof needed for cancellation and retry. Lost ACK retains the sender obligation
and corresponding receiver proof until a verified common decision permits
reclamation. Do not substitute a caller floor, independent sampled minima or
a successful socket/launcher check for complete evidence.

## Bounded implementation sequence

1. Joint PDES/distributed/security review freezes the exact interface, typed
   failures, limits, canonical encoding, dependency versions and journal
   transaction semantics against the independently accepted native merge.
2. Prepare the Track48 engine external adapter/checkpoint leaf and Track49
   canonical signed-record codec leaf. They may run on disjoint reserved paths
   only after that freeze. Integrate manifests and lockfiles serially.
3. Join journal/verifier and recovery, with real process-kill oracles at every
   admission, effect, retirement and release commit boundary.
4. Run the gRPC two-OS-process transport leaf with genuinely disjoint models.
5. Run MPI two-rank, then four-rank transport with the same records/accounting.
6. Join distributed cut/migration behavior and independently accept live
   serial model/RNG/committed-trace parity; suffix replay; changed tick/payload/
   destination; anti-before-positive; duplicates/conflicts; delayed applied
   proof; lost ACK; in-flight antis; restart/stale-owner rejection; complete
   migration; equality; old30/new25 and pre-floor rejection.

Every future packet binds actual current APIs/base/input hashes, small explicit
owned paths, ignored output paths and resolved commands at dispatch. Use the
single-maintainer harness and separate implementation/independent acceptance.
Resolve actual macOS compiler/LLVM, launcher/protoc paths and executable targets;
historical Windows commands are not current execution authority.

## Authority and evidence boundary

The Track29 conditional Track49 phase-entry ADR explicitly expires if its bound
interface/local semantics change. The joined owned interface requires renewed
joint review and a fresh specific maintainer phase-entry disposition. Refreshing
hashes cannot renew the earlier approval. Obtain that disposition after native
independent acceptance and the normal exact-head implementation PR/checks/merge
prerequisites, before Track49 production dispatch.

Preserve fresh raw49-to48 dependency-gate failures. Recheck35/47 independently;
this proposal changes no dependency or status. Strict package/security gates
remain independent; EXC199 does not extend to changed bytes or another PR.
Installed OpenMPI5.0.11, protoc36.2 and PowerShell7.6.6 identify available tools
only. No actual rank launch, socket, distributed rollback or recovery acceptance
is claimed. Track48 stays In Progress until the required live evidence and final
review are accepted. Global status files remain with their current Q4 owner.

## Reviewed source basis

Native issuer witnesses: `owned.rs:38`, pointer comparison `owned.rs:108`,
initial recovery generation `owned.rs:159`; process-local ID allocation
`optimistic.rs:34`; structural reconstruction `optimistic.rs:391`; complete native
group cut `owned_execution.rs:1090`, all at candidate9e2ca142. Existing Track49
`simulation.proto` ticks are uint64 and placeholders omit complete owned
authority/ancestry/incarnation/payload. Reviewed Track48/49 specs, plans,
ownership contracts, test matrices, external boundary review and Track29 ADR
remain the authority/evidence references. Line references are a review snapshot,
not an enduring API or execution grant.
