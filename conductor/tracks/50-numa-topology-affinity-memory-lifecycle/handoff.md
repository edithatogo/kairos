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

## Final Phase 5 Closeout Evidence — 17 October 2026

- `$conductor-review`: Full review report recorded in `review-report.md`.
- accepted fixes: Added `NumaTopology::discover()` with sysfs and `KAIRO_NUMA_NODES` support, reversible opt-in affinity execution, chunked lifecycle arena `EventArena<T>`, and multithreaded contention verification.
- commit SHA: `b0c980c` (prior wave head)
- pushed ref: `origin/codex/track49-distributed-synchronization`
- `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: Enforced and verified after commit.
- next-phase decision: Track 50 implementation complete; allocator and memory layout contracts ready for consumption by Track 52 (GPU Persistent Memory) and Track 55 (End-to-End Scaling Certification).

## Linux topology, affinity and arena reuse — 1 November 2026

Chat 01a19600-0a41-7ca2-9e4b-e27d7339c8e7 owns the bounded Track 50 packet in
/Users/doughnut/.codex/worktrees/track48-rollback/kairos on codex/track50-linux-affinity,
base 956d8399339b8589bbb5bbdea28f4cc5539588a9. Paths are the core library,
Linux integration tests, NUMA guide and this track directory. Native scope was
temporary unprivileged test processes on the host-key-verified Pleiades NAS.
Next operation is protected PR integration.

The baseline returned Unsupported on the NAS despite its populated Linux sysfs,
and the old arena allocated a third chunk when two retained chunks could hold
four items after reset. Both failures were reproduced. Discovery now reads
per-node CPU lists and memory totals. Explicit opt-in binding saves the calling
thread's allowed mask, verifies the target through procfs, and requires exact
restoration. The synthetic environment override remains labelled. Independent
review identified missing cross-thread error coverage, sparse/memory-only node
fixtures and cleanup guidance; all three were addressed.

The Mac used Rust/Cargo 1.99.1 to build a locked x86_64-unknown-linux-musl
core test binary against an isolated target standard library. Its SHA-256
was verified against the retained Rust toolchain manifest before use. On the
Linux x86-64 NAS, node 0 exposed logical CPUs 0–7 and 33,391,509,504 bytes.
The real calling thread bound to CPU 0 and restored mask 0–7. The negative
controls covered invalid CPU, missing opt-in, nested bind, wrong reversal,
foreign thread and double restore. No global affinity was changed.

Commands from the worktree root included `cargo test --locked --offline
-p kairo-ecs-core -p kairo-ecs-state -p kairo-ecs-ffi --features numa`
(152 passed on macOS), the corresponding all-target Clippy gate, Linux-target
core Clippy, and `cargo test --locked --offline --target
x86_64-unknown-linux-musl -p kairo-ecs-core --features numa --lib --test
linux_numa --no-run` (selectors built separately). The two Linux binaries ran
on the NAS: 33 library cases and two kernel integration cases passed. All
final commands exited zero. Fixed inputs were the actual kernel masks and
node memory rows plus the sparse-node and arena reset fixtures.

Raw receipts in /Volumes/PortableSSD/codex-builds/kairos-track50-linux:

- `red.log` — 58f171fda399afa0d915303d7cfa8a8a85d4cc2deb76af474a7f86ddaa54b77f
- `nas-final.log` — fa5aa485667966354a6b7ffc97b9f5c85ab9b68fce55030b64b9e406fae48311
- `nas-integration.log` — ea97d4b7031b9662910257c76f79216c9153f1999dafdd6b901f7aed58bfd54b
- `native-final.log` — 7632147699338a93397c01ac82cabff497715e59f800c5e11ae13df50384087a
- `clippy.log` — 5dca387a3ce6033acc5f0b127912a41ab3acf34380df2426f91b5273f65f0203
- `linux-clippy.log` — 9bfbdda71e4660ce02f72d05e13c05e1ff588b7fe3bdf03664dae7e04f02febd

Multi-node locality and performance, broader cache/hwloc reporting and Track 55 scaling acceptance remain separate.
