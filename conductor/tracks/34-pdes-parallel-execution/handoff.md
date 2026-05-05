# Handoff: Track 34 PDES & Parallel Execution

## Current status

Phase 0 and the Phase 1 contracts now have concrete artifacts. The Track 34
implementation is a conservative PDES scaffold, not a production scheduler.

Implemented artifacts:

- `crates/kairo-ecs-pdes/` with:
  - `LpId`, `Tick`, `WorldSegment`, `RemoteEvent`, `NullMessage`, `PdesMessage`;
  - `LogicalProcess` trait;
  - `PdesTransport` trait;
  - deterministic `ThreadChannelTransport`;
  - `PdesScheduler` scaffold that exchanges remote events, emits CMB null
    messages, and computes a transport-reduced GVT.
- `docs/pdes/sequential-determinism-baseline.md`
- `docs/pdes/logical-process-trait.md`
- `docs/pdes/event-exchange-protocol.md`
- `docs/pdes/gvt-algorithm.md`
- `docs/pdes/benchmark-results.md`
- `benches/pdes/README.md`

## Validation

Passed:

```powershell
cargo check --manifest-path crates/kairo-ecs-pdes/Cargo.toml --features pdes
```

Attempted unit-test command:

```powershell
cargo test --manifest-path crates/kairo-ecs-pdes/Cargo.toml --features pdes
```

The test build reached the linker and failed because `link.exe` resolves to
`C:\Users\60217257\scoop\apps\git\current\usr\bin\link.exe`, which failed with
Win32 error 5 while creating signal/mapping objects. Retrying with
`RUSTFLAGS='-C linker=rust-lld'` failed because Windows SDK import libraries
such as `kernel32.lib` were not on the linker search path.

## Not complete

- Sequential parity tests are not implemented yet.
- Deadlock-free stress tests are not implemented yet.
- Scaling benchmarks are not implemented or run yet.
- Time Warp research spike is not implemented.
- Quality gates under global conductor files are not added because they are
  deferred to the next control-gate pass.

## Downstream contract for Track 35

Track 35 should consume `PdesTransport`, `RemoteEvent`, `NullMessage`, and the
GVT reduction boundary from `crates/kairo-ecs-pdes/`. MPI and gRPC transports
must preserve the Track 34 event and null-message semantics.
