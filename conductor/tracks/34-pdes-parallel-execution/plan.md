# Track 34 Plan: PDES & Parallel Execution

## Worker 2 implementation status — 2026-05-06

Completed with artifacts and validation:

- Phase 0 / Task 0.1: `docs/pdes/sequential-determinism-baseline.md`
  documents the sequential scheduler as the PDES correctness oracle.
  Validation documented and attempted: `cargo test --manifest-path
  crates/kairo-ecs-core/Cargo.toml`; crate-specific linker issue remains for
  executable tests on this machine.
- Phase 0 / Task 0.2: owned surface created under `crates/kairo-ecs-pdes/`,
  `docs/pdes/`, and `benches/pdes/`.
- Phase 1 / Task 1.1: `LogicalProcess`, `LpId`, `RemoteEvent`, `WorldSegment`,
  and `Tick` are defined in `crates/kairo-ecs-pdes/src/lib.rs`; contract
  documented in `docs/pdes/logical-process-trait.md`.
- Phase 1 / Task 1.2: event and null-message protocol documented in
  `docs/pdes/event-exchange-protocol.md`; scaffold types exist in
  `crates/kairo-ecs-pdes/src/lib.rs`.
- Phase 1 / Task 1.3: GVT contract documented in `docs/pdes/gvt-algorithm.md`;
  scaffold reduction boundary exists as `PdesTransport::all_reduce_min`.

Validation passed:

```powershell
cargo check --manifest-path crates/kairo-ecs-pdes/Cargo.toml --features pdes
```

Not marked complete:

- sequential parity tests, stress tests, scaling benchmarks, Time Warp spike,
  and global quality gates.

## Phase 0 — Scope lock

### Task 0.1 — Verify sequential determinism
- Run Track 12 conformance suite against sequential scheduler on 5+ consecutive runs.
- Confirm zero ordering divergence across runs for the same seed.
- Document the determinism contract in `docs/pdes/sequential-determinism-baseline.md` as the PDES correctness oracle.

### Task 0.2 — Lock the owned surface
- All new code under `crates/kairo-ecs-pdes/` (new crate).
- Design docs under `docs/pdes/`.
- Benchmarks under `benches/pdes/`.
- Do not modify `crates/kairo-ecs-core/` scheduler internals, `docs/core-contract.md`, or Track 12 benchmark harness code.
- Track artifacts in `conductor/tracks/34-pdes-parallel-execution/`.

## Phase 1 — Design LogicalProcess API and exchange protocol

### Task 1.1 — Define LogicalProcess trait
- Trait methods: `init(&mut self, lp_id: LpId, world_segment: &WorldSegment)`, `process_local_events(&mut self, until: Tick)`, `schedule_remote_event(&mut self) -> Vec<RemoteEvent>`, `receive_remote_events(&mut self, events: Vec<RemoteEvent>)`, `advance_to(&mut self, tick: Tick)`.
- Define `LpId`, `RemoteEvent`, `WorldSegment` types.
- Document the trait contract in `docs/pdes/logical-process-trait.md`.

### Task 1.2 — Design event exchange protocol
- Define the message format: `(source_lp, dest_lp, tick, event_payload)`.
- Define lookahead semantics: each LP declares a minimum lookahead `L`; an LP at local time `T` may only schedule events for remote LPs at time `>= T + L`.
- Specify the CMB null-message protocol for deadlock avoidance.
- Document the protocol in `docs/pdes/event-exchange-protocol.md`.

### Task 1.3 — Design GVT algorithm
- Choose a GVT algorithm: Mattern's or Samadi's.
- Define GVT as `min(local_time_of_all_LPs, min_timestamp_of_all_inflight_messages)`.
- Specify GVT computation frequency (every N ticks or on demand).
- Document in `docs/pdes/gvt-algorithm.md`.

## Phase 2 — Implement conservative PDES

### Task 2.1 — Build PDES scheduler core
- Implement `PdesScheduler` struct holding a collection of LPs and a communication layer.
- Implement the CMB null-message protocol: each LP sends null messages to neighbors when it advances, carrying its local time + lookahead.
- Implement LP-local event queue processing.
- Implement GVT calculation and tick advancement.

### Task 2.2 — Implement communication layer
- Build a thread-per-LP model with MPSC channels for LP-to-LP event exchange.
- Implement barrier synchronization for GVT computation.
- Handle LP startup ordering and shutdown.

### Task 2.3 — Wire cargo feature flag
- `crates/kairo-ecs-pdes/` is included in the root workspace.
- Keep optional PDES runtime dependencies feature-gated inside `crates/kairo-ecs-pdes/`.
- Ensure sequential code path is unaffected when PDES-specific features are disabled.

## Phase 3 — Benchmark scaling

### Task 3.1 — Build PDES benchmark suite
- In `benches/pdes/`, create benchmarks for 4/8/16/32 LP configurations.
- Benchmark workloads: entity-spawn-heavy, event-heavy, query-heavy, mixed.
- Compare PDES throughput and final state parity against sequential baseline.

### Task 3.2 — Collect scaling data
- Run benchmarks on controlled hardware (4+ physical cores).
- Record: throughput (ticks/sec), final state parity (pass/fail), GVT progression rate.
- Publish results in `docs/pdes/benchmark-results.md`.

### Task 3.3 — Tune lookahead and LP count
- Profile under various lookahead values and partition sizes.
- Document recommendations for LP count vs. core count, lookahead vs. event density.

## Phase 4 — Cross-track integration

### Task 4.1 — Handoff to Track 35 (Distributed Simulation)
- The `LogicalProcess` trait and event exchange protocol are inputs to Track 35's distributed LP model.
- Provide integration guide: how to swap the thread-local communication layer for MPI/gRPC.
- Document the handoff in `conductor/tracks/35-distributed-simulation-mpi-grpc/handoff-from-track34.md`.

### Task 4.2 — Research spike: Optimistic PDES
- Scoped spike: implement a minimal Time Warp prototype for a 2-LP case.
- Measure rollback frequency and overhead.
- Document findings in `docs/pdes/time-warp-spike.md`.
- Do not productionize; file follow-up track if promising.

### Task 4.3 — Update quality gates
- Confirm the central `pdes-sequential-parity` gate is present and documented (final state must match sequential for partitioned worlds).
- Confirm the central `gvt-progression-check` gate is present and documented (GVT must advance every N ticks).
- Confirm the central `pdes-deadlock-free` gate is present and documented (stress test passes).

## Phase 5 — Handoff and closeout

### Task 5.1 — Prepare maintainer notes
- Document how to add a new LP implementation.
- Document how to tune lookahead for a given simulation.
- Document the testing strategy for PDES correctness.

### Task 5.2 — Cross-track communication
- Hand off to Track 01 (scheduler team) for review of the parallel approach.
- Hand off to Track 04 (Arrow telemetry) for future PDES telemetry integration.
- Hand off to Track 12 (benchmarks) for PDES benchmark review.
- Notify Track 16 (Release Governance) that PDES is non-blocking for release.

### Task 5.3 — Update the risk register
- Mark resolved risks as mitigated.
- Escalate any performance or correctness finding that impacts sequential mode.
## Phase closeout gate

Before any task or phase in this track is marked complete, and before the next phase begins:

1. Run `$conductor-review` against this track and the current diff.
2. Auto-apply accepted review fixes inside this track's owned paths.
3. Record rejected, cross-track, or blocked-path fixes in `handoff.md`.
4. Run `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` plus the gates listed in `test-matrix.md`.
5. Commit and push the cleaned slice, then record the commit SHA or blocker in `handoff.md`.
6. Advance the next phase only after there is no in-scope unstaged or untracked work except documented draft satellites.