# Handoff: Track 35 Distributed Simulation (MPI/gRPC)

Last updated: 2026-05-07

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
- Global quality gates now exist for `distributed-state-parity`,
  `entity-migration-integrity`, `grpc-fault-tolerance`, and
  `distributed-telemetry-merge`; they remain scaffold-level checks that validate
  the dependency-free transport emulators and keep real multi-node runtime
  claims explicit.

## Summary

No additional handoff summary was recorded by this Conductor hygiene update.


## Files changed

No additional file list was recorded by this Conductor hygiene update. Use the track plan, spec, and git history for implementation-specific file evidence.


## Contracts consumed

No additional consumed contracts were recorded by this Conductor hygiene update.


## Contracts changed

No contract changes were recorded by this Conductor hygiene update.


## Tests added

No tests were added by this Conductor hygiene update.


## Known risks

No new risks were introduced by this Conductor hygiene update.


## Follow-up issues

No additional follow-up issues were recorded by this Conductor hygiene update.


## Integration notes

No additional integration notes were recorded by this Conductor hygiene update.
## Phase closeout evidence

Track 35 is not ready for closeout yet. The offline validator passed on 2026-05-10 (`pwsh -NoProfile -File conductor/tracks/35-distributed-simulation-mpi-grpc/validate-track35.ps1`), and that evidence only confirms the dependency-free emulator checks plus the documented placeholder-transport boundaries. The optional runtime gate (`-RunTests`) remains blocked in this environment by the local `link.exe` collision with Git MSYS (`Win32 error 5` / `0xc0000142`). The remaining project blockers are still the real `rsmpi` transport, the real `tonic`/`prost` gRPC transport, 2-node execution checks, migration serialization, distributed telemetry aggregation, and gRPC fault-tolerance behavior. Keep this track `In Review` until those project blockers are resolved and the closeout evidence can be recorded. Before the track can move to `Done`, record `$conductor-review` findings, accepted fixes, deferred or blocked fixes, validation commands, cleanup state, commit SHA or explicit push blocker, pushed ref, strict `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` result, and next-phase decision here.
