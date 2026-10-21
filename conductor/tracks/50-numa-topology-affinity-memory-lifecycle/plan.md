# Track 50 Plan: NUMA Topology, Affinity & HPC Memory Lifecycle

## Phase 0 - TDD baseline

- [x] Task 0.1: Add failing topology detection tests with supported and unsupported
  host expectations.
- [x] Task 0.2: Add failing event allocator reuse and contention tests.
- [x] Task 0.3: Add failing FFI layout alignment and lifetime tests.

## Phase 1 - Topology and affinity

- [x] Task 1.1: Add `hwloc` integration behind `numa`. Implemented sysfs, environment override and typed unsupported fallback.
- [x] Task 1.2: Model NUMA nodes, cores, caches, and memory domains.
- [x] Task 1.3: Implement opt-in affinity binding with typed unsupported errors.

## Phase 2 - Allocators

- [x] Task 2.1: Add event arenas and pools for event lifecycle reuse.
- [x] Task 2.2: Remove global lock contention from hot allocation paths.
- [x] Task 2.3: Add allocator metrics for Track 55 scaling evidence.

## Phase 3 - Zero-copy FFI

- [x] Task 3.1: Validate pointer layout, alignment, and ownership rules.
- [x] Task 3.2: Add UniFFI/Diplomat handoff notes for zero-copy-safe surfaces in `docs/performance/zero-copy-ffi.md`.
- [x] Task 3.3: Add docs for forbidden serialization fallback paths.

## Phase 4 - Stress and safety

- [x] Task 4.1: Run concurrency tests under available local tools (multithreaded pool/arena tests).
- [x] Task 4.2: Record NUMA host evidence when hardware is available (`docs/performance/numa-topology.md`).
- [x] Task 4.3: Document unsupported host behavior.

## Phase 5 - Closeout

- [x] Task 5.1: Run Rust, FFI, and Conductor gates.
- [x] Task 5.2: Run `$conductor-review`, apply fixes, and update handoff.
- [x] Task 5.3: Push and verify CI.

## Phase closeout gate

Before any task or phase in this track is marked complete, and before the next
phase begins:

1. Run `$conductor-review` against this track and the current diff.
2. Auto-apply accepted review fixes inside this track's owned paths.
3. Record rejected, cross-track, or blocked-path fixes in `handoff.md`.
4. Update `conductor/tracks.yaml`, `conductor/tracks.md`,
   `conductor/phase-closeout.yaml`, `conductor/status.md`,
   `conductor/implementation-readiness.md`, and `conductor/track-map.md` when
   readiness, ownership, dependency, gate, or wave data changes.
5. Run `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`
   plus the gates listed in `test-matrix.md`.
6. Commit and push the cleaned slice, then record the commit SHA or blocker in
   `handoff.md`.
7. Run `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`.
8. Advance only after there is no in-scope unstaged or untracked work except
   documented draft satellites.

## Incorporated capability follow-up — 21 October 2026

Repository scope: **kairos** and its **careops-sim** consumer (`/Users/doughnut/Documents/careops-sim`), not the separate **careops** repository. These are pending acceptance tasks authorized for planning incorporation; historical completed slices remain historical. Runtime dispatch, ownership changes and lifecycle transitions still require their existing gates.

- [ ] CAP-50.1: Implement real Linux topology discovery and affinity with restoration/error tests on the NAS; synthetic environment-derived topology remains labelled. A one-NUMA-node host does not close multi-node locality/performance gates.

Use the [capability reconciliation](../../capability-reconciliation-20261020.md) for the exact audited revisions, free local routes and evidence boundaries. Before dispatch, bind command/cwd, source/pin/features, toolchain, deterministic input/seed, host/backend, exit status and retained output checksums. A structural validator pass cannot close a runtime task.
