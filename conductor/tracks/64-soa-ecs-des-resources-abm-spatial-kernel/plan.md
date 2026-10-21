# Track 64 Plan: Archetype/SoA ECS Storage, Full DES Process/Resource Kernel & ABM Spatial Topologies

## Phase 0 - 64-Byte Aligned SoA Columnar ECS & Multi-Component Queries

- [x] Task 0.1: Upgrade `ComponentRegistry` in `kairo-ecs-state` from `HashMap<EntityId, T>` to dense sparse-set / archetype SoA columns aligned to 64-byte cache lines (directly compatible with Arrow buffers and SIMD).
- [x] Task 0.2: Implement ergonomic typed multi-component queries (`Query<(&A, &mut B)>`, `With<C>`, `Without<D>`) with deterministic iteration order.
## Phase 1 - SimPy/AnyLogic-Grade DES Resource, Store & Process Kernel

- [x] Task 1.1: Implement `CapacityResource` (FIFO and priority queues), `PreemptiveResource` (interrupts, remaining-work tracking), `Container` (continuous/discrete bulk levels), and `Store` (typed item queues) in `kairo-ecs-des`.
- [x] Task 1.2: Implement wait conditions (`AllOf`, `AnyOf`), balking, reneging (timeout deadlines), and zero-Tokio deterministic `async`/`await` process coroutines driven by `Scheduler` virtual time.
- [x] Task 1.3: Verify golden trace parity against SimPy reference fixtures (`tests/conformance/queue_simpy_v1.py`).
## Phase 2 - Agents.jl/Mesa-Grade ABM Spatial & Network Environments

- [x] Task 2.1: Implement `GridSpace2D` and `GridSpace3D` (Moore/Von Neumann neighborhoods, toroidal/bounded topologies, single/multi-occupancy) in `kairo-ecs-abm`.
- [x] Task 2.2: Implement `ContinuousSpace2D` and `ContinuousSpace3D` with flat cell-list spatial hashing and k-d tree radius queries (`neighbors_within`).
- [x] Task 2.3: Implement `GraphSpace` integrating Track 59 `graph-relations` with static CSR adjacency views for fast contagion/diffusion workloads.

## Phase 3 - Closeout

- [x] Task 3.1: Run track-specific test matrix, workspace tests, and Conductor validators.
- [x] Task 3.2: Run `$conductor-review`, auto-apply accepted fixes, and update `handoff.md`.
- [x] Task 3.3: Commit and push the cleaned slice and verify GitHub Actions.

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

- [ ] CAP-64.1: Separate event joining from resource granting: implement and verify AllOf/AnyOf wait semantics independently of atomic bundle allocation. Preserve careops-sim Q/C/Flow compatibility before consumer migration.
- [ ] CAP-64.2: Require contention, timeout-at-grant ties, cancellation, expiry, partial failure, rollback and stale-completion tests with resource conservation after every event. Keep safe clinical interruption policy in careops-sim; acquire-all versus staged approximation must be explicit.

Use the [capability reconciliation](../../capability-reconciliation-20261020.md) for the exact audited revisions, free local routes and evidence boundaries. Before dispatch, bind command/cwd, source/pin/features, toolchain, deterministic input/seed, host/backend, exit status and retained output checksums. A structural validator pass cannot close a runtime task.

## Proposed careops-sim composition follow-up — 20 October 2026

[Pending owner worklist](../../design/careops-composition-plan-20261020.md)
records neutral relationship, allocation, transfer, progression, runner and
observation requirements. Only rows naming this track are its prospective work;
other owner rows require handoff. Existing accepted tasks and receipts retain
their scope. Bind an exact-source packet and reconcile the owner registry/phase
before implementation; this planning link does not reopen or complete a phase.
