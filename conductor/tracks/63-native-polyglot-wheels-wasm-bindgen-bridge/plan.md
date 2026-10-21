# Track 63 Plan: Native Polyglot Bindings (PyO3/Maturin, Wasm-Bindgen, Extendr & C ABI)

## Phase 0 - Native Extension Build & CI Wiring

- [x] Task 0.1: Add `PyO3` / `maturin` native extension module (`kairo_ecs._native`) wrapping `kairo-ecs-core`, `kairo-ecs-state`, `kairo-ecs-des`, `kairo-ecs-abm`, and `kairo-ecs-arrow`.
- [x] Task 0.2: Wire `wasm-bindgen` compilation (`wasm32-unknown-unknown`) in `crates/kairo-ecs-wasm` and load the compiled `.wasm` artifact directly in `bindings/typescript`.
## Phase 1 - C ABI Dynamic Library Wiring for Julia, R, C# & Go

- [x] Task 1.1: Wire Julia `ccall`, R `.Call` / `extendr`, C# `LibraryImport` (`NativeBindingStatus.Available`), and Go `cgo` / purego bindings to `libkairo_ecs_ffi`.
- [x] Task 1.2: Make native Rust execution the default in all six bindings and require explicit opt-in if a reference pure-language fallback is used.
## Phase 2 - Cross-Language Native Conformance & Benchmarks

- [x] Task 2.1: Run the full conformance fixture suite (`scheduler_ordering_v1`, `scheduler_cancellation_v1`, `rng_reproducibility_v1`, `vvuq_scenario_replay_v1`) through the compiled native extensions across all six languages.
- [x] Task 2.2: Benchmark Python/Wasm/Julia native call overhead and Arrow batch transfer throughput.

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

- [ ] CAP-63.1: Exercise native scheduler operations and deterministic traces from each claimed language consumer, including install/load/error/lifecycle behavior; a ctypes version probe does not qualify a native scheduler. Qualify browser and Node Wasm execution separately.

Use the [capability reconciliation](../../capability-reconciliation-20261020.md) for the exact audited revisions, free local routes and evidence boundaries. Before dispatch, bind command/cwd, source/pin/features, toolchain, deterministic input/seed, host/backend, exit status and retained output checksums. A structural validator pass cannot close a runtime task.
