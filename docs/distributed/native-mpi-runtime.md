# Native MPI external-driver transport

Maturity: additive experimental API on `native-mpi`. This is an opaque fixed
communicator transport for the complete external optimistic driver, not a
substitute for its durable admission, retirement, recovery or common-cut proof.
It preserves the existing event/null preview API and its 64 KiB batch limit.

## API and compatibility decision

`NativeMpiExchange` borrows the caller's `Universe`, duplicates its world
communicator and remains neither Send nor Sync. The caller owns MPI
initialization, thread-level compliance and finalization. No unsafe Rust,
dependency, feature/default, native ABI or existing scheduler interface changes
are introduced by this packet.

- `NativeMpiRecord { destination, bytes }` sends one byte-exact opaque record.
- `NativeMpiInbound { source_rank, destination, bytes }` identifies its actual
  MPI source slot and fixed destination rank. Its public fields are data, not
  unforgeable accounting capabilities.
- `exchange_records(records)` validates bounds/framing/destination only.
- `exchange_records_validated(records, validate)` additionally calls the pure
  local validator for **all** records on **all** ranks, in source-rank then sender
  FIFO order. A rejection on any rank rejects the round for every rank before
  any results are returned. Empty ranks still participate.

The validated method first gathers bounded batch lengths and collectively
rejects invalid local input before payload exchange. It then gathers payloads,
validates every batch/record, and gathers validation outcomes before returning
only the records addressed to the calling rank. No early return occurs between
payload and validation-outcome collectives. A valid round can follow a bounded
framing/policy rejection without communicator replacement.

A validator must not mutate models/stores, perform I/O, call MPI or publish
records. The external driver verifies canonical framing, immutable context,
signature and signed issuer-to-native-rank binding there, then passes returned
records through the durable runtime's existing fallible ingress boundary. A
batch is not an all-rank model transaction or a durable receipt. In particular,
calling durable `receive_from` inside the validator would violate this boundary.

The transport framing is version1: version byte, big-endian u16 record count,
then repeated big-endian u32 destination/u32 length and unchanged nonempty bytes.
Unknown versions, trailing bytes, truncation, impossible destinations and bounds
reject. Limits are128 ranks,64 records/rank,128 KiB/record and512 KiB total
encoded bytes/rank (including framing); the aggregate MPI payload is at most
64 MiB. MPI_Allgather exposes every batch to every participant: use a trusted
allocation, with no rank-to-rank confidentiality or implicit TLS claim.

## Scheduler and failure boundary

This packet deliberately does not implement the legacy `PdesTransport` trait:
its errors only represent unknown LP/destination mismatch, `barrier` returns
void, and a scalar reduction is not complete optimistic accounting. A trait
adapter that mislabeled an MPI failure or used a timestamp minimum as GVT would
weaken the contract. The fallible external driver instead consumes these
records and obtains advancement authority only from its proved common cut.

Rust errors represent framing/policy rejection. Native MPI errors retain the
configured MPI error-handler behavior; rank loss can block or abort an entire
allocation. No ULFM, live rank replacement, native MPI timeout or native MPI
retry is claimed. Recovery qualification must terminate the owned failed job,
reopen the existing durable stores in standby and establish a fresh complete
membership generation before dispatch. The complete driver and relaunch oracle
remain the integrator's qualification scope.

## Conformance and red-team responses

The expanded existing `native_mpi_smoke` executable runs unchanged through real
`mpiexec -n 2` and `mpiexec -n 4`. It covers ring/null exchange, full-width
scalar diagnostics, unequal/fan-in source/FIFO order and explicit record source
binding; one-rank malformed input, oversize record/count/batch and one-validator
rejection fail at **all** ranks, and a subsequent valid exchange succeeds. It
also covers exact128 KiB records and512 KiB batches. Unit cases reject version,
truncation, trailing bytes, zero length and impossible count/destination, and
exercise the64-record boundary. No retry or test skip is added.

- **A sender can put another issuer in signed bytes.** The MPI source remains
  its actual communicator slot; the pure driver verifier must match that slot
  to the immutable signature registry. Transport-only exchange does not claim
  to authenticate opaque contents.
- **One rank could accept while another rejects.** Validation outcomes are
  gathered before any returned records; one reject produces errors everywhere.
- **An ACK or scalar minimum could authorize reclamation.** Neither API returns
  accounting authority. Only the external engine's durable common-cut protocol
  can authorize reclamation.
- **A killed rank could deadlock a collective.** This is a whole-job failure
  boundary, not silent recovery. The controller must bound/terminate and relaunch
  the job from durable state; live fault-tolerant MPI is not asserted.

Local source qualification used actual Rust1.99.1 (Homebrew, LLVM23.1.3),
Open MPI5.0.11 on Darwin arm64. Exact commands, native executable/log hashes and
raw stdout are retained in the worker's ignored `artifacts/track49-native-worker/`
for integrator archival. These local results are not hosted or Linux evidence.
