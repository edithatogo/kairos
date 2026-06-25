# Track 50: NUMA Topology, Affinity & HPC Memory Lifecycle

## Purpose

Add NUMA-aware topology discovery, CPU affinity, cache-local arenas and pools,
event lifecycle allocation, and zero-copy FFI layout checks without global lock
contention.

## Maturity

Spec Approved planning track. No hwloc, affinity, NUMA allocator, or zero-copy
runtime implementation is claimed by this artifact.

## Inputs

- Core scheduler and ECS storage from Track 01.
- FFI layout contracts from Track 02.
- HPC evidence manifest from Track 46.

## Outputs

- `numa` feature-gated topology discovery backed by `hwloc`.
- Thread/core affinity controls with safe fallbacks.
- Event arena and pool allocators for simulation lifecycle reuse.
- Zero-copy pointer layout documentation and tests for FFI bindings.
- NUMA locality and allocator contention evidence.

## Owned paths

- `crates/kairo-ecs-core/`
- `crates/kairo-ecs-state/`
- `crates/kairo-ecs-ffi/`
- `docs/performance/`
- `conductor/tracks/50-numa-topology-affinity-memory-lifecycle/`

## Blocked paths

- Binding package APIs without Track 06-11 handoff.
- GPU memory layout owned by Track 52.
- Distributed launch topology owned by Track 54.

## Dependencies

Tracks 01, 02, and 46.

## Parallel-safe tracks

Track 52 may consume allocator and layout contracts after they are reviewed.
Track 55 may consume topology metadata for scaling manifests.

## Acceptance criteria

- Topology discovery reports NUMA nodes, cores, caches, and memory locality on
  supported hosts and returns typed unsupported errors elsewhere.
- Affinity binding is opt-in, observable, and reversible.
- Event lifecycle allocations use arenas or pools without global lock
  contention on hot paths.
- FFI zero-copy layouts are validated for alignment, lifetime, and ownership.
- Data-race tests and concurrency reviews cover shared allocator paths.

## Quality gates

- `numa-topology-discovery`
- `core-affinity-binding`
- `event-arena-lifecycle`
- `zero-copy-ffi-layout`
- `allocator-contention-budget`
- `phase-closeout-check`

## Release implications

This track is release-critical for NUMA or zero-copy HPC memory claims. It must
preserve existing safe defaults on non-NUMA hosts.
