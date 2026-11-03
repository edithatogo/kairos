# Native MPI collective exchange preview

Maturity: experimental. Track 49 remains In Progress; CAP-48.1 is open.

`native-mpi` enables `kairo_ecs_mpi::native::NativeMpiExchange` using rsmpi
0.8.2 (published package `mpi`), with default features disabled. This avoids
libffi/user-defined reduction callbacks. The existing `mpi` feature and
`MpiProcessTransport` retain their custom TCP behavior. Launching that TCP
backend with `mpiexec` never proves MPI API transport.

## Decision and public API boundary

The preview accepts a borrowed MPI `Universe` and duplicates its world
communicator. The caller initializes/finalizes MPI; the exchange cannot outlive
the universe. Construct/use on the MPI initializing thread and obey the provided
MPI thread level. The exchange is explicitly neither Send nor Sync, including
on MPI implementations with integer communicator handles. Every rank owns exactly `LpId(rank)`. Calls are collective and
must occur in identical order on every rank. Event/null batches use native
`MPI_Allgather`; `barrier` uses `MPI_Barrier`. `minimum` gathers full-width
unsigned 128-bit contributions and compares them without truncation.

A round rejects invalid source/destination or oversized local input on **all**
ranks before the payload collective. Limits are 128 ranks and 64 KiB encoded
per-rank batches; the aggregate receive buffer is at most 8 MiB. Batches preserve
rank order then sender FIFO order. Allgather exposes every batch to every rank;
only the destination is returned by the API. There is no rank-to-rank confidentiality.
The codec validates every received batch before returning any messages.

This separate API deliberately does not implement `PdesTransport`: a scalar
minimum is not a consistent GVT cut. It does not authorize fossil collection,
optimistic admission/retirement, checkpoint migration, or failure recovery.
MPI's native error handler applies to MPI errors. A failed/nonparticipating rank
may block a collective or abort the MPI job. There is no timeout/retry guarantee,
authentication, encryption, or production credential provisioning in this leaf.
Use only a trusted, controlled MPI allocation. No unsafe Rust was added to Kairos;
rsmpi and mpi-sys own the underlying native FFI boundary.

The additive feature/API leaves scheduler, ABI, RNG and existing schemas intact.
It is not a replacement backend until the scheduler adapter, complete channel
accounting and two-host failure/restart oracles are qualified. Optimistic positive
and anti-messages remain outside this preview. No performance claim is made.

## Reproduction

Install an MPI implementation and development headers plus libclang for bindgen.
The qualified local implementation is Open MPI 5.0.11 on Darwin arm64, Rust 1.99.1.
The lockfile pins rsmpi's dependency closure. Upstream API:
[rsmpi collective documentation](https://docs.rs/mpi/0.8.2/mpi/collective/trait.CommunicatorCollectives.html).

```sh
cargo test --locked -p kairo-ecs-mpi --features native-mpi --test native_mpi_smoke --no-run --message-format=json > /tmp/native-mpi-build.jsonl
```

Use the `compiler-artifact` executable for `native_mpi_smoke` from that JSON, then:

```sh
mpiexec -n 2 "$NATIVE_MPI_TEST" --test-threads=1 --nocapture
mpiexec -n 4 "$NATIVE_MPI_TEST" --test-threads=1 --nocapture
```

Expected: every rank prints `native MPI API rank=... size=...` and passes ring,
full-width minimum, rejection and recovery checks. The ordinary Cargo test also
executes the single-rank case; that alone is not multi-rank acceptance.
Cross-host execution and CAP-48.1 are not proved by these local runs.
