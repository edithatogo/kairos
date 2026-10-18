# Track 64 Plan: Archetype/SoA ECS Storage, Full DES Process/Resource Kernel & ABM Spatial Topologies

## Phase 0 - 64-Byte Aligned SoA Columnar ECS & Multi-Component Queries

- [ ] Task 0.1: Upgrade `ComponentRegistry` in `kairo-ecs-state` from `HashMap<EntityId, T>` to dense sparse-set / archetype SoA columns aligned to 64-byte cache lines (directly compatible with Arrow buffers and SIMD).
- [ ] Task 0.2: Implement ergonomic typed multi-component queries (`Query<(&A, &mut B)>`, `With<C>`, `Without<D>`) with deterministic iteration order.
## Phase 1 - SimPy/AnyLogic-Grade DES Resource, Store & Process Kernel

- [ ] Task 1.1: Implement `CapacityResource` (FIFO and priority queues), `PreemptiveResource` (interrupts, remaining-work tracking), `Container` (continuous/discrete bulk levels), and `Store` (typed item queues) in `kairo-ecs-des`.
- [ ] Task 1.2: Implement wait conditions (`AllOf`, `AnyOf`), balking, reneging (timeout deadlines), and zero-Tokio deterministic `async`/`await` process coroutines driven by `Scheduler` virtual time.
- [ ] Task 1.3: Verify golden trace parity against SimPy reference fixtures (`tests/conformance/queue_simpy_v1.py`).
## Phase 2 - Agents.jl/Mesa-Grade ABM Spatial & Network Environments

- [ ] Task 2.1: Implement `GridSpace2D` and `GridSpace3D` (Moore/Von Neumann neighborhoods, toroidal/bounded topologies, single/multi-occupancy) in `kairo-ecs-abm`.
- [ ] Task 2.2: Implement `ContinuousSpace2D` and `ContinuousSpace3D` with flat cell-list spatial hashing and k-d tree radius queries (`neighbors_within`).
- [ ] Task 2.3: Implement `GraphSpace` integrating Track 59 `graph-relations` with static CSR adjacency views for fast contagion/diffusion workloads.

## Phase 3 - Closeout

- [ ] Task 3.1: Run track-specific test matrix, workspace tests, and Conductor validators.
- [ ] Task 3.2: Run `$conductor-review`, auto-apply accepted fixes, and update `handoff.md`.
- [ ] Task 3.3: Commit and push the cleaned slice and verify GitHub Actions.

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
