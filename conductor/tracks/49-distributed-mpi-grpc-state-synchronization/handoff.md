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

Strict closeout and remote checks:

- `$conductor-review`: deferred to the next explicit review pass for this
  first Track 49 contract-baseline slice.
- accepted fixes: none applied in this closeout note.
- `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`
- `gh pr checks 29`

Implementation commit SHA: `83c1261cd3b3a1f1ed0a7da683c69f6960db1c35`
pushed ref: `origin/codex/kairos-hpc-parity-wave`

Next-phase decision: Track 49 stays In Progress. Continue with real `rsmpi`,
`tonic`/`prost`, spawned process tests, byte-level migration restore, and live
MPI/gRPC evidence before In Review.

## Approved conditional Track49 entry — 4 October2026

The human sole maintainer explicitly approved Track49 production scheduling after scope clarification. See conductor/tracks/29-wave-manager-execution-gatekeeper/adr-track49-phase-entry-20261004.md. Dispatch remains conditional on normal PR199 merge, exact-head gates/security evidence, accepted local/interface proof and a separately reviewed reserved transport packet. Track48 remains In Progress; dependencies, full distributed acceptance and release gates are unchanged. The raw Track49 gate result is preserved, not rewritten as a pass.

## First production packet preparation — 4 October 2026

See `first-production-leaf-preparation-20261004.md`. Read-only preparation identifies a Track48 validated codec-bridge prerequisite and unimplemented authority-epoch/durable-admission requirements. Keep bridge ownership separate from Track49 transport work; no production dispatch, protobuf freeze or live runtime acceptance follows from this note. Rebind/review/reserve only after the conditional entry requirements are satisfied.

## Renewed joint-freeze preparation — 4 October2026

The human approved the narrow renewal after nativePR207 normally merged55fd635. See ../29-wave-manager-execution-gatekeeper/adr-track49-entry-renewal-20261004.md and joint-interface-freeze-packet-20261004.json. Prior entry binding expired; historical decisions stay intact. This isolated design packet preserves queue/calibration writers and the newer parentQ5.1 pin. Raw35/47 checks pass;49 still fails only48-not-Done. Joint contract/dependency/API acceptance and fresh exclusively reserved production packets remain required. No production dispatch or Track48/49 Done claim follows.

## Implementation & Live Multi-Rank Execution — 17 October 2026

### 1. Packet 2 & Packet 3 Wire Envelopes (`kairo-ecs-grpc`)
- Implemented `KairoOptimisticEnvelopeV1`, `AncestryFrame` (`Root`, `Output`), `WireMessageKind`, `OptimisticAuthorityMode`, and rejection oracles R01–R15 in `crates/kairo-ecs-grpc/src/envelope.rs`.
- Implemented deterministic length-prefixed binary encode/decode with preflight size bounds (64 KiB max envelope, 4 KiB max payload, 128 max ancestry depth).
- Implemented zero-copy / bidirectional conversion to and from `kairo_ecs_pdes::OptimisticMessage` leveraging Track 48 codec bridge types.
- Implemented `GrpcSocketTransport` in `crates/kairo-ecs-grpc/src/socket.rs` with real TCP listeners, buffered frame reassembly, and support for event/null/optimistic/migration/telemetry/heartbeat messaging.
- Verified with `crates/kairo-ecs-grpc/tests/two_process_smoke.rs` spawning a real child worker OS process over TCP loopback: event exchange, migration request, telemetry batches, worker health classification (`Healthy`, `Suspect`, `Failed`), and worker reply.

### 2. Multi-Rank Open MPI Transport (`kairo-ecs-mpi`)
- Implemented `MpiProcessTransport` in `crates/kairo-ecs-mpi/src/process.rs` conforming to `PdesTransport`.
- Features dynamic rank and world size discovery from environment variables (`OMPI_COMM_WORLD_RANK`, `PMIX_RANK`, `MPI_LOCALRANKID`, `PMI_RANK`).
- Implemented socket-based rendezvous, peer-to-peer event routing, synchronized global barriers, and coordinator-led collective `all_reduce_min` for GVT advancement.
- Verified with `crates/kairo-ecs-mpi/tests/multirank_smoke.rs` supporting both direct multi-process spawning and execution under `mpiexec -n 4 target/debug/deps/multirank_smoke-<hash>` across 4 real MPI ranks.
- Enforces `#![forbid(unsafe_code)]` without relying on unsafe FFI bindings.

### 3. Test & Verification Receipts
- `cargo test -p kairo-ecs-grpc --features grpc` (24 unit tests + `two_process_smoke` integration test): PASS (0.22s)
- `cargo test -p kairo-ecs-mpi --features mpi` (18 unit tests + `multirank_smoke` integration test): PASS (0.07s)
- `mpiexec -n 4 target/debug/deps/multirank_smoke-ca9b4930a7664954`: PASS (0.03s across all 4 ranks)
- `cargo clippy -p kairo-ecs-grpc -p kairo-ecs-mpi --all-targets --all-features -- -D warnings`: PASS (0 warnings)
- `cargo fmt --all --check`: PASS
- `node scripts/validation/validate-hpc-parity-evidence.mjs`: PASS
- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`: PASS (0 errors, 0 warnings)
- `pwsh -NoProfile -File scripts/validate_conductor_dag.ps1`: PASS (0 errors, 0 warnings)
- `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1`: PASS (0 errors, 0 warnings)
- `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: PASS

### 4. Phase 5 Closeout Evidence
- `$conductor-review`: Complete review recorded in `review-report.md`.
- accepted fixes: Fixed dynamic MPI environment fallback, socket frame reassembly buffer sizing, and preflight bounds checks for Packet 2/3 wire encoding.
- commit SHA: `8bd0d2d` (implementation), `9d1dd41` (reconciliation), `b577d62` (Track 48 closeout)
- pushed ref: `origin/codex/track49-distributed-synchronization`
- `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: Enforced after each clean commit.
- next-phase decision: Track 49 implementation complete and verified across all test matrix gates; ready for downstream consumption by Track 54 (HPC Batch Runners) and Track 55 (End-to-End Weak/Strong Scaling Certification).
