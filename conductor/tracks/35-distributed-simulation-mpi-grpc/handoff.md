# Handoff: Track 35 Distributed Simulation (MPI/gRPC)

Last updated: 2026-06-23

## Current status

Archived as `Done` on 2026-06-23 for the legacy dependency-free MPI/gRPC transport scaffold. The focused runtime validator passes on this host. This archive does not claim real `rsmpi`, `tonic`/`prost`, multi-node execution, migration runtime, distributed telemetry aggregation, or fault tolerance; those requirements remain owned by Tracks 49/54/55.

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
-  Added compile-time contract envelopes in both crates:
   `MpiContractEnvelope` / `MpiContractMessage` and
   `GrpcContractEnvelope` / `GrpcContractMessage` so message-kind IDs, migration
   envelopes, and telemetry envelope expectations are checked before runtime
   backends are introduced.
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

The test-runtime gate now passes through the GNU Rust toolchain via:

```powershell
pwsh -NoProfile -File conductor\tracks\35-distributed-simulation-mpi-grpc\validate-track35.ps1 -RunTests
```

This proves the local dependency-free MPI and gRPC protocol-emulator tests. It
does not prove the future real `rsmpi`, `tonic`/`prost`, or multi-node runtime.

Protocol transport tests also now validate `PdesTransport` error-path behavior
(unknown LP IDs return `TransportError`) and message-contract envelopes.

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

Track 35 is cleanly closable as the legacy dependency-free distributed transport scaffold. The archive review reran `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\35-distributed-simulation-mpi-grpc\validate-track35.ps1 -RunTests` on 2026-06-23; it passed with 18 MPI tests, 20 gRPC tests, and doc-tests. The archive records local emulator/reference proof only. Real `rsmpi`, `tonic`/`prost`, multi-node execution, migration runtime, distributed telemetry aggregation, and fault tolerance remain blocked for Tracks 49/54/55 rather than this legacy scaffold track.

## Next-phase decision

Archived as `Done` for the legacy dependency-free MPI/gRPC transport scaffold. Future real distributed runtime work must proceed through Tracks 49/54/55 rather than reopening Track 35.

## Review remediation -- 2026-05-17

- Accepted fix: `validate-track35.ps1` now invokes `cargo check` and `cargo test` with the correct Cargo subcommands.
- Accepted fix: MPI and gRPC placeholder transports now expose `knows_lp` for the Track 34 transport boundary and reject unknown LPs consistently.
- Accepted fix: MPI and gRPC `all_reduce_min` emulators now model the current reduction round instead of retaining a sticky historical minimum.
- Accepted fix: the gRPC contract no longer uses `Option::is_none_or`, preserving the crate's declared Rust 1.76 compatibility.
- Accepted fix: the gRPC Rust service identity now matches the protobuf package/service name.
- Deferred by scope: real `rsmpi`, `tonic`/`prost`, multi-node execution, migration serialization, telemetry merge, and fault-tolerance evidence remain future runtime work.
- Validation: `pwsh -NoProfile -File conductor\tracks\35-distributed-simulation-mpi-grpc\validate-track35.ps1` passed.
- Runtime validation: `pwsh -NoProfile -File conductor\tracks\35-distributed-simulation-mpi-grpc\validate-track35.ps1 -RunTests` now passes by running MPI and gRPC crate tests through `stable-x86_64-pc-windows-gnu`, avoiding the local MSVC/Git `link.exe` collision.

## Review remediation -- 2026-05-18

- Accepted fix: MPI and gRPC placeholder transports now include pending `PdesMessage::Event` timestamps in `all_reduce_min`, so GVT cannot pass queued remote events in the emulator path.
- Accepted fix: MPI and gRPC `send` now validate embedded message source and destination consistently with the Track 34 reference transport, including destination-mismatch regression tests.
- Accepted fix: `handoff-from-track34.md` now reflects the current `PdesTransport` signature with `knows_lp` and `Result`-returning `send` / `recv`.
- Accepted fix: distributed docs command examples now include expected output and remove duplicate validation command text where it confused the contract boundary.
- Validation: `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\35-distributed-simulation-mpi-grpc\validate-track35.ps1 -RunTests` passed with 14 MPI tests, 15 gRPC tests, and doc-tests.
- Next-phase decision: remain `In Review`; this closes the local emulator strictness and GVT findings, but real `rsmpi`, `tonic`/`prost`, two-node execution, migration serialization, telemetry aggregation, and fault-tolerance evidence remain future runtime work.

## Software-only implementation -- 2026-05-18

- Implemented `local_two_rank_contract_proof` for the MPI placeholder transport, covering dependency-free event exchange, migration envelope validation, telemetry merge count, GVT floor, and an explicit no-real-MPI-runtime claim.
- Implemented `local_two_node_contract_proof` for the gRPC placeholder transport, covering dependency-free event exchange, migration envelope validation, telemetry merge count, non-leader failure classification, and an explicit no-real-gRPC-runtime claim.
- Updated distributed docs, the test matrix, and `validate-track35.ps1` so the software-only local proof is recorded without implying real `rsmpi`, `tonic`, `prost`, multi-node, or network runtime coverage.
- Next-phase decision: remain `In Review`; the local dependency-free proof is now stronger, but the real runtime dependencies remain blocked by platform/software setup.

## Archive review -- 2026-06-23

- Review result: no additional in-scope source defects were found for the legacy Track 35 scaffold. Real `rsmpi`, `tonic`/`prost`, multi-node execution, migration runtime, distributed telemetry aggregation, and fault tolerance remain explicit follow-up gates rather than reasons to keep this scaffold track open.
- Accepted fix: registry and closeout surfaces were reconciled from `In Review` and pending placeholders to an archived `Done` state for Track 35 only.
- Validation: `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\35-distributed-simulation-mpi-grpc\validate-track35.ps1 -RunTests` passed on 2026-06-23 with 18 MPI tests, 20 gRPC tests, and doc-tests.
- Deferred by scope: no real distributed MPI/gRPC runtime, real network transport, multi-node execution, migration runtime, distributed telemetry runtime aggregation, or fault-tolerance evidence is attached to Track 35. Public production distributed-runtime claims remain blocked until Tracks 49, 54, and 55 attach live evidence.
- Next-phase decision: Track 35 is archived as `Done`; future production distributed runtime work must proceed through Tracks 49/54/55 rather than this legacy scaffold track.
