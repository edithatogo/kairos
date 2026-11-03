# ADR: optional collective MPI API preview

Status: proposed for source integration; distributed/runtime acceptance pending.

## Context

The existing MPI-named process backend uses TCP and environment-discovered ranks.
CAP-49.1 requires actual MPI calls. The pending durable external-accounting
contract cannot be inferred from a scalar minimum or local collective success.

## Decision

Add a separate optional `native-mpi` feature using published rsmpi 0.8.2 without
user-operation/libffi features. Borrow the caller-owned Universe, duplicate the
world communicator and exchange bounded event/null batches using Allgather.
Expose full-width scalar minimum and barrier as explicitly raw collectives.
Retain `#![forbid(unsafe_code)]` in project source and keep upstream FFI ownership
with rsmpi/mpi-sys. No public ABI, RNG, existing transport or schema is changed.

## Consequences and compatibility

The source is additive and experimental. All-feature builds require MPI headers,
a library and libclang; the existing CI consumers must install prerequisites.
Default builds do not link MPI. API scope, native-error behavior, trusted-rank
assumption, bounds, test invocation and unsupported distributed/GVT semantics are
specified in [the preview documentation](../../../docs/distributed/native-mpi-preview.md).

The native test is a new conformance oracle: 1/2/4 participants, deterministic
ring delivery, u128 timestamps, single-rank invalid/oversized rejection on every
participant, and a subsequent valid empty exchange. This does not replace shared
scheduler conformance fixtures or satisfy positive/anti-message cut/restart gates.

Before integration require independent MPI/dependency review, focused/hosted
checks and unchanged security/licence gates. No release or parent pin promotion.
