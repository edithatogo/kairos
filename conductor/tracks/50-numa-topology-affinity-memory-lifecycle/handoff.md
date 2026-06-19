# Track 50 Handoff

Last updated: 2026-06-19

## Summary

Track 50 owns NUMA topology, affinity, allocator lifecycle, and zero-copy FFI
layout work. The first implementation slice adds a `numa` feature-gated
contract baseline for typed unsupported topology reporting, opt-in reversible
affinity plans, event-pool reuse, component locality metadata, and FFI
zero-copy layout validation. It does not claim live `hwloc`, OS affinity
binding, production arena allocation, or NUMA hardware proof.

## Files changed

- `crates/kairo-ecs-core/Cargo.toml`
- `crates/kairo-ecs-core/src/lib.rs`
- `crates/kairo-ecs-state/Cargo.toml`
- `crates/kairo-ecs-state/src/lib.rs`
- `crates/kairo-ecs-ffi/Cargo.toml`
- `crates/kairo-ecs-ffi/src/lib.rs`
- `conductor/tracks/50-numa-topology-affinity-memory-lifecycle/*`

## Contracts consumed

- Track 01 scheduler/state contracts.
- Track 02 FFI layout contracts.
- Track 46 evidence manifest.

## Contracts changed

- `NumaTopology`, `NumaNode`, `NumaSupport`, and `NumaError` define the
  feature-gated topology contract and unsupported-host behavior.
- `AffinityBindingPlan` defines opt-in reversible affinity intent without
  performing OS binding.
- `EventPool` and `EventPoolMetrics` define dependency-free event lifecycle
  reuse metrics with no global-lock claim.
- `ComponentLocalityPlan` records dense component storage locality metadata.
- `KairoEcsZeroCopyLayout`, `KairoEcsOwnership`, and `KairoEcsLayoutStatus`
  validate FFI pointer address, length, alignment, and ownership metadata.

## Tests added

- `numa_topology_reports_typed_unsupported_fallback`
- `affinity_plan_is_opt_in_and_reversible_without_binding`
- `event_pool_reuses_slots_without_global_lock_contract`
- `component_locality_plan_preserves_dense_order_and_node_hint`
- `zero_copy_layout_validation_rejects_misaligned_or_unowned_buffers`

## Known risks

No NUMA hardware, `hwloc` topology discovery, OS affinity mutation, production
arena allocator, Miri run, or contention benchmark proof exists in this slice.
The feature-gated contracts are deliberately dependency-free and fallback-safe.

## Follow-up issues

- Add `hwloc` integration behind `numa` with typed unsupported errors.
- Add real, reversible OS affinity binding where supported.
- Replace the event-pool contract with production arena/pool allocator paths.
- Add Miri/concurrency stress when the local toolchain supports it.
- Add NUMA host and allocator contention evidence manifests for Track 55.

## Integration notes

Track 52 consumes allocator and memory layout constraints for persistent device
buffers.

## Phase closeout evidence

Red step captured with:

- `CARGO_INCREMENTAL=0 rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-core -p kairo-ecs-state --features numa`
- `CARGO_INCREMENTAL=0 rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-ffi --features numa`

Both failed before implementation because the selected crates did not expose a
`numa` feature.

Passing implementation gates:

- `rustup run stable-x86_64-pc-windows-gnu cargo fmt -p kairo-ecs-core -p kairo-ecs-state -p kairo-ecs-ffi`
- `CARGO_INCREMENTAL=0 rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-core -p kairo-ecs-state --features numa`
- `CARGO_INCREMENTAL=0 rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-ffi --features numa`
- `CARGO_INCREMENTAL=0 rustup run stable-x86_64-pc-windows-gnu cargo clippy -p kairo-ecs-core -p kairo-ecs-state -p kairo-ecs-ffi --all-targets --all-features -- -D warnings`
- `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate_conductor_phase_gates.ps1`
- `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate_conductor_dag.ps1`

Run `$conductor-review`, record accepted fixes, commit SHA, pushed ref,
`validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`, and the
next-phase decision before advancing beyond this contract-baseline slice.
