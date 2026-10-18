# Track 66 Plan: Parallel Ensemble Sweeps (Sobol/CRN), SBI Calibration & Copy-on-Write Wasm Studio

## Phase 0 - Quasi-Random Ensemble Runner (Sobol, LHS & Common Random Numbers)

- [ ] Task 0.1: Add Sobol sequence and Latin Hypercube Sampling (LHS) parameter generators plus Common Random Numbers (CRN) substream synchronization across policy counterfactuals in `kairo-ecs-cli`.
- [ ] Task 0.2: Compute automated Sobol first-order/total sensitivity indices ($S_i, S_{Ti}$) and bootstrap confidence intervals over Arrow/Parquet ensemble outputs.
## Phase 1 - Optuna & Approximate Bayesian Computation (ABC-SMC) Calibration Bridge

- [ ] Task 1.1: Implement a lossless batch objective evaluation hook for Optuna (TPE/CMA-ES) and Approximate Bayesian Computation (ABC-SMC) empirical time-series fitting.
- [ ] Task 1.2: Add a reproducible calibration study example with posterior diagnostics.
## Phase 2 - Copy-on-Write Delta Snapshots & Live Wasm Time-Travel Studio

- [ ] Task 2.1: Upgrade `kairo-ecs-debug` from full-world clones to page-level Copy-on-Write (CoW) delta snapshots and causal event provenance graphs.
- [ ] Task 2.2: Wire the compiled `kairo-ecs-wasm` time-travel debugger into `website/playground` with interactive timeline scrubbing, entity/component inspector tables, and breakpoint expressions.

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
