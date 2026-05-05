# Handoff: Track 35 Distributed Simulation (MPI/gRPC)

## Current status

Phase 0 and Phase 1 design artifacts now exist. The Track 35 implementation is a
transport scaffold only. It does not yet include real `rsmpi`, `tonic`, or
`prost` runtime wiring.

Implemented artifacts:

- `crates/kairo-ecs-mpi/` with:
  - `MpiRankAssignment`;
  - stable `MpiMessageTag` values for event, null, migration, and telemetry
    messages;
  - `MpiTransport` placeholder implementing the Track 34 `PdesTransport`
    boundary and panicking until the real backend is wired.
- `crates/kairo-ecs-grpc/` with:
  - `GrpcPeer`;
  - `GrpcTransportConfig`;
  - `GrpcTransport` placeholder implementing the Track 34 `PdesTransport`
    boundary and panicking until the real backend is wired.
- `docs/distributed/transport-trait.md`
- `docs/distributed/entity-migration-protocol.md`
- `docs/distributed/telemetry-aggregation.md`
- `docs/distributed/deployment-guide.md`

## Validation

Passed:

```powershell
cargo check --manifest-path crates/kairo-ecs-mpi/Cargo.toml --features mpi
cargo check --manifest-path crates/kairo-ecs-grpc/Cargo.toml --features grpc
```

Attempted unit-test commands:

```powershell
cargo test --manifest-path crates/kairo-ecs-mpi/Cargo.toml --features mpi
cargo test --manifest-path crates/kairo-ecs-grpc/Cargo.toml --features grpc
```

The test builds reached the linker and failed because `link.exe` resolves to
`C:\Users\60217257\scoop\apps\git\current\usr\bin\link.exe`, which failed with
Win32 error 5 while creating signal/mapping objects.

## Not complete

- Real `rsmpi` transport is not implemented.
- Real `tonic`/`prost` gRPC transport is not implemented.
- End-to-end 2-node MPI/gRPC tests are not implemented.
- Entity migration runtime serialization and handshake are not implemented.
- Distributed telemetry runtime aggregation is not implemented.
- gRPC fault tolerance is not implemented.
- Quality gates under global conductor files are not added because they are
  deferred to the next control-gate pass.
