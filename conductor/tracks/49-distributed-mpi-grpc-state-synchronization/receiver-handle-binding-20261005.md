# Track49 receiver and verified-handle binding — 2026-10-05

Disposition: reviewed within-design API clarification; full type/schema freeze HOLD.
Base d7f868ef4f5b1e82822bb3c6bb9e03c68b4129d8. Design SHA
 a15115a7c27ff76513dbad36711bf9ffe43bd6aa223873fd5fd9cbd79fb74de5
remains unchanged. No schema amendment adoption, source/API exports, manifests,
status, protected inventory or parent pin changes. No dispatch or release acceptance.

## Exact receiver clarification

For the seven named fixed-driver verification methods, bind:

```rust
fn verify_delivery(&self, bytes: &[u8]) -> Result<VerifiedExternalDelivery, ExternalVerificationError>;
fn verify_admission(&self, bytes: &[u8]) -> Result<VerifiedExternalAdmission, ExternalVerificationError>;
fn verify_cancellation(&self, bytes: &[u8]) -> Result<VerifiedExternalCancellation, ExternalVerificationError>;
fn verify_retirement(&self, bytes: &[u8]) -> Result<VerifiedExternalRetirement, ExternalVerificationError>;
fn verify_cut_plan(&self, bytes: &[u8]) -> Result<VerifiedExternalCutPlan, ExternalVerificationError>;
fn verify_cut_decision(&self, bytes: &[u8]) -> Result<VerifiedExternalCutDecision, ExternalVerificationError>;
fn verify_readback_request(&self, bytes: &[u8]) -> Result<VerifiedExternalReadbackRequest, ExternalVerificationError>;
fn readback(&self, request: VerifiedExternalReadbackRequest) -> Result<ExternalReadbackResponse, ExternalOperationFailure>;
fn export_checkpoint(&self) -> Result<ExternalCheckpoint, ExternalOperationFailure>;
```

For the external runtime, bind these previously prose-only receivers:

```rust
fn verify_external_record(&self, bytes: &[u8]) -> Result<VerifiedExternalRecord, ExternalVerificationError>;
fn readback_external(&self, request: VerifiedExternalReadbackRequest) -> Result<ExternalReadbackResponse, ExternalOperationFailure>;
fn export_external_checkpoint(&self) -> Result<ExternalCheckpoint, ExternalOperationFailure>;
fn prepare_external_manifest(&mut self, proposal: VerifiedExternalManifestProposal) -> Result<ExternalManifestPrepareRecord, ExternalOperationFailure>;
fn activate_external(&mut self, decision: VerifiedExternalManifestDecision) -> Result<ExternalActivationRecord, ExternalOperationFailure>;
```

Verification, readback and export inspect a consistent committed snapshot. They do
not allocate durable facts, identities or revisions, repeat committed effects or mutate
model/RNG/accounting state. Readback may sign an observational ReadbackResponse bound
to the exact authenticated request and materialize an exact already committed canonical
fact preimage when its receipt was pending. Original retained fact bytes/preimages stay
unchanged; this neither commits a new fact nor fabricates a new successful commit. Manifest preparation and activation commit state and use exclusive
mutable runtime access. The existing constructor/other section2 operation signatures
retain their approved by-value arguments and mutability.

For PortableModelCodec<P>, bind:

```rust
fn encode_process(&self, process: &P) -> Result<Vec<u8>, CodecError>;
fn decode_process(&self, bytes: &[u8]) -> Result<P, CodecError>;
fn encode_snapshot(&self, snapshot: &P::Snapshot) -> Result<Vec<u8>, CodecError>;
fn decode_snapshot(&self, bytes: &[u8]) -> Result<P::Snapshot, CodecError>;
```

Configuration/limits are immutable; codec calls cannot mutate model/RNG/accounting
state or issue verified authority. Exact error fields, schema/version/limit accessors
and resource enforcement remain unresolved. These signatures are contract bindings,
not compile-tested source declarations. `&self` confers no Send/Sync, concurrent access,
thread-safety, lock-free storage, interior-mutation or hardware support promise.

## Opaque handle lifecycle and retry

Every VerifiedExternal argument is passed by value. This follows actual native
admission/retirement capability consumption and requires no public Clone/Copy,
Deserialize, constructor, mutable proof field or raw signing/journal accessor.
The fixed driver alone creates verified handles; current driver/session/journal and
activation/revision/prerequisite checks occur again at every adapter entry.

For retry, retain the original signed bytes, invoke the fixed verifier again and
pass the fresh handle to the operation. Rust value consumption is not durable
exactly-once authority. The complete authenticated durable request identity provides
idempotence: an exact already committed request returns its original retained fact
without allocating a new identity/revision or repeating model/accounting effects.
Conflict rejection remains complete-data comparison, not digest-only equality.

A stale handle cannot be refreshed by Clone or by rewriting fields. Reverification
must still reject expired current-authority traffic. Cross-restart historical facts,
retained outbox publication and changed-owner routing use only the separately
specified historical readback/RecoveryAuthorization/migration paths. This binding
does not authorize accepting old traffic as current or change native cohort authority.
CommitOutcomeUnknown remains fenced until authoritative recovery/readback; a new
handle is not evidence that a failed commit rolled back or that publication is safe.

## Review and next bindings

PDES reviewer read design sections1/2 and actual native borrowing/capability seams,
and classified these receivers and consumption rules as within-design clarification,
without new wire fields or public proof authority. Exact record review corrected an overly broad signing prohibition: observational
responses and deferred materialization of already committed facts remain allowed.
Final corrected hash review is required before integration. Full API/type acceptance still requires complete fields/enum visibility,
all22 wire schemas/goldens, RecoveryOpen/coordinator entry-point mapping, feature/MSRV,
backend/provider/dependency proof and Track25 inventory alignment. Material schema
amendment remains pending human disposition. No implementation dispatch follows.

## Verified inputs

- `docs/distributed/external-accounting-v1.md` SHA256 `a15115a7c27ff76513dbad36711bf9ffe43bd6aa223873fd5fd9cbd79fb74de5`.
- `conductor/tracks/49-distributed-mpi-grpc-state-synchronization/api-governance-binding-20261005.md` SHA256 `ace74a53fa486d12feb34cf518805f6793188ce5d845eb716c2c386eea29d54e`.
- `crates/kairo-ecs-pdes/src/optimistic/owned_routing.rs` SHA256 `0e69cf12c8f4e8ada560572d808622bf3eb8aa8c6b341df5cbfb0669a3288959`.
- `crates/kairo-ecs-pdes/src/optimistic/owned_execution.rs` SHA256 `3b6f8470bd70170068c74cc12252f68c80c3366edfdbcef301d725a476c4434c`.
