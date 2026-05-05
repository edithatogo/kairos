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
  - `validate_rank_assignments`, `MpiMigrationRequest::validate`, and
    `MpiTelemetryBatch::validate` for dependency-free local protocol checks;
  - dependency-free `MpiTransport` protocol emulator implementing the Track 34
    `PdesTransport` boundary for compile-checked message round-trip and GVT
    reduction tests.
- `crates/kairo-ecs-grpc/` with:
  - `GrpcPeer`;
  - `GrpcTransportConfig`;
  - `validate_config`, `validate_peers`, `GrpcMigrationRequest::validate`,
    `GrpcTelemetryBatch::validate`, and `classify_worker` for dependency-free
    local protocol and heartbeat/fault-tolerance checks;
  - dependency-free `GrpcTransport` protocol emulator implementing the Track 34
    `PdesTransport` boundary for compile-checked message round-trip and GVT
    reduction tests.
- `docs/distributed/transport-trait.md`
- `docs/distributed/entity-migration-protocol.md`
- `docs/distributed/telemetry-aggregation.md`
- `docs/distributed/deployment-guide.md`

## Validation

Passed:

```powershell
cargo check --manifest-path crates/kairo-ecs-mpi/Cargo.toml --features mpi
cargo check --manifest-path crates/kairo-ecs-grpc/Cargo.toml --features grpc
cargo check --manifest-path crates/kairo-ecs-mpi/Cargo.toml --features mpi --tests
cargo check --manifest-path crates/kairo-ecs-grpc/Cargo.toml --features grpc --tests
cargo fmt --manifest-path crates/kairo-ecs-mpi/Cargo.toml -- --check
cargo fmt --manifest-path crates/kairo-ecs-grpc/Cargo.toml -- --check
```

Latest focused validation on this handoff also passed:

```powershell
cargo check --manifest-path crates/kairo-ecs-mpi/Cargo.toml --features mpi --tests
cargo check --manifest-path crates/kairo-ecs-grpc/Cargo.toml --features grpc --tests
cargo fmt --manifest-path crates/kairo-ecs-mpi/Cargo.toml -- --check
cargo fmt --manifest-path crates/kairo-ecs-grpc/Cargo.toml -- --check
```

Attempted unit-test commands:

```powershell
cargo test --manifest-path crates/kairo-ecs-mpi/Cargo.toml --features mpi
cargo test --manifest-path crates/kairo-ecs-grpc/Cargo.toml --features grpc
cargo test --manifest-path crates/kairo-ecs-mpi/Cargo.toml --features mpi --lib
cargo test --manifest-path crates/kairo-ecs-grpc/Cargo.toml --features grpc --lib
```

The test builds reached the linker and failed because `link.exe` resolves to
`C:\Users\60217257\scoop\apps\git\current\usr\bin\link.exe`, which failed with
Win32 error 5 while creating signal/mapping objects. The latest `--lib` attempts
failed with the same Git MSYS `link.exe` collision and exit code `0xc0000142`.

## Not complete

- Real `rsmpi` transport is not implemented.
- Real `tonic`/`prost` gRPC transport is not implemented.
- End-to-end 2-node MPI/gRPC tests are not implemented; only local protocol
  emulator checks are present.
- Entity migration runtime serialization and handshake are not implemented.
- Distributed telemetry runtime aggregation is not implemented.
- gRPC fault tolerance is not implemented.
- Local validators now cover protocol envelope shape and heartbeat
  classification, but they are not a substitute for real multi-node runtime
  tests.
- Quality gates under global conductor files are not added because they are
  deferred to the next control-gate pass.
