# Review Report: Track 50 NUMA Topology, Affinity & HPC Memory Lifecycle

## Review of the native qualification

The independent reviewer checked the scheduler reservation at `c44a17b`
and the hwloc and native validation packet at `6569d406`. Both APIs are
additive. The review verified replay through cancellation, checkpoint and
ordered dispatch, the allocator budget boundaries and the actual exported
C tick buffer ownership.

The parser now accepts a missing hwloc memory-size attribute and rejects a
malformed value. The allocation runner checks the event generation along
with the index and bounds the worker start gate. The saved CSV schema
includes the trial field. The reviewer recomputed the 640 worker samples,
matched all 29 Linux topology records and checked the loaded C library
against the reproduced consumer artifact. The three bounded capabilities
passed review. The cross-node performance gate remains open.
[Native receipts](evidence/native-qualification/README.md).

## API review

| Surface | Change | Compatibility |
| --- | --- | --- |
| Rust `kairo-ecs-core` | `Scheduler::with_capacity(usize)` adds initial event storage reservation. | Existing construction and ordering remain available. The reservation is opt-in and consumes space for initial queue and cancellation storage. |
| Rust `kairo-ecs-core/numa` | `HwlocTopology::discover(&Path)` adds explicit native machine discovery. | The existing `NumaTopology` API remains available. Callers supply a trusted absolute executable path. |
| C ABI and Arrow schema | Consumer qualification exercises the existing `native-data` stream and release callbacks. | The published header and schema are reused. |
| Host language | Installed PyArrow imports the C Data batch. | The consumer owns the import until the last view drops. |

Each scheduler owns its queue. Provisioning capacity changes allocation
timing and memory use while keeping event identity, cancellation,
checkpoints and deterministic dispatch. The hwloc query uses a clean
child environment and reports process or parse failures as I/O errors.
The measured allocator and native topology checks qualify the explicit
paths. Broader placement requires the CAP-50.5 evidence.
The public API review records the decision, compatibility and red-team
responses in [ADR 0028](../../../docs/adr/0028-opt-in-scheduler-reservation-and-hwloc.md).
It binds the review to the
[scheduler reservation fixture](../../../conformance/fixtures/track50_scheduler_reservation.json)
and [native topology fixture](../../../conformance/fixtures/track50-hwloc-v2.xml).

> **Current qualification correction, 3 November 2026:** This historical
> review covers contract slices. Its complete-plan, cache locality, zero
> contention and FFI lifetime claims are not established by those tests. The
> environment override is synthetic, not an observed cgroup topology. See the
> current plan and handoff for the outstanding native acceptance work.

## Summary

Track 50 implements NUMA topology discovery, CPU core affinity management, lifecycle event arenas and pools, and FFI zero-copy buffer validation contracts for KairoECS:
1. `kairo-ecs-core`:
   - Feature-gated `numa` topology discovery (`NumaTopology`, `NumaNode`, `NumaSupport`, `NumaError`) supporting Linux sysfs, HPC container cgroup overrides (`KAIRO_NUMA_NODES`), and typed unsupported fallback.
   - Opt-in and reversible affinity binding (`AffinityBindingPlan`, `AffinityAction`).
   - Chunked lifecycle event arena (`EventArena`) and object pool (`EventPool`) with zero global lock contention (`uses_global_lock == false`).
   - Scalable telemetry metrics (`EventPoolMetrics`, `EventArenaMetrics`) exported for Track 55 scaling evidence.
2. `kairo-ecs-ffi`:
   - FFI zero-copy layout validation (`KairoEcsZeroCopyLayout`, `KairoEcsOwnership`, `KairoEcsLayoutStatus`) checking pointer address non-nullity, non-empty bounds, alignment multiples, and explicit ownership.
3. Strict safety: `#![forbid(unsafe_code)]` preserved workspace-wide.

## Verification Checks

- [x] **Plan Compliance**: Tasks 0.1 through 5.3 completed.
- [x] **Safety Invariant**: `#![forbid(unsafe_code)]` preserved across all owned crates (`kairo-ecs-core`, `kairo-ecs-state`, `kairo-ecs-ffi`).
- [x] **Concurrency & Lock Contention**: Verified concurrent thread safety of `EventPool` and `EventArena` across worker threads with zero mutex contention.
- [x] **Documentation**: Complete performance guides published in `docs/performance/numa-topology.md`, `docs/performance/allocator-lifecycle.md`, and `docs/performance/zero-copy-ffi.md`.

## Gate Receipts

- `cargo test -p kairo-ecs-core -p kairo-ecs-state --features numa`: PASS (34 tests passed, 0.02s)
- `cargo test -p kairo-ecs-ffi --features numa`: PASS (20 tests passed, 0.00s)
- `cargo fmt --all --check`: PASS
- `cargo clippy -p kairo-ecs-core -p kairo-ecs-state -p kairo-ecs-ffi --all-targets --all-features -- -D warnings`: PASS (0 warnings)
- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`: PASS (0 errors, 0 warnings)
- `pwsh -NoProfile -File scripts/validate_conductor_dag.ps1`: PASS (0 errors, 0 warnings)
- `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1`: PASS (0 errors, 0 warnings)
- `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: PASS
