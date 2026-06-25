# Track 50 Plan: NUMA Topology, Affinity & HPC Memory Lifecycle

## Phase 0 - TDD baseline

- [x] Task 0.1: Add failing topology detection tests with supported and unsupported
  host expectations.
- [x] Task 0.2: Add failing event allocator reuse and contention tests.
- [x] Task 0.3: Add failing FFI layout alignment and lifetime tests.

## Phase 1 - Topology and affinity

- Task 1.1: Add `hwloc` integration behind `numa`.
- [x] Task 1.2: Model NUMA nodes, cores, caches, and memory domains.
- Task 1.3: Implement opt-in affinity binding with typed unsupported errors.

## Phase 2 - Allocators

- [x] Task 2.1: Add event arenas and pools for event lifecycle reuse.
- Task 2.2: Remove global lock contention from hot allocation paths.
- Task 2.3: Add allocator metrics for Track 55 scaling evidence.

## Phase 3 - Zero-copy FFI

- Task 3.1: Validate pointer layout, alignment, and ownership rules.
- Task 3.2: Add UniFFI/Diplomat handoff notes for zero-copy-safe surfaces.
- Task 3.3: Add docs for forbidden serialization fallback paths.

## Phase 4 - Stress and safety

- Task 4.1: Run concurrency tests under available local tools.
- Task 4.2: Record NUMA host evidence when hardware is available.
- Task 4.3: Document unsupported host behavior.

## Phase 5 - Closeout

- Task 5.1: Run Rust, FFI, and Conductor gates.
- Task 5.2: Run `$conductor-review`, apply fixes, and update handoff.
- Task 5.3: Push and verify CI.

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
