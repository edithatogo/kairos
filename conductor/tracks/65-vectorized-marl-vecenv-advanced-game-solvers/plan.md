# Track 65 Plan: Vectorized Multi-World RL (`VecEnv`), PettingZoo Bridge & Advanced Game Theory Solvers

## Phase 0 - Mixed-Strategy Nash, Correlated Equilibrium & Evolutionary Dynamics

- [ ] Task 0.1: Implement 2-player Mixed-Strategy Nash Equilibrium solvers (support enumeration and Lemke-Howson complementary pivoting) in `kairo-ecs-game-theory`.
- [ ] Task 0.2: Implement LP-based Correlated Equilibrium and continuous/discrete-time Replicator Dynamics over ECS strategy populations.
## Phase 1 - Counterfactual Regret Minimization (`CFR` / `CFR+`) for Extensive-Form Games

- [ ] Task 1.1: Implement tabular `CFR` and `CFR+` over Track 61 extensive-form Graph-ECS information sets with chance-node sampling.
- [ ] Task 1.2: Verify exploitability convergence on Kuhn Poker and Leduc Poker reference fixtures.
## Phase 2 - Rayon Vectorized Batch Environment (`VecEnv`) & PettingZoo/ONNX Integration

- [ ] Task 2.1: Implement `ParallelVecEnv` in `kairo-ecs-ml` using `rayon` to step $N$ seeded simulation worlds concurrently and write observations/rewards directly into contiguous pre-allocated `f32`/`i64` slices.
- [ ] Task 2.2: Expose Gymnasium `VectorEnv` and PettingZoo `ParallelEnv` / `AECEnv` adapters in Python plus optional in-loop `ort` (ONNX Runtime) policy inference.

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
