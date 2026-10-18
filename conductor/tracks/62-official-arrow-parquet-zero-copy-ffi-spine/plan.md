# Track 62 Plan: Official Apache Arrow, Parquet & Zero-Copy C Data Interface Spine

## Phase 0 - Official Dependency & Security Admission

- [ ] Task 0.1: Audit and admit `arrow-schema`, `arrow-array`, `arrow-buffer`, `arrow-ipc`, `arrow-ffi`, and `parquet` in `deny.toml` and `Cargo.toml` behind optional `arrow-native` and `parquet-native` features while preserving default-off lightweight smoke compatibility.
- [ ] Task 0.2: Add red/green schema parity tests verifying that `EVENT_LOG_FIELDS` and ECS component snapshots map identically to `arrow_schema::Schema`.
## Phase 1 - Native Arrow RecordBatch & Parquet Checkpoints

- [ ] Task 1.1: Implement `RecordBatch` builders for `EventLogRow`, `WorldSnapshot`, and numeric ECS component columns in `kairo-ecs-arrow`.
- [ ] Task 1.2: Implement real Apache Arrow IPC stream/file and Zstd-compressed Parquet checkpoint writers and readers with SHA-256 footer metadata verification.
- [ ] Task 1.3: Add cross-reader roundtrip tests validating generated `.parquet` and `.arrow` files.
## Phase 2 - Zero-Copy Arrow C Data Interface (`arrow-ffi`)

- [ ] Task 2.1: Export `FFI_ArrowArray` and `FFI_ArrowSchema` C ABI export functions in `kairo-ecs-ffi` and `include/kairo_ecs.h` with deterministic release callbacks.
- [ ] Task 2.2: Expose `__arrow_c_array__` and `__arrow_c_stream__` PyCapsule protocols for zero-copy Polars (`pl.from_arrow`) and DuckDB SQL queries over live simulation state.
- [ ] Task 2.3: Document zero-copy ownership, alignment, and lifetime boundaries in `docs/parallel-io/`.

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
