# Track 48/49 ownership and evidence reconciliation — 2 November 2026

## Current lane

Chat `01a19bed-7060-7910-af22-38cb31f51446` owns bounded Track 49 MPI API
implementation and its dependency lock integration in managed worktree
`/Users/doughnut/.codex/worktrees/kairos-track49-runtime/kairos`, branch
`codex/track49-runtime-20261102`, based on origin/main
`c2bb5bbcc3539b158c892278da2f977da6cc89be`. Claimed paths: the MPI crate,
Cargo.lock, this track directory, the native MPI preview documentation and the
Track 48 handoff cross-reference. Native scope: local controlled test processes;
no tenant/authentication changes, downstream pin, registry publication or launch
onto a remote host. Next operation: independent source/dependency review,
all-feature CI prerequisite integration, then exact-head hosted qualification.
Earlier assignments remain history. This is not a repository-wide main
integration takeover; the lane produces a reviewed scoped source handoff.

## Ownership readback

The shared harness reported no active leases. No cargo/mpiexec/native test
operation was running before this lane. App snapshots show previous integrator
`01a18344-cfb8-7711-8c38-fef1052b9fcf` and qualification chat
`01a19600-0a41-7ca2-9e4b-e27d7339c8e7` finished their latest turns.
Other worktrees were preserved. The primary checkout remains at 339aba1;
origin/main was fetched and this lane starts at c2bb5bb instead.

The first claim mistakenly treated the MPI manifest (an owned output) as an
immutable input. The check correctly rejected its intentional edit. At a terminal
build checkpoint, exact owned changes were preserved outside the checkout, only
those owned changes were restored, the claim was released, a clean corrected
claim was acquired with the unchanged spec as input, and the exact changes were
reapplied. No harness bypass or other writer's files were involved.

## Reusable accepted evidence and boundaries

- 17 October custom TCP, launcher rank discovery, local sockets and codec
  receipts remain historical local evidence. The plan's checked rsmpi task and
  old review's "production-ready" wording overstate the owning source: main's
  `mpi = []` marker carries no MPI API implementation.
- Main includes the November synchronization safety packet and Tonic/Prost
  transport. Its handoff records 76 two-crate tests, local four-rank TCP evidence
  and Linux NAS curl HTTP/2 heartbeat via a host-key-verified SSH forward to the
  Mac Tonic endpoint (HTTP 200 / grpc-status 0). This proves cross-host protocol
  connectivity only, not an executing remote model or restart parity.
- CAP-48.1 still requires a consistent complete cut including positive/anti
  channels, rollback/GVT behavior under delay/duplication/failure, and two-host
  restart parity against the serial model. Existing scalar reductions do not
  satisfy these gates. PDES source ownership remains Track 48.
- CAP-49.1 still needs the scheduler-facing native MPI adapter, optimistic
  traffic and reviewed complete accounting. CAP-49.2 still needs two-host
  source/toolchain/identity receipts, failure/restart and serial parity.
- Frozen October external-accounting proposals are not implemented durable
  admission/retirement or authenticated journal proof. This exchange leaf does
  not activate that proposed protocol or relax its gates.

## Bounded implementation

See [API/compatibility decision](../../../docs/distributed/native-mpi-preview.md).
Optional rsmpi 0.8.2, no default features; published native MPI dependency closure
retained in Cargo.lock. No unrelated dependency upgrades. Existing TCP APIs stay
unchanged. This is an additive collective event/null exchange preview, not GVT
or a production distributed runtime. Track statuses and phase acceptance remain
unchanged. Review, test and source hashes will be retained with the handoff.
