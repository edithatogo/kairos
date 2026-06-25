# Track 53 Plan: FMI 2/3 Co-Simulation Master Runtime

## Phase 0 - TDD baseline

- [x] Task 0.1: Add failing `.fmu` archive extraction and malformed archive
  tests.
  - Red: `rustup run stable-x86_64-pc-windows-gnu cargo test -p
    kairo-ecs-fmi --features fmi-runtime,fmi2,fmi3` failed because the
    `fmi-runtime` feature did not exist.
  - Green: same command passed after adding `fmi-runtime`,
    `FmuArchive::extract_to`, stored-entry archive extraction, traversal
    rejection, and archive layout validation tests.
- Task 0.2: Add failing dynamic symbol binding tests with fixture FMUs.
- Task 0.3: Add failing FMI 2 and FMI 3 1,000-step co-simulation tests.

## Phase 1 - FMU archive handling

- [x] Task 1.1: Add zip extraction with path traversal protections.
  - Contract baseline supports dependency-free stored ZIP entries and rejects
    absolute paths, parent traversal, data descriptors, and unsupported
    compression methods.
- [x] Task 1.2: Validate `modelDescription.xml` and platform binary layout.
  - Extracted archives are immediately passed through the existing
    `FmuLayout::from_unpacked_dir` validator.
- [x] Task 1.3: Preserve unpacked-directory import for local fixtures.
  - `FmuArchive::open_unpacked` remains unchanged and covered by existing unit
    tests.

## Phase 2 - Dynamic loading

- Task 2.1: Add dynamic library loading behind `fmi-runtime`.
- [x] Task 2.2: Bind FMI 2 lifecycle functions safely.
  - Red: `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-fmi --features fmi-runtime,fmi2 --test fmi2_lifecycle` failed with missing `Fmi2LifecycleState`, checked raw-parts constructor, and null-component error.
  - Green: same command passed after adding explicit lifecycle state tracking, a null-rejecting unsafe checked constructor, idempotent terminate/free cleanup coverage, and safety docs.
- Task 2.3: Bind FMI 3 lifecycle functions safely.

## Phase 3 - Co-simulation master

- Task 3.1: Implement instantiate, setup, step, terminate, and free lifecycle.
- Task 3.2: Couple FMU time steps to KairoECS scheduler ticks.
- Task 3.3: Add error propagation and resource cleanup tests.

## Phase 4 - Third-party roundtrip

- Task 4.1: Package exported FMUs as valid zip artifacts.
- Task 4.2: Validate with OpenModelica or equivalent tool.
- Task 4.3: Handoff integrated scenario metrics to Track 55.

## Phase 5 - Closeout

- Task 5.1: Run FMI feature tests, third-party roundtrip, and Conductor gates.
- Task 5.2: Run `$conductor-review` and apply accepted fixes.
- Task 5.3: Push and verify GitHub Actions.

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
