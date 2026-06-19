# Track 49 Handoff

Last updated: 2026-06-19

## Summary

Track 49 owns real MPI and gRPC distributed synchronization. The first
implementation slice adds contract-level MPI multi-rank launch descriptors,
gRPC two-process launch descriptors, and entity migration snapshots that
preserve entity/component generations and pending-event metadata. This remains
a contract baseline, not live MPI/gRPC runtime proof.

## Files changed

- `crates/kairo-ecs-mpi/src/lib.rs`
- `crates/kairo-ecs-grpc/src/lib.rs`
- `conductor/tracks/49-distributed-mpi-grpc-state-synchronization/*`

## Contracts consumed

- Track 35 placeholder protocol boundaries.
- Track 47 LP contract.
- Track 48 anti-message contract.
- Track 46 evidence manifest.

## Contracts changed

- `MpiLaunchPlan` and `mpi_multirank_smoke_contract` define the required
  multi-rank command shape while keeping `real_mpi_runtime_claimed = false`.
- `GrpcProcessLaunchPlan` and `grpc_two_process_smoke_contract` define the
  required two-process endpoint/service shape while keeping
  `real_grpc_runtime_claimed = false`.
- `MpiEntityMigrationSnapshot` and `GrpcEntityMigrationSnapshot` preserve
  entity generation, component generation, and pending event metadata.

## Tests added

- `mpi_multirank_launch_contract_requires_real_rank_count`
- `entity_migration_snapshot_preserves_generation_and_pending_events`
- `grpc_two_process_launch_contract_requires_distinct_real_socket_endpoints`
- `grpc_entity_migration_snapshot_preserves_generation_and_pending_events`

## Known risks

Real MPI, real gRPC, multi-node evidence, socket-spawned tests, protobuf
schemas, and runtime transport replacement remain unavailable. The new
contracts make launch and migration evidence explicit but do not satisfy the
Track 49 live-proof gates.

## Follow-up issues

- Add `rsmpi` transport behind `mpi` without breaking fallback-disabled builds.
- Add `tonic`/`prost` service/client behind `grpc`.
- Add real `mpiexec -n 4` and process-spawned gRPC tests.
- Add byte-level migration serialization and restore tests.
- Replace placeholder transports behind feature gates after contract tests pass.

## Integration notes

Track 54 consumes launch commands and scheduler requirements from this track.

## Phase closeout evidence

Red step captured with:

- `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-mpi --features mpi`
- `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-grpc --features grpc`

Both failed before implementation because the launch-plan, smoke-contract,
migration-snapshot, versioned-component, and pending-event metadata types did
not exist.

Passing implementation gates:

- `rustup run stable-x86_64-pc-windows-gnu cargo fmt -p kairo-ecs-mpi -p kairo-ecs-grpc`
- `CARGO_INCREMENTAL=0 rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-mpi --features mpi`
- `CARGO_INCREMENTAL=0 rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-grpc --features grpc`
- `CARGO_INCREMENTAL=0 rustup run stable-x86_64-pc-windows-gnu cargo clippy -p kairo-ecs-mpi -p kairo-ecs-grpc --all-targets --all-features -- -D warnings`
- `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate_conductor_phase_gates.ps1`
- `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate_conductor_dag.ps1`

Run `$conductor-review`, record accepted fixes, commit SHA, pushed ref,
`validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`, and the
next-phase decision before advancing beyond this contract-baseline slice.

Implementation commit SHA: `83c1261cd3b3a1f1ed0a7da683c69f6960db1c35`
pushed ref: pending push
