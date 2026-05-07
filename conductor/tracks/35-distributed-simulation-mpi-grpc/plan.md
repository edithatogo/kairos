# Track 35 Plan: Distributed Simulation (MPI/gRPC)

## Worker 2 implementation status — 2026-05-06

Completed with artifacts and validation:

- Phase 0 / Task 0.1: Track 34 handoff was refreshed in
  `handoff-from-track34.md` with actual available contracts and blockers.
- Phase 0 / Task 0.2: owned surfaces created under `crates/kairo-ecs-mpi/`,
  `crates/kairo-ecs-grpc/`, and `docs/distributed/`.
- Phase 0 / Task 0.3: environment setup is documented as pending. No system MPI,
  `rsmpi`, `tonic`, or `prost` dependency was introduced in this scaffold.
- Phase 1 / Task 1.1: transport boundary is documented in
  `docs/distributed/transport-trait.md`; `MpiTransport` and `GrpcTransport`
  dependency-free protocol emulators implement Track 34's `PdesTransport`.
- Phase 1 / Task 1.2: entity migration protocol is documented in
  `docs/distributed/entity-migration-protocol.md`.
- Phase 1 / Task 1.3: distributed telemetry aggregation is documented in
  `docs/distributed/telemetry-aggregation.md`.
- Phase 3 / Task 3.1 design artifact: protobuf schema scaffold added at
  `crates/kairo-ecs-grpc/proto/simulation.proto`.

Validation passed:

```powershell
cargo check --manifest-path crates/kairo-ecs-mpi/Cargo.toml --features mpi
cargo check --manifest-path crates/kairo-ecs-grpc/Cargo.toml --features grpc
```

Not marked complete:

- real MPI backend, real gRPC backend, 2-node tests, migration runtime,
  telemetry runtime, fault tolerance, and global quality gates.

## Phase 0 — Scope lock

### Task 0.1 — Verify Track 34 handoff
- Confirm `LogicalProcess` trait is stabilized by Track 34.
- Confirm event exchange protocol is documented and tested in single-node mode.
- Confirm GVT algorithm works correctly under the Track 34 thread-based transport.
- Document any Track 34 gaps that block distributed work.

### Task 0.2 — Lock the owned surface
- All new code under `crates/kairo-ecs-mpi/` and `crates/kairo-ecs-grpc/` (new crates).
- Design docs under `docs/distributed/`.
- Do not modify `crates/kairo-ecs-pdes/` scheduler algorithm, `crates/kairo-ecs-core/`, or `crates/kairo-ecs-arrow/`.
- Track artifacts in `conductor/tracks/35-distributed-simulation-mpi-grpc/`.

### Task 0.3 — Environment setup
- MPI: install `libmpi-dev` / `msmpi`, verify `rsmpi` crate compiles and links.
- gRPC: verify `tonic` + `prost` build chain works.
- Set up CI runners with MPI support (or mock MPI for unit tests).

## Phase 1 — Define transport abstraction and protocols

### Task 1.1 — Define transport trait
- Trait `LpTransport`: methods `send_event(dest: LpId, event: RemoteEvent)`, `recv_events() -> Vec<RemoteEvent>`, `barrier()`, `all_reduce_min(timestamp) -> Timestamp`.
- Implementations: `ThreadChannelTransport` (Track 34 existing), `MpiTransport`, `GrpcTransport`.
- Document in `docs/distributed/transport-trait.md`.

### Task 1.2 — Design entity migration protocol
- Migration message: `(entity_id, component_data: Vec<(ComponentTypeId, Vec<u8>)>, source_lp, dest_lp, migration_id)`.
- Handshake: source LP sends migration request → coordinator (LP 0) acknowledges → destination LP receives and applies → destination LP acknowledges → source LP deletes entity.
- At-most-once semantics: migration ID for deduplication.
- Document in `docs/distributed/entity-migration-protocol.md`.

### Task 1.3 — Design distributed telemetry aggregation
- Per-node: collect Arrow record batches using Track 04 schema.
- Aggregation modes:
  - MPI: `MPI_Gather` of serialized Arrow IPC buffers, merged at root.
  - gRPC: streaming RPC where workers push batches to an aggregator service; aggregator merges and emits consolidated batches.
- Merge strategy: concatenate record batches (per-node data is disjoint by LP).
- Document in `docs/distributed/telemetry-aggregation.md`.

## Phase 2 — Implement MPI communication layer

### Task 2.1 — Build MPI transport
- Implement `MpiTransport` using `rsmpi`.
- MPI bootstrap: `MPI_Init`, rank assignment, LP-to-rank mapping.
- Event exchange: point-to-point `MPI_Send`/`MPI_Recv` with tagged messages (event vs. null message vs. migration vs. telemetry).
- GVT synchronization: `MPI_Allreduce` with `MPI_MIN` over local minimum timestamps.

### Task 2.2 — Build MPI entity migration
- Implement migration message serialization with `bincode` or `rmp-serde`.
- Implement migration handshake over MPI point-to-point messages.
- Handle migration during the exchange phase of the CMB algorithm.

### Task 2.3 — Add `mpi` feature flag
- `crates/kairo-ecs-mpi/` is included in the root workspace.
- Keep real MPI backend dependencies gated behind the crate-local `mpi` feature.
- Ensure non-MPI builds are unaffected.

## Phase 3 — Implement gRPC communication layer

### Task 3.1 — Define Protobuf service
- `simulation.proto`: service definitions for `ExchangeEvents` (bidirectional stream), `MigrateEntity` (unary), `StreamTelemetry` (client-streaming), `GvtSync` (unary).
- Message types: `RemoteEvent`, `MigrationRequest`, `MigrationAck`, `TelemetryBatch`, `GvtProposal`.

### Task 3.2 — Build gRPC transport
- Implement `GrpcTransport` using `tonic`.
- gRPC server per worker node, gRPC client connections to peers.
- Event exchange over bidirectional streaming RPC.
- GVT synchronization over unary RPC with coordinator aggregation.

### Task 3.3 — Implement gRPC fault tolerance
- Worker health checks: periodic heartbeat to coordinator.
- Worker failure detection: heartbeat timeout → coordinator removes worker from active set.
- Degraded mode: simulation continues with remaining workers; affected LP's entities are reassigned or simulation scope is reduced.
- Reconnection: failed worker can rejoin with state catch-up from coordinator snapshot.

### Task 3.4 — Add `grpc` feature flag
- `crates/kairo-ecs-grpc/` is included in the root workspace.
- Keep real gRPC backend dependencies gated behind the crate-local `grpc` feature.
- Ensure non-gRPC builds are unaffected.

## Phase 4 — Integration and testing

### Task 4.1 — End-to-end 2-node test
- Spin up 2 processes (MPI: `mpirun -np 2`; gRPC: two binary instances).
- Run identical partitioned world on both nodes.
- Verify final state parity: component data matches single-node PDES.

### Task 4.2 — Entity migration test
- Migrate an entity between LPs on different nodes during simulation.
- Verify all component data is preserved after migration.
- Verify the entity is accessible on the destination LP and removed from the source LP.

### Task 4.3 — Fault tolerance test (gRPC mode)
- Kill a non-leader worker mid-simulation.
- Verify simulation continues with remaining workers.
- Verify the failed worker's entities are either migrated or simulation scope is adjusted.

### Task 4.4 — Telemetry aggregation test
- Run distributed simulation with telemetry enabled on all nodes.
- Aggregate telemetry batches.
- Compare aggregated Arrow record batches against single-node collection.

### Task 4.5 — Update quality gates [x]
- `distributed-state-parity` gate is present in `conductor/quality-gates.md`.
- `entity-migration-integrity` gate is present in `conductor/quality-gates.md`.
- `grpc-fault-tolerance` gate is present in `conductor/quality-gates.md`.
- `distributed-telemetry-merge` gate is present in `conductor/quality-gates.md`.

Evidence 2026-05-06: the central gate catalogue now includes scaffold-level
definitions for all four distributed gates, tied to `validate-track35.ps1`.

## Phase 5 — Handoff and closeout

### Task 5.1 — Prepare deployment guide
- MPI: HPC cluster setup, Slurm job script template, InfiniBand configuration.
- gRPC: cloud deployment (containerized workers, service discovery, TLS).
- Document in `docs/distributed/deployment-guide.md`.

### Task 5.2 — Cross-track communication
- Hand off to Track 34 (PDES) for transport abstraction review.
- Hand off to Track 04 (Arrow telemetry) for distributed aggregation review.
- Hand off to Track 12 (benchmarks) for distributed benchmark scenarios.
- Notify Track 16 (Release Governance) that distributed mode is non-blocking.

### Task 5.3 — Update the risk register
- Mark resolved risks as mitigated.
- Escalate any stability or performance finding that affects single-node mode.
